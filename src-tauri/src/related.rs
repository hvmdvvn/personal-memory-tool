//! Related captures via embedding similarity (issue #20).

use crate::db::Database;
use crate::embeddings::EmbedError;
use crate::semantic::{cosine_similarity, le_bytes_to_f32s, search_semantic_with_vector, SemanticHit};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelatedHit {
    pub id: String,
    pub snippet: String,
    pub captured_at: String,
    pub source_kind: Option<String>,
    pub source_app: Option<String>,
    pub score: f32,
    /// True when topics/keywords overlap contributed a boost.
    pub metadata_boost: bool,
}

impl From<SemanticHit> for RelatedHit {
    fn from(h: SemanticHit) -> Self {
        Self {
            id: h.id,
            snippet: h.snippet,
            captured_at: h.captured_at,
            source_kind: h.source_kind,
            source_app: h.source_app,
            score: h.score,
            metadata_boost: false,
        }
    }
}

fn parse_string_array(json: Option<&str>) -> Vec<String> {
    let Some(raw) = json else {
        return vec![];
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
            .collect(),
        _ => vec![],
    }
}

fn metadata_overlap_boost(db: &Database, source_id: &str, other_id: &str) -> Result<f32, EmbedError> {
    let src = db
        .get_ai_enrichment(source_id)
        .map_err(|e| EmbedError::Database(e.to_string()))?;
    let other = db
        .get_ai_enrichment(other_id)
        .map_err(|e| EmbedError::Database(e.to_string()))?;
    let (Some(s), Some(o)) = (src, other) else {
        return Ok(0.0);
    };
    let mut a: Vec<String> = parse_string_array(s.topics_json.as_deref());
    a.extend(parse_string_array(s.keywords_json.as_deref()));
    let mut b: Vec<String> = parse_string_array(o.topics_json.as_deref());
    b.extend(parse_string_array(o.keywords_json.as_deref()));
    if a.is_empty() || b.is_empty() {
        return Ok(0.0);
    }
    let mut shared = 0usize;
    for term in &a {
        if b.iter().any(|x| x == term) {
            shared += 1;
        }
    }
    if shared == 0 {
        Ok(0.0)
    } else {
        // Small additive boost so vector order dominates.
        Ok((shared as f32).min(3.0) * 0.02)
    }
}

/// Related captures for `capture_id`, excluding itself. Empty if no embedding stored.
pub fn related_to(
    db: &Database,
    capture_id: &str,
    limit: usize,
) -> Result<Vec<RelatedHit>, EmbedError> {
    let exists = db
        .get_raw_capture(capture_id)
        .map_err(|e| EmbedError::Database(e.to_string()))?;
    if exists.is_none() {
        return Err(EmbedError::NotFound(capture_id.to_string()));
    }

    let blob = db
        .get_embedding_blob(capture_id)
        .map_err(|e| EmbedError::Database(e.to_string()))?;
    let Some(blob) = blob else {
        return Ok(vec![]);
    };
    let query = le_bytes_to_f32s(&blob)?;
    // Fetch extra so we can drop self and still fill limit.
    let mut hits = search_semantic_with_vector(db, &query, limit.saturating_add(1))?;
    hits.retain(|h| h.id != capture_id);

    let mut related: Vec<RelatedHit> = Vec::with_capacity(hits.len());
    for h in hits {
        let boost = metadata_overlap_boost(db, capture_id, &h.id)?;
        let mut item = RelatedHit::from(h);
        if boost > 0.0 {
            item.score = (item.score + boost).min(1.0);
            item.metadata_boost = true;
        }
        related.push(item);
    }
    related.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    related.truncate(limit);
    Ok(related)
}

/// Pure cosine helper exposed for tests asserting neighbor order without DB metadata.
pub fn score_pair(a: &[f32], b: &[f32]) -> f32 {
    cosine_similarity(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawCapture;

    fn capture(id: &str, text: &str) -> RawCapture {
        RawCapture {
            id: id.into(),
            original_content: text.into(),
            captured_at: "2026-09-11T12:00:00Z".into(),
            source_kind: Some("clipboard".into()),
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        }
    }

    fn blob(v: &[f32]) -> Vec<u8> {
        let mut b = Vec::new();
        for x in v {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b
    }

    #[test]
    fn related_excludes_self_and_orders_by_score() {
        let db = Database::open_in_memory().unwrap();
        for id in ["src", "near", "far"] {
            db.insert_raw_capture(&capture(id, id)).unwrap();
        }
        db.upsert_embedding("src", "m", 3, &blob(&[1.0, 0.0, 0.0]), "t", "r:src")
            .unwrap();
        db.upsert_embedding("near", "m", 3, &blob(&[0.9, 0.1, 0.0]), "t", "r:near")
            .unwrap();
        db.upsert_embedding("far", "m", 3, &blob(&[0.0, 1.0, 0.0]), "t", "r:far")
            .unwrap();

        let hits = related_to(&db, "src", 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(!hits.iter().any(|h| h.id == "src"));
        assert_eq!(hits[0].id, "near");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn missing_capture_errors_and_no_embedding_empty() {
        let db = Database::open_in_memory().unwrap();
        let err = related_to(&db, "nope", 5).unwrap_err();
        assert!(matches!(err, EmbedError::NotFound(_)));

        db.insert_raw_capture(&capture("bare", "text")).unwrap();
        assert!(related_to(&db, "bare", 5).unwrap().is_empty());
    }

    #[test]
    fn metadata_boost_can_reorder_close_neighbors() {
        let db = Database::open_in_memory().unwrap();
        for id in ["src", "a", "b"] {
            db.insert_raw_capture(&capture(id, id)).unwrap();
        }
        // Nearly equal vectors; metadata should tip a over b.
        db.upsert_embedding("src", "m", 2, &blob(&[1.0, 0.0]), "t", "r:src")
            .unwrap();
        db.upsert_embedding("a", "m", 2, &blob(&[0.99, 0.01]), "t", "r:a")
            .unwrap();
        db.upsert_embedding("b", "m", 2, &blob(&[0.995, 0.005]), "t", "r:b")
            .unwrap();
        db.upsert_enrichment(
            "src",
            r#"["leadership"]"#,
            r#"["habits"]"#,
            "[]",
            "src",
            "enriched",
            "t",
        )
        .unwrap();
        db.upsert_enrichment(
            "a",
            r#"["leadership"]"#,
            r#"["habits"]"#,
            "[]",
            "a",
            "enriched",
            "t",
        )
        .unwrap();
        db.upsert_enrichment("b", r#"["cooking"]"#, "[]", "[]", "b", "enriched", "t")
            .unwrap();

        let hits = related_to(&db, "src", 10).unwrap();
        assert_eq!(hits[0].id, "a");
        assert!(hits[0].metadata_boost);
    }
}
