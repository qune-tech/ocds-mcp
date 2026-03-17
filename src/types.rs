use rmcp::schemars;
use serde::Serialize;

// -- Tool parameter structs --

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchTextParams {
    #[schemars(description = "Text query to search for (German works best). The query is embedded locally and matched against tender chunks via cosine similarity.")]
    pub query: String,
    #[schemars(description = "Number of results to return (default: 10)")]
    pub k: Option<usize>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct GetIndexInfoParams {}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct GetReleaseParams {
    #[schemars(description = "The OCID (Open Contracting ID) of the release to retrieve")]
    pub ocid: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TenderFilterParams {
    #[schemars(description = "Filter by CPV code prefix (e.g. '45' for construction, '72' for IT)")]
    pub cpv_prefix: Option<String>,
    #[schemars(
        description = "Filter by main procurement category (e.g. 'works', 'goods', 'services')"
    )]
    pub main_procurement_category: Option<String>,
    #[schemars(description = "Filter by procurement method (e.g. 'open', 'selective', 'limited')")]
    pub procurement_method: Option<String>,
    #[schemars(description = "Filter by tender status (e.g. 'active', 'complete', 'cancelled')")]
    pub status: Option<String>,
    #[schemars(description = "Minimum tender value")]
    pub value_min: Option<f64>,
    #[schemars(description = "Maximum tender value")]
    pub value_max: Option<f64>,
    #[schemars(description = "Filter by buyer name (case-insensitive substring match)")]
    pub buyer_name: Option<String>,
    #[schemars(description = "Only include tenders with deadline on or before this ISO-8601 datetime")]
    pub deadline_before: Option<String>,
    #[schemars(description = "Only include tenders with deadline on or after this ISO-8601 datetime")]
    pub deadline_after: Option<String>,
    #[schemars(description = "Filter by lifecycle tag (e.g. 'tender', 'award', 'planning')")]
    pub tag: Option<String>,
    #[schemars(description = "Filter by whether the release has awards with suppliers (true/false)")]
    pub has_awards: Option<bool>,
    #[schemars(description = "Filter by EU funding status (true = EU funded)")]
    pub eu_funded: Option<bool>,
    #[schemars(description = "Filter by delivery location NUTS code prefix (e.g. 'DE3' for Berlin)")]
    pub location_nuts: Option<String>,
}

impl TenderFilterParams {
    pub fn has_any(&self) -> bool {
        self.cpv_prefix.is_some()
            || self.main_procurement_category.is_some()
            || self.procurement_method.is_some()
            || self.status.is_some()
            || self.value_min.is_some()
            || self.value_max.is_some()
            || self.buyer_name.is_some()
            || self.deadline_before.is_some()
            || self.deadline_after.is_some()
            || self.tag.is_some()
            || self.has_awards.is_some()
            || self.eu_funded.is_some()
            || self.location_nuts.is_some()
    }

    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = Vec::new();
        if let Some(ref v) = self.cpv_prefix {
            pairs.push(("cpv_prefix", v.clone()));
        }
        if let Some(ref v) = self.main_procurement_category {
            pairs.push(("main_procurement_category", v.clone()));
        }
        if let Some(ref v) = self.procurement_method {
            pairs.push(("procurement_method", v.clone()));
        }
        if let Some(ref v) = self.status {
            pairs.push(("status", v.clone()));
        }
        if let Some(v) = self.value_min {
            pairs.push(("value_min", v.to_string()));
        }
        if let Some(v) = self.value_max {
            pairs.push(("value_max", v.to_string()));
        }
        if let Some(ref v) = self.buyer_name {
            pairs.push(("buyer_name", v.clone()));
        }
        if let Some(ref v) = self.deadline_before {
            pairs.push(("deadline_before", v.clone()));
        }
        if let Some(ref v) = self.deadline_after {
            pairs.push(("deadline_after", v.clone()));
        }
        if let Some(ref v) = self.tag {
            pairs.push(("tag", v.clone()));
        }
        if let Some(v) = self.has_awards {
            pairs.push(("has_awards", v.to_string()));
        }
        if let Some(v) = self.eu_funded {
            pairs.push(("eu_funded", v.to_string()));
        }
        if let Some(ref v) = self.location_nuts {
            pairs.push(("location_nuts", v.clone()));
        }
        pairs
    }
}

/// Extended filter params for the list_releases tool (adds eForms-specific filters).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ListReleasesFilterParams {
    #[serde(flatten)]
    pub base: TenderFilterParams,
    #[schemars(description = "Only include tenders with submission deadline on or before this ISO-8601 datetime")]
    pub submission_deadline_before: Option<String>,
    #[schemars(description = "Only include tenders with submission deadline on or after this ISO-8601 datetime")]
    pub submission_deadline_after: Option<String>,
    #[schemars(description = "Filter by result code (e.g. 'selec-w' for selected winner, 'clos-nw' for closed no winner)")]
    pub result_code: Option<String>,
    #[schemars(description = "Filter by NUTS code prefix (e.g. 'DE2' for Baden-Württemberg)")]
    pub nuts_code: Option<String>,
}

impl ListReleasesFilterParams {
    pub fn to_query_pairs(&self) -> Vec<(&'static str, String)> {
        let mut pairs = self.base.to_query_pairs();
        if let Some(ref v) = self.submission_deadline_before {
            pairs.push(("submission_deadline_before", v.clone()));
        }
        if let Some(ref v) = self.submission_deadline_after {
            pairs.push(("submission_deadline_after", v.clone()));
        }
        if let Some(ref v) = self.result_code {
            pairs.push(("result_code", v.clone()));
        }
        if let Some(ref v) = self.nuts_code {
            pairs.push(("nuts_code", v.clone()));
        }
        pairs
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ListReleasesParams {
    #[schemars(description = "Filter by month in YYYY-MM format")]
    pub month: Option<String>,
    #[serde(flatten)]
    pub filters: ListReleasesFilterParams,
    #[schemars(description = "Maximum number of results (default 20, max 200)")]
    pub limit: Option<usize>,
    #[schemars(description = "Offset for pagination (default 0)")]
    pub offset: Option<usize>,
}

// -- Company profile param structs --

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateCompanyProfileParams {
    #[schemars(description = "Company name")]
    pub name: String,
    #[schemars(description = "Description of the company's activities, products, and services. German text recommended for best matching quality.")]
    pub description: String,
    #[schemars(description = "CPV codes the company is interested in (e.g. ['45000000', '72000000'])")]
    pub cpv_codes: Option<Vec<String>>,
    #[schemars(description = "Procurement categories of interest (e.g. ['works', 'services', 'goods'])")]
    pub categories: Option<Vec<String>>,
    #[schemars(description = "Company location (e.g. 'Berlin, Germany')")]
    pub location: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct GetCompanyProfileParams {
    #[schemars(description = "The UUID of the company profile to retrieve")]
    pub id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ListCompanyProfilesParams {}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateCompanyProfileParams {
    #[schemars(description = "The UUID of the company profile to update")]
    pub id: String,
    #[schemars(description = "New company name")]
    pub name: Option<String>,
    #[schemars(description = "New description")]
    pub description: Option<String>,
    #[schemars(description = "New CPV codes (replaces existing)")]
    pub cpv_codes: Option<Vec<String>>,
    #[schemars(description = "New procurement categories (replaces existing)")]
    pub categories: Option<Vec<String>>,
    #[schemars(description = "New location. Use empty string to clear.")]
    pub location: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteCompanyProfileParams {
    #[schemars(description = "The UUID of the company profile to delete")]
    pub id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct MatchTendersParams {
    #[schemars(description = "The UUID of the company profile to match against tenders")]
    pub profile_id: String,
    #[schemars(description = "Number of matching tenders to return (default: 10)")]
    pub k: Option<usize>,
    #[serde(flatten)]
    pub filters: TenderFilterParams,
}

// -- Result types --

/// KNN search result deserialized from the REST API `POST /search` response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ApiSearchResult {
    pub doc_id: String,
    pub ocid: String,
    pub chunk_type: String,
    pub text: String,
    pub cpv_codes: Vec<String>,
    pub score: f32,
}

/// Formatted search result returned to the LLM.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub rank: usize,
    pub id: String,
    pub ocid: String,
    pub chunk_type: String,
    pub text: String,
    pub cpv_codes: Vec<String>,
    pub score: f32,
}

/// Enriched tender match result from profile-based KNN search.
#[derive(Debug, Clone, Serialize)]
pub struct TenderMatch {
    pub rank: usize,
    pub doc_id: String,
    pub ocid: String,
    pub url: Option<String>,
    pub chunk_type: String,
    pub text: String,
    pub cpv_codes: Vec<String>,
    pub score: f32,
    pub title: Option<String>,
    pub buyer_name: Option<String>,
    pub procurement_method: Option<String>,
    pub main_procurement_category: Option<String>,
    pub value_amount: Option<f64>,
    pub value_currency: Option<String>,
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IndexInfo {
    pub embedder_loaded: bool,
    pub api_url: String,
    pub api_release_count: usize,
    pub api_embedding_count: usize,
    pub dimension: usize,
    pub company_profile_count: usize,
    pub unembedded_profile_count: usize,
}
