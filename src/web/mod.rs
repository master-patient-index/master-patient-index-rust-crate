//! HTML web UI: Tera templates + HTMX + Lily Design System
//!
//! Renders patient management pages alongside the JSON REST API. Mounted by
//! the Loco `Hooks::after_routes` callback in [`crate::app`].

use std::sync::Arc;

use axum::Router;
use loco_rs::controller::views::engines::TeraView;

use crate::db::PatientRepository;
use crate::matching::PatientMatcher;
use crate::search::SearchEngine;

pub mod handlers;

#[cfg(test)]
mod tests {
    use loco_rs::controller::views::ViewRenderer;
    use loco_rs::controller::views::engines::TeraView;
    use serde_json::json;

    /// All Tera templates load and render without referencing undefined identifiers.
    #[test]
    fn templates_render() {
        let views = TeraView::from_custom_dir(&"assets/views").expect("view engine builds");

        let home = views
            .render(
                "pages/home.html",
                json!({"nav": "home", "stats": {"total": 0, "active": 0, "review_queue": 0}}),
            )
            .expect("home renders");
        assert!(home.contains("Master Patient Index"));

        let empty_rows: Vec<serde_json::Value> = vec![];
        let list = views
            .render(
                "pages/patients/index.html",
                json!({"nav": "patients", "patients": empty_rows, "q": ""}),
            )
            .expect("patients/index renders");
        assert!(list.contains("Patients"));
        assert!(list.contains("data-table"));

        let new_form = views
            .render(
                "pages/patients/new.html",
                json!({"nav": "patients", "form": {}}),
            )
            .expect("patients/new renders");
        assert!(new_form.contains("Register"));
        assert!(new_form.contains("class=\"text-input\""));

        let match_form = views
            .render(
                "pages/patients/match.html",
                json!({"nav": "match", "form": {}}),
            )
            .expect("patients/match renders");
        assert!(match_form.contains("Find a match"));

        let merge_form = views
            .render(
                "pages/patients/merge.html",
                json!({"nav": "merge", "form": {}}),
            )
            .expect("patients/merge renders");
        assert!(merge_form.contains("Merge"));

        let patient = json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "id_short": "11111111",
            "family_name": "Smith",
            "given_names": "Jane",
            "birth_date": "1980-01-15",
            "gender": "Female",
            "active": true,
            "tax_id": null,
            "identifiers": [],
            "telecom": [],
            "addresses": [],
        });
        let show = views
            .render(
                "pages/patients/show.html",
                json!({"nav": "patients", "patient": patient}),
            )
            .expect("patients/show renders");
        assert!(show.contains("Smith"));
        assert!(show.contains("Jane"));

        let edit = views
            .render(
                "pages/patients/edit.html",
                json!({"nav": "patients", "patient": patient}),
            )
            .expect("patients/edit renders");
        assert!(edit.contains("Edit patient"));
        assert!(edit.contains("class=\"text-input\""));

        let delete = views
            .render(
                "pages/patients/delete.html",
                json!({"nav": "patients", "patient": patient}),
            )
            .expect("patients/delete renders");
        assert!(delete.contains("Delete patient"));
        assert!(delete.contains("class=\"button danger\""));
    }
}

/// Shared services injected into web handlers.
#[derive(Clone)]
pub struct WebState {
    pub patient_repository: Arc<dyn PatientRepository>,
    pub search_engine: Arc<SearchEngine>,
    pub matcher: Arc<dyn PatientMatcher>,
    pub views: Arc<TeraView>,
}

/// Build the web (HTML) router.
pub fn router(state: WebState) -> Router {
    use axum::routing::{get, post};

    Router::new()
        .route("/", get(handlers::home))
        .route("/patients", get(handlers::patients_index))
        .route("/patients", post(handlers::patients_create))
        .route("/patients/search", get(handlers::patients_search))
        .route("/patients/new", get(handlers::patients_new))
        .route("/patients/match", get(handlers::match_form))
        .route("/patients/match", post(handlers::match_submit))
        .route("/patients/merge", get(handlers::merge_form))
        .route("/patients/merge", post(handlers::merge_submit))
        .route("/patients/:id", get(handlers::patients_show))
        .route("/patients/:id/edit", get(handlers::patients_edit))
        .route("/patients/:id/edit", post(handlers::patients_update))
        .route(
            "/patients/:id/delete",
            get(handlers::patients_delete_confirm),
        )
        .route("/patients/:id/delete", post(handlers::patients_delete))
        .with_state(state)
}
