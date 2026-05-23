//! Binary entry point — runs the Loco CLI for the MPI app.
//!
//! Common commands:
//!   master-patient-index start            # serve HTTP
//!   master-patient-index doctor           # diagnose config
//!   master-patient-index routes           # list routes

use loco_rs::cli;
use master_patient_index::app::{App, Migrator};

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
