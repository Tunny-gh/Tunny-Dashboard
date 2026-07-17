//! Storage dispatch (a thin delegation to tunny-core).
//!
//! The detection rules and credential handling (not echoing the storage
//! string into errors, masking the password in RDB URLs) are centralized in
//! `tunny_core::io::storage`. This module only re-exports the 2 functions
//! used by the tool layer.

pub use tunny_core::io::storage::{load_study, scan_studies};
