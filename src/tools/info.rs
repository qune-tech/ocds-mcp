use crate::embedder::{EMBEDDING_CONTRACT_VERSION, EMBEDDING_DIM, MODEL_ID};
use crate::state::SharedState;
use crate::types::{ApiHealth, ApiVersion, IndexInfo};

pub async fn get_index_info(state: &SharedState) -> String {
    // Both endpoints are public but go through the standard authed helper
    // (consistent helper use — no bypass of the auth header).
    let health = match super::api_get::<ApiHealth>(state, "/api/v1/health").await {
        Ok(h) => h,
        Err(e) => return e,
    };
    let version = match super::api_get::<ApiVersion>(state, "/api/v1/version").await {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Report-only contract check against the client's own constants. When
    // the server does not advertise its embedding contract yet (pre vector
    // deploy), `contract_match` is None rather than a false negative.
    let contract_match = match (&version.embedding_model, version.embedding_contract) {
        (Some(model), Some(contract)) => {
            Some(model == MODEL_ID && contract == EMBEDDING_CONTRACT_VERSION)
        }
        _ => None,
    };

    let (company_profile_count, unembedded_profile_count) = match super::lock_db(state) {
        Ok(db) => (
            db.company_profile_count().unwrap_or(0),
            db.unembedded_profile_count().unwrap_or(0),
        ),
        Err(e) => return e,
    };

    let info = IndexInfo {
        api_url: state.api_url.clone(),
        api_status: health.status,
        api_store_version: version.store,
        api_web_version: version.web,
        embedder_loaded: state.embedder.get().is_some(),
        client_embedding_model: MODEL_ID,
        client_embedding_contract: EMBEDDING_CONTRACT_VERSION,
        client_embedding_dim: EMBEDDING_DIM,
        server_embedding_model: version.embedding_model,
        server_embedding_contract: version.embedding_contract,
        contract_match,
        company_profile_count,
        unembedded_profile_count,
    };

    super::to_json_string(&info)
}
