//! Enrich capture AI metadata via local Ollama (issue #17).

use crate::db::{AiEnrichment, Database};
use serde::Deserialize;
use serde_json::json;

/// Traceable prompt identity for enrichment.
pub const ENRICH_PROMPT_ID: &str = "enrich_v1";

/// Default model (same family as classification).
pub const DEFAULT_ENRICH_MODEL: &str = crate::classify::DEFAULT_CLASSIFY_MODEL;

pub const STATUS_ENRICHED: &str = "enriched";
pub const STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, PartialEq)]
pub enum EnrichError {
    NotFound(String),
    EmptyContent,
    Http(String),
    Database(String),
    InvalidResponse(String),
}

impl std::fmt::Display for EnrichError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnrichError::NotFound(id) => write!(f, "capture not found: {id}"),
            EnrichError::EmptyContent => write!(f, "capture original_content is empty"),
            EnrichError::Http(e) => write!(f, "enrich http error: {e}"),
            EnrichError::Database(e) => write!(f, "database error: {e}"),
            EnrichError::InvalidResponse(e) => write!(f, "invalid enrich response: {e}"),
        }
    }
}

impl std::error::Error for EnrichError {}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnrichJson {
    topics: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    entities: Option<Vec<String>>,
    short_description: Option<String>,
}

fn build_prompt(content: &str, source_kind: Option<&str>, source_title: Option<&str>) -> String {
    let mut ctx = String::new();
    if let Some(k) = source_kind {
        ctx.push_str(&format!("source_kind: {k}\n"));
    }
    if let Some(t) = source_title {
        ctx.push_str(&format!("source_title: {t}\n"));
    }
    format!(
        "Enrich the following personal memory capture.\n\
         Prompt id: {ENRICH_PROMPT_ID}\n\
         Reply with ONLY a JSON object with keys:\n\
         topics (array of short topic strings),\n\
         keywords (array of keyword strings),\n\
         entities (array of named entities as strings),\n\
         short_description (one short sentence).\n\
         Keep arrays small (max ~8 items each). Do not rewrite the original text.\n\
         {ctx}\n\
         Capture text:\n{content}\n"
    )
}

fn strings_to_json(items: Option<Vec<String>>) -> String {
    let list = items.unwrap_or_default();
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())
}

/// Call Ollama `/api/generate` with JSON format. Injectable for tests.
pub fn fetch_enrichment_with<F>(
    base_url: &str,
    model: &str,
    prompt: &str,
    post_json: F,
) -> Result<(String, String, String, String), EnrichError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let base = crate::ollama::normalize_base_url(base_url);
    let url = format!("{base}/api/generate");
    let body = json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": "json",
    })
    .to_string();
    let resp = post_json(&url, &body).map_err(EnrichError::Http)?;
    let parsed: GenerateResponse =
        serde_json::from_str(&resp).map_err(|e| EnrichError::InvalidResponse(e.to_string()))?;
    let text = parsed
        .response
        .ok_or_else(|| EnrichError::InvalidResponse("missing response field".into()))?;
    let inner: EnrichJson = serde_json::from_str(text.trim()).map_err(|e| {
        EnrichError::InvalidResponse(format!("parse enrich json: {e}; body={text}"))
    })?;
    let topics = strings_to_json(inner.topics);
    let keywords = strings_to_json(inner.keywords);
    let entities = strings_to_json(inner.entities);
    let short_description = inner
        .short_description
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok((topics, keywords, entities, short_description))
}

fn default_post_json(url: &str, body: &str) -> Result<String, String> {
    let resp = ureq::post(url)
        .timeout(std::time::Duration::from_secs(120))
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}

pub fn enrich_capture(
    db: &Database,
    capture_id: &str,
    base_url: &str,
    model: &str,
) -> Result<AiEnrichment, EnrichError> {
    enrich_capture_with(db, capture_id, base_url, model, default_post_json)
}

pub fn enrich_capture_with<F>(
    db: &Database,
    capture_id: &str,
    base_url: &str,
    model: &str,
    post_json: F,
) -> Result<AiEnrichment, EnrichError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let capture = db
        .get_raw_capture(capture_id)
        .map_err(|e| EnrichError::Database(e.to_string()))?
        .ok_or_else(|| EnrichError::NotFound(capture_id.to_string()))?;

    if capture.original_content.is_empty() {
        return Err(EnrichError::EmptyContent);
    }

    let original_before = capture.original_content.clone();
    let prompt = build_prompt(
        &capture.original_content,
        capture.source_kind.as_deref(),
        capture.source_title.as_deref(),
    );

    let (topics, keywords, entities, short_description) =
        match fetch_enrichment_with(base_url, model, &prompt, post_json) {
            Ok(v) => v,
            Err(e) => {
                let updated_at = crate::capture::utc_now_iso8601_for_media();
                let _ = db.mark_ai_processing_failed(capture_id, &updated_at);
                return Err(e);
            }
        };

    let updated_at = crate::capture::utc_now_iso8601_for_media();
    db.upsert_enrichment(
        capture_id,
        &topics,
        &keywords,
        &entities,
        &short_description,
        STATUS_ENRICHED,
        &updated_at,
    )
    .map_err(|e| EnrichError::Database(e.to_string()))?;

    let after = db
        .get_raw_capture(capture_id)
        .map_err(|e| EnrichError::Database(e.to_string()))?
        .ok_or_else(|| EnrichError::NotFound(capture_id.to_string()))?;
    if after.original_content != original_before {
        return Err(EnrichError::Database(
            "original_content changed unexpectedly".into(),
        ));
    }

    db.get_ai_enrichment(capture_id)
        .map_err(|e| EnrichError::Database(e.to_string()))?
        .ok_or_else(|| EnrichError::Database("enrichment row missing after upsert".into()))
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
            source_kind: Some("browser".into()),
            source_app: None,
            source_title: Some("RAG notes".into()),
            source_url: Some("https://example.com".into()),
            source_extra: None,
            media_path: None,
        }
    }

    #[test]
    fn enrich_writes_metadata_without_changing_original() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("e1", "Notes on retrieval-augmented generation"))
            .unwrap();

        let mock = |_url: &str, body: &str| {
            assert!(body.contains(ENRICH_PROMPT_ID));
            assert!(body.contains("retrieval-augmented"));
            Ok(r#"{"response":"{\"topics\":[\"AI\",\"RAG\"],\"keywords\":[\"retrieval\",\"embeddings\"],\"entities\":[\"RAG\"],\"short_description\":\"Notes about RAG.\"}"}"#.into())
        };

        let row = enrich_capture_with(
            &db,
            "e1",
            "http://127.0.0.1:9",
            DEFAULT_ENRICH_MODEL,
            mock,
        )
        .unwrap();
        assert_eq!(row.topics_json.as_deref(), Some(r#"["AI","RAG"]"#));
        assert_eq!(
            row.keywords_json.as_deref(),
            Some(r#"["retrieval","embeddings"]"#)
        );
        assert_eq!(row.entities_json.as_deref(), Some(r#"["RAG"]"#));
        assert_eq!(row.short_description.as_deref(), Some("Notes about RAG."));
        assert_eq!(row.processing_status.as_deref(), Some(STATUS_ENRICHED));
        assert_eq!(
            db.get_raw_capture("e1").unwrap().unwrap().original_content,
            "Notes on retrieval-augmented generation"
        );
    }

    #[test]
    fn enrich_preserves_classification_and_embedding_ref() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("e2", "buy milk")).unwrap();
        db.upsert_classification(
            "e2",
            Some("task"),
            Some(0.9),
            "classified",
            "2026-09-11T20:00:00Z",
        )
        .unwrap();
        db.upsert_embedding(
            "e2",
            "nomic-embed-text",
            2,
            &[0u8; 8],
            "2026-09-11T20:00:00Z",
            "capture_embeddings:e2",
        )
        .unwrap();

        let mock = |_: &str, _: &str| {
            Ok(r#"{"response":"{\"topics\":[\"errands\"],\"keywords\":[\"milk\"],\"entities\":[],\"short_description\":\"Buy milk.\"}"}"#.into())
        };
        enrich_capture_with(&db, "e2", "http://127.0.0.1:9", DEFAULT_ENRICH_MODEL, mock)
            .unwrap();

        let class = db.get_ai_classification("e2").unwrap().unwrap();
        assert_eq!(class.content_type.as_deref(), Some("task"));
        assert_eq!(
            db.get_embedding_ref("e2").unwrap().as_deref(),
            Some("capture_embeddings:e2")
        );
    }

    #[test]
    fn http_failure_marks_failed_keeps_raw() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("e3", "keep me")).unwrap();
        let err = enrich_capture_with(
            &db,
            "e3",
            "http://127.0.0.1:9",
            DEFAULT_ENRICH_MODEL,
            |_, _| Err("boom".into()),
        )
        .unwrap_err();
        assert!(matches!(err, EnrichError::Http(_)));
        let meta = db.get_ai_enrichment("e3").unwrap().unwrap();
        assert_eq!(meta.processing_status.as_deref(), Some(STATUS_FAILED));
        assert_eq!(
            db.get_raw_capture("e3").unwrap().unwrap().original_content,
            "keep me"
        );
    }

    #[test]
    fn missing_and_empty_errors() {
        let db = Database::open_in_memory().unwrap();
        let err = enrich_capture_with(
            &db,
            "nope",
            "http://127.0.0.1:9",
            DEFAULT_ENRICH_MODEL,
            |_, _| Ok("{}".into()),
        )
        .unwrap_err();
        assert!(matches!(err, EnrichError::NotFound(_)));

        db.insert_raw_capture(&sample_capture("empty", "")).unwrap();
        let err = enrich_capture_with(
            &db,
            "empty",
            "http://127.0.0.1:9",
            DEFAULT_ENRICH_MODEL,
            |_, _| Ok("{}".into()),
        )
        .unwrap_err();
        assert_eq!(err, EnrichError::EmptyContent);
    }

    #[test]
    fn prompt_id_is_stable() {
        assert_eq!(ENRICH_PROMPT_ID, "enrich_v1");
    }
}
