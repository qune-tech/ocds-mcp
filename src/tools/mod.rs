pub mod company;
pub mod info;
pub mod list;
pub mod match_tenders;
pub mod release;
pub mod search;

use crate::embedder::SentenceEmbedder;
use serde::Serialize;
use serde::de::DeserializeOwned;

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

fn auth(state: &SharedState, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match state.api_key {
        Some(ref key) => req.header("Authorization", format!("Bearer {key}")),
        None => req,
    }
}

pub async fn api_post<T: DeserializeOwned, B: Serialize>(
    state: &SharedState,
    path: &str,
    body: &B,
) -> Result<T, String> {
    let url = format!("{}{}", state.api_url, path);
    let resp = auth(state, state.http.post(&url).json(body))
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

pub async fn api_get<T: DeserializeOwned>(state: &SharedState, path: &str) -> Result<T, String> {
    let url = format!("{}{}", state.api_url, path);
    let resp = auth(state, state.http.get(&url))
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

/// Outcome of a GET that may legitimately resolve to "not found".
pub enum GetOutcome<T> {
    Found(T),
    /// A genuine API 404: an HTTP 404 carrying a real JSON error body
    /// (`{"error":...}`), not an empty/HTML static fallback.
    NotFound,
}

/// GET where a *genuine* API 404 is a valid answer ("this OCID does not
/// exist") but transport, parse, and non-JSON 404s (HTML/static fallback
/// from a misrouted path) are hard errors. This is the honest replacement
/// for the legacy `api_get_optional`, which mapped any 404 — including the
/// Caddy static fallback that fired when the API was dark — to "not found".
pub async fn api_get_found<T: DeserializeOwned>(
    state: &SharedState,
    path: &str,
) -> Result<GetOutcome<T>, String> {
    let url = format!("{}{}", state.api_url, path);
    let resp = auth(state, state.http.get(&url))
        .send()
        .await
        .map_err(|e| format!("REST API request failed: {e}"))?;

    let status = resp.status();
    let is_json = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false);

    if status == reqwest::StatusCode::NOT_FOUND {
        let text = resp.text().await.unwrap_or_default();
        // A genuine API 404 carries a JSON error envelope. A 404 with no
        // JSON body is a misrouted path / static fallback — fail loudly.
        if is_json && serde_json::from_str::<serde_json::Value>(&text).is_ok() {
            return Ok(GetOutcome::NotFound);
        }
        return Err(format!(
            "REST API returned 404 without a JSON error body for {path}. \
             This is not a genuine 'not found' — the path may be misrouted \
             or the API surface unavailable. Body: {text:?}"
        ));
    }

    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("REST API error ({status}): {text}"));
    }

    resp.json()
        .await
        .map(GetOutcome::Found)
        .map_err(|e| format!("Failed to parse REST API response: {e}"))
}

pub fn to_json_string<T: Serialize>(value: &T) -> String {
    match serde_json::to_string_pretty(value) {
        Ok(s) => s,
        Err(e) => format!("Serialization error: {e}"),
    }
}
