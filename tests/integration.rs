use std::sync::Arc;

use ocds_mcp::profile_db::ProfileDb;
use ocds_mcp::state::SharedState;
use ocds_mcp::tools;
use ocds_mcp::types::*;
use tempfile::TempDir;

fn test_state(dir: &TempDir) -> Arc<SharedState> {
    let db_path = dir.path().join("test.db");
    let db = ProfileDb::open(db_path.to_str().unwrap()).unwrap();
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

    // Without a running REST API, get_index_info should return a connection error
    let result = tools::info::get_index_info(&state).await;
    assert!(
        result.contains("Cannot reach API") || result.contains("REST API"),
        "Expected API connection error, got: {result}"
    );
}

#[tokio::test]
async fn test_search_text_no_embedder() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    let result = tools::search::search_text(&state, "test query", None).await;
    assert!(
        result.contains("still loading"),
        "Expected embedder loading message, got: {result}"
    );
}

#[tokio::test]
async fn test_match_tenders_no_embedding() {
    let dir = TempDir::new().unwrap();
    let state = test_state(&dir);

    // Create a profile without embedding
    let id = {
        let db = state.db.lock().unwrap();
        db.create_company_profile("Test Co", "description", &[], &[], None)
            .unwrap()
    };

    let result = tools::match_tenders::match_tenders(
        &state,
        MatchTendersParams {
            profile_id: id,
            k: Some(5),
            filters: TenderFilterParams::default(),
        },
    )
    .await;
    assert!(
        result.contains("No embedding found"),
        "Expected no embedding error, got: {result}"
    );
}
