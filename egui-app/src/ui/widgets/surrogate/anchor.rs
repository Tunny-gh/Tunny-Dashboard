//! アンカー点（候補設計点）解決の共有ヘルパー。
//!
//! ロバスト性解析（`robustness.rs`）と応答曲面 3D ビューア（`response_surface.rs`）の
//! 両方が、Best trial または pin 留めした trial を中心点として使う。元は
//! `robustness.rs` にあった `CenterChoice` をここへ移し、両ウィジェットで共有する。
//!
//! `CenterChoice` はセッションファイル（`.tunny`）に永続化される設定のため、
//! variant 名（`BestTrial` / `Pinned`）は変更しない（型パスが変わっても serde の
//! シリアライズ形式は variant 名ベースなので互換性は保たれる）。

use crate::state::types::{Direction, StudyView};
use tunny_core::surrogate_opt::TrainedSurrogate;

/// アンカー点（候補設計点）の選び方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CenterChoice {
    /// 選択目的のベスト観測 trial。
    #[default]
    BestTrial,
    /// pin 留めした trial（trial_id）。消えている場合は BestTrial にフォールバックする。
    Pinned(u32),
}

/// Center/Anchor コンボのラベル。Pinned が指す trial が既に存在しない場合は
/// 実際に使われる中心点（フォールバック後）と食い違わないよう素直に "Best trial" と表示する。
pub fn center_label(choice: CenterChoice, view: &StudyView) -> String {
    match choice {
        CenterChoice::BestTrial => "Best trial".to_string(),
        CenterChoice::Pinned(id) => match view.trial_ids.iter().position(|&t| t == id) {
            Some(row) => {
                let number = view.df.get_trial_number(row).unwrap_or(id);
                format!("Trial #{number}")
            }
            None => "Best trial".to_string(),
        },
    }
}

/// 選択目的の観測ベスト行（方向を考慮した argmin/argmax）を返す。
pub fn best_trial_row(
    view: &StudyView,
    obj_names: &[String],
    directions: &[Direction],
    objective_name: &str,
) -> Option<usize> {
    let obj_idx = obj_names.iter().position(|n| n == objective_name)?;
    let col = view.numeric_column(objective_name)?;
    let minimize = directions
        .get(obj_idx)
        .map(|d| matches!(d, Direction::Minimize))
        .unwrap_or(true);

    let mut best_row = None;
    let mut best_val = if minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for i in 0..view.row_count() {
        let Some(v) = col.get(i).copied() else {
            continue;
        };
        if !v.is_finite() {
            continue;
        }
        let better = if minimize { v < best_val } else { v > best_val };
        if better {
            best_val = v;
            best_row = Some(i);
        }
    }
    best_row
}

/// 中心点を元単位のベクトル（`trained.param_names` と同順）として解決する。
/// Pinned trial が消えている場合は Best trial にフォールバックする。
pub fn resolve_center(
    trained: &TrainedSurrogate,
    choice: CenterChoice,
    view: &StudyView,
    obj_names: &[String],
    directions: &[Direction],
) -> Option<Vec<f64>> {
    let row = match choice {
        CenterChoice::Pinned(id) => view.trial_ids.iter().position(|&t| t == id),
        CenterChoice::BestTrial => None,
    }
    .or_else(|| best_trial_row(view, obj_names, directions, &trained.objective_name))?;

    trained
        .param_names
        .iter()
        .map(|name| view.numeric_column(name)?.get(row).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_choice_default_is_best_trial() {
        assert_eq!(CenterChoice::default(), CenterChoice::BestTrial);
    }

    #[test]
    fn center_choice_serde_round_trip_keeps_variant_names() {
        // セッションファイル互換性の回帰防止: variant 名が JSON に出ることを確認する。
        let best = serde_json::to_string(&CenterChoice::BestTrial).unwrap();
        assert_eq!(best, "\"BestTrial\"");
        let pinned = serde_json::to_string(&CenterChoice::Pinned(7)).unwrap();
        assert_eq!(pinned, "{\"Pinned\":7}");
    }
}
