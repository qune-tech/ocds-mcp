use std::collections::HashMap;

use serde::Deserialize;

const NOTICE_BASE_URL: &str = "https://oeffentlichevergabe.de/ui/de/search/details?noticeId=";

fn notice_url(release_id: &str) -> String {
    format!("{NOTICE_BASE_URL}{release_id}")
}

use crate::state::SharedState;
use crate::types::{ApiSearchResult, MatchTendersParams, TenderMatch};

/// Truncate text to a maximum number of characters, appending "…" if truncated.
fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        let mut end = max_chars;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &text[..end])
    }
}

#[derive(Deserialize)]
struct ApiRelease {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    tag: Vec<String>,
    #[serde(default)]
    tender: Option<ApiTender>,
    #[serde(default)]
    buyer: Option<ApiBuyer>,
    #[serde(default)]
    awards: Vec<ApiAward>,
}

#[derive(Deserialize)]
struct ApiAward {
    #[serde(default)]
    suppliers: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct ApiTender {
    title: Option<String>,
    status: Option<String>,
    #[serde(rename = "procurementMethod")]
    procurement_method: Option<String>,
    #[serde(rename = "mainProcurementCategory")]
    main_procurement_category: Option<String>,
    value: Option<ApiValue>,
    #[serde(rename = "tenderPeriod")]
    tender_period: Option<ApiPeriod>,
    #[serde(rename = "submissionDeadline")]
    submission_deadline: Option<String>,
    #[serde(default)]
    eu_funded: Option<bool>,
    #[serde(default)]
    location: Option<ApiLocation>,
    #[serde(default)]
    lots: Vec<ApiLot>,
    #[serde(rename = "documentsUrl")]
    documents_url: Option<String>,
}

#[derive(Deserialize)]
struct ApiLocation {
    nuts_code: Option<String>,
}

#[derive(Deserialize)]
struct ApiLot {
    #[serde(default)]
    location: Option<ApiLocation>,
}

#[derive(Deserialize)]
struct ApiBuyer {
    name: Option<String>,
}

#[derive(Deserialize)]
struct ApiValue {
    amount: Option<f64>,
    currency: Option<String>,
}

#[derive(Deserialize)]
struct ApiPeriod {
    #[serde(rename = "endDate")]
    end_date: Option<String>,
}

struct ReleaseMetadata {
    notice_id: Option<String>,
    title: Option<String>,
    buyer_name: Option<String>,
    procurement_method: Option<String>,
    main_procurement_category: Option<String>,
    value_amount: Option<f64>,
    value_currency: Option<String>,
    deadline: Option<String>,
    submission_deadline: Option<String>,
    status: Option<String>,
    tag: Vec<String>,
    has_awards: bool,
    eu_funded: Option<bool>,
    location_nuts: Vec<String>,
    documents_url: Option<String>,
}

pub async fn match_tenders(state: &SharedState, params: MatchTendersParams) -> String {
    let k = params.k.unwrap_or(10);

    // Get profile embedding from local DB
    let embedding = match super::lock_db(state) {
        Ok(db) => match db.get_profile_embedding(&params.profile_id) {
            Ok(Some(emb)) => emb,
            Ok(None) => {
                // Check if profile exists at all
                return match db.get_company_profile(&params.profile_id) {
                    Ok(Some(_)) => {
                        "No embedding found for this profile. The embedder may not have been available when the profile was created.".to_string()
                    }
                    Ok(None) => {
                        format!("No company profile found with ID: {}", params.profile_id)
                    }
                    Err(e) => format!("Error: {e}"),
                };
            }
            Err(e) => return format!("Error getting profile embedding: {e}"),
        },
        Err(e) => return e,
    };

    // Determine over-fetch factor based on filters
    let has_filters = params.filters.has_any();
    let fetch_k = if has_filters { k * 5 } else { k * 3 };

    // POST to REST API /search with the profile embedding
    let body = serde_json::json!({ "vector": embedding, "k": fetch_k });
    let search_results: Vec<ApiSearchResult> = match super::api_post(state, "/search", &body).await
    {
        Ok(r) => r,
        Err(e) => return e,
    };

    if search_results.is_empty() {
        let response = serde_json::json!({
            "profile_id": params.profile_id,
            "count": 0,
            "matches": [],
        });
        return serde_json::to_string_pretty(&response).unwrap_or_default();
    }

    // Collect unique OCIDs
    let unique_ocids: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        search_results
            .iter()
            .filter(|r| seen.insert(r.ocid.clone()))
            .map(|r| r.ocid.clone())
            .collect()
    };

    // Fetch release metadata for each unique OCID (best-effort, silent failure)
    let mut metadata_map: HashMap<String, ReleaseMetadata> = HashMap::new();

    for ocid in &unique_ocids {
        let release_url = format!("{}/releases/{}", state.api_url, ocid);
        let mut req = state.http.get(&release_url);
        if let Some(ref key) = state.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        if let Ok(resp) = req.send().await {
            if resp.status().is_success() {
                if let Ok(release) = resp.json::<ApiRelease>().await {
                    let tender = release.tender.as_ref();
                    let has_awards = release
                        .awards
                        .iter()
                        .any(|a| !a.suppliers.is_empty());
                    let eu_funded = tender.and_then(|t| t.eu_funded);
                    let mut location_nuts = Vec::new();
                    if let Some(ref nuts) = tender
                        .and_then(|t| t.location.as_ref())
                        .and_then(|l| l.nuts_code.as_ref())
                    {
                        location_nuts.push((*nuts).clone());
                    }
                    if let Some(t) = tender {
                        for lot in &t.lots {
                            if let Some(ref nuts) = lot
                                .location
                                .as_ref()
                                .and_then(|l| l.nuts_code.as_ref())
                            {
                                if !location_nuts.contains(nuts) {
                                    location_nuts.push((*nuts).clone());
                                }
                            }
                        }
                    }
                    metadata_map.insert(
                        ocid.clone(),
                        ReleaseMetadata {
                            notice_id: release.id,
                            title: tender.and_then(|t| t.title.clone()),
                            buyer_name: release.buyer.as_ref().and_then(|b| b.name.clone()),
                            procurement_method: tender
                                .and_then(|t| t.procurement_method.clone()),
                            main_procurement_category: tender
                                .and_then(|t| t.main_procurement_category.clone()),
                            value_amount: tender.and_then(|t| t.value.as_ref()?.amount),
                            value_currency: tender
                                .and_then(|t| t.value.as_ref()?.currency.clone()),
                            deadline: tender
                                .and_then(|t| t.tender_period.as_ref()?.end_date.clone()),
                            submission_deadline: tender
                                .and_then(|t| t.submission_deadline.clone()),
                            status: tender.and_then(|t| t.status.clone()),
                            tag: release.tag,
                            has_awards,
                            eu_funded,
                            location_nuts,
                            documents_url: tender
                                .and_then(|t| t.documents_url.clone()),
                        },
                    );
                }
            }
        }
    }

    // Enrich search results with metadata, apply filters, dedup by OCID
    let mut enriched: Vec<TenderMatch> = Vec::new();

    for result in &search_results {
        let meta = metadata_map.get(&result.ocid);

        // Apply post-filters
        if let Some(ref cpv) = params.filters.cpv_prefix {
            if !result.cpv_codes.iter().any(|c| c.starts_with(cpv.as_str())) {
                continue;
            }
        }
        if let Some(ref cat) = params.filters.main_procurement_category {
            if meta
                .and_then(|m| m.main_procurement_category.as_ref())
                .map_or(true, |c| c != cat)
            {
                continue;
            }
        }
        if let Some(ref method) = params.filters.procurement_method {
            if meta
                .and_then(|m| m.procurement_method.as_ref())
                .map_or(true, |m| m != method)
            {
                continue;
            }
        }
        if let Some(ref status) = params.filters.status {
            if meta
                .and_then(|m| m.status.as_ref())
                .map_or(true, |s| s != status)
            {
                continue;
            }
        }
        if let Some(vmin) = params.filters.value_min {
            if meta
                .and_then(|m| m.value_amount)
                .map_or(true, |v| v < vmin)
            {
                continue;
            }
        }
        if let Some(vmax) = params.filters.value_max {
            if meta
                .and_then(|m| m.value_amount)
                .map_or(true, |v| v > vmax)
            {
                continue;
            }
        }
        if let Some(ref buyer) = params.filters.buyer_name {
            let buyer_lower = buyer.to_lowercase();
            if meta
                .and_then(|m| m.buyer_name.as_ref())
                .map_or(true, |b| !b.to_lowercase().contains(&buyer_lower))
            {
                continue;
            }
        }
        if let Some(ref deadline_before) = params.filters.deadline_before {
            let dl = meta.and_then(|m| {
                m.submission_deadline.as_ref().or(m.deadline.as_ref())
            });
            if dl.map_or(true, |d| d.as_str() > deadline_before.as_str()) {
                continue;
            }
        }
        if let Some(ref deadline_after) = params.filters.deadline_after {
            let dl = meta.and_then(|m| {
                m.submission_deadline.as_ref().or(m.deadline.as_ref())
            });
            if dl.map_or(true, |d| d.as_str() < deadline_after.as_str()) {
                continue;
            }
        }
        if let Some(ref tag) = params.filters.tag {
            if meta.map_or(true, |m| !m.tag.iter().any(|t| t == tag)) {
                continue;
            }
        }
        if let Some(has_awards) = params.filters.has_awards {
            if meta.map_or(true, |m| m.has_awards != has_awards) {
                continue;
            }
        }
        if let Some(eu_funded) = params.filters.eu_funded {
            if meta
                .and_then(|m| m.eu_funded)
                .map_or(true, |v| v != eu_funded)
            {
                continue;
            }
        }
        if let Some(ref nuts) = params.filters.location_nuts {
            if meta.map_or(true, |m| {
                !m.location_nuts
                    .iter()
                    .any(|n| n.starts_with(nuts.as_str()))
            }) {
                continue;
            }
        }

        enriched.push(TenderMatch {
            rank: 0, // assigned after dedup
            doc_id: result.doc_id.clone(),
            ocid: result.ocid.clone(),
            url: meta.and_then(|m| m.notice_id.as_ref()).map(|id| notice_url(id)),
            chunk_type: result.chunk_type.clone(),
            text: truncate_text(&result.text, 200),
            cpv_codes: result.cpv_codes.clone(),
            score: result.score,
            title: meta.and_then(|m| m.title.clone()),
            buyer_name: meta.and_then(|m| m.buyer_name.clone()),
            procurement_method: meta.and_then(|m| m.procurement_method.clone()),
            main_procurement_category: meta.and_then(|m| m.main_procurement_category.clone()),
            value_amount: meta.and_then(|m| m.value_amount),
            value_currency: meta.and_then(|m| m.value_currency.clone()),
            deadline: meta.and_then(|m| {
                m.submission_deadline.clone().or(m.deadline.clone())
            }),
            documents_url: meta.and_then(|m| m.documents_url.clone()),
        });
    }

    // Dedup by OCID (keep highest score)
    let mut best_by_ocid: HashMap<String, TenderMatch> = HashMap::new();
    for m in enriched {
        let entry = best_by_ocid.entry(m.ocid.clone());
        entry
            .and_modify(|existing| {
                if m.score > existing.score {
                    *existing = m.clone();
                }
            })
            .or_insert(m);
    }

    let mut deduped: Vec<TenderMatch> = best_by_ocid.into_values().collect();
    deduped.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    deduped.truncate(k);

    // Assign ranks
    for (i, m) in deduped.iter_mut().enumerate() {
        m.rank = i + 1;
    }

    let response = serde_json::json!({
        "profile_id": params.profile_id,
        "count": deduped.len(),
        "matches": deduped,
    });
    super::to_json_string(&response)
}
