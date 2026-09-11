//! Classify captures via local Ollama (issue #16).

use crate::db::{AiClassification, Database};
use serde::Deserialize;
use serde_json::json;

/// Default Ollama chat/generate model for classification.
pub const DEFAULT_CLASSIFY_MODEL: &str = "llama3.2";

/// Allowed `content_type` values (data-model README / PLAN).
pub const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "book_quote",
    "idea",
    "note",
    "article",
    "web_link",
    "video_post",
    "conversation",
    "task",
    "general",
];

pub const STATUS_CLASSIFIED: &str = "classified";
pub const STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, PartialEq)]
pub enum ClassifyError {
    NotFound(String),
    EmptyContent,
    Http(String),
    Database(String),
    InvalidResponse(String),
}

impl std::fmt::Display for ClassifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassifyError::NotFound(id) => write!(f, "capture not found: {id}"),
            ClassifyError::EmptyContent => write!(f, "capture original_content is empty"),
            ClassifyError::Http(e) => write!(f, "classify http error: {e}"),
            ClassifyError::Database(e) => write!(f, "database error: {e}"),
            ClassifyError::InvalidResponse(e) => write!(f, "invalid classify response: {e}"),
        }
    }
}

impl std::error::Error for ClassifyError {}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClassifyJson {
    content_type: Option<String>,
    confidence: Option<f64>,
}

fn is_allowed_type(t: &str) -> bool {
    ALLOWED_CONTENT_TYPES.iter().any(|a| *a == t)
}

fn normalize_type(raw: &str) -> String {
    let t = raw.trim().to_lowercase().replace('-', "_").replace(' ', "_");
    if is_allowed_type(&t) {
        t
    } else {
        "general".into()
    }
}

fn clamp_confidence(c: f64) -> f64 {
    if c.is_nan() {
        0.0
    } else {
        c.clamp(0.0, 1.0)
    }
}

fn build_prompt(content: &str, source_kind: Option<&str>, source_title: Option<&str>) -> String {
    let types = ALLOWED_CONTENT_TYPES.join(", ");
    let mut ctx = String::new();
    if let Some(k) = source_kind {
        ctx.push_str(&format!("source_kind: {k}\n"));
    }
    if let Some(t) = source_title {
        ctx.push_str(&format!("source_title: {t}\n"));
    }
    format!(
        "Classify the following personal memory capture.\n\
         Reply with ONLY a JSON object: {{\"content_type\":\"...\",\"confidence\":0.0}}\n\
         content_type must be one of: {types}\n\
         confidence is a number from 0 to 1.\n\
         {ctx}\n\
         Capture text:\n{content}\n"
    )
}

/// Call Ollama `/api/generate` with JSON format. Injectable `post_json` for tests.
pub fn fetch_classification_with<F>(
    base_url: &str,
    model: &str,
    prompt: &str,
    post_json: F,
) -> Result<(String, f64), ClassifyError>
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
    let resp = post_json(&url, &body).map_err(ClassifyError::Http)?;
    let parsed: GenerateResponse =
        serde_json::from_str(&resp).map_err(|e| ClassifyError::InvalidResponse(e.to_string()))?;
    let text = parsed
        .response
        .ok_or_else(|| ClassifyError::InvalidResponse("missing response field".into()))?;
    let inner: ClassifyJson = serde_json::from_str(text.trim()).map_err(|e| {
        ClassifyError::InvalidResponse(format!("parse classify json: {e}; body={text}"))
    })?;
    let content_type = normalize_type(inner.content_type.as_deref().unwrap_or("general"));
    let confidence = clamp_confidence(inner.confidence.unwrap_or(0.5));
    Ok((content_type, confidence))
}

fn default_post_json(url: &str, body: &str) -> Result<String, String> {
    let resp = ureq::post(url)
        .timeout(std::time::Duration::from_secs(120))
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}

/// Classify one capture; writes AI metadata only.
pub fn classify_capture(
    db: &Database,
    capture_id: &str,
    base_url: &str,
    model: &str,
) -> Result<AiClassification, ClassifyError> {
    classify_capture_with(db, capture_id, base_url, model, default_post_json)
}

pub fn classify_capture_with<F>(
    db: &Database,
    capture_id: &str,
    base_url: &str,
    model: &str,
    post_json: F,
) -> Result<AiClassification, ClassifyError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let capture = db
        .get_raw_capture(capture_id)
        .map_err(|e| ClassifyError::Database(e.to_string()))?
        .ok_or_else(|| ClassifyError::NotFound(capture_id.to_string()))?;

    if capture.original_content.is_empty() {
        return Err(ClassifyError::EmptyContent);
    }

    let original_before = capture.original_content.clone();
    let prompt = build_prompt(
        &capture.original_content,
        capture.source_kind.as_deref(),
        capture.source_title.as_deref(),
    );

    let (content_type, confidence) =
        match fetch_classification_with(base_url, model, &prompt, post_json) {
            Ok(v) => v,
            Err(e) => {
                let updated_at = crate::capture::utc_now_iso8601_for_media();
                let _ = db.upsert_classification(
                    capture_id,
                    None,
                    None,
                    STATUS_FAILED,
                    &updated_at,
                );
                return Err(e);
            }
        };

    let updated_at = crate::capture::utc_now_iso8601_for_media();
    db.upsert_classification(
        capture_id,
        Some(&content_type),
        Some(confidence),
        STATUS_CLASSIFIED,
        &updated_at,
    )
    .map_err(|e| ClassifyError::Database(e.to_string()))?;

    let after = db
        .get_raw_capture(capture_id)
        .map_err(|e| ClassifyError::Database(e.to_string()))?
        .ok_or_else(|| ClassifyError::NotFound(capture_id.to_string()))?;
    if after.original_content != original_before {
        return Err(ClassifyError::Database(
            "original_content changed unexpectedly".into(),
        ));
    }

    db.get_ai_classification(capture_id)
        .map_err(|e| ClassifyError::Database(e.to_string()))?
        .ok_or_else(|| ClassifyError::Database("classification row missing after upsert".into()))
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
            source_title: Some("Notes".into()),
            source_url: None,
            source_extra: None,
            media_path: None,
        }
    }

    #[test]
    fn classify_writes_type_without_changing_original() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture(
            "c1",
            "The only way to do great work is to love what you do. — Jobs",
        ))
        .unwrap();

        let mock = |_url: &str, body: &str| {
            assert!(body.contains("llama3.2") || body.contains(DEFAULT_CLASSIFY_MODEL));
            assert!(body.contains("book_quote"));
            Ok(r#"{"response":"{\"content_type\":\"book_quote\",\"confidence\":0.91}"}"#.into())
        };

        let row = classify_capture_with(
            &db,
            "c1",
            "http://127.0.0.1:9",
            DEFAULT_CLASSIFY_MODEL,
            mock,
        )
        .unwrap();
        assert_eq!(row.content_type.as_deref(), Some("book_quote"));
        assert!((row.confidence.unwrap() - 0.91).abs() < 1e-9);
        assert_eq!(row.processing_status.as_deref(), Some(STATUS_CLASSIFIED));

        let loaded = db.get_raw_capture("c1").unwrap().unwrap();
        assert!(loaded.original_content.contains("great work"));
    }

    #[test]
    fn unknown_type_falls_back_to_general() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("c2", "random thought"))
            .unwrap();
        let mock = |_: &str, _: &str| {
            Ok(r#"{"response":"{\"content_type\":\"poem\",\"confidence\":1.5}"}"#.into())
        };
        let row = classify_capture_with(
            &db,
            "c2",
            "http://127.0.0.1:9",
            DEFAULT_CLASSIFY_MODEL,
            mock,
        )
        .unwrap();
        assert_eq!(row.content_type.as_deref(), Some("general"));
        assert!((row.confidence.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn reclassify_updates_ai_preserves_embedding_ref() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&sample_capture("c3", "buy milk"))
            .unwrap();
        db.upsert_embedding(
            "c3",
            "nomic-embed-text",
            2,
            &[0u8; 8],
            "2026-09-11T20:00:00Z",
            "capture_embeddings:c3",
        )
        .unwrap();

        let mock1 = |_: &str, _: &str| {
            Ok(r#"{"response":"{\"content_type\":\"task\",\"confidence\":0.8}"}"#.into())
        };
        classify_capture_with(&db, "c3", "http://127.0.0.1:9", DEFAULT_CLASSIFY_MODEL, mock1)
            .unwrap();

        let mock2 = |_: &str, _: &str| {
            Ok(r#"{"response":"{\"content_type\":\"idea\",\"confidence\":0.4}"}"#.into())
        };
        let row = classify_capture_with(
            &db,
            "c3",
            "http://127.0.0.1:9",
            DEFAULT_CLASSIFY_MODEL,
            mock2,
        )
        .unwrap();
        assert_eq!(row.content_type.as_deref(), Some("idea"));
        assert_eq!(
            db.get_embedding_ref("c3").unwrap().as_deref(),
            Some("capture_embeddings:c3")
        );
    }

    #[test]
    fn missing_and_empty_errors() {
        let db = Database::open_in_memory().unwrap();
        let err = classify_capture_with(
            &db,
            "nope",
            "http://127.0.0.1:9",
            DEFAULT_CLASSIFY_MODEL,
            |_, _| Ok("{}".into()),
        )
        .unwrap_err();
        assert!(matches!(err, ClassifyError::NotFound(_)));

        db.insert_raw_capture(&sample_capture("empty", "")).unwrap();
        let err = classify_capture_with(
            &db,
            "empty",
            "http://127.0.0.1:9",
            DEFAULT_CLASSIFY_MODEL,
            |_, _| Ok("{}".into()),
        )
        .unwrap_err();
        assert_eq!(err, ClassifyError::EmptyContent);
    }

    #[test]
    fn allowed_types_match_data_model() {
        assert!(ALLOWED_CONTENT_TYPES.contains(&"book_quote"));
        assert!(ALLOWED_CONTENT_TYPES.contains(&"task"));
        assert_eq!(ALLOWED_CONTENT_TYPES.len(), 9);
    }
}
