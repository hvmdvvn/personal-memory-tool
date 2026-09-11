//! Unified exact + semantic search (issue #18).

use crate::db::{CaptureSummary, Database};
use crate::semantic::{search_semantic_with_vector, SemanticHit};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchReason {
    Exact,
    Semantic,
    Both,
}

impl MatchReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchReason::Exact => "exact",
            MatchReason::Semantic => "semantic",
            MatchReason::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedHit {
    pub id: String,
    pub snippet: String,
    pub captured_at: String,
    pub source_kind: Option<String>,
    pub source_app: Option<String>,
    pub match_reason: String,
    /// Cosine score when semantic contributed; `null` for exact-only hits.
    pub score: Option<f32>,
}

/// Merge exact + semantic hit lists. Pure function for unit tests.
pub fn merge_exact_and_semantic(
    exact: &[CaptureSummary],
    semantic: &[SemanticHit],
    limit: usize,
) -> Vec<UnifiedHit> {
    if limit == 0 {
        return vec![];
    }

    let mut by_id: HashMap<String, UnifiedHit> = HashMap::new();

    for (rank, e) in exact.iter().enumerate() {
        by_id.insert(
            e.id.clone(),
            UnifiedHit {
                id: e.id.clone(),
                snippet: e.snippet.clone(),
                captured_at: e.captured_at.clone(),
                source_kind: e.source_kind.clone(),
                source_app: e.source_app.clone(),
                match_reason: MatchReason::Exact.as_str().into(),
                // Slight FTS rank proxy so exact-only can sort stably (higher better).
                score: Some(1.0 - (rank as f32) * 0.001),
            },
        );
    }

    for s in semantic {
        match by_id.get_mut(&s.id) {
            Some(existing) => {
                existing.match_reason = MatchReason::Both.as_str().into();
                existing.score = Some(s.score);
            }
            None => {
                by_id.insert(
                    s.id.clone(),
                    UnifiedHit {
                        id: s.id.clone(),
                        snippet: s.snippet.clone(),
                        captured_at: s.captured_at.clone(),
                        source_kind: s.source_kind.clone(),
                        source_app: s.source_app.clone(),
                        match_reason: MatchReason::Semantic.as_str().into(),
                        score: Some(s.score),
                    },
                );
            }
        }
    }

    let mut out: Vec<UnifiedHit> = by_id.into_values().collect();
    out.sort_by(|a, b| {
        let ra = reason_rank(&a.match_reason);
        let rb = reason_rank(&b.match_reason);
        rb.cmp(&ra).then_with(|| {
            let sa = a.score.unwrap_or(0.0);
            let sb = b.score.unwrap_or(0.0);
            sb.partial_cmp(&sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        })
    });
    out.truncate(limit);
    out
}

fn reason_rank(reason: &str) -> u8 {
    match reason {
        "both" => 3,
        "exact" => 2,
        "semantic" => 1,
        _ => 0,
    }
}

/// Run FTS + semantic (soft-fail semantic → empty), then merge.
pub fn search_unified(
    db: &Database,
    query: &str,
    limit: usize,
    base_url: &str,
) -> Result<Vec<UnifiedHit>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }

    let exact = db
        .search_exact(trimmed)
        .map_err(|e| format!("exact search: {e}"))?;

    let semantic = match crate::semantic::search_semantic(db, trimmed, limit.max(50), base_url) {
        Ok(hits) => hits,
        Err(e) => {
            eprintln!("[search_unified] semantic soft-fail: {e}");
            Vec::new()
        }
    };

    Ok(merge_exact_and_semantic(&exact, &semantic, limit))
}

/// Test/helper path: semantic from a precomputed query vector (no HTTP).
pub fn search_unified_with_vector(
    db: &Database,
    query: &str,
    query_vector: &[f32],
    limit: usize,
) -> Result<Vec<UnifiedHit>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    let exact = db
        .search_exact(trimmed)
        .map_err(|e| format!("exact search: {e}"))?;
    let semantic = search_semantic_with_vector(db, query_vector, limit.max(50))
        .map_err(|e| format!("semantic search: {e}"))?;
    Ok(merge_exact_and_semantic(&exact, &semantic, limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawCapture;

    fn summary(id: &str, snippet: &str) -> CaptureSummary {
        CaptureSummary {
            id: id.into(),
            snippet: snippet.into(),
            captured_at: "2026-09-11T12:00:00Z".into(),
            source_kind: Some("clipboard".into()),
            source_app: None,
        }
    }

    fn sem(id: &str, score: f32) -> SemanticHit {
        SemanticHit {
            id: id.into(),
            snippet: format!("sem-{id}"),
            captured_at: "2026-09-11T12:00:00Z".into(),
            source_kind: None,
            source_app: None,
            score,
        }
    }

    #[test]
    fn merge_marks_both_and_ranks() {
        let exact = vec![summary("a", "exact-a"), summary("b", "exact-b")];
        let semantic = vec![sem("b", 0.9), sem("c", 0.8)];
        let merged = merge_exact_and_semantic(&exact, &semantic, 10);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].id, "b");
        assert_eq!(merged[0].match_reason, "both");
        assert!((merged[0].score.unwrap() - 0.9).abs() < 1e-6);

        let reasons: Vec<_> = merged.iter().map(|h| h.match_reason.as_str()).collect();
        assert!(reasons.contains(&"exact"));
        assert!(reasons.contains(&"semantic"));
    }

    #[test]
    fn empty_query_and_limit_zero() {
        assert!(merge_exact_and_semantic(&[summary("a", "x")], &[sem("a", 1.0)], 0).is_empty());
    }

    #[test]
    fn unified_with_vector_overlap_fixture() {
        let db = Database::open_in_memory().unwrap();
        for (id, text) in [
            ("a", "discipline quote about hard work"),
            ("b", "chocolate cake recipe"),
            ("c", "unrelated gardening tip"),
        ] {
            db.insert_raw_capture(&RawCapture {
                id: id.into(),
                original_content: text.into(),
                captured_at: "2026-09-11T12:00:00Z".into(),
                source_kind: Some("clipboard".into()),
                source_app: None,
                source_title: None,
                source_url: None,
                source_extra: None,
                media_path: None,
            })
            .unwrap();
        }

        // a and c embedded; query close to a
        let va = [1.0f32, 0.0, 0.0];
        let vc = [0.0f32, 1.0, 0.0];
        let q = [0.99f32, 0.01, 0.0];
        let blob = |v: [f32; 3]| {
            let mut b = Vec::new();
            for x in v {
                b.extend_from_slice(&x.to_le_bytes());
            }
            b
        };
        db.upsert_embedding("a", "m", 3, &blob(va), "t", "ref:a")
            .unwrap();
        db.upsert_embedding("c", "m", 3, &blob(vc), "t", "ref:c")
            .unwrap();

        let hits = search_unified_with_vector(&db, "discipline", &q, 10).unwrap();
        let a = hits.iter().find(|h| h.id == "a").expect("a");
        assert_eq!(a.match_reason, "both");
        assert!(hits.iter().any(|h| h.id == "c" && h.match_reason == "semantic"));
        // b is FTS miss for "discipline" and has no embedding
        assert!(!hits.iter().any(|h| h.id == "b"));
    }

    #[test]
    fn empty_query_skips_work() {
        let db = Database::open_in_memory().unwrap();
        assert!(search_unified_with_vector(&db, "  ", &[1.0], 5)
            .unwrap()
            .is_empty());
    }
}
