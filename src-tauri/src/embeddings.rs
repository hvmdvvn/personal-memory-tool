//! Embed captures via local Ollama (issue #14).

use crate::db::Database;
use serde::Deserialize;
use serde_json::json;

/// Default Ollama embedding model.
pub const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedError {
    NotFound(String),
    EmptyContent,
    Http(String),
    Database(String),
    InvalidResponse(String),
}

impl std::fmt::Display for EmbedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedError::NotFound(id) => write!(f, "capture not found: {id}"),
            EmbedError::EmptyContent => write!(f, "capture original_content is empty"),
            EmbedError::Http(e) => write!(f, "embed http error: {e}"),
            EmbedError::Database(e) => write!(f, "database error: {e}"),
            EmbedError::InvalidResponse(e) => write!(f, "invalid embed response: {e}"),
        }
    }
}

impl std::error::Error for EmbedError {}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embedding: Option<Vec<f32>>,
}

fn f32_slice_to_le_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

/// Call Ollama embeddings API. `post_json(url, body) -> response body string` is injectable for tests.
pub fn fetch_embedding_with<F>(
    base_url: &str,
    model: &str,
    prompt: &str,
    post_json: F,
) -> Result<Vec<f32>, EmbedError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let base = crate::ollama::normalize_base_url(base_url);
    let url = format!("{base}/api/embeddings");
    let body = json!({ "model": model, "prompt": prompt }).to_string();
    let resp = post_json(&url, &body).map_err(EmbedError::Http)?;
    let parsed: EmbedResponse =
        serde_json::from_str(&resp).map_err(|e| EmbedError::InvalidResponse(e.to_string()))?;
    let embedding = parsed
        .embedding
        .ok_or_else(|| EmbedError::InvalidResponse("missing embedding field".into()))?;
    if embedding.is_empty() {
        return Err(EmbedError::InvalidResponse("empty embedding".into()));
    }
    Ok(embedding)
}

fn default_post_json(url: &str, body: &str) -> Result<String, String> {
    let resp = ureq::post(url)
        .timeout(std::time::Duration::from_secs(60))
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}

/// Embed one capture and store the vector. Does not modify original_content.
pub fn embed_capture(
    db: &Database,
    capture_id: &str,
    base_url: &str,
    model: &str,
) -> Result<String, EmbedError> {
    embed_capture_with(db, capture_id, base_url, model, default_post_json)
}

pub fn embed_capture_with<F>(
    db: &Database,
    capture_id: &str,
    base_url: &str,
    model: &str,
    post_json: F,
) -> Result<String, EmbedError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let capture = db
        .get_raw_capture(capture_id)
        .map_err(|e| EmbedError::Database(e.to_string()))?
        .ok_or_else(|| EmbedError::NotFound(capture_id.to_string()))?;

    if capture.original_content.is_empty() {
        return Err(EmbedError::EmptyContent);
    }

    let original_before = capture.original_content.clone();
    let vector = fetch_embedding_with(base_url, model, &capture.original_content, post_json)?;
    let blob = f32_slice_to_le_bytes(&vector);
    let created_at = crate::capture::utc_now_iso8601_for_media();
    let embedding_ref = format!("capture_embeddings:{capture_id}");

    db.upsert_embedding(
        capture_id,
        model,
        vector.len() as i64,
        &blob,
        &created_at,
        &embedding_ref,
    )
    .map_err(|e| EmbedError::Database(e.to_string()))?;

    let after = db
        .get_raw_capture(capture_id)
        .map_err(|e| EmbedError::Database(e.to_string()))?
        .ok_or_else(|| EmbedError::NotFound(capture_id.to_string()))?;
    if after.original_content != original_before {
        return Err(EmbedError::Database(
            "original_content changed unexpectedly".into(),
        ));
    }

    Ok(embedding_ref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawCapture;

    fn sample_capture(id: &str, content: &str) -> RawCapture {
        RawCapture {
            id: id.into(),
            original_content: content.into(),
            captured_at: "2026-09-11T20:00:00Z".into(),
            source_kind: Some("clipboard".into()),
            source_app: None,
            source_title: None,
            source_url: None,
            source_extra: None,
            media_path: None,
        }
    }

    #[test]
    fn embed_stores_vector_and_ref_without_changing_original() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("c1", "hello embedding world"))
            .unwrap();

        let mock = |_url: &str, body: &str| {
            assert!(body.contains("hello embedding world"));
            assert!(body.contains(DEFAULT_EMBED_MODEL));
            Ok(r#"{"embedding":[0.1,0.2,0.3,0.4]}"#.to_string())
        };

        let pref = embed_capture_with(
            &db,
            "c1",
            "http://127.0.0.1:9",
            DEFAULT_EMBED_MODEL,
            mock,
        )
        .unwrap();
        assert_eq!(pref, "capture_embeddings:c1");

        let blob = db.get_embedding_blob("c1").unwrap().unwrap();
        assert_eq!(blob.len(), 16);
        let refer = db.get_embedding_ref("c1").unwrap().unwrap();
        assert_eq!(refer, "capture_embeddings:c1");

        let loaded = db.get_raw_capture("c1").unwrap().unwrap();
        assert_eq!(loaded.original_content, "hello embedding world");
    }

    #[test]
    fn missing_capture_errors() {
        let db = Database::open_in_memory().unwrap();
        let err = embed_capture_with(
            &db,
            "nope",
            "http://127.0.0.1:9",
            DEFAULT_EMBED_MODEL,
            |_, _| Ok(r#"{"embedding":[1.0]}"#.into()),
        )
        .unwrap_err();
        assert!(matches!(err, EmbedError::NotFound(_)));
    }

    #[test]
    fn empty_content_errors() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("empty", "")).unwrap();
        let err = embed_capture_with(
            &db,
            "empty",
            "http://127.0.0.1:9",
            DEFAULT_EMBED_MODEL,
            |_, _| Ok(r#"{"embedding":[1.0]}"#.into()),
        )
        .unwrap_err();
        assert_eq!(err, EmbedError::EmptyContent);
        assert!(db.get_embedding_blob("empty").unwrap().is_none());
    }
}
