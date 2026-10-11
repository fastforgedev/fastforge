//! The wire contract: response envelope, capability reporting, and the request
//! bodies both hosts accept.

mod capabilities;
mod dto;
mod envelope;
mod validate;

pub use capabilities::{Capabilities, StudioMode, capabilities_for};
pub use dto::{
    CreateProjectRequest, CreateStoreAppRequest, PutCatalogFileRequest, PutPackagingFileRequest,
    UpdateProjectRequest, UpdateStoreAppRequest,
};
pub use envelope::{ApiError, error_envelope, success_envelope};
pub use validate::{validate_identifier, validate_project_name};
