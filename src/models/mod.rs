//! Data models for the MPI system

use serde::{Deserialize, Serialize};

pub mod consent;
pub mod document;
pub mod emergency_contact;
pub mod identifier;
pub mod merge;
pub mod organization;
pub mod patient;
pub mod review_queue;

pub use consent::{Consent, ConsentStatus, ConsentType};
pub use document::{DocumentType, IdentityDocument};
pub use emergency_contact::EmergencyContact;
pub use identifier::{Identifier, IdentifierType, IdentifierUse};
pub use merge::{MergeRecord, MergeRequest, MergeResponse, MergeStatus};
pub use organization::Organization;
pub use patient::{HumanName, LinkType, NameUse, Patient, PatientLink};
pub use review_queue::{
    BatchDeduplicationRequest, BatchDeduplicationResponse, ReviewQueueItem, ReviewStatus,
};

/// Gender enumeration per FHIR specification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Other,
    Unknown,
}

/// Address information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Address {
    pub use_type: Option<AddressUse>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// Address use type
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AddressUse {
    Home,
    Work,
    Temp,
    Old,
    Billing,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContactPoint {
    pub system: ContactPointSystem,
    pub value: String,
    pub use_type: Option<ContactPointUse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointSystem {
    Phone,
    Fax,
    Email,
    Pager,
    Url,
    Sms,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointUse {
    Home,
    Work,
    Temp,
    Old,
    Mobile,
}
