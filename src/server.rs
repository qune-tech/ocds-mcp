use std::sync::Arc;

use rmcp::{
    RoleServer, ServerHandler,
    handler::server::router::tool::ToolRouter, handler::server::wrapper::Parameters,
    model::*, service::NotificationContext, tool, tool_handler, tool_router,
};

use crate::state::SharedState;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct OcdsMcpServer {
    state: Arc<SharedState>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl OcdsMcpServer {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Search tenders by text query. The query is embedded locally using multilingual-e5-small and matched against tender chunks via cosine similarity on the REST API. German text works best for matching German procurement data.")]
    async fn search_text(&self, Parameters(params): Parameters<SearchTextParams>) -> String {
        crate::tools::search::search_text(&self.state, &params.query, params.k).await
    }

    #[tool(description = "Get combined statistics: release and embedding counts from the REST API, plus local company profile counts. Call this first to check connectivity and see what data is available.")]
    async fn get_index_info(&self, Parameters(_params): Parameters<GetIndexInfoParams>) -> String {
        crate::tools::info::get_index_info(&self.state).await
    }

    #[tool(description = "Get the full OCDS release data for a specific tender by its OCID (Open Contracting ID). Returns the complete release JSON including tender details, buyer, parties, awards, and lot structure. Data is fetched from the REST API.")]
    async fn get_release(&self, Parameters(params): Parameters<GetReleaseParams>) -> String {
        crate::tools::release::get_release(&self.state, &params.ocid).await
    }

    #[tool(description = "List and filter OCDS procurement releases by structured criteria: month, CPV code prefix, procurement category, procurement method, value range, and buyer name. Supports pagination. Data is fetched from the REST API. Use this for tender discovery before drilling into specific releases with get_release.")]
    async fn list_releases(&self, Parameters(params): Parameters<ListReleasesParams>) -> String {
        crate::tools::list::list_releases(&self.state, params).await
    }

    #[tool(description = "Create a company profile for tender matching. Store company name, description, CPV codes, categories, and location. The description is automatically embedded for semantic matching if the embedder is loaded. Returns the profile ID and embedding status.")]
    async fn create_company_profile(
        &self,
        Parameters(params): Parameters<CreateCompanyProfileParams>,
    ) -> String {
        crate::tools::company::create_company_profile(&self.state, params).await
    }

    #[tool(description = "Update a company profile. Only provided fields are changed. If the description changes, the profile is automatically re-embedded.")]
    async fn update_company_profile(
        &self,
        Parameters(params): Parameters<UpdateCompanyProfileParams>,
    ) -> String {
        crate::tools::company::update_company_profile(&self.state, params).await
    }

    #[tool(description = "Get a company profile by its ID. Returns the full profile including name, description, CPV codes, categories, location, and embedding status.")]
    fn get_company_profile(
        &self,
        Parameters(params): Parameters<GetCompanyProfileParams>,
    ) -> String {
        crate::tools::company::get_company_profile(&self.state, params)
    }

    #[tool(description = "List all company profiles. Returns a summary of each profile (without description) ordered by creation date.")]
    fn list_company_profiles(
        &self,
        Parameters(_params): Parameters<ListCompanyProfilesParams>,
    ) -> String {
        crate::tools::company::list_company_profiles(&self.state)
    }

    #[tool(description = "Delete a company profile and its embedding by ID.")]
    fn delete_company_profile(
        &self,
        Parameters(params): Parameters<DeleteCompanyProfileParams>,
    ) -> String {
        crate::tools::company::delete_company_profile(&self.state, params)
    }

    #[tool(description = "Match a company profile against tenders using semantic similarity. The profile's embedding is sent to the REST API for KNN cosine search against all tender chunks. Results are enriched with release metadata from the API and can be post-filtered by CPV prefix, category, method, value range, buyer name, deadline, and status. Deduplicates by OCID (keeps highest score).")]
    async fn match_tenders(&self, Parameters(params): Parameters<MatchTendersParams>) -> String {
        crate::tools::match_tenders::match_tenders(&self.state, params).await
    }
}

#[tool_handler]
impl ServerHandler for OcdsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OCDS procurement data MCP server (thin client). \
                 Connects to a REST API for release data and vector search, \
                 manages company profiles locally with embedded descriptions. \
                 Call get_index_info to check connectivity and data availability. \
                 search_text finds tenders by semantic text query. \
                 list_releases filters and paginates releases by month, CPV code, category, method, value range, or buyer. \
                 get_release retrieves full release data by OCID. \
                 Company profiles: create/update/get/list/delete_company_profile for profile management. \
                 match_tenders finds tenders matching a profile via semantic similarity with optional filters. \
                 Read the ocds://guide resource for a comprehensive reference on interpreting OCDS data."
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
            resources: vec![RawResource {
                uri: "ocds://guide".into(),
                name: "ocds-guide".into(),
                title: Some("OCDS Data Reference Guide".into()),
                description: Some(
                    "Comprehensive reference for interpreting OCDS procurement data: \
                     entity hierarchy, field explanations, CPV codes, and practical tips."
                        .into(),
                ),
                mime_type: Some("text/markdown".into()),
                size: None,
                icons: None,
                meta: None,
            }
            .no_annotation()],
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, ErrorData>> + Send + '_ {
        let result = if request.uri == "ocds://guide" {
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    crate::guide::OCDS_GUIDE,
                    "ocds://guide",
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
