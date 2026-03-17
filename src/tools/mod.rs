pub mod company;
pub mod info;
pub mod list;
pub mod match_tenders;
pub mod release;
pub mod search;

use crate::embedder::SentenceEmbedder;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::profile_db::ProfileDb;
use crate::state::SharedState;

pub fn lock_db(state: &SharedState) -> Result<std::sync::MutexGuard<'_, ProfileDb>, String> {
    state
        .db
        .lock()
        .map_err(|e| format!("Database lock error: {e}"))
}

pub fn require_embedder(state: &SharedState) -> Result<&SentenceEmbedder, String> {
    state
        .embedder
        .get()
        .ok_or_else(|| "Embedder is still loading, please try again shortly. Check get_index_info for status.".to_string())
}

pub async fn api_post<T: DeserializeOwned, B: Serialize>(
    state: &SharedState,
    path: &str,
    body: &B,
) -> Result<T, String> {
    let url = format!("{}{}", state.api_url, path);
    let mut req = state.http.post(&url).json(body);
    if let Some(ref key) = state.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("REST API request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("REST API error ({status}): {text}"));
    }

    resp.json()
        .await
        .map_err(|e| format!("Failed to parse REST API response: {e}"))
}

pub async fn api_get<T: DeserializeOwned>(
    state: &SharedState,
    path: &str,
) -> Result<T, String> {
    let url = format!("{}{}", state.api_url, path);
    let mut req = state.http.get(&url);
    if let Some(ref key) = state.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("REST API request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("REST API error ({status}): {text}"));
    }

    resp.json()
        .await
        .map_err(|e| format!("Failed to parse REST API response: {e}"))
}

pub async fn api_get_optional<T: DeserializeOwned>(
    state: &SharedState,
    path: &str,
) -> Result<Option<T>, String> {
    let url = format!("{}{}", state.api_url, path);
    let mut req = state.http.get(&url);
    if let Some(ref key) = state.api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("REST API request failed: {e}"))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("REST API error ({status}): {text}"));
    }

    resp.json()
        .await
        .map(Some)
        .map_err(|e| format!("Failed to parse REST API response: {e}"))
}

pub fn to_json_string<T: Serialize>(value: &T) -> String {
    match serde_json::to_string_pretty(value) {
        Ok(s) => s,
        Err(e) => format!("Serialization error: {e}"),
    }
}
