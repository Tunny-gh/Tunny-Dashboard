//! セッション（プロジェクト）ファイルの保存・復元。
//!
//! キャンバスレイアウト・各ウィジェット設定・グローバル表示設定を JSON として
//! 保存し、後で復元する。データ本体（journal / SQLite / CSV）は保存しない —
//! 復元後に別のデータセットを開いても可視化構成が維持されることが狙い。
//! 列参照の不整合（存在しない列名・範囲外インデックス）は Study 切替と同じ
//! 経路で各ウィジェットがフォールバックするため、復元側での検証は行わない。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::state::app_state::{AppState, ColormapName};
use crate::state::layout_state::LayoutState;
use crate::ui::help::help_types::HelpLanguage;
use crate::ui::widget_states::WidgetStates;
use tunny_core::indicators::MoIndicator;

/// セッションファイルのフォーマットバージョン。
/// 後方互換の追加（フィールド追加）ではインクリメントせず、
/// 互換性が壊れる変更時のみ上げる。
pub const SESSION_VERSION: u32 = 1;

/// セッションファイルの拡張子。中身は素の JSON なので、独自拡張子ではなく
/// `.json` を使う（何のファイルか一目で分かり、エディタでも開ける）。
/// ファイル名の慣例は `*-session.json`。
pub const SESSION_EXTENSION: &str = "json";

/// AppState のうち「どう見せるか」を表すグローバル表示設定。
/// 「どのデータを見るか」（journal_path / study 選択 / 比較セッション）は
/// 意図的に含めない。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViewSettings {
    pub selected_colormap: ColormapName,
    /// 列名 → (min, max)。存在しない列のエントリはフィルタ適用時に無視される。
    pub filter_ranges: HashMap<String, (f64, f64)>,
    /// ピン留め trial ID。復元先のデータに存在しない ID は各ウィジェットが無視する。
    pub pinned_trials: Vec<u32>,
    pub hv_ref_point_override: Option<Vec<f64>>,
    pub convergence_indicator: MoIndicator,
    pub help_language: HelpLanguage,
    /// ダークテーマか（`#[serde(default)]` により旧セッションはライト扱い）。
    pub dark_mode: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            selected_colormap: ColormapName::Viridis,
            filter_ranges: HashMap::new(),
            pinned_trials: Vec::new(),
            hv_ref_point_override: None,
            convergence_indicator: MoIndicator::Hypervolume,
            help_language: HelpLanguage::default(),
            dark_mode: false,
        }
    }
}

impl ViewSettings {
    /// 現在の AppState からセッションに保存すべき表示設定を抜き出す。
    pub fn capture(app_state: &AppState) -> Self {
        Self {
            selected_colormap: app_state.selected_colormap.clone(),
            filter_ranges: app_state.filter_ranges.clone(),
            pinned_trials: app_state.pinned_trials.clone(),
            hv_ref_point_override: app_state.hv_ref_point_override.clone(),
            convergence_indicator: app_state.convergence_indicator,
            help_language: app_state.help_language,
            dark_mode: app_state.dark_mode,
        }
    }

    /// 表示設定を AppState へ書き戻す。
    /// HV 参照点の変更は収束履歴の再計算をトリガーするため無効化する。
    pub fn apply(self, app_state: &mut AppState) {
        app_state.selected_colormap = self.selected_colormap;
        app_state.filter_ranges = self.filter_ranges;
        app_state.pinned_trials = self.pinned_trials;
        if app_state.hv_ref_point_override != self.hv_ref_point_override {
            app_state.hv_ref_point_override = self.hv_ref_point_override;
            app_state.convergence_history = None;
        }
        app_state.convergence_indicator = self.convergence_indicator;
        app_state.help_language = self.help_language;
        app_state.dark_mode = self.dark_mode;
    }
}

/// セッションファイル本体（読み込み側）。
/// `WidgetStates` は `Clone` を持たないため、書き込み側は参照を束ねた
/// `SessionFileRef` を内部で使い、この型は復元専用とする。
#[derive(serde::Deserialize)]
pub struct SessionFile {
    /// フォーマットバージョン。`SESSION_VERSION` より新しいファイルは読み込みを拒否する。
    pub version: u32,
    /// キャンバスレイアウト（アイテム配置・pan/zoom・パネル状態）。
    pub layout: LayoutState,
    /// キャンバスアイテム ID → ウィジェット設定。
    /// ランタイム状態（計算結果・キャッシュ）は `#[serde(skip)]` で除外されており、
    /// 復元後の初回描画で再計算される。
    pub widgets: HashMap<u64, WidgetStates>,
    /// グローバル表示設定。
    pub view: ViewSettings,
}

/// 現在のアプリ状態をクローンなしで JSON 化する。
pub fn to_json(
    layout: &LayoutState,
    widgets: &HashMap<u64, WidgetStates>,
    view: &ViewSettings,
) -> Result<String, String> {
    #[derive(serde::Serialize)]
    struct SessionFileRef<'a> {
        version: u32,
        layout: &'a LayoutState,
        widgets: &'a HashMap<u64, WidgetStates>,
        view: &'a ViewSettings,
    }
    serde_json::to_string_pretty(&SessionFileRef {
        version: SESSION_VERSION,
        layout,
        widgets,
        view,
    })
    .map_err(|e| format!("Failed to serialize session: {e}"))
}

/// JSON 文字列からセッションを復元する。
/// 未知のフィールドは無視し、欠落フィールドは既定値で補う（前方互換）。
pub fn from_json(text: &str) -> Result<SessionFile, String> {
    let session: SessionFile =
        serde_json::from_str(text).map_err(|e| format!("Failed to parse session file: {e}"))?;
    if session.version > SESSION_VERSION {
        return Err(format!(
            "Session file version {} is newer than supported version {}. \
             Please update the application.",
            session.version, SESSION_VERSION
        ));
    }
    Ok(session)
}

/// セッションを指定パスへ書き込む。
pub fn write_session_to_path(
    layout: &LayoutState,
    widgets: &HashMap<u64, WidgetStates>,
    view: &ViewSettings,
    path: &Path,
) -> Result<(), String> {
    let json = to_json(layout, widgets, view)?;
    crate::io::file::write_atomic(path, json.as_bytes())
        .map_err(|e| format!("Failed to write session file: {e}"))
}

/// 指定パスからセッションを読み込む。
pub fn read_session_from_path(path: &Path) -> Result<SessionFile, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read session file: {e}"))?;
    from_json(&text)
}

/// ファイル保存ダイアログを開いてセッションを保存する。
/// キャンセル時は `Ok(None)`、成功時は保存先パスを返す。
pub fn save_session_dialog(
    layout: &LayoutState,
    widgets: &HashMap<u64, WidgetStates>,
    view: &ViewSettings,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Tunny Dashboard Session (*.json)", &[SESSION_EXTENSION])
        .set_file_name("tunny-session.json")
        .save_file()
    else {
        return Ok(None);
    };
    write_session_to_path(layout, widgets, view, &path)?;
    Ok(Some(path))
}

/// セッションファイル選択ダイアログを開く（キャンセル時は `None`）。
/// 読み込み・適用は呼び出し側（`apply_toolbar_actions`）が行う。
pub fn pick_session_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Tunny Dashboard Session (*.json)", &[SESSION_EXTENSION])
        .pick_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_json() -> String {
        to_json(
            &LayoutState::default(),
            &HashMap::new(),
            &ViewSettings::default(),
        )
        .unwrap()
    }

    #[test]
    fn roundtrip_preserves_version_and_defaults() {
        let restored = from_json(&make_json()).unwrap();
        assert_eq!(restored.version, SESSION_VERSION);
        assert!(restored.widgets.is_empty());
    }

    #[test]
    fn newer_version_is_rejected() {
        let mut value: serde_json::Value = serde_json::from_str(&make_json()).unwrap();
        value["version"] = serde_json::json!(SESSION_VERSION + 1);
        match from_json(&value.to_string()) {
            Err(err) => assert!(err.contains("newer than supported")),
            Ok(_) => panic!("newer version must be rejected"),
        }
    }

    #[test]
    fn missing_view_fields_fall_back_to_defaults() {
        // 将来のフィールド追加を想定: view が空オブジェクトでも読めること
        let mut value: serde_json::Value = serde_json::from_str(&make_json()).unwrap();
        value["view"] = serde_json::json!({});
        let restored = from_json(&value.to_string()).unwrap();
        assert_eq!(restored.view.selected_colormap, ColormapName::Viridis);
        assert!(restored.view.pinned_trials.is_empty());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let mut value: serde_json::Value = serde_json::from_str(&make_json()).unwrap();
        value["future_field"] = serde_json::json!({"nested": true});
        let restored = from_json(&value.to_string()).unwrap();
        assert_eq!(restored.version, SESSION_VERSION);
    }

    #[test]
    fn widget_settings_survive_roundtrip() {
        let mut widgets = WidgetStates::default();
        widgets.histogram.selected_col = "Obj".to_string();
        widgets.histogram.manual_bins = 42;
        widgets.pareto_2d.x_axis = "x".to_string();
        widgets.pareto_2d.y_axis = "y".to_string();
        widgets.opt_history.log_scale = true;
        widgets.opt_history.window_size = 25;

        let mut layout = LayoutState::default();
        layout.canvas.pan_x = 12.5;
        layout.canvas.zoom = 1.5;

        let mut map = HashMap::new();
        map.insert(7u64, widgets);
        let json = to_json(&layout, &map, &ViewSettings::default()).unwrap();
        let restored = from_json(&json).unwrap();

        assert_eq!(restored.layout.canvas.pan_x, 12.5);
        assert_eq!(restored.layout.canvas.zoom, 1.5);
        let w = restored.widgets.get(&7).expect("item 7 restored");
        assert_eq!(w.histogram.selected_col, "Obj");
        assert_eq!(w.histogram.manual_bins, 42);
        assert_eq!(w.pareto_2d.x_axis, "x");
        assert_eq!(w.pareto_2d.y_axis, "y");
        assert!(w.opt_history.log_scale);
        assert_eq!(w.opt_history.window_size, 25);
    }

    #[test]
    fn view_settings_capture_apply_roundtrip() {
        let mut app_state = AppState::new();
        app_state.selected_colormap = ColormapName::Turbo;
        app_state.pinned_trials = vec![3, 8];
        app_state.filter_ranges.insert("x".to_string(), (0.5, 2.5));
        app_state.hv_ref_point_override = Some(vec![10.0, 20.0]);

        let view = ViewSettings::capture(&app_state);
        let mut fresh = AppState::new();
        view.apply(&mut fresh);

        assert_eq!(fresh.selected_colormap, ColormapName::Turbo);
        assert_eq!(fresh.pinned_trials, vec![3, 8]);
        assert_eq!(fresh.filter_ranges.get("x"), Some(&(0.5, 2.5)));
        assert_eq!(fresh.hv_ref_point_override, Some(vec![10.0, 20.0]));
        // HV 参照点が変わったので収束履歴は無効化される
        assert!(fresh.convergence_history.is_none());
    }

    #[test]
    fn write_and_read_roundtrip_via_file() {
        let dir = std::env::temp_dir().join("tunny_session_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip-session.json");
        write_session_to_path(
            &LayoutState::default(),
            &HashMap::new(),
            &ViewSettings::default(),
            &path,
        )
        .unwrap();
        let restored = read_session_from_path(&path).unwrap();
        assert_eq!(restored.version, SESSION_VERSION);
        let _ = std::fs::remove_file(&path);
    }
}
