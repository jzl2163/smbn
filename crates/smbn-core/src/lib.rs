//! Shared configuration, validation, and IPC types for SMBN.
#![forbid(unsafe_code)]

pub mod config;
pub mod protocol;
pub mod validation;

pub use config::*;
pub use protocol::*;
pub use validation::{validate_config, ConfigIssue, IssueSeverity};
