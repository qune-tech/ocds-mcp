use crate::state::SharedState;
use crate::types::{ListReleasesParams, ListReleasesResponse};

pub async fn list_releases(state: &SharedState, params: ListReleasesParams) -> String {
    let mut pairs: Vec<(&str, String)> = params.filters.to_query_pairs();
    if let Some(v) = params.limit {
        pairs.push(("limit", v.to_string()));
    }
    if let Some(v) = params.offset {
        pairs.push(("offset", v.to_string()));
    }

    let path = if pairs.is_empty() {
        "/api/v1/releases".to_string()
    } else {
        let qs: Vec<String> = pairs
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencode(v)))
            .collect();
        format!("/api/v1/releases?{}", qs.join("&"))
    };

    match super::api_get::<ListReleasesResponse>(state, &path).await {
        Ok(resp) => super::to_json_string(&resp.releases),
        Err(e) => e,
    }
}

/// Minimal RFC3986 query-component encoding for filter values (CPV
/// prefixes, buyer names, RFC3339 deadlines with `:` and `+`).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
