//! Asynchronous dispatch for scanning the Artifacts folder.
//!
//! Pure logic such as path validation, Journal metadata parsing, and legacy layout
//! scanning has already been moved to `tunny_core::io::artifacts` since it has no
//! dependency on egui. This module only handles asynchronous execution via
//! `spawn_task` and sending `AppMessage`.

pub use tunny_core::io::artifacts::{
    parse_artifact_metadata, resolve_from_metadata, scan_legacy_layout, ArtifactEntry,
    ArtifactFileType,
};

// ============================================================
// scan_artifacts_dir
// ============================================================

/// Scans the `artifacts/` folder and groups artifacts by trial_id.
/// Sends `AppMessage::ArtifactsDirScanned` on completion (REQ-007-A/C).
///
/// Primary path: resolves `trial_id <-> artifact_id` from `journal_path`'s metadata
/// (`artifacts:<id>`) and maps it to the actual file at `base_dir/<artifact_id>`.
/// Fallback: only when there's no metadata, infers a legacy layout like
/// `artifacts/<trial_id>/file` from the leading number in the file name.
pub fn scan_artifacts_dir(
    base_dir: std::path::PathBuf,
    journal_path: Option<std::path::PathBuf>,
    tx: std::sync::mpsc::SyncSender<crate::state::messages::AppMessage>,
) {
    crate::app::spawn_task(tx, move || {
        let meta_by_trial = journal_path
            .as_deref()
            .map(parse_artifact_metadata)
            .unwrap_or_default();

        let trial_artifacts = if meta_by_trial.is_empty() {
            scan_legacy_layout(&base_dir)
        } else {
            resolve_from_metadata(&base_dir, &meta_by_trial)
        };

        crate::state::messages::AppMessage::ArtifactsDirScanned {
            trial_artifacts,
            artifacts_dir: base_dir,
        }
    });
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task2128_artifacts_dir_scanned_message_channel() {
        use crate::state::messages::AppMessage;
        use std::collections::HashMap;
        use std::path::PathBuf;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::sync_channel::<AppMessage>(8);

        let mut trial_artifacts: HashMap<u32, Vec<ArtifactEntry>> = HashMap::new();
        trial_artifacts.insert(
            0,
            vec![ArtifactEntry {
                path: PathBuf::from("/tmp/artifacts/abc123"),
                filename: "result.png".into(),
                mimetype: "image/png".into(),
            }],
        );
        let artifacts_dir = PathBuf::from("/tmp/artifacts");

        tx.send(AppMessage::ArtifactsDirScanned {
            trial_artifacts: trial_artifacts.clone(),
            artifacts_dir: artifacts_dir.clone(),
        })
        .unwrap();

        match rx.recv().unwrap() {
            AppMessage::ArtifactsDirScanned {
                trial_artifacts: received,
                artifacts_dir: received_dir,
            } => {
                assert_eq!(received.len(), 1);
                assert_eq!(received.get(&0).unwrap()[0].filename, "result.png");
                assert_eq!(received_dir, artifacts_dir);
            }
            _ => panic!("unexpected message type"),
        }
    }
}
