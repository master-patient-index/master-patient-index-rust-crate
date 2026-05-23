//! Loco application wiring.
//!
//! Implements [`Hooks`] to boot the app, register routes, and attach the
//! existing Axum REST API plus the new HTML web UI via `after_routes`.

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result as LocoResult,
    app::{AppContext, Hooks},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    controller::{AppRoutes, views::engines::TeraView},
    environment::Environment,
    errors::Error as LocoError,
    task::Tasks,
};
use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

use crate::api::rest::{AppState, create_router as create_api_router};
use crate::config::Config as MpiConfig;
use crate::db::{AuditLogRepository, PatientRepository, SeaOrmPatientRepository};
use crate::matching::{PatientMatcher, ProbabilisticMatcher};
use crate::search::SearchEngine;
use crate::streaming::{EventProducer, InMemoryEventPublisher};
use crate::web;

/// Empty migrator. Our SQL migrations live under `migrations/` and are run
/// out-of-band via `sea-orm-cli migrate up`; the Loco migrator is not used.
pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

/// The Loco application.
pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        "master-patient-index"
    }

    fn app_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn boot(mode: StartMode, environment: &Environment) -> LocoResult<BootResult> {
        create_app::<Self, Migrator>(mode, environment).await
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        // All HTTP routes are mounted via `after_routes` so they can share
        // construction of the existing REST API state.
        AppRoutes::with_default_routes()
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> LocoResult<AxumRouter> {
        let services = AppServices::build(ctx)?;

        let api_state = services.api_state();
        let api_router = create_api_router(api_state);

        let web_state = services.web_state()?;
        let web_router = web::router(web_state);

        Ok(router.merge(api_router).merge(web_router))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> LocoResult<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut Tasks) {}

    async fn truncate(_db: &DatabaseConnection) -> LocoResult<()> {
        Ok(())
    }

    async fn seed(_db: &DatabaseConnection, _base: &std::path::Path) -> LocoResult<()> {
        Ok(())
    }
}

/// Bundle of shared services used by both the REST API and the HTML web UI.
struct AppServices {
    db: sea_orm::DatabaseConnection,
    patient_repository: Arc<dyn PatientRepository>,
    event_publisher: Arc<dyn EventProducer>,
    audit_log: Arc<AuditLogRepository>,
    search_engine: Arc<SearchEngine>,
    matcher: Arc<dyn PatientMatcher>,
    config: Arc<MpiConfig>,
}

impl AppServices {
    fn build(ctx: &AppContext) -> LocoResult<Self> {
        let mpi_config = MpiConfig::from_env()
            .map_err(|e| LocoError::string(&format!("config load failed: {e}")))?;

        let search_engine = Arc::new(
            SearchEngine::new(&mpi_config.search.index_path)
                .map_err(|e| LocoError::string(&format!("search engine init failed: {e}")))?,
        );

        let matcher: Arc<dyn PatientMatcher> =
            Arc::new(ProbabilisticMatcher::new(mpi_config.matching.clone()));

        let event_publisher: Arc<dyn EventProducer> = Arc::new(InMemoryEventPublisher::new());
        let audit_log = Arc::new(AuditLogRepository::new(ctx.db.clone()));
        let patient_repository: Arc<dyn PatientRepository> = Arc::new(
            SeaOrmPatientRepository::new(ctx.db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone()),
        );

        Ok(Self {
            db: ctx.db.clone(),
            patient_repository,
            event_publisher,
            audit_log,
            search_engine,
            matcher,
            config: Arc::new(mpi_config),
        })
    }

    fn api_state(&self) -> AppState {
        AppState {
            db: self.db.clone(),
            patient_repository: self.patient_repository.clone(),
            event_publisher: self.event_publisher.clone(),
            audit_log: self.audit_log.clone(),
            search_engine: self.search_engine.clone(),
            matcher: self.matcher.clone(),
            config: self.config.clone(),
        }
    }

    fn web_state(&self) -> LocoResult<web::WebState> {
        let views = TeraView::build()
            .map_err(|e| LocoError::string(&format!("Tera view init failed: {e}")))?;
        Ok(web::WebState {
            patient_repository: self.patient_repository.clone(),
            search_engine: self.search_engine.clone(),
            matcher: self.matcher.clone(),
            views: Arc::new(views),
        })
    }
}
