use crate::state::app_state::{Direction, GpuBufferData, StudyMeta};
use crate::state::messages::AppMessage;

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
    let max_rank = pareto_ranks.iter().max().copied().unwrap_or(0);

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

/// バックグラウンドで select_study を実行し AppMessage を返す。
///
/// 行指向 `Vec<TrialRow>` は複製せず、`study_id` と Pareto ランクのみを送る（MEM-001）。
/// UI 側は `StudySelected` 受信時に共有ストアから `Arc<DataFrame>` を取得し
/// `StudyView` を構築する。
pub fn select_study_task(meta: StudyMeta) -> AppMessage {
    let is_minimize: Vec<bool> = meta
        .directions
        .iter()
        .map(|d| matches!(d, Direction::Minimize))
        .collect();

    let study_id = meta.study_id;
    match tunny_core::dataframe::select_study(study_id) {
        Ok(result) => {
            let pareto = tunny_core::pareto::compute_pareto_ranks(&is_minimize);
            let pareto_indices = pareto.pareto_indices;
            let gpu_data = build_gpu_buffer_data(result.gpu_buffer_data, &pareto.ranks);
            AppMessage::StudySelected {
                meta,
                study_id,
                pareto_rank: pareto.ranks,
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
