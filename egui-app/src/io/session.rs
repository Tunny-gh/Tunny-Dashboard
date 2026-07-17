//! Save / restore of session (project) files.
//!
//! Saves the canvas layout, each widget's settings, and the global view
//! settings as JSON, to be restored later. The underlying data (journal /
//! SQLite / CSV) is not saved — the intent is that the visualization
//! configuration is preserved even if a different dataset is opened after
//! restoring. Inconsistent column references (nonexistent column names,
//! out-of-range indices) are handled by each widget's fallback via the same
//! path used for Study switching, so no validation is performed on the
//! restore side.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::state::app_state::{AppState, ColormapName};
use crate::state::layout_state::LayoutState;
use crate::ui::help::help_types::HelpLanguage;
use crate::ui::widget_states::WidgetStates;
use tunny_core::indicators::MoIndicator;

/// Session file format version.
/// Not incremented for backward-compatible additions (adding fields);
/// bumped only when making a breaking change.
pub const SESSION_VERSION: u32 = 1;

/// Session file extension. Since the content is plain JSON, this uses
/// `.json` rather than a custom extension (so it's obvious what kind of
/// file it is, and it can still be opened in an editor). The file naming
/// convention is `*-session.json`.
pub const SESSION_EXTENSION: &str = "json";

/// The subset of `AppState` representing "how to display" as a global view
/// setting. "Which data to view" (journal_path / study selection /
/// comparison session) is intentionally excluded.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViewSettings {
    pub selected_colormap: ColormapName,
    /// column name -> (min, max). Entries for nonexistent columns are
    /// ignored when the filter is applied.
    pub filter_ranges: HashMap<String, (f64, f64)>,
    /// Pinned trial IDs. IDs that don't exist in the restored data are
    /// ignored by each widget.
    pub pinned_trials: Vec<u32>,
    pub hv_ref_point_override: Option<Vec<f64>>,
    pub convergence_indicator: MoIndicator,
    pub help_language: HelpLanguage,
    /// Whether the dark theme is active (older sessions are treated as
    /// light via `#[serde(default)]`).
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
    /// Extracts the view settings to save into the session from the
    /// current AppState.
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

    /// Writes the view settings back into AppState.
    /// Invalidates the convergence history if the HV reference point
    /// changed, since that requires recomputation.
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

/// The session file body (read side).
/// Since `WidgetStates` does not implement `Clone`, the write side
/// internally uses `SessionFileRef`, which bundles references; this type
/// is dedicated to restoring.
#[derive(serde::Deserialize)]
pub struct SessionFile {
    /// Format version. Files newer than `SESSION_VERSION` are rejected on load.
    pub version: u32,
    /// Canvas layout (item placement, pan/zoom, panel state).
    pub layout: LayoutState,
    /// Canvas item ID -> widget settings.
    /// Runtime state (computation results, caches) is excluded via
    /// `#[serde(skip)]` and is recomputed on the first render after
    /// restoring.
    pub widgets: HashMap<u64, WidgetStates>,
    /// Global view settings.
    pub view: ViewSettings,
}

/// Serializes the current app state to JSON without cloning.
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

/// Restores a session from a JSON string.
/// Unknown fields are ignored, and missing fields fall back to defaults
/// (forward compatibility).
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

/// Writes the session to the specified path.
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

/// Reads the session from the specified path.
pub fn read_session_from_path(path: &Path) -> Result<SessionFile, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read session file: {e}"))?;
    from_json(&text)
}

/// Opens a file save dialog and saves the session.
/// Returns `Ok(None)` on cancel, or the destination path on success.
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

/// Opens a session file picker dialog (returns `None` on cancel).
/// Loading and applying is done by the caller (`apply_toolbar_actions`).
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
        // Anticipates future field additions: view must still load even as an empty object.
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
        // The convergence history is invalidated because the HV reference point changed.
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
