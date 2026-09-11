//! Daily resurfacing picker (issue #24).

use crate::db::{Database, ResurfaceItem};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// How many captures to pick for a day.
pub const DEFAULT_TODAY_LIMIT: usize = 5;

/// Minimum age in whole days before a capture is eligible.
pub const DEFAULT_MIN_AGE_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodayResurfacing {
    pub day: String,
    pub items: Vec<ResurfaceItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResurfaceError {
    Database(String),
    InvalidClock(String),
}

impl std::fmt::Display for ResurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResurfaceError::Database(e) => write!(f, "database error: {e}"),
            ResurfaceError::InvalidClock(e) => write!(f, "invalid clock: {e}"),
        }
    }
}

impl std::error::Error for ResurfaceError {}

/// Parse `YYYY-MM-DD` (or longer ISO prefix) into day string.
pub fn utc_day_from_iso(iso: &str) -> Result<String, ResurfaceError> {
    let day = iso.trim().chars().take(10).collect::<String>();
    if day.len() != 10 || day.as_bytes().get(4) != Some(&b'-') || day.as_bytes().get(7) != Some(&b'-')
    {
        return Err(ResurfaceError::InvalidClock(format!(
            "expected YYYY-MM-DD prefix, got {iso}"
        )));
    }
    Ok(day)
}

/// Subtract whole days from a `YYYY-MM-DD` (simple civil arithmetic; good enough for tests/v1).
pub fn subtract_days(day: &str, days: i64) -> Result<String, ResurfaceError> {
    let parts: Vec<_> = day.split('-').collect();
    if parts.len() != 3 {
        return Err(ResurfaceError::InvalidClock(day.into()));
    }
    let y: i32 = parts[0]
        .parse()
        .map_err(|_| ResurfaceError::InvalidClock(day.into()))?;
    let m: u32 = parts[1]
        .parse()
        .map_err(|_| ResurfaceError::InvalidClock(day.into()))?;
    let d: u32 = parts[2]
        .parse()
        .map_err(|_| ResurfaceError::InvalidClock(day.into()))?;
    let mut ymd = (y, m, d);
    for _ in 0..days {
        ymd = prev_day(ymd);
    }
    Ok(format!("{:04}-{:02}-{:02}", ymd.0, ymd.1, ymd.2))
}

fn prev_day(ymd: (i32, u32, u32)) -> (i32, u32, u32) {
    let (y, m, d) = ymd;
    if d > 1 {
        return (y, m, d - 1);
    }
    let (py, pm) = if m > 1 { (y, m - 1) } else { (y - 1, 12) };
    let pd = days_in_month(py, pm);
    (py, pm, pd)
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    content_type: String,
}

/// Diversity-aware deterministic pick.
///
/// Rule: bucket by `content_type` (or `untyped`), sort ids within each bucket,
/// then round-robin across buckets in bucket-name order. Tie-break with
/// `day|id` lexicographic order for stability.
fn pick_candidates(day: &str, candidates: &[Candidate], limit: usize) -> Vec<String> {
    if limit == 0 || candidates.is_empty() {
        return vec![];
    }
    let mut buckets: HashMap<String, Vec<&Candidate>> = HashMap::new();
    for c in candidates {
        buckets
            .entry(c.content_type.clone())
            .or_default()
            .push(c);
    }
    for list in buckets.values_mut() {
        list.sort_by(|a, b| {
            let ka = format!("{day}|{}", a.id);
            let kb = format!("{day}|{}", b.id);
            ka.cmp(&kb)
        });
    }
    let mut keys: Vec<String> = buckets.keys().cloned().collect();
    keys.sort();
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut picked = Vec::new();
    let mut seen = HashSet::new();
    while picked.len() < limit {
        let mut progressed = false;
        for k in &keys {
            let list = buckets.get(k).unwrap();
            let i = *indices.get(k).unwrap_or(&0);
            if i < list.len() {
                let id = list[i].id.clone();
                indices.insert(k.clone(), i + 1);
                if seen.insert(id.clone()) {
                    picked.push(id);
                    progressed = true;
                    if picked.len() >= limit {
                        break;
                    }
                }
            }
        }
        if !progressed {
            break;
        }
    }
    picked
}

fn load_eligible(
    db: &Database,
    cutoff_iso: &str,
) -> Result<Vec<Candidate>, ResurfaceError> {
    let rows = db
        .list_captures_older_than(cutoff_iso)
        .map_err(|e| ResurfaceError::Database(e.to_string()))?;
    let mut out = Vec::new();
    for (id, _captured_at) in rows {
        let content_type = db
            .get_ai_classification(&id)
            .map_err(|e| ResurfaceError::Database(e.to_string()))?
            .and_then(|m| m.content_type)
            .unwrap_or_else(|| "untyped".into());
        out.push(Candidate {
            id,
            content_type,
        });
    }
    Ok(out)
}

/// Ensure today’s set exists; return it. Idempotent for `now_iso`’s UTC day.
pub fn ensure_today_resurfacing(
    db: &Database,
    now_iso: &str,
    limit: usize,
    min_age_days: i64,
) -> Result<TodayResurfacing, ResurfaceError> {
    let day = utc_day_from_iso(now_iso)?;
    if let Some(existing) = db
        .get_today_resurfacing(&day)
        .map_err(|e| ResurfaceError::Database(e.to_string()))?
    {
        if !existing.is_empty() {
            return Ok(TodayResurfacing {
                day,
                items: existing,
            });
        }
    }

    let cutoff_day = subtract_days(&day, min_age_days)?;
    let cutoff_iso = format!("{cutoff_day}T00:00:00Z");
    let candidates = load_eligible(db, &cutoff_iso)?;
    let ids = pick_candidates(&day, &candidates, limit);
    let picked_at = now_iso.to_string();
    db.replace_today_resurfacing(&day, &ids, &picked_at)
        .map_err(|e| ResurfaceError::Database(e.to_string()))?;
    let items = db
        .get_today_resurfacing(&day)
        .map_err(|e| ResurfaceError::Database(e.to_string()))?
        .unwrap_or_default();
    Ok(TodayResurfacing { day, items })
}

pub fn get_today_resurfacing(
    db: &Database,
    now_iso: &str,
) -> Result<TodayResurfacing, ResurfaceError> {
    let day = utc_day_from_iso(now_iso)?;
    let items = db
        .get_today_resurfacing(&day)
        .map_err(|e| ResurfaceError::Database(e.to_string()))?
        .unwrap_or_default();
    Ok(TodayResurfacing { day, items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawCapture;

    fn cap(id: &str, at: &str) -> RawCapture {
        RawCapture {
            id: id.into(),
            original_content: format!("body-{id}"),
            captured_at: at.into(),
            source_kind: Some("clipboard".into()),
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        }
    }

    #[test]
    fn subtract_and_day_parse() {
        assert_eq!(subtract_days("2026-09-11", 7).unwrap(), "2026-09-04");
        assert_eq!(utc_day_from_iso("2026-09-11T15:00:00Z").unwrap(), "2026-09-11");
    }

    #[test]
    fn pick_diversifies_types() {
        let cands = vec![
            Candidate {
                id: "t1".into(),
                content_type: "task".into(),
            },
            Candidate {
                id: "t2".into(),
                content_type: "task".into(),
            },
            Candidate {
                id: "i1".into(),
                content_type: "idea".into(),
            },
        ];
        let picked = pick_candidates("2026-09-11", &cands, 2);
        assert_eq!(picked.len(), 2);
        let types: HashSet<_> = picked
            .iter()
            .map(|id| {
                cands
                    .iter()
                    .find(|c| &c.id == id)
                    .unwrap()
                    .content_type
                    .as_str()
            })
            .collect();
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn ensure_skips_recent_and_is_idempotent() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&cap("old1", "2026-08-01T00:00:00Z"))
            .unwrap();
        db.insert_raw_capture(&cap("old2", "2026-08-02T00:00:00Z"))
            .unwrap();
        db.insert_raw_capture(&cap("new1", "2026-09-10T00:00:00Z"))
            .unwrap();
        db.upsert_classification("old1", Some("idea"), Some(0.9), "classified", "t")
            .unwrap();
        db.upsert_classification("old2", Some("task"), Some(0.9), "classified", "t")
            .unwrap();

        let now = "2026-09-11T12:00:00Z";
        let first = ensure_today_resurfacing(&db, now, 5, 7).unwrap();
        assert!(!first.items.iter().any(|i| i.id == "new1"));
        assert!(first.items.len() >= 2);
        let second = ensure_today_resurfacing(&db, now, 5, 7).unwrap();
        assert_eq!(first.items, second.items);
        assert_eq!(first.day, "2026-09-11");
    }

    #[test]
    fn get_empty_when_not_run() {
        let db = Database::open_in_memory().unwrap();
        let today = get_today_resurfacing(&db, "2026-09-11T00:00:00Z").unwrap();
        assert!(today.items.is_empty());
    }
}
