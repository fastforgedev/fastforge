//! Everything that touches the developer's machine.
//!
//! `studio_core` decides what things mean; this module is the only
//! place that reads or writes them.

pub mod catalog;
pub mod config;
pub mod env;
pub mod listing;
pub mod packaging;
pub mod paths;
pub mod project;
pub mod registry;

use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};

/// The wire format for every timestamp Studio emits.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn rfc3339(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Secs, true)
}
