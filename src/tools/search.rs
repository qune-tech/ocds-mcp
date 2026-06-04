use crate::embedder::TextType;

use crate::state::SharedState;
use crate::types::{SearchFilters, SearchTextParams, SearchTextResponse};

const DEFAULT_K: usize = 10;

pub async fn search_text(state: &SharedState, params: &SearchTextParams) -> String {
    let k = params.k.unwrap_or(DEFAULT_K);

    let embedder = match super::require_embedder(state) {
        Ok(e) => e,
        Err(e) => return e,
    };

    // Embed the query locally with the e5 `query: ` prefix (E1). Only the
    // resulting vector leaves the machine — never the query text.
    let embedding = match embedder.embed_text(&params.query, TextType::Query).await {
        Ok(v) => v,
        Err(e) => return format!("Embedding error: {e}"),
    };

    let body = build_body(embedding, k, params.filters.as_ref());
    match super::api_post::<SearchTextResponse, _>(state, "/api/v1/search/vector", &body).await {
        Ok(resp) => super::to_json_string(&resp.results),
        Err(e) => e,
    }
}

/// `{vector, k, filters}` — the vector-endpoint request body. `filters` is
/// omitted entirely when absent.
pub fn build_body(
    vector: Vec<f32>,
    k: usize,
    filters: Option<&SearchFilters>,
) -> serde_json::Value {
    match filters {
        Some(f) => serde_json::json!({ "vector": vector, "k": k, "filters": f }),
        None => serde_json::json!({ "vector": vector, "k": k }),
    }
}
