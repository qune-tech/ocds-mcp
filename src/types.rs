use rmcp::schemars;
use serde::{Deserialize, Serialize};

// -- Filter taxonomy (mirrors the HTTP API `SearchFilters`) --

/// Server-side filter taxonomy. Field names are verbatim copies of the
/// HTTP API `SearchFilters` (see `docs/architecture/search-filters.md` in
/// the backend). One struct serves all three filtered surfaces:
/// `search_text` and `match_tenders` send it as the JSON `filters` object
/// on the vector endpoints; `list_releases` flattens it into query params
/// (with `procurement_method` as repeated params).
#[derive(Debug, Default, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchFilters {
    #[schemars(description = "Lifecycle phase: planning, open, closed, awarded, or unsuccessful.")]
    pub phase: Option<String>,
    #[schemars(description = "CPV code prefix, 2..8 digits (e.g. '45' for construction, '72' for IT services).")]
    pub cpv_starts_with: Option<String>,
    #[schemars(description = "ISO-3166 alpha-2 country code (e.g. 'DE').")]
    pub country: Option<String>,
    #[schemars(description = "Minimum estimated value in EUR.")]
    pub value_min: Option<f64>,
    #[schemars(description = "Maximum estimated value in EUR.")]
    pub value_max: Option<f64>,
    #[schemars(description = "Only tenders with a deadline on or after this RFC3339 datetime.")]
    pub deadline_after: Option<chrono::DateTime<chrono::Utc>>,
    #[schemars(description = "Only tenders with a deadline on or before this RFC3339 datetime.")]
    pub deadline_before: Option<chrono::DateTime<chrono::Utc>>,
    #[schemars(description = "Raw eForms procurement-procedure codes (e.g. 'de-open', 'neg-w-call'). Match-any.")]
    pub procurement_method: Option<Vec<String>>,
    #[schemars(description = "Main procurement category: goods, works, or services.")]
    pub main_procurement_category: Option<String>,
    #[schemars(description = "Buyer name. Exact match today; case-insensitive once the pending backend cutover deploys.")]
    pub buyer_name: Option<String>,
    #[schemars(description = "Data source (e.g. 'de').")]
    pub data_source: Option<String>,
}

impl SearchFilters {
    /// Serialize to repeated query-string pairs for `GET /releases`.
    /// `procurement_method` becomes one pair per code; deadlines are
    /// emitted as RFC3339.
    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(ref v) = self.phase {
            pairs.push(("phase", v.clone()));
        }
        if let Some(ref v) = self.cpv_starts_with {
            pairs.push(("cpv_starts_with", v.clone()));
        }
        if let Some(ref v) = self.country {
            pairs.push(("country", v.clone()));
        }
        if let Some(v) = self.value_min {
            pairs.push(("value_min", v.to_string()));
        }
        if let Some(v) = self.value_max {
            pairs.push(("value_max", v.to_string()));
        }
        if let Some(v) = self.deadline_after {
            pairs.push(("deadline_after", v.to_rfc3339()));
        }
        if let Some(v) = self.deadline_before {
            pairs.push(("deadline_before", v.to_rfc3339()));
        }
        if let Some(ref methods) = self.procurement_method {
            for m in methods {
                pairs.push(("procurement_method", m.clone()));
            }
        }
        if let Some(ref v) = self.main_procurement_category {
            pairs.push(("main_procurement_category", v.clone()));
        }
        if let Some(ref v) = self.buyer_name {
            pairs.push(("buyer_name", v.clone()));
        }
        if let Some(ref v) = self.data_source {
            pairs.push(("data_source", v.clone()));
        }
        pairs
    }
}

// -- Tool parameter structs --

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchTextParams {
    #[schemars(description = "Text query to search for (German works best). The query is embedded locally with the e5 'query: ' prefix and only the resulting 384-float vector is sent to the API.")]
    pub query: String,
    #[schemars(description = "Number of results to return (default 10, max 100).")]
    pub k: Option<usize>,
    #[serde(default)]
    #[schemars(description = "Optional server-side filters narrowing the result set.")]
    pub filters: Option<SearchFilters>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetIndexInfoParams {}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetReleaseParams {
    #[schemars(description = "The OCID (Open Contracting ID) of the release to retrieve.")]
    pub ocid: String,
    #[serde(default)]
    #[schemars(description = "Optional: fetch a specific notice of this OCID (EU siblings share one OCID). Use a notice_id from linked_notices to address an older sibling; absent fetches the latest notice.")]
    pub notice_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LinkedNoticesParams {
    #[schemars(description = "The OCID (Open Contracting ID) whose notice lineage to retrieve.")]
    pub ocid: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListReleasesParams {
    #[serde(flatten)]
    pub filters: SearchFilters,
    #[schemars(description = "Maximum number of results (default 20, max 200).")]
    pub limit: Option<usize>,
    #[schemars(description = "Offset for pagination (default 0).")]
    pub offset: Option<usize>,
}

// -- Company profile param structs --

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CreateCompanyProfileParams {
    #[schemars(description = "Company name.")]
    pub name: String,
    #[schemars(description = "Description of the company's activities, products, and services. German text recommended for best matching quality. Embedded locally; never sent to the API.")]
    pub description: String,
    #[schemars(description = "CPV codes the company is interested in (e.g. ['45000000', '72000000']).")]
    pub cpv_codes: Option<Vec<String>>,
    #[schemars(description = "Procurement categories of interest (e.g. ['works', 'services', 'goods']).")]
    pub categories: Option<Vec<String>>,
    #[schemars(description = "Company location (e.g. 'Berlin, Germany').")]
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetCompanyProfileParams {
    #[schemars(description = "The UUID of the company profile to retrieve.")]
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListCompanyProfilesParams {}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UpdateCompanyProfileParams {
    #[schemars(description = "The UUID of the company profile to update.")]
    pub id: String,
    #[schemars(description = "New company name.")]
    pub name: Option<String>,
    #[schemars(description = "New description. Re-embedded locally on change.")]
    pub description: Option<String>,
    #[schemars(description = "New CPV codes (replaces existing).")]
    pub cpv_codes: Option<Vec<String>>,
    #[schemars(description = "New procurement categories (replaces existing).")]
    pub categories: Option<Vec<String>>,
    #[schemars(description = "New location. Use empty string to clear.")]
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DeleteCompanyProfileParams {
    #[schemars(description = "The UUID of the company profile to delete.")]
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MatchTendersParams {
    #[schemars(description = "The UUID of the company profile to match against tenders.")]
    pub profile_id: String,
    #[schemars(description = "Number of matching tenders to return (default 10, max 100).")]
    pub k: Option<usize>,
    #[serde(default)]
    #[schemars(description = "Optional server-side filters narrowing the matches.")]
    pub filters: Option<SearchFilters>,
}

// -- API response types (deserialized from /api/v1) --

/// One row of `/search/vector`, `/match/vector`, and `/releases` output.
/// Mirrors the backend `SearchResult` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub ocid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    pub title: Option<String>,
    pub buyer_name: Option<String>,
    pub procurement_method: Option<String>,
    pub main_procurement_category: Option<String>,
    pub value_amount: Option<f64>,
    pub value_currency: Option<String>,
    pub currency_eur_amount: Option<f64>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub award_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub cpv_codes: Vec<String>,
    pub phase: String,
    pub documents_url: Option<String>,
    pub data_source: String,
    pub country: Option<String>,
}

/// `{ "results": [...] }` — the body of `/search/vector` and `/match/vector`.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchTextResponse {
    pub results: Vec<SearchResult>,
}

/// `{ "releases": [...] }` — the body of `GET /releases`.
#[derive(Debug, Clone, Deserialize)]
pub struct ListReleasesResponse {
    pub releases: Vec<SearchResult>,
}

/// `GET /releases/:ocid` envelope: per-row metadata + the raw eForms XML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseDetailResponse {
    pub ocid: String,
    pub notice_id: String,
    pub data_source: String,
    pub country: Option<String>,
    pub raw_xml: String,
}

/// One notice's identity within a linked set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticeRef {
    pub ocid: String,
    pub notice_id: String,
}

/// `GET /releases/:ocid/linked` envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedNoticesResponse {
    pub key: String,
    pub notices: Vec<NoticeRef>,
}

/// `GET /api/v1/version`. `embedding_model` / `embedding_contract` are
/// part of the vector-API contract and may be absent on backends that
/// predate the vector-endpoint deploy.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiVersion {
    pub store: Option<String>,
    pub web: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_contract: Option<u32>,
}

/// `GET /api/v1/health`.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiHealth {
    pub status: String,
}

/// Reported by `get_index_info`: the local client's embedding contract,
/// the server's, and whether they match.
#[derive(Debug, Serialize)]
pub struct IndexInfo {
    pub api_url: String,
    pub api_status: String,
    pub api_store_version: Option<String>,
    pub api_web_version: Option<String>,
    pub embedder_loaded: bool,
    pub client_embedding_model: &'static str,
    pub client_embedding_contract: u32,
    pub client_embedding_dim: usize,
    pub server_embedding_model: Option<String>,
    pub server_embedding_contract: Option<u32>,
    /// `Some(true)` / `Some(false)` when the server advertises its contract;
    /// `None` when the server does not expose the embedding fields yet.
    pub contract_match: Option<bool>,
    pub company_profile_count: usize,
    pub unembedded_profile_count: usize,
}
