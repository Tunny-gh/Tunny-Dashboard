//! Scan dispatch only. No global worker store, installation or analysis.
use crate::state::messages::AppMessage;
use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

pub fn dispatch_scan_journal(
    path: PathBuf,
    source: tdashboard_study::SourceInstanceId,
    scan: u64,
    tx: SyncSender<AppMessage>,
) {
    std::thread::spawn(move || {
        let result = (|| {
            let (bytes, studies) = if let Some(url) = crate::io::rdb::path_as_rdb_url(&path) {
                (Vec::new(), tdashboard_io::rdb::scan_study_list_url(&url)?)
            } else if crate::io::sqlite::is_sqlite_path(&path) {
                (Vec::new(), tdashboard_io::sqlite::scan_study_list(&path)?)
            } else {
                let bytes = tdashboard_io::storage::read_journal(&path)?;
                let studies = if crate::io::flat_csv::is_csv_path(&path) {
                    vec![
                        tdashboard_io::flat_csv::parse_flat_csv(
                            &bytes,
                            path.file_stem().and_then(|s| s.to_str()).unwrap_or("csv"),
                        )?
                        .meta,
                    ]
                } else {
                    tdashboard_io::journal::parser::scan_study_list(&bytes)?
                };
                (bytes, studies)
            };
            Ok((
                bytes,
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
