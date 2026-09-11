//! Contextual resurfacing from active window (issue #26).

use crate::db::Database;
use crate::search::{apply_filters, merge_exact_and_semantic, SearchFilters, UnifiedHit};
use crate::semantic::search_semantic_with_vector;
use crate::window_context::{foreground_window_context, WindowContext};

/// Default max suggestions.
pub const DEFAULT_CONTEXTUAL_LIMIT: usize = 5;

/// Drop semantic-only hits below this cosine score (exact hits always kept).
pub const DEFAULT_MIN_SEMANTIC_SCORE: f32 = 0.15;

#[derive(Debug, Clone, PartialEq)]
pub enum ContextualError {
    Database(String),
}

impl std::fmt::Display for ContextualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextualError::Database(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for ContextualError {}

fn query_from_context(ctx: &WindowContext) -> String {
    let mut parts = Vec::new();
    if let Some(t) = ctx.title.as_deref() {
        let t = t.trim();
        if !t.is_empty() {
            parts.push(t);
        }
    }
    if let Some(a) = ctx.app.as_deref() {
        let a = a.trim();
        if !a.is_empty() {
            parts.push(a);
        }
    }
    parts.join(" ")
}

/// Build suggestions from an explicit context (tests) or live window.
pub fn contextual_suggestions_with_context(
    db: &Database,
    ctx: &WindowContext,
    limit: usize,
    min_semantic_score: f32,
    query_vector: Option<&[f32]>,
) -> Result<Vec<UnifiedHit>, ContextualError> {
    let query = query_from_context(ctx);
    if query.trim().is_empty() || limit == 0 {
        return Ok(vec![]);
    }

    let exact = db
        .search_exact(&query)
        .map_err(|e| ContextualError::Database(e.to_string()))?;

    let semantic = if let Some(vec) = query_vector {
        search_semantic_with_vector(db, vec, limit.max(20))
            .map_err(|e| ContextualError::Database(e.to_string()))?
    } else {
        match crate::semantic::search_semantic(
            db,
            &query,
            limit.max(20),
            &crate::ollama::ollama_base_url(),
        ) {
            Ok(hits) => hits,
            Err(e) => {
                eprintln!("[contextual] semantic soft-fail: {e}");
                Vec::new()
            }
        }
    };

    let mut merged = merge_exact_and_semantic(&exact, &semantic, limit.max(20));
    merged.retain(|h| {
        if h.match_reason == "exact" || h.match_reason == "both" {
            true
        } else {
            h.score.unwrap_or(0.0) >= min_semantic_score
        }
    });
    let filtered = apply_filters(merged, &SearchFilters::default());
    Ok(filtered.into_iter().take(limit).collect())
}

pub fn contextual_suggestions(
    db: &Database,
    limit: usize,
    min_semantic_score: f32,
) -> Result<Vec<UnifiedHit>, ContextualError> {
    let ctx = foreground_window_context();
    contextual_suggestions_with_context(db, &ctx, limit, min_semantic_score, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RawCapture;

    fn cap(id: &str, text: &str) -> RawCapture {
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
    fn empty_context_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&cap("a", "hello")).unwrap();
        let ctx = WindowContext {
            app: None,
            title: None,
        };
        let hits = contextual_suggestions_with_context(
            &db,
            &ctx,
            5,
            DEFAULT_MIN_SEMANTIC_SCORE,
            Some(&[1.0]),
        )
        .unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn title_query_finds_exact_and_semantic() {
        let db = Database::open_in_memory().unwrap();
        db.insert_raw_capture(&cap("a", "discipline notes for deep work"))
            .unwrap();
        db.insert_raw_capture(&cap("b", "unrelated pasta recipe"))
            .unwrap();
        db.upsert_embedding("a", "m", 2, &blob(&[1.0, 0.0]), "t", "r:a")
            .unwrap();
        db.upsert_embedding("b", "m", 2, &blob(&[0.0, 1.0]), "t", "r:b")
            .unwrap();

        let ctx = WindowContext {
            app: Some("Notion".into()),
            title: Some("discipline".into()),
        };
        let hits =
            contextual_suggestions_with_context(&db, &ctx, 5, 0.1, Some(&[0.99, 0.01])).unwrap();
        assert!(hits.iter().any(|h| h.id == "a"));
        assert!(!hits.is_empty());
    }
}
