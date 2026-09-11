//! Personal Q&A over stored captures (issue #21).

use crate::db::Database;
use crate::search::{search_unified_with_vector, SearchFilters};
use crate::semantic::search_semantic_with;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Default chat model for Q&A (same family as classify/enrich).
pub const DEFAULT_QA_MODEL: &str = crate::classify::DEFAULT_CLASSIFY_MODEL;

/// How many memories to stuff into the prompt.
pub const DEFAULT_TOP_K: usize = 5;

/// Max characters of `original_content` per memory in the prompt.
pub const MAX_CHARS_PER_CAPTURE: usize = 800;

/// Soft total budget for all stuffed memory text (chars).
pub const MAX_CONTEXT_CHARS: usize = 4_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QaAnswer {
    pub answer: String,
    pub citation_ids: Vec<String>,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QaError {
    EmptyQuestion,
    Http(String),
    Database(String),
    InvalidResponse(String),
}

impl std::fmt::Display for QaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QaError::EmptyQuestion => write!(f, "question is empty"),
            QaError::Http(e) => write!(f, "qa http error: {e}"),
            QaError::Database(e) => write!(f, "database error: {e}"),
            QaError::InvalidResponse(e) => write!(f, "invalid qa response: {e}"),
        }
    }
}

impl std::error::Error for QaError {}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: Option<String>,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

fn build_prompt(question: &str, memories: &[(String, String)]) -> String {
    let mut block = String::new();
    let mut used = 0usize;
    for (id, text) in memories {
        let piece = truncate(text, MAX_CHARS_PER_CAPTURE);
        if used + piece.len() > MAX_CONTEXT_CHARS {
            break;
        }
        block.push_str(&format!("### Memory id={id}\n{piece}\n\n"));
        used += piece.len();
    }
    format!(
        "You are a personal memory assistant. Answer ONLY using the memories below.\n\
         If the memories do not contain enough information, say you do not know.\n\
         Do not invent facts. Prefer quoting or paraphrasing the memories.\n\
         When you use a memory, mention its id in parentheses like (id=<uuid>).\n\n\
         Memories:\n{block}\n\
         Question: {question}\n\n\
         Answer:"
    )
}

fn default_post_json(url: &str, body: &str) -> Result<String, String> {
    let resp = ureq::post(url)
        .timeout(std::time::Duration::from_secs(120))
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}

fn call_generate<F>(
    base_url: &str,
    model: &str,
    prompt: &str,
    post_json: F,
) -> Result<String, QaError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let base = crate::ollama::normalize_base_url(base_url);
    let url = format!("{base}/api/generate");
    let body = json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
    })
    .to_string();
    let resp = post_json(&url, &body).map_err(QaError::Http)?;
    let parsed: GenerateResponse =
        serde_json::from_str(&resp).map_err(|e| QaError::InvalidResponse(e.to_string()))?;
    parsed
        .response
        .ok_or_else(|| QaError::InvalidResponse("missing response field".into()))
}

/// Retrieve candidate capture ids for a question (semantic preferred, soft-fail → exact via vector helper in tests).
fn retrieve_ids_with_vector(
    db: &Database,
    question: &str,
    query_vector: &[f32],
    top_k: usize,
) -> Result<Vec<String>, QaError> {
    let hits = search_unified_with_vector(
        db,
        question,
        query_vector,
        top_k,
        &SearchFilters::default(),
    )
    .map_err(QaError::Database)?;
    Ok(hits.into_iter().map(|h| h.id).collect())
}

fn retrieve_ids_live(
    db: &Database,
    question: &str,
    top_k: usize,
    base_url: &str,
) -> Result<Vec<String>, QaError> {
    // Prefer semantic; on failure fall back to exact-only.
    let sem = search_semantic_with(
        db,
        question,
        top_k,
        base_url,
        crate::embeddings::DEFAULT_EMBED_MODEL,
        |url, body| {
            let resp = ureq::post(url)
                .timeout(std::time::Duration::from_secs(60))
                .set("Content-Type", "application/json")
                .send_string(body)
                .map_err(|e| e.to_string())?;
            resp.into_string().map_err(|e| e.to_string())
        },
    );
    match sem {
        Ok(hits) if !hits.is_empty() => Ok(hits.into_iter().map(|h| h.id).collect()),
        _ => {
            let exact = db
                .search_exact(question)
                .map_err(|e| QaError::Database(e.to_string()))?;
            Ok(exact.into_iter().take(top_k).map(|h| h.id).collect())
        }
    }
}

fn load_memories(db: &Database, ids: &[String]) -> Result<Vec<(String, String)>, QaError> {
    let mut out = Vec::new();
    for id in ids {
        if let Some(c) = db
            .get_raw_capture(id)
            .map_err(|e| QaError::Database(e.to_string()))?
        {
            if !c.original_content.is_empty() {
                out.push((id.clone(), c.original_content));
            }
        }
    }
    Ok(out)
}

pub fn ask_memories(
    db: &Database,
    question: &str,
    base_url: &str,
    model: &str,
    top_k: usize,
) -> Result<QaAnswer, QaError> {
    ask_memories_with(db, question, base_url, model, top_k, None, default_post_json)
}

/// `query_vector` when set skips live embedding HTTP for retrieval (tests).
pub fn ask_memories_with<F>(
    db: &Database,
    question: &str,
    base_url: &str,
    model: &str,
    top_k: usize,
    query_vector: Option<&[f32]>,
    post_json: F,
) -> Result<QaAnswer, QaError>
where
    F: FnOnce(&str, &str) -> Result<String, String>,
{
    let q = question.trim();
    if q.is_empty() {
        return Err(QaError::EmptyQuestion);
    }

    let ids = if let Some(vec) = query_vector {
        retrieve_ids_with_vector(db, q, vec, top_k)?
    } else {
        retrieve_ids_live(db, q, top_k, base_url)?
    };

    if ids.is_empty() {
        return Ok(QaAnswer {
            answer: "I could not find any stored memories relevant to that question.".into(),
            citation_ids: vec![],
            model: model.into(),
        });
    }

    let memories = load_memories(db, &ids)?;
    if memories.is_empty() {
        return Ok(QaAnswer {
            answer: "I could not find any stored memories relevant to that question.".into(),
            citation_ids: vec![],
            model: model.into(),
        });
    }

    let citation_ids: Vec<String> = memories.iter().map(|(id, _)| id.clone()).collect();
    let prompt = build_prompt(q, &memories);
    let answer = call_generate(base_url, model, &prompt, post_json)?;

    Ok(QaAnswer {
        answer,
        citation_ids,
        model: model.into(),
    })
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
    fn empty_question_errors() {
        let db = Database::open_in_memory().unwrap();
        let err = ask_memories_with(
            &db,
            "  ",
            "http://127.0.0.1:9",
            DEFAULT_QA_MODEL,
            3,
            Some(&[1.0]),
            |_, _| Ok("{}".into()),
        )
        .unwrap_err();
        assert_eq!(err, QaError::EmptyQuestion);
    }

    #[test]
    fn no_hits_returns_gap_answer_without_llm() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&capture("x", "gardening tips")).unwrap();
        let ans = ask_memories_with(
            &db,
            "quantum physics",
            "http://127.0.0.1:9",
            DEFAULT_QA_MODEL,
            3,
            Some(&[1.0, 0.0]),
            |_, _| panic!("should not call LLM"),
        )
        .unwrap();
        assert!(ans.answer.contains("could not find"));
        assert!(ans.citation_ids.is_empty());
    }

    #[test]
    fn retrieves_and_cites_stuffed_memories() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&capture(
            "m1",
            "Discipline is doing hard things when you do not feel like it.",
        ))
        .unwrap();
        db.insert_raw_capture(&capture("m2", "Chocolate cake needs cocoa powder."))
            .unwrap();
        db.upsert_embedding("m1", "m", 2, &blob(&[1.0, 0.0]), "t", "r:m1")
            .unwrap();
        db.upsert_embedding("m2", "m", 2, &blob(&[0.0, 1.0]), "t", "r:m2")
            .unwrap();

        let mock = |_url: &str, body: &str| {
            assert!(body.contains("m1"));
            assert!(body.contains("Discipline is doing hard things"));
            assert!(body.contains("Question:"));
            Ok(r#"{"response":"Discipline means doing hard things (id=m1)."}"#.into())
        };

        let ans = ask_memories_with(
            &db,
            "discipline",
            "http://127.0.0.1:9",
            DEFAULT_QA_MODEL,
            5,
            Some(&[0.99, 0.01]),
            mock,
        )
        .unwrap();
        assert!(ans.citation_ids.contains(&"m1".to_string()));
        assert!(ans.answer.contains("Discipline"));
        assert_eq!(ans.model, DEFAULT_QA_MODEL);
    }

    #[test]
    fn defaults_documented() {
        assert_eq!(DEFAULT_TOP_K, 5);
        assert_eq!(MAX_CHARS_PER_CAPTURE, 800);
        assert_eq!(DEFAULT_QA_MODEL, "llama3.2");
    }
}
