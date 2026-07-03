//! Artifacts フォルダスキャンの非同期ディスパッチ。
//!
//! パス検証・Journal メタデータ解析・レガシーレイアウト走査などの純粋なロジックは
//! egui に依存しないため `tunny_core::io::artifacts` に移設済み。ここでは
//! `spawn_task` を介した非同期実行と `AppMessage` 送信のみを扱う。

pub use tunny_core::io::artifacts::{
    parse_artifact_metadata, resolve_from_metadata, scan_legacy_layout, ArtifactEntry,
    ArtifactFileType,
};

// ============================================================
// scan_artifacts_dir
// ============================================================

/// `artifacts/` フォルダをスキャンし、trial_id 別にアーティファクトをグループ化する。
/// 完了後に `AppMessage::ArtifactsDirScanned` を送信する（REQ-007-A/C）。
///
/// 主経路: `journal_path` のメタデータ（`artifacts:<id>`）から `trial_id ↔ artifact_id` を解決し、
/// `base_dir/<artifact_id>` の実体ファイルを対応付ける。
/// フォールバック: メタデータが無い場合のみ、`artifacts/<trial_id>/file` のような
/// レガシーレイアウトをファイル名の先頭数値から推測する。
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
// テスト
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
            _ => panic!("予期しないメッセージタイプ"),
        }
    }
}
