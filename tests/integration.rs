use std::sync::Arc;

use tempfile::TempDir;
use vergabe_mcp::embedder::EMBEDDING_CONTRACT_VERSION;
use vergabe_mcp::profile_db::ProfileDb;
use vergabe_mcp::state::SharedState;
use vergabe_mcp::tools;
use vergabe_mcp::types::*;

fn test_state(dir: &TempDir) -> Arc<SharedState> {
    let db_path = dir.path().join("test.db");
    let db = ProfileDb::open(db_path.to_str().unwrap(), EMBEDDING_CONTRACT_VERSION).unwrap();
    Arc::new(SharedState {
        db: std::sync::Mutex::new(db),
        data_dir: dir.path().to_str().unwrap().to_string(),
        embedder: std::sync::OnceLock::new(),
        api_url: "http://localhost:19999".to_string(), // not running — REST API tests skipped
        http: reqwest::Client::new(),
        api_key: None,
    })
}

#[tokio::test]
async fn test_company_profile_lifecycle() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    // Create (no embedder → embedded=false)
    let create_result = tools::company::create_company_profile(
        &state,
        CreateCompanyProfileParams {
            name: "Acme Corp".to_string(),
            description: "We build bridges and roads".to_string(),
            cpv_codes: Some(vec!["45000000".to_string()]),
            categories: Some(vec!["works".to_string()]),
            location: Some("Berlin, Germany".to_string()),
        },
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&create_result).unwrap();
    assert_eq!(json["name"], "Acme Corp");
    assert_eq!(json["embedded"], false); // no embedder in test
    let profile_id = json["id"].as_str().unwrap().to_string();

    // Get
    let get_result = tools::company::get_company_profile(
        &state,
        GetCompanyProfileParams {
            id: profile_id.clone(),
        },
    );
    let json: serde_json::Value = serde_json::from_str(&get_result).unwrap();
    assert_eq!(json["name"], "Acme Corp");
    assert_eq!(json["description"], "We build bridges and roads");
    assert_eq!(json["location"], "Berlin, Germany");
    assert_eq!(json["has_embedding"], false);

    // Update name only
    let update_result = tools::company::update_company_profile(
        &state,
        UpdateCompanyProfileParams {
            id: profile_id.clone(),
            name: Some("Acme Industries".to_string()),
            description: None,
            cpv_codes: None,
            categories: None,
            location: None,
        },
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&update_result).unwrap();
    assert_eq!(json["updated"], true);
    assert_eq!(json["re_embedded"], false);

    // Verify name changed
    let get_result = tools::company::get_company_profile(
        &state,
        GetCompanyProfileParams {
            id: profile_id.clone(),
        },
    );
    let json: serde_json::Value = serde_json::from_str(&get_result).unwrap();
    assert_eq!(json["name"], "Acme Industries");
    assert_eq!(json["description"], "We build bridges and roads"); // unchanged

    // List — should have 1 profile
    let list_result = tools::company::list_company_profiles(&state);
    let json: serde_json::Value = serde_json::from_str(&list_result).unwrap();
    assert_eq!(json["count"], 1);
    assert_eq!(json["profiles"][0]["name"], "Acme Industries");

    // Delete
    let delete_result = tools::company::delete_company_profile(
        &state,
        DeleteCompanyProfileParams {
            id: profile_id.clone(),
        },
    );
    assert!(delete_result.contains("deleted successfully"));

    // Get after delete — not found
    let get_result = tools::company::get_company_profile(
        &state,
        GetCompanyProfileParams {
            id: profile_id.clone(),
        },
    );
    assert!(get_result.contains("No company profile found"));
}

#[tokio::test]
async fn test_get_index_info_no_api() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    // Without a running REST API, get_index_info should surface a request error.
    let result = tools::info::get_index_info(&state).await;
    assert!(
        result.contains("REST API request failed"),
        "Expected API connection error, got: {result}"
    );
}

#[tokio::test]
async fn test_search_text_no_embedder() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    let result = tools::search::search_text(
        &state,
        &SearchTextParams {
            query: "test query".to_string(),
            k: None,
            filters: None,
        },
    )
    .await;
    assert!(
        result.contains("still loading"),
        "Expected embedder loading message, got: {result}"
    );
}

#[tokio::test]
async fn test_match_tenders_no_embedder_for_unembedded_profile() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    // Create a profile without embedding (no embedder loaded in tests).
    let id = {
        let db = state.db.lock().unwrap();
        db.create_company_profile("Test Co", "description", &[], &[], None)
            .unwrap()
    };

    // match_tenders tries to lazily re-embed; with no embedder it surfaces
    // the loading message rather than contacting the API.
    let result = tools::match_tenders::match_tenders(
        &state,
        MatchTendersParams {
            profile_id: id,
            k: Some(5),
            filters: None,
        },
    )
    .await;
    assert!(
        result.contains("still loading"),
        "Expected embedder loading message, got: {result}"
    );
}

#[tokio::test]
async fn test_match_tenders_unknown_profile() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    let result = tools::match_tenders::match_tenders(
        &state,
        MatchTendersParams {
            profile_id: "does-not-exist".to_string(),
            k: None,
            filters: None,
        },
    )
    .await;
    assert!(
        result.contains("No company profile found"),
        "Expected profile-not-found, got: {result}"
    );
}

/// The Query→Passage contract switch is recorded as a meta marker; opening a
/// DB whose stored marker differs clears existing embeddings so they are
/// re-embedded under the current contract.
#[tokio::test]
async fn test_stale_embedding_cleared_on_contract_change() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("contract.db");
    let path = db_path.to_str().unwrap();

    // Open under an old contract, store an embedding.
    let id = {
        let db = ProfileDb::open(path, 0).unwrap();
        let id = db
            .create_company_profile("Co", "desc", &[], &[], None)
            .unwrap();
        db.set_profile_embedding(&id, &vec![0.1f32; 384]).unwrap();
        assert!(db.get_profile_embedding(&id).unwrap().is_some());
        id
    };

    // Reopen under the current contract → the stale embedding is cleared,
    // the description is retained.
    let db = ProfileDb::open(path, EMBEDDING_CONTRACT_VERSION).unwrap();
    assert!(
        db.get_profile_embedding(&id).unwrap().is_none(),
        "stale embedding should be cleared on contract change"
    );
    assert_eq!(
        db.get_company_profile(&id).unwrap().unwrap().description,
        "desc",
        "description retained for re-embed"
    );

    // Reopening again under the same contract leaves embeddings alone.
    db.set_profile_embedding(&id, &vec![0.2f32; 384]).unwrap();
    drop(db);
    let db = ProfileDb::open(path, EMBEDDING_CONTRACT_VERSION).unwrap();
    assert!(
        db.get_profile_embedding(&id).unwrap().is_some(),
        "embedding preserved when contract unchanged"
    );
}
