use crate::embedder::TextType;

use crate::state::SharedState;
use crate::types::{ApiSearchResult, SearchResult};

const DEFAULT_K: usize = 10;

pub async fn search_text(state: &SharedState, query: &str, k: Option<usize>) -> String {
    let k = k.unwrap_or(DEFAULT_K);

    let embedder = match super::require_embedder(state) {
        Ok(e) => e,
        Err(e) => return e,
    };

    // Embed the query locally
    let embedding = match embedder.embed_text(query, TextType::Query).await {
        Ok(v) => v,
        Err(e) => return format!("Embedding error: {e}"),
    };

    // POST to REST API /search
    let body = serde_json::json!({ "vector": embedding, "k": k });
    let results: Vec<ApiSearchResult> = match super::api_post(state, "/search", &body).await {
        Ok(r) => r,
        Err(e) => return e,
    };

    let search_results: Vec<SearchResult> = results
        .into_iter()
        .enumerate()
        .map(|(rank, r)| SearchResult {
            rank: rank + 1,
            id: r.doc_id,
            ocid: r.ocid,
            chunk_type: r.chunk_type,
            text: r.text,
            cpv_codes: r.cpv_codes,
            score: r.score,
        })
        .collect();

    super::to_json_string(&search_results)
}
