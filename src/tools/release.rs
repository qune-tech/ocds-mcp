use crate::state::SharedState;
use crate::tools::GetOutcome;
use crate::types::{LinkedNoticesResponse, ReleaseDetailResponse};

pub async fn get_release(
    state: &SharedState,
    ocid: &str,
    notice_id: Option<&str>,
) -> String {
    let path = match notice_id {
        Some(nid) => format!("/api/v1/releases/{ocid}?notice_id={nid}"),
        None => format!("/api/v1/releases/{ocid}"),
    };
    match super::api_get_found::<ReleaseDetailResponse>(state, &path).await {
        Ok(GetOutcome::Found(release)) => super::to_json_string(&release),
        Ok(GetOutcome::NotFound) => format!("No release found with OCID: {ocid}"),
        Err(e) => e,
    }
}

pub async fn linked_notices(state: &SharedState, ocid: &str) -> String {
    let path = format!("/api/v1/releases/{ocid}/linked");
    match super::api_get_found::<LinkedNoticesResponse>(state, &path).await {
        Ok(GetOutcome::Found(linked)) => super::to_json_string(&linked),
        Ok(GetOutcome::NotFound) => format!("No release found with OCID: {ocid}"),
        Err(e) => e,
    }
}
