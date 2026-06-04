use std::sync::Arc;

use rmcp::{
    RoleServer, ServerHandler, handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, service::NotificationContext, tool,
    tool_handler, tool_router,
};

use crate::state::SharedState;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct VergabeMcpServer {
    state: Arc<SharedState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl VergabeMcpServer {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Semantic full-text search over the German procurement corpus. The query is embedded locally with multilingual-e5-small and only the 384-float vector is sent to the API (the query text never leaves the machine). Returns the top `k` SearchResult rows grouped by OCID, each carrying ocid, score, title, buyer_name, procurement_method, main_procurement_category, value_amount/currency/currency_eur_amount, deadline (RFC3339), award_date, cpv_codes, phase, documents_url, data_source, country. Optional `filters`: phase (planning/open/closed/awarded/unsuccessful), cpv_starts_with (2..8 digit CPV prefix), country (ISO-2), value_min/value_max (EUR), deadline_after/deadline_before (RFC3339), main_procurement_category (goods/works/services), buyer_name, procurement_method (eForms codes, match-any), data_source."
    )]
    async fn search_text(&self, Parameters(params): Parameters<SearchTextParams>) -> String {
        crate::tools::search::search_text(&self.state, &params).await
    }

    #[tool(
        description = "Filter-only browse over the German procurement corpus (no semantic query — only filter values cross the wire). Same filter taxonomy as search_text. Paginate with `limit` (max 200, default 20) and `offset`. Returns SearchResult rows ordered by deadline DESC."
    )]
    async fn list_releases(&self, Parameters(params): Parameters<ListReleasesParams>) -> String {
        crate::tools::list::list_releases(&self.state, params).await
    }

    #[tool(
        description = "Fetch the raw eForms XML envelope for a single OCID (5 fields: ocid, notice_id, data_source, country, raw_xml). Optional `notice_id` selects a specific notice of the OCID (EU siblings share one OCID); absent fetches the latest notice. Pair with linked_notices to walk a procurement's lifecycle (PIN->CN->CAN). Parse the XML yourself for detailed eForms fields — the envelope is intentionally thin."
    )]
    async fn get_release(&self, Parameters(params): Parameters<GetReleaseParams>) -> String {
        crate::tools::release::get_release(
            &self.state,
            &params.ocid,
            params.notice_id.as_deref(),
        )
        .await
    }

    #[tool(
        description = "List the notices that make up one procurement: given an OCID, return the linked set of {ocid, notice_id} refs (the procurement's notice lineage — PIN/CN/CAN siblings sharing the procurement key). Ids only; fetch each notice's XML via get_release with the returned notice_id. A length-1 set means a single isolated notice with no lifecycle to walk (the sub-threshold case)."
    )]
    async fn linked_notices(&self, Parameters(params): Parameters<LinkedNoticesParams>) -> String {
        crate::tools::release::linked_notices(&self.state, &params.ocid).await
    }

    #[tool(
        description = "Health + version check of the API plus local profile counts and the embedding-contract comparison. Reports the API status/version, the client's embedding model/contract/dimension, the server's advertised contract (when present), and whether they match. Call this first to check connectivity."
    )]
    async fn get_index_info(&self, Parameters(_params): Parameters<GetIndexInfoParams>) -> String {
        crate::tools::info::get_index_info(&self.state).await
    }

    #[tool(
        description = "Create a company profile for tender matching. Stores name, description, CPV codes, categories, and location in a local SQLite database. The description is embedded locally (passage prefix); neither the text nor the embedding is sent until a match_tenders call sends the vector. Returns the profile UUID and embedding status. German description recommended."
    )]
    async fn create_company_profile(
        &self,
        Parameters(params): Parameters<CreateCompanyProfileParams>,
    ) -> String {
        crate::tools::company::create_company_profile(&self.state, params).await
    }

    #[tool(
        description = "Update a company profile. Only provided fields change. If the description changes, the profile is re-embedded locally."
    )]
    async fn update_company_profile(
        &self,
        Parameters(params): Parameters<UpdateCompanyProfileParams>,
    ) -> String {
        crate::tools::company::update_company_profile(&self.state, params).await
    }

    #[tool(
        description = "Get a company profile by its UUID. Returns name, description, CPV codes, categories, location, and embedding status."
    )]
    fn get_company_profile(
        &self,
        Parameters(params): Parameters<GetCompanyProfileParams>,
    ) -> String {
        crate::tools::company::get_company_profile(&self.state, params)
    }

    #[tool(
        description = "List all company profiles (summary without description), ordered by creation date."
    )]
    fn list_company_profiles(
        &self,
        Parameters(_params): Parameters<ListCompanyProfilesParams>,
    ) -> String {
        crate::tools::company::list_company_profiles(&self.state)
    }

    #[tool(description = "Delete a company profile and its embedding by UUID.")]
    fn delete_company_profile(
        &self,
        Parameters(params): Parameters<DeleteCompanyProfileParams>,
    ) -> String {
        crate::tools::company::delete_company_profile(&self.state, params)
    }

    #[tool(
        description = "Semantic match: find tenders similar to a profile's stored description vector. The vector is embedded locally and sent to the API (the profile text never leaves the machine). Optional `filters` (same taxonomy as search_text) apply server-side. Returns SearchResult rows. `k` defaults to 10, max 100."
    )]
    async fn match_tenders(&self, Parameters(params): Parameters<MatchTendersParams>) -> String {
        crate::tools::match_tenders::match_tenders(&self.state, params).await
    }
}

#[tool_handler]
impl ServerHandler for VergabeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Local MCP server (thin client) for German public procurement search via the \
                 Vergabe Dashboard API. Queries and company-profile descriptions are embedded \
                 locally with multilingual-e5-small; only the resulting 384-float vectors, OCIDs, \
                 and filter values cross the wire — never query or profile text. \
                 Call get_index_info first to check connectivity and the embedding contract. \
                 search_text: semantic query. list_releases: filter-only browse. \
                 get_release: the raw eForms XML envelope for an OCID (optional notice_id selects a \
                 sibling) — parse the XML for detailed fields. linked_notices: a procurement's \
                 notice lineage (PIN->CN->CAN) so you can walk siblings via get_release. \
                 Filter taxonomy: phase (planning/open/closed/awarded/unsuccessful), cpv_starts_with, \
                 country, value_min/value_max EUR, deadline_after/deadline_before (RFC3339), \
                 main_procurement_category, buyer_name, procurement_method (eForms codes, match-any), \
                 data_source. Company profiles live in a local SQLite DB: \
                 create/update/get/list/delete_company_profile; match_tenders finds tenders \
                 matching a profile via semantic similarity with optional filters. \
                 Read the vergabe://guide resource for a full reference."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_logging()
                .build(),
            ..Default::default()
        }
    }

    fn on_initialized(
        &self,
        _context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        std::future::ready(())
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListResourcesResult {
            meta: None,
            next_cursor: None,
            resources: vec![
                RawResource {
                    uri: "vergabe://guide".into(),
                    name: "vergabe-guide".into(),
                    title: Some("Vergabe Data Reference Guide".into()),
                    description: Some(
                        "Reference for the German procurement data this server exposes: \
                         the eForms XML envelope, the filter taxonomy, lifecycle phases, \
                         the privacy architecture, and recommended workflows."
                            .into(),
                    ),
                    mime_type: Some("text/markdown".into()),
                    size: None,
                    icons: None,
                    meta: None,
                }
                .no_annotation(),
            ],
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_ {
        let result = if request.uri == "vergabe://guide" {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    crate::guide::VERGABE_GUIDE,
                    "vergabe://guide",
                )],
            })
        } else {
            Err(ErrorData::resource_not_found(
                format!("Unknown resource: {}", request.uri),
                None,
            ))
        };
        std::future::ready(result)
    }

    fn set_level(
        &self,
        _request: SetLevelRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), ErrorData>> + Send + '_ {
        std::future::ready(Ok(()))
    }
}
