//! Centralizes dispatch on storage kind (journal / SQLite / RDB URL).
//!
//! The same detection rules had drifted across mcp-server, examples, and egui-app
//! as separate copies, so they are consolidated here. Detection rules:
//! 1. If [`RdbUrl::parse`] recognizes it as a URL, it's PostgreSQL / MySQL
//! 2. If the extension is `.log` / `.journal`, it's an Optuna journal file
//! 3. Otherwise, it's a local SQLite file
//!
//! ## Handling of credentials
//!
//! The display storage name returned for an RDB URL always goes through
//! [`RdbUrl::masked`]. Error messages never embed the raw storage string —
//! even a string that fails to parse as a URL may contain a password (e.g. a
//! typo'd scheme such as `postgresqll://user:pass@...`), so it is never echoed
//! back regardless of the code path taken.

use std::path::Path;

use crate::data::dataframe::DataFrame;
use crate::data::extras::StudyExtras;
use crate::io::journal::parser as journal_parser;
use crate::io::journal::parser::StudyMeta;
use crate::io::rdb::{parse_single_study_url, scan_study_list_url, RdbUrl};
use crate::io::sqlite;

/// Retrieves the list of studies from a storage string.
pub fn scan_studies(storage: &str) -> Result<Vec<StudyMeta>, String> {
    if let Some(url) = RdbUrl::parse(storage) {
        return scan_study_list_url(&url);
    }
    let path = Path::new(storage);
    if is_journal(path) {
        let data = read_journal(path)?;
        return journal_parser::scan_study_list(&data);
    }
    sqlite::scan_study_list(path)
}

/// Loads a single study from a storage string.
///
/// The 4th element of the return value is the display storage name (RDB URLs have their password masked).
pub fn load_study(
    storage: &str,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras, String), String> {
    if let Some(url) = RdbUrl::parse(storage) {
        let masked = url.masked();
        let (meta, df, extras) = parse_single_study_url(&url, study_id)?;
        return Ok((meta, df, extras, masked));
    }
    let path = Path::new(storage);
    if is_journal(path) {
        let data = read_journal(path)?;
        let (meta, df, extras) = journal_parser::parse_single_study(&data, study_id)?;
        return Ok((meta, df, extras, storage.to_string()));
    }
    let (meta, df, extras) = sqlite::parse_single_study(path, study_id)?;
    Ok((meta, df, extras, storage.to_string()))
}

/// Whether the extension identifies it as a journal file.
pub fn is_journal(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("log") | Some("journal")
    )
}

/// Reads a journal file. Does not embed the path string in the error
/// (see the module doc's credentials-handling policy).
fn read_journal(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("failed to read journal file: {e}"))
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
