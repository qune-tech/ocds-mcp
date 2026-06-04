use crate::embedder::TextType;
use crate::state::SharedState;
use crate::types::{MatchTendersParams, SearchTextResponse};

const DEFAULT_K: usize = 10;

pub async fn match_tenders(state: &SharedState, params: MatchTendersParams) -> String {
    let k = params.k.unwrap_or(DEFAULT_K);

    // Resolve the profile's vector. Embeddings written under an older
    // embedding contract were cleared on DB open (Query→Passage switch),
    // so a profile with a description but no embedding is re-embedded here
    // from its stored description with the `passage: ` prefix (E2) — never
    // sending the profile text to the API.
    let embedding = match resolve_embedding(state, &params.profile_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    // `{vector, k, filters}` to the match endpoint. Filters apply
    // server-side; the SearchResult rows already carry the metadata, so
    // there is no per-OCID enrichment and no client-side post-filtering.
    let body = crate::tools::search::build_body(embedding, k, params.filters.as_ref());
    match super::api_post::<SearchTextResponse, _>(state, "/api/v1/match/vector", &body).await {
        Ok(resp) => {
            let response = serde_json::json!({
                "profile_id": params.profile_id,
                "count": resp.results.len(),
                "matches": resp.results,
            });
            super::to_json_string(&response)
        }
        Err(e) => e,
    }
}

/// Return the profile's `passage: `-prefixed embedding, re-embedding from
/// the stored description if it was cleared by a contract change.
async fn resolve_embedding(state: &SharedState, profile_id: &str) -> Result<Vec<f32>, String> {
    let (existing, description) = {
        let db = super::lock_db(state)?;
        let existing = db
            .get_profile_embedding(profile_id)
            .map_err(|e| format!("Error getting profile embedding: {e}"))?;
        let description = match db
            .get_company_profile(profile_id)
            .map_err(|e| format!("Error: {e}"))?
        {
            Some(p) => p.description,
            None => return Err(format!("No company profile found with ID: {profile_id}")),
        };
        (existing, description)
    };

    if let Some(emb) = existing {
        return Ok(emb);
    }

    // No stored vector — re-embed the retained description (Passage).
    let embedder = super::require_embedder(state)?;
    let embedding = embedder
        .embed_text(&description, TextType::Passage)
        .await
        .map_err(|e| format!("Embedding error: {e}"))?;
    if let Ok(db) = super::lock_db(state) {
        if let Err(e) = db.set_profile_embedding(profile_id, &embedding) {
            tracing::warn!("Failed to persist re-embedded profile: {e}");
        }
    }
    Ok(embedding)
}
