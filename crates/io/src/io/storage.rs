//! Storage acquisition returns owned updates. Management/application is caller-owned.
use crate::{
    journal::parser,
    rdb::{self, RdbUrl},
    sqlite,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tdashboard_study::{StudyBatch, StudyKey};

#[derive(Clone, Debug)]
pub enum ReadSource {
    Journal(Arc<Vec<u8>>),
    Csv { bytes: Arc<Vec<u8>>, name: String },
    Sqlite(PathBuf),
    Rdb(RdbUrl),
}
impl ReadSource {
    pub fn read(
        &self,
        key: StudyKey,
        streaming: bool,
        sink: impl FnMut(StudyBatch) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut sink = sink;
        match self {
            Self::Journal(bytes) if streaming => {
                parser::study::read_batches(bytes, key, 1000, sink)
            }
            Self::Journal(bytes) => sink(parser::study::read_full(bytes, key)?),
            Self::Csv { bytes, name } => {
                if key.stored_id != 0 {
                    return Err(format!("study_id {} not found in CSV", key.stored_id));
                }
                sink(crate::flat_csv::parse_flat_csv(bytes, name)?.into_batch(key))
            }
            Self::Sqlite(path) => {
                sink(sqlite::parse_single_study_rows(path, key.stored_id)?.into_batch(key))
            }
            Self::Rdb(url) => {
                sink(rdb::parse_single_study_rows_url(url, key.stored_id)?.into_batch(key))
            }
        }
    }
}
pub fn is_journal(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("log") | Some("journal")
    )
}
pub fn scan_studies(storage: &str) -> Result<Vec<parser::StudyMeta>, String> {
    if let Some(url) = RdbUrl::parse(storage) {
        return rdb::scan_study_list_url(&url);
    }
    let path = Path::new(storage);
    if is_journal(path) {
        parser::scan_study_list(&read_journal(path)?)
    } else {
        sqlite::scan_study_list(path)
    }
}
pub fn load_study(storage: &str, key: StudyKey) -> Result<(StudyBatch, String), String> {
    if let Some(url) = RdbUrl::parse(storage) {
        let display = url.masked();
        return Ok((
            rdb::parse_single_study_rows_url(&url, key.stored_id)?.into_batch(key),
            display,
        ));
    }
    let path = Path::new(storage);
    let batch = if is_journal(path) {
        parser::study::read_full(&read_journal(path)?, key)?
    } else {
        sqlite::parse_single_study_rows(path, key.stored_id)?.into_batch(key)
    };
    Ok((batch, storage.to_string()))
}
pub fn read_journal(path: &Path) -> Result<Vec<u8>, String> {
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
        let key = tdashboard_study::StudyKey {
            source: tdashboard_study::IdentityAllocator::new()
                .unwrap()
                .source()
                .unwrap(),
            stored_id: 1,
        };
        assert!(scan_studies("/nonexistent/definitely_missing.db").is_err());
        assert!(load_study("/nonexistent/x.db", key).is_err());
    }

    #[test]
    fn journal_error_does_not_echo_storage_string() {
        let storage = "postgresqll://user:secret_pw@host/audit.journal";
        let err = scan_studies(storage).unwrap_err();
        assert!(!err.contains("secret_pw"), "leak: {err}");
        assert!(!err.contains(storage), "leak: {err}");
    }
}
