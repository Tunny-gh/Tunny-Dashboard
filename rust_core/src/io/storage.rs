//! Centralizes dispatch on storage kind (journal / SQLite / RDB URL).
//!
//! The same detection rules had drifted across mcp-server, examples, and egui-app
//! as separate copies, so they are consolidated here. Detection rules:
//! 1. If [`tdashboard_io::rdb::RdbUrl::parse`] recognizes it as a URL, it's PostgreSQL / MySQL
//! 2. If the extension is `.log` / `.journal`, it's an Optuna journal file
//! 3. Otherwise, it's a local SQLite file
//!
//! ## Handling of credentials
//!
//! The display storage name returned for an RDB URL always goes through
//! [`tdashboard_io::rdb::RdbUrl::masked`]. Error messages never embed the raw storage string —
//! even a string that fails to parse as a URL may contain a password (e.g. a
//! typo'd scheme such as `postgresqll://user:pass@...`), so it is never echoed
//! back regardless of the code path taken.

use std::path::Path;

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
use crate::io::journal::parser::StudyMeta;

/// Retrieves the list of studies from a storage string.
pub fn scan_studies(storage: &str) -> Result<Vec<StudyMeta>, String> {
    tdashboard_io::storage::scan_studies(storage)
}

/// Loads a single study from a storage string.
///
/// The 4th element of the return value is the display storage name (RDB URLs have their password masked).
pub fn load_study(
    storage: &str,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras, String), String> {
    use tdashboard_study::*;
    // D7 removes this legacy entry facade; this request owns its manager.
    let key = StudyKey {
        source: IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id: study_id,
    };
    let (batch, display) = tdashboard_io::storage::load_study(storage, key)?;
    let mut manager = StudyManager::new(key);
    let ticket = manager
        .begin(LoadKind::Initial)
        .map_err(|e| e.to_string())?;
    let receipt = manager
        .apply(manager.header(ticket).map_err(|e| e.to_string())?, batch)
        .map_err(|e| e.to_string())?;
    manager
        .finish(ticket, receipt.next_sequence)
        .map_err(|e| e.to_string())?;
    let projection = crate::journal_parser::study_projection::project(
        &manager.acquire().ok_or("No acquired Study")?,
    );
    Ok((
        crate::journal_parser::io_meta(projection.meta),
        projection.dataframe,
        projection.extras,
        display,
    ))
}

/// Whether the extension identifies it as a journal file.
pub fn is_journal(path: &Path) -> bool {
    tdashboard_io::storage::is_journal(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_extension_detection() {
        assert!(is_journal(Path::new("/tmp/study.log")));
        assert!(is_journal(Path::new("optuna.journal")));
        assert!(!is_journal(Path::new("/tmp/study.db")));
        assert!(!is_journal(Path::new("/tmp/study")));
    }

    #[test]
    fn missing_storage_is_error_not_panic() {
        assert!(scan_studies("/nonexistent/definitely_missing.db").is_err());
        assert!(load_study("/nonexistent/x.db", 1).is_err());
    }

    #[test]
    fn journal_error_does_not_echo_storage_string() {
        // Case where a string with a typo'd scheme plus a password ends with a
        // journal extension. RdbUrl::parse returns None → falls into the journal
        // branch → read fails. The error must not contain the password (or the
        // full storage string).
        let storage = "postgresqll://user:secret_pw@host/audit.journal";
        let err = scan_studies(storage).unwrap_err();
        assert!(!err.contains("secret_pw"), "leak: {err}");
        assert!(!err.contains(storage), "leak: {err}");
    }
}
