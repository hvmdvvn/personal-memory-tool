//! Semantic nearest-neighbor search over stored embeddings (issue #15).

use crate::db::{CaptureSummary, Database};
use crate::embeddings::{fetch_embedding_with, DEFAULT_EMBED_MODEL, EmbedError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticHit {
    pub id: String,
    pub snippet: String,
    pub captured_at: String,
    pub source_kind: Option<String>,
    pub source_app: Option<String>,
    pub score: f32,
}

impl From<(CaptureSummary, f32)> for SemanticHit {
    fn from((s, score): (CaptureSummary, f32)) -> Self {
        Self {
            id: s.id,
            snippet: s.snippet,
            captured_at: s.captured_at,
            source_kind: s.source_kind,
            source_app: s.source_app,
            score,
        }
    }
}

pub fn le_bytes_to_f32s(blob: &[u8]) -> Result<Vec<f32>, EmbedError> {
    if blob.len() % 4 != 0 {
        return Err(EmbedError::InvalidResponse("vector blob length invalid".into()));
    }
    let mut out = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(out)
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Rank stored embeddings against a query vector.
pub fn search_semantic_with_vector(
    db: &Database,
    query: &[f32],
    limit: usize,
) -> Result<Vec<SemanticHit>, EmbedError> {
    if limit == 0 {
        return Ok(vec![]);
    }
    let blobs = db
        .list_embedding_blobs()
        .map_err(|e| EmbedError::Database(e.to_string()))?;
    let mut scored = Vec::new();
    for (id, blob) in blobs {
        let vec = le_bytes_to_f32s(&blob)?;
        let score = cosine_similarity(query, &vec);
        if let Some(summary) = db
            .summary_for_capture(&id)
            .map_err(|e| EmbedError::Database(e.to_string()))?
        {
            scored.push(SemanticHit::from((summary, score)));
        }
    }
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    Ok(scored)
}

pub fn search_semantic_with<F>(
    db: &Database,
    query: &str,
    limit: usize,
    base_url: &str,
    model: &str,
    post_json: F,
) -> Result<Vec<SemanticHit>, EmbedError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    let q = fetch_embedding_with(base_url, model, trimmed, post_json)?;
    search_semantic_with_vector(db, &q, limit)
}

fn default_post_json(url: &str, body: &str) -> Result<String, String> {
    let resp = ureq::post(url)
        .timeout(std::time::Duration::from_secs(60))
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}

pub fn search_semantic(
    db: &Database,
    query: &str,
    limit: usize,
    base_url: &str,
) -> Result<Vec<SemanticHit>, EmbedError> {
    search_semantic_with(db, query, limit, base_url, DEFAULT_EMBED_MODEL, default_post_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawCapture;

    fn f32s_to_le(v: &[f32]) -> Vec<u8> {
        let mut out = Vec::new();
        for x in v {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out
    }

    fn insert(db: &Database, id: &str, content: &str, vec: &[f32]) {
        db.insert_raw_capture(&RawCapture {
            id: id.into(),
            original_content: content.into(),
            captured_at: "2026-09-11T21:00:00Z".into(),
            source_kind: None,
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        })
        .unwrap();
        db.upsert_embedding(
            id,
            DEFAULT_EMBED_MODEL,
            vec.len() as i64,
            &f32s_to_le(vec),
            "2026-09-11T21:00:00Z",
            &format!("capture_embeddings:{id}"),
        )
        .unwrap();
    }

    #[test]
    fn ranks_closest_vector_first() {
        let db = Database::open_in_memory().unwrap();
        insert(&db, "a", "alpha", &[1.0, 0.0, 0.0]);
        insert(&db, "b", "beta", &[0.0, 1.0, 0.0]);
        insert(&db, "c", "close-to-alpha", &[0.9, 0.1, 0.0]);

        let hits = search_semantic_with_vector(&db, &[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "a");
        assert!(hits[0].score >= hits[1].score);
        assert_eq!(hits[1].id, "c");
    }

    #[test]
    fn empty_query_returns_empty_without_http() {
        let db = Database::open_in_memory().unwrap();
        insert(&db, "a", "x", &[1.0, 0.0]);
        let hits = search_semantic_with(
            &db,
            "   ",
            5,
            "http://127.0.0.1:9",
            DEFAULT_EMBED_MODEL,
            |_, _| panic!("should not call embed"),
        )
        .unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn skips_captures_without_embeddings() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&RawCapture {
            id: "bare".into(),
            original_content: "no vector".into(),
            captured_at: "2026-09-11T21:00:00Z".into(),
            source_kind: None,
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        })
        .unwrap();
        insert(&db, "with", "has vector", &[0.0, 1.0]);
        let hits = search_semantic_with_vector(&db, &[0.0, 1.0], 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "with");
    }
}
