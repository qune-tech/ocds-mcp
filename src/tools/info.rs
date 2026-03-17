use serde::Deserialize;

use crate::state::SharedState;
use crate::types::IndexInfo;

#[derive(Deserialize)]
struct ApiInfo {
    release_count: usize,
    embedding_count: usize,
    dimension: usize,
}

pub async fn get_index_info(state: &SharedState) -> String {
    // Fetch remote stats from REST API
    let url = format!("{}/info", state.api_url);
    let (api_release_count, api_embedding_count, dimension) =
        match state.http.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<ApiInfo>().await {
                Ok(info) => (info.release_count, info.embedding_count, info.dimension),
                Err(e) => {
                    return format!(
                        "Failed to parse REST API /info response: {e}. Is the REST API running at {}?",
                        state.api_url
                    );
                }
            },
            Ok(resp) => {
                let status = resp.status();
                return format!(
                    "REST API /info returned {status}. Is the REST API running at {}?",
                    state.api_url
                );
            }
            Err(e) => {
                return format!(
                    "Cannot reach API at {}: {e}. Check your --api-url and internet connection.",
                    state.api_url
                );
            }
        };

    // Local profile stats
    let (company_profile_count, unembedded_profile_count) = match super::lock_db(state) {
        Ok(db) => (
            db.company_profile_count().unwrap_or(0),
            db.unembedded_profile_count().unwrap_or(0),
        ),
        Err(e) => return e,
    };

    let info = IndexInfo {
        embedder_loaded: state.embedder.get().is_some(),
        api_url: state.api_url.clone(),
        api_release_count,
        api_embedding_count,
        dimension,
        company_profile_count,
        unembedded_profile_count,
    };

    super::to_json_string(&info)
}
