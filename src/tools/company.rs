use crate::embedder::TextType;
use serde::Serialize;

use crate::state::SharedState;
use crate::types::{
    CreateCompanyProfileParams, DeleteCompanyProfileParams, GetCompanyProfileParams,
    UpdateCompanyProfileParams,
};

#[derive(Serialize)]
struct CreateProfileResult {
    id: String,
    name: String,
    embedded: bool,
}

#[derive(Serialize)]
struct UpdateProfileResult {
    id: String,
    updated: bool,
    re_embedded: bool,
}

pub async fn create_company_profile(
    state: &SharedState,
    params: CreateCompanyProfileParams,
) -> String {
    let cpv_codes = params.cpv_codes.unwrap_or_default();
    let categories = params.categories.unwrap_or_default();

    // Create profile in local DB
    let id = match super::lock_db(state) {
        Ok(db) => {
            match db.create_company_profile(
                &params.name,
                &params.description,
                &cpv_codes,
                &categories,
                params.location.as_deref(),
            ) {
                Ok(id) => id,
                Err(e) => return format!("Error creating company profile: {e}"),
            }
        }
        Err(e) => return e,
    };

    // Embed description if embedder is available
    let mut embedded = false;
    if let Some(embedder) = state.embedder.get() {
        match embedder.embed_text(&params.description, TextType::Query).await {
            Ok(embedding) => {
                if let Ok(db) = super::lock_db(state) {
                    if let Err(e) = db.set_profile_embedding(&id, &embedding) {
                        tracing::warn!("Failed to set profile embedding: {e}");
                    } else {
                        embedded = true;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to embed profile description: {e}");
            }
        }
    }

    let result = CreateProfileResult {
        id,
        name: params.name,
        embedded,
    };
    super::to_json_string(&result)
}

pub async fn update_company_profile(
    state: &SharedState,
    params: UpdateCompanyProfileParams,
) -> String {
    let location = params.location.as_ref().map(|l| {
        if l.is_empty() { None } else { Some(l.as_str()) }
    });

    let description_changed = params.description.is_some();
    let new_description = params.description.clone();

    let updated = match super::lock_db(state) {
        Ok(db) => {
            match db.update_company_profile(
                &params.id,
                params.name.as_deref(),
                params.description.as_deref(),
                params.cpv_codes.as_deref(),
                params.categories.as_deref(),
                location,
            ) {
                Ok(updated) => updated,
                Err(e) => return format!("Error updating company profile: {e}"),
            }
        }
        Err(e) => return e,
    };

    if !updated {
        return format!("No company profile found with ID: {}", params.id);
    }

    // Re-embed if description changed and embedder is available
    let mut re_embedded = false;
    if description_changed {
        if let (Some(embedder), Some(desc)) = (state.embedder.get(), &new_description) {
            match embedder.embed_text(desc, TextType::Query).await {
                Ok(embedding) => {
                    if let Ok(db) = super::lock_db(state) {
                        if let Err(e) = db.set_profile_embedding(&params.id, &embedding) {
                            tracing::warn!("Failed to set profile embedding: {e}");
                        } else {
                            re_embedded = true;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to re-embed profile description: {e}");
                }
            }
        }
    }

    let result = UpdateProfileResult {
        id: params.id,
        updated: true,
        re_embedded,
    };
    super::to_json_string(&result)
}

pub fn get_company_profile(state: &SharedState, params: GetCompanyProfileParams) -> String {
    let db = match super::lock_db(state) {
        Ok(db) => db,
        Err(e) => return e,
    };

    match db.get_company_profile(&params.id) {
        Ok(Some(profile)) => super::to_json_string(&profile),
        Ok(None) => format!("No company profile found with ID: {}", params.id),
        Err(e) => format!("Error looking up company profile: {e}"),
    }
}

pub fn list_company_profiles(state: &SharedState) -> String {
    let db = match super::lock_db(state) {
        Ok(db) => db,
        Err(e) => return e,
    };

    match db.list_company_profiles() {
        Ok(profiles) => {
            let response = serde_json::json!({
                "count": profiles.len(),
                "profiles": profiles,
            });
            super::to_json_string(&response)
        }
        Err(e) => format!("Error listing company profiles: {e}"),
    }
}

pub fn delete_company_profile(state: &SharedState, params: DeleteCompanyProfileParams) -> String {
    let db = match super::lock_db(state) {
        Ok(db) => db,
        Err(e) => return e,
    };

    match db.delete_company_profile(&params.id) {
        Ok(true) => format!("Company profile {} deleted successfully.", params.id),
        Ok(false) => format!("No company profile found with ID: {}", params.id),
        Err(e) => format!("Error deleting company profile: {e}"),
    }
}
