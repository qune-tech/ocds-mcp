use crate::state::SharedState;

pub async fn get_release(state: &SharedState, ocid: &str) -> String {
    let path = format!("/releases/{ocid}");
    match super::api_get_optional::<serde_json::Value>(state, &path).await {
        Ok(Some(json)) => super::to_json_string(&json),
        Ok(None) => format!("No release found with OCID: {ocid}"),
        Err(e) => e,
    }
}
