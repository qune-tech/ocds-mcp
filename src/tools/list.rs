use crate::state::SharedState;
use crate::types::ListReleasesParams;

pub async fn list_releases(state: &SharedState, params: ListReleasesParams) -> String {
    let mut query_parts: Vec<String> = Vec::new();

    if let Some(ref v) = params.month {
        query_parts.push(format!("month={v}"));
    }
    for (key, value) in params.filters.to_query_pairs() {
        query_parts.push(format!("{key}={value}"));
    }
    if let Some(v) = params.limit {
        query_parts.push(format!("limit={v}"));
    }
    if let Some(v) = params.offset {
        query_parts.push(format!("offset={v}"));
    }

    let query_string = if query_parts.is_empty() {
        String::new()
    } else {
        format!("?{}", query_parts.join("&"))
    };

    let path = format!("/releases{query_string}");
    match super::api_get::<serde_json::Value>(state, &path).await {
        Ok(json) => super::to_json_string(&json),
        Err(e) => e,
    }
}
