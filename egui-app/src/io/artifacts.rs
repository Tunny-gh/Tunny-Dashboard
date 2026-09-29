//! Asynchronous dispatch for scanning the Artifacts folder.
//!
//! Pure logic such as path validation, Journal metadata parsing, and legacy layout
//! scanning has already been moved to `tunny_core::io::artifacts` since it has no
//! dependency on egui. This module handles asynchronous execution via
//! `spawn_task` and sends `AppMessage`, and formats file URIs for egui's loader.

pub use tunny_core::io::artifacts::{
    parse_artifact_metadata, resolve_from_metadata, scan_legacy_layout, ArtifactEntry,
    ArtifactFileType,
};

/// Builds a file URI in the form expected by egui_extras' file loader.
pub fn file_image_uri(path: &std::path::Path) -> Option<String> {
    let path = path.to_str()?;
    #[cfg(windows)]
    {
        let path = path.replace('\\', "/");
        if let Some(unc_path) = path.strip_prefix("//") {
            Some(format!("file://{unc_path}"))
        } else {
            Some(format!("file:///{path}"))
        }
    }
    #[cfg(not(windows))]
    {
        Some(format!("file://{path}"))
    }
}

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
    scan_id: u64,
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
            scan_id: Some(scan_id),
        }
    });
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn image_uri_uses_windows_file_loader_paths() {
        assert_eq!(
            file_image_uri(std::path::Path::new(r"C:\artifacts\trial 0.png")),
            Some("file:///C:/artifacts/trial 0.png".to_string())
        );
        assert_eq!(
            file_image_uri(std::path::Path::new(r"\\server\share\trial.png")),
            Some("file://server/share/trial.png".to_string())
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn image_uri_uses_posix_file_loader_paths() {
        assert_eq!(
            file_image_uri(std::path::Path::new("/artifacts/trial.png")),
            Some("file:///artifacts/trial.png".to_string())
        );
    }

    #[test]
    fn image_uri_loads_a_local_png_through_egui() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trial 0.png");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let uri = file_image_uri(&path).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match ctx.try_load_image(&uri, egui::load::SizeHint::default()) {
                Ok(egui::load::ImagePoll::Ready { image }) => {
                    assert_eq!(image.size, [1, 1]);
                    break;
                }
                Ok(egui::load::ImagePoll::Pending { .. }) => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(err) => panic!("failed to load {uri}: {err}"),
            }
        }
    }

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
            scan_id: Some(1),
        })
        .unwrap();

        match rx.recv().unwrap() {
            AppMessage::ArtifactsDirScanned {
                trial_artifacts: received,
                artifacts_dir: received_dir,
                scan_id,
            } => {
                assert_eq!(received.len(), 1);
                assert_eq!(received.get(&0).unwrap()[0].filename, "result.png");
                assert_eq!(received_dir, artifacts_dir);
                assert_eq!(scan_id, Some(1));
            }
            _ => panic!("unexpected message type"),
        }
    }
}
