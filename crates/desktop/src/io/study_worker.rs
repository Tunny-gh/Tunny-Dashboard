//! Scan dispatch only. No global worker store, installation or analysis.
use crate::state::messages::AppMessage;
use std::path::PathBuf;
use std::sync::{mpsc::SyncSender, Arc};
use tdashboard_io::{journal::parser::JournalSnapshot, storage::ReadSource};

pub fn dispatch_scan_journal(
    path: PathBuf,
    source: tdashboard_study::SourceInstanceId,
    scan: u64,
    tx: SyncSender<AppMessage>,
) {
    std::thread::spawn(move || {
        let result = (|| {
            let (read_source, studies) = if let Some(url) = crate::io::rdb::path_as_rdb_url(&path) {
                let studies = tdashboard_io::rdb::scan_study_list_url(&url)?;
                (ReadSource::Rdb(url), studies)
            } else if crate::io::sqlite::is_sqlite_path(&path) {
                (
                    ReadSource::Sqlite(path.clone()),
                    tdashboard_io::sqlite::scan_study_list(&path)?,
                )
            } else if crate::io::flat_csv::is_csv_path(&path) {
                let bytes = Arc::new(tdashboard_io::storage::read_journal(&path)?);
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("csv");
                let meta = tdashboard_io::flat_csv::parse_flat_csv(&bytes, name)?.meta;
                (
                    ReadSource::Csv {
                        bytes,
                        name: name.to_string(),
                    },
                    vec![meta],
                )
            } else {
                let snapshot = JournalSnapshot::new(tdashboard_io::storage::read_journal(&path)?)?;
                let studies = snapshot.studies().to_vec();
                (ReadSource::Journal(Arc::new(snapshot)), studies)
            };
            Ok((
                read_source,
                studies
                    .into_iter()
                    .map(crate::io::journal::convert_study_meta)
                    .collect(),
            ))
        })();
        let _ = tx.send(AppMessage::JournalScanned {
            source,
            scan,
            path,
            result,
        });
    });
}
