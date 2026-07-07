pub mod anchor;
pub mod compare;
pub mod response_surface;
pub mod robustness;
pub mod surrogate_opt;

use tunny_core::surrogate_opt::SurrogateModelKind;

/// モデル選択肢（コンボ表示順）。`surrogate_opt` / `robustness` / `response_surface` の
/// 3 ウィジェットで共有する単一情報源。新モデル追加時はここだけ更新すれば全コンボへ反映される
/// （以前は各ファイルに同じ配列が重複しており、更新漏れの温床だった）。
pub(crate) const MODEL_CHOICES: [SurrogateModelKind; 5] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::GpMoe,
    SurrogateModelKind::Lgbm,
];
