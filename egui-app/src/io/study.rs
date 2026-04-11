use crate::state::app_state::{
    Direction, GpuBufferData, StudyMeta, TrialRow, TrialState,
};
use crate::state::messages::AppMessage;
use std::collections::HashMap;

// parse_journal を同スレッドで再実行するために io::file を使用
use tunny_core::io::journal::parser;

/// 完了試行数が最多の Study を自動選択する（REQ-021 準拠）
pub fn auto_select_study(studies: &[StudyMeta]) -> Option<&StudyMeta> {
    studies.iter().max_by_key(|s| s.completed_trials)
}

/// rust_core の GpuBufferData (colors なし) → egui-app の GpuBufferData (colors あり) に変換
/// Pareto ランクに基づいて colors を計算する
pub fn build_gpu_buffer_data(
    core_gpu: tunny_core::dataframe::GpuBufferData,
    pareto_ranks: &[u32],
) -> GpuBufferData {
    let n = core_gpu.trial_count;
    let max_rank = pareto_ranks.iter().max().copied().unwrap_or(1).max(1);

    // RGBA colors: Pareto rank 0 = vivid, higher = dimmer
    let mut colors = Vec::with_capacity(n * 4);
    for i in 0..n {
        let rank = pareto_ranks.get(i).copied().unwrap_or(max_rank);
        let t = 1.0 - (rank as f32 / (max_rank + 1) as f32);
        // Simple blue-to-yellow color scale based on pareto rank
        let r = t;
        let g = 0.5 + t * 0.5;
        let b = 1.0 - t;
        let a = 0.8_f32;
        colors.push(r);
        colors.push(g);
        colors.push(b);
        colors.push(a);
    }

    GpuBufferData {
        positions: core_gpu.positions,
        positions3d: core_gpu.positions3d,
        colors,
        sizes: core_gpu.sizes,
        trial_count: n as u32,
    }
}

/// rust_core の with_active_df から TrialRow を取得
fn extract_trial_rows(_meta: &StudyMeta) -> Vec<TrialRow> {
    tunny_core::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let obj_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        (0..n)
            .map(|row| {
                let params: HashMap<String, f64> = param_names
                    .iter()
                    .map(|name| {
                        let val = df
                            .get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0);
                        (name.clone(), val)
                    })
                    .collect();

                let objectives: Vec<f64> = obj_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0)
                    })
                    .collect();

                let trial_id = df.get_trial_id(row).unwrap_or(row as u32);

                TrialRow {
                    trial_id,
                    params,
                    objectives,
                    pareto_rank: 0, // filled in later
                    cluster_id: None,
                    state: TrialState::Complete,
                    user_attrs: HashMap::new(),
                }
            })
            .collect()
    })
    .unwrap_or_default()
}

/// ジャーナルファイルを再パースしてから select_study を実行する。
/// thread_local の GLOBAL_STATE はスレッドをまたいで共有されないため、
/// パースと選択を必ず同一スレッドで行う必要がある。
pub fn load_and_select_task(
    path: std::path::PathBuf,
    meta: StudyMeta,
) -> AppMessage {
    match crate::io::file::read_journal_file(&path) {
        Ok(data) => match parser::parse_journal(&data) {
            Ok(_) => select_study_task(meta),
            Err(e) => AppMessage::Error(e),
        },
        Err(e) => AppMessage::Error(e),
    }
}

/// バックグラウンドで select_study を実行し AppMessage を返す
pub fn select_study_task(meta: StudyMeta) -> AppMessage {
    let is_minimize: Vec<bool> = meta
        .directions
        .iter()
        .map(|d| matches!(d, Direction::Minimize))
        .collect();

    match tunny_core::dataframe::select_study(meta.study_id) {
        Ok(result) => {
            // Pareto ランク計算
            let pareto = tunny_core::pareto::compute_pareto_ranks(&is_minimize);
            let ranks = pareto.ranks.clone();
            let pareto_indices = pareto.pareto_indices;

            // GPU バッファデータ構築
            let gpu_data = build_gpu_buffer_data(result.gpu_buffer_data, &ranks);

            // Trial rows を with_active_df から取得
            let mut trial_rows = extract_trial_rows(&meta);
            // Pareto ランクを TrialRow に設定
            for (i, row) in trial_rows.iter_mut().enumerate() {
                row.pareto_rank = ranks.get(i).copied().unwrap_or(0);
            }

            AppMessage::StudySelected {
                meta,
                trial_rows,
                gpu_data,
                pareto_indices,
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::Direction;

    fn make_study(id: u32, name: &str, completed: usize) -> StudyMeta {
        StudyMeta {
            study_id: id,
            name: name.to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: completed,
            total_trials: completed,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
        }
    }

    #[test]
    fn auto_select_study_empty() {
        let studies: Vec<StudyMeta> = vec![];
        assert!(auto_select_study(&studies).is_none());
    }

    #[test]
    fn auto_select_study_picks_most_completed() {
        let studies = vec![
            make_study(0, "a", 5),
            make_study(1, "b", 100),
            make_study(2, "c", 10),
        ];
        let selected = auto_select_study(&studies).unwrap();
        assert_eq!(selected.study_id, 1);
        assert_eq!(selected.name, "b");
    }

    #[test]
    fn auto_select_study_single() {
        let studies = vec![make_study(0, "only", 3)];
        let selected = auto_select_study(&studies).unwrap();
        assert_eq!(selected.study_id, 0);
    }

    #[test]
    fn build_gpu_buffer_data_color_length() {
        let core_gpu = tunny_core::dataframe::GpuBufferData {
            positions: vec![0.0, 0.0, 1.0, 1.0], // 2 points × 2 coords
            positions3d: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0], // 2 points × 3 coords
            sizes: vec![1.0, 1.0],
            trial_count: 2,
        };
        let pareto_ranks = [0u32, 1u32];
        let gpu = build_gpu_buffer_data(core_gpu, &pareto_ranks);
        // 2 points × 4 RGBA components
        assert_eq!(gpu.colors.len(), 8);
        assert_eq!(gpu.trial_count, 2);
        assert_eq!(gpu.positions.len(), 4);
    }

    #[test]
    fn build_gpu_buffer_data_pareto_front_has_different_color() {
        let core_gpu = tunny_core::dataframe::GpuBufferData {
            positions: vec![0.0, 0.0, 1.0, 1.0],
            positions3d: vec![],
            sizes: vec![1.0, 1.0],
            trial_count: 2,
        };
        let pareto_ranks = [0u32, 2u32]; // rank 0 = front, rank 2 = dominated
        let gpu = build_gpu_buffer_data(core_gpu, &pareto_ranks);
        // First point (rank 0) should have different red component than second (rank 2)
        let r0 = gpu.colors[0];
        let r1 = gpu.colors[4];
        assert!(
            (r0 - r1).abs() > 0.01,
            "Pareto front and dominated should have different colors"
        );
    }

    #[test]
    fn select_study_task_invalid_id_returns_error() {
        let meta = make_study(999, "nonexistent", 0);
        let msg = select_study_task(meta);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error for nonexistent study"),
        }
    }
}
