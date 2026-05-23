//! HTML page handlers for the web UI.
//!
//! Renders Tera templates (Lily Design System markup) and handles form
//! submissions for patient CRUD, match, and merge flows.

use axum::extract::{Form, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use chrono::{NaiveDate, Utc};
use loco_rs::controller::views::ViewRenderer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::WebState;
use crate::models::{
    ContactPoint, ContactPointSystem, Gender, HumanName, NameUse, Patient, PatientLink,
};

/// Render a template to an HTML response.
fn render(state: &WebState, key: &str, ctx: Value) -> Response {
    match state.views.render(key, ctx) {
        Ok(body) => Html(body).into_response(),
        Err(err) => {
            tracing::error!(template = key, ?err, "template render failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {err}"),
            )
                .into_response()
        }
    }
}

/// Project a Patient into a Tera-friendly JSON shape used by the list views.
fn patient_row(p: &Patient) -> Value {
    let id_full = p.id.to_string();
    let id_short = id_full
        .split('-')
        .next()
        .unwrap_or(id_full.as_str())
        .to_string();
    json!({
        "id": id_full,
        "id_short": id_short,
        "family_name": p.name.family,
        "given_names": p.name.given.join(" "),
        "birth_date": p.birth_date.map(|d| d.to_string()),
        "gender": format!("{:?}", p.gender),
        "active": p.active,
    })
}

/// Project a Patient into a richer Tera shape used by detail/edit views.
fn patient_view(p: &Patient) -> Value {
    let mut v = patient_row(p);
    v["tax_id"] = json!(p.tax_id);
    v["identifiers"] = json!(
        p.identifiers
            .iter()
            .map(|id| json!({
                "identifier_type": id.identifier_type.to_string(),
                "system": id.system,
                "value": id.value,
            }))
            .collect::<Vec<_>>()
    );
    v["telecom"] = json!(
        p.telecom
            .iter()
            .map(|cp| json!({
                "system": format!("{:?}", cp.system).to_lowercase(),
                "value": cp.value,
                "use_type": cp.use_type.as_ref().map(|u| format!("{:?}", u).to_lowercase()),
            }))
            .collect::<Vec<_>>()
    );
    v["addresses"] = json!(p.addresses);
    v
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

pub async fn home(State(state): State<WebState>) -> Response {
    let total = state
        .patient_repository
        .list_active(1000, 0)
        .await
        .map(|v| v.len())
        .unwrap_or(0);
    render(
        &state,
        "pages/home.html",
        json!({
            "nav": "home",
            "stats": {
                "total": total,
                "active": total,
                "review_queue": 0,
            },
        }),
    )
}

// ---------------------------------------------------------------------------
// Patient list + HTMX search
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    pub q: String,
}

pub async fn patients_index(
    State(state): State<WebState>,
    Query(params): Query<SearchParams>,
) -> Response {
    let patients = if params.q.trim().is_empty() {
        state
            .patient_repository
            .list_active(50, 0)
            .await
            .unwrap_or_default()
    } else {
        state
            .patient_repository
            .search(&params.q)
            .await
            .unwrap_or_default()
    };
    let rows: Vec<Value> = patients.iter().map(patient_row).collect();
    render(
        &state,
        "pages/patients/index.html",
        json!({
            "nav": "patients",
            "patients": rows,
            "q": params.q,
        }),
    )
}

pub async fn patients_search(
    State(state): State<WebState>,
    Query(params): Query<SearchParams>,
) -> Response {
    let patients = if params.q.trim().is_empty() {
        state
            .patient_repository
            .list_active(50, 0)
            .await
            .unwrap_or_default()
    } else {
        state
            .patient_repository
            .search(&params.q)
            .await
            .unwrap_or_default()
    };
    let rows: Vec<Value> = patients.iter().map(patient_row).collect();
    render(
        &state,
        "partials/_patient_rows.html",
        json!({ "patients": rows }),
    )
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

pub async fn patients_new(State(state): State<WebState>) -> Response {
    render(
        &state,
        "pages/patients/new.html",
        json!({ "nav": "patients", "form": {} }),
    )
}

#[derive(Debug, Deserialize)]
pub struct PatientForm {
    pub family: String,
    pub given: String,
    #[serde(default)]
    pub birth_date: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub tax_id: String,
    #[serde(default)]
    pub email: String,
}

fn parse_gender(s: &str) -> Gender {
    match s.to_lowercase().as_str() {
        "female" => Gender::Female,
        "male" => Gender::Male,
        "other" => Gender::Other,
        _ => Gender::Unknown,
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    if s.trim().is_empty() {
        None
    } else {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
    }
}

fn nonempty(s: String) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn build_patient_from_form(form: &PatientForm) -> Patient {
    let given: Vec<String> = form.given.split_whitespace().map(str::to_string).collect();
    let mut p = Patient::new(
        HumanName {
            use_type: Some(NameUse::Official),
            family: form.family.trim().to_string(),
            given,
            prefix: vec![],
            suffix: vec![],
        },
        parse_gender(&form.gender),
    );
    p.birth_date = parse_date(&form.birth_date);
    p.tax_id = nonempty(form.tax_id.clone());
    if let Some(email) = nonempty(form.email.clone()) {
        p.telecom.push(ContactPoint {
            system: ContactPointSystem::Email,
            value: email,
            use_type: None,
        });
    }
    p
}

pub async fn patients_create(
    State(state): State<WebState>,
    Form(form): Form<PatientForm>,
) -> Response {
    let patient = build_patient_from_form(&form);
    match state.patient_repository.create(&patient).await {
        Ok(saved) => {
            let _ = state.search_engine.index_patient(&saved);
            Redirect::to(&format!("/patients/{}", saved.id)).into_response()
        }
        Err(err) => {
            tracing::error!(?err, "patient create failed");
            render(
                &state,
                "pages/patients/new.html",
                json!({
                    "nav": "patients",
                    "form": form_payload(&form),
                    "flash": { "kind": "danger", "message": format!("Could not create patient: {err}") },
                }),
            )
        }
    }
}

fn form_payload(f: &PatientForm) -> Value {
    json!({
        "family": f.family,
        "given": f.given,
        "birth_date": f.birth_date,
        "gender": f.gender,
        "tax_id": f.tax_id,
        "email": f.email,
    })
}

// ---------------------------------------------------------------------------
// Show / Edit / Update / Delete
// ---------------------------------------------------------------------------

pub async fn patients_show(State(state): State<WebState>, Path(id): Path<Uuid>) -> Response {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(p)) => render(
            &state,
            "pages/patients/show.html",
            json!({ "nav": "patients", "patient": patient_view(&p) }),
        ),
        Ok(None) => not_found(&state),
        Err(err) => internal_error(&state, err),
    }
}

pub async fn patients_edit(State(state): State<WebState>, Path(id): Path<Uuid>) -> Response {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(p)) => render(
            &state,
            "pages/patients/edit.html",
            json!({ "nav": "patients", "patient": patient_view(&p) }),
        ),
        Ok(None) => not_found(&state),
        Err(err) => internal_error(&state, err),
    }
}

pub async fn patients_update(
    State(state): State<WebState>,
    Path(id): Path<Uuid>,
    Form(form): Form<PatientForm>,
) -> Response {
    let existing = match state.patient_repository.get_by_id(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(&state),
        Err(err) => return internal_error(&state, err),
    };
    let mut updated = existing;
    updated.name.family = form.family.trim().to_string();
    updated.name.given = form.given.split_whitespace().map(str::to_string).collect();
    updated.birth_date = parse_date(&form.birth_date);
    updated.gender = parse_gender(&form.gender);
    updated.tax_id = nonempty(form.tax_id.clone());
    updated.updated_at = Utc::now();
    match state.patient_repository.update(&updated).await {
        Ok(saved) => {
            let _ = state.search_engine.index_patient(&saved);
            Redirect::to(&format!("/patients/{}", saved.id)).into_response()
        }
        Err(err) => internal_error(&state, err),
    }
}

pub async fn patients_delete_confirm(
    State(state): State<WebState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.patient_repository.get_by_id(&id).await {
        Ok(Some(p)) => render(
            &state,
            "pages/patients/delete.html",
            json!({ "nav": "patients", "patient": patient_view(&p) }),
        ),
        Ok(None) => not_found(&state),
        Err(err) => internal_error(&state, err),
    }
}

pub async fn patients_delete(State(state): State<WebState>, Path(id): Path<Uuid>) -> Response {
    match state.patient_repository.delete(&id).await {
        Ok(()) => {
            let _ = state.search_engine.delete_patient(&id.to_string());
            Redirect::to("/patients").into_response()
        }
        Err(err) => internal_error(&state, err),
    }
}

// ---------------------------------------------------------------------------
// Match
// ---------------------------------------------------------------------------

pub async fn match_form(State(state): State<WebState>) -> Response {
    render(
        &state,
        "pages/patients/match.html",
        json!({ "nav": "match", "form": {} }),
    )
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MatchForm {
    pub family: String,
    #[serde(default)]
    pub given: String,
    #[serde(default)]
    pub birth_date: String,
    #[serde(default)]
    pub threshold: String,
}

pub async fn match_submit(State(state): State<WebState>, Form(form): Form<MatchForm>) -> Response {
    let threshold = form.threshold.parse::<f64>().unwrap_or(0.7);

    // Build a query patient from the form
    let query = Patient::new(
        HumanName {
            use_type: None,
            family: form.family.trim().to_string(),
            given: form.given.split_whitespace().map(str::to_string).collect(),
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Unknown,
    );
    let mut query = query;
    query.birth_date = parse_date(&form.birth_date);

    // Pull candidates via the patient repository's search by family name.
    let candidates = state
        .patient_repository
        .search(&form.family)
        .await
        .unwrap_or_default();

    let mut results: Vec<Value> = Vec::new();
    if let Ok(matches) = state.matcher.find_matches(&query, &candidates) {
        for m in matches.into_iter().filter(|m| m.score >= threshold) {
            let quality = if m.score >= 0.95 {
                "Definite"
            } else if m.score >= threshold {
                "Probable"
            } else if m.score >= 0.5 {
                "Possible"
            } else {
                "Unlikely"
            };
            results.push(json!({
                "patient": patient_row(&m.patient),
                "score_pct": (m.score * 100.0).round() as i64,
                "quality": quality,
            }));
        }
    }

    render(
        &state,
        "pages/patients/match.html",
        json!({
            "nav": "match",
            "form": form,
            "matches": results,
        }),
    )
}

// ---------------------------------------------------------------------------
// Merge
// ---------------------------------------------------------------------------

pub async fn merge_form(State(state): State<WebState>) -> Response {
    render(
        &state,
        "pages/patients/merge.html",
        json!({ "nav": "merge", "form": {} }),
    )
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MergeForm {
    pub master_patient_id: String,
    pub duplicate_patient_id: String,
    pub merge_reason: String,
}

pub async fn merge_submit(State(state): State<WebState>, Form(form): Form<MergeForm>) -> Response {
    let master_id = match Uuid::parse_str(form.master_patient_id.trim()) {
        Ok(u) => u,
        Err(_) => return invalid_uuid(&state, &form, "master_patient_id"),
    };
    let duplicate_id = match Uuid::parse_str(form.duplicate_patient_id.trim()) {
        Ok(u) => u,
        Err(_) => return invalid_uuid(&state, &form, "duplicate_patient_id"),
    };

    let master = match state.patient_repository.get_by_id(&master_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return merge_error(&state, &form, "Master patient not found"),
        Err(err) => return internal_error(&state, err),
    };
    let duplicate = match state.patient_repository.get_by_id(&duplicate_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return merge_error(&state, &form, "Duplicate patient not found"),
        Err(err) => return internal_error(&state, err),
    };

    // Minimal merge: bring across identifiers/addresses/telecom not already present,
    // record the duplicate's name as a former alias, and link master -> Replaces.
    let mut merged = master.clone();
    for id in &duplicate.identifiers {
        if !merged
            .identifiers
            .iter()
            .any(|e| e.value == id.value && e.identifier_type == id.identifier_type)
        {
            merged.identifiers.push(id.clone());
        }
    }
    for addr in &duplicate.addresses {
        merged.addresses.push(addr.clone());
    }
    for cp in &duplicate.telecom {
        if !merged.telecom.iter().any(|e| e.value == cp.value) {
            merged.telecom.push(cp.clone());
        }
    }
    let mut dup_name = duplicate.name.clone();
    dup_name.use_type = Some(NameUse::Old);
    merged.additional_names.push(dup_name);
    if merged.tax_id.is_none() {
        merged.tax_id = duplicate.tax_id.clone();
    }
    merged.links.push(PatientLink {
        other_patient_id: duplicate.id,
        link_type: crate::models::LinkType::Replaces,
    });
    merged.updated_at = Utc::now();

    if let Err(err) = state.patient_repository.update(&merged).await {
        return internal_error(&state, err);
    }
    if let Err(err) = state.patient_repository.delete(&duplicate.id).await {
        tracing::warn!(?err, "soft-delete of duplicate failed");
    }
    let _ = state
        .search_engine
        .delete_patient(&duplicate.id.to_string());
    let _ = state.search_engine.index_patient(&merged);

    tracing::info!(
        master = %merged.id,
        duplicate = %duplicate.id,
        reason = %form.merge_reason,
        "patients merged via web UI"
    );

    render(
        &state,
        "pages/patients/merge.html",
        json!({
            "nav": "merge",
            "form": form,
            "result": {
                "master_id": merged.id.to_string(),
                "duplicate_id": duplicate.id.to_string(),
            },
        }),
    )
}

fn merge_error(state: &WebState, form: &MergeForm, message: &str) -> Response {
    render(
        state,
        "pages/patients/merge.html",
        json!({
            "nav": "merge",
            "form": form,
            "flash": { "kind": "danger", "message": message },
        }),
    )
}

fn invalid_uuid(state: &WebState, form: &MergeForm, field: &str) -> Response {
    let mut resp = render(
        state,
        "pages/patients/merge.html",
        json!({
            "nav": "merge",
            "form": form,
            "flash": { "kind": "danger", "message": format!("Invalid UUID in {field}") },
        }),
    );
    *resp.status_mut() = StatusCode::BAD_REQUEST;
    resp
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

fn not_found(state: &WebState) -> Response {
    let mut resp = render(
        state,
        "pages/patients/index.html",
        json!({
            "nav": "patients",
            "patients": Vec::<Value>::new(),
            "q": "",
            "flash": { "kind": "warning", "message": "Patient not found." },
        }),
    );
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}

fn internal_error(state: &WebState, err: crate::Error) -> Response {
    tracing::error!(?err, "internal error in web handler");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    let body = state
        .views
        .render(
            "pages/patients/index.html",
            json!({
                "nav": "patients",
                "patients": Vec::<Value>::new(),
                "q": "",
                "flash": { "kind": "danger", "message": format!("Server error: {err}") },
            }),
        )
        .unwrap_or_else(|_| format!("Server error: {err}"));
    (StatusCode::INTERNAL_SERVER_ERROR, headers, Html(body)).into_response()
}
