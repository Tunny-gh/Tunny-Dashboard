use super::*;

use std::collections::HashSet;

/// Whether the toolbar Reload button is pressable.
///
/// Flat CSV is excluded: it is a one-time import whose optimization directions
/// and variable ranges are supplied by the user through the import dialog
/// rather than read from the file, so re-reading it would mean re-answering
/// that dialog instead of picking up new trials.
pub fn can_reload(app_state: &AppState, is_loading: bool) -> bool {
    !is_loading
        && app_state.current_study.is_some()
        && app_state
            .journal_path
            .as_deref()
            .is_some_and(|p| !crate::io::flat_csv::is_csv_path(p))
}

/// UI state carried across a toolbar Reload.
///
/// Reload deliberately re-runs the same scan → select path as opening the
/// file, which is what makes it authoritative: the worker thread drops its
/// cached journal bytes and its `loaded_study_ids`, so the study is genuinely
/// re-read from storage rather than re-activated from the in-memory snapshot.
/// The cost is that the path resets view state through [`AppState::clear`], so
/// everything the user chose (rather than the app derived) is captured here
/// beforehand and re-applied once the study has finished loading.
///
/// Every restored id here is a trial_id, not a row index, so trials appended
/// since the last read never invalidate them. Ids whose trial is no longer in
/// the storage are dropped on restore rather than kept as dangling entries.
pub(super) struct ReloadRestore {
    /// The study to bring back up once the re-scan reports the study list.
    study_id: u32,
    /// study_ids of the comparison studies, in the order they were added (the
    /// order decides their assigned color, so it has to be preserved).
    comparison_study_ids: Vec<u32>,
    comparison_mode: bool,
    comparison_base_study: Option<u32>,
    selected_indices: Vec<u32>,
    pinned_trials: Vec<u32>,
    highlighted_trial: Option<u32>,
    filter_ranges: HashMap<String, (f64, f64)>,
    artifacts_dir: Option<std::path::PathBuf>,
    hv_ref_point_override: Option<Vec<f64>>,
    /// Set once the post-scan study re-selection has been dispatched, so a
    /// later scan result (e.g. the user opening another file mid-reload)
    /// cannot dispatch it a second time.
    reselect_dispatched: bool,
}

impl ReloadRestore {
    /// Captures the state to carry across a reload. `None` when no study is
    /// active, in which case there is nothing to reload in the first place.
    fn capture(app_state: &AppState) -> Option<Self> {
        let study_id = app_state.current_study.as_ref()?.meta.study_id;
        Some(Self {
            study_id,
            comparison_study_ids: app_state
                .comparison_studies
                .iter()
                .map(|c| c.meta.study_id)
                .collect(),
            comparison_mode: app_state.comparison_mode,
            comparison_base_study: app_state.comparison_base_study,
            selected_indices: app_state.selected_indices.clone(),
            pinned_trials: app_state.pinned_trials.clone(),
            highlighted_trial: app_state.highlighted_trial,
            filter_ranges: app_state.filter_ranges.clone(),
            artifacts_dir: app_state.artifacts_dir.clone(),
            hv_ref_point_override: app_state.hv_ref_point_override.clone(),
            reselect_dispatched: false,
        })
    }

    /// Re-applies the captured state to the freshly reloaded study, dropping
    /// anything that refers to a trial the storage no longer has.
    ///
    /// When a filter is active, the selection is recomputed from the filter
    /// instead of being restored verbatim: `set_filter` is the only writer of
    /// `filter_ranges` and it always derives the selection from it, so a
    /// non-empty map means the selection *was* the filter result — and trials
    /// added since the last read should fall inside it too.
    fn apply(self, app_state: &mut AppState) -> Vec<u32> {
        let live: HashSet<u32> = app_state
            .current_study
            .as_ref()
            .map(|study| study.view.trial_ids.iter().copied().collect())
            .unwrap_or_default();
        let keep = |ids: Vec<u32>| -> Vec<u32> {
            ids.into_iter().filter(|id| live.contains(id)).collect()
        };

        app_state.pinned_trials = keep(self.pinned_trials);
        app_state.highlighted_trial = self.highlighted_trial.filter(|id| live.contains(id));
        app_state.artifacts_dir = self.artifacts_dir;
        app_state.hv_ref_point_override = self.hv_ref_point_override;
        app_state.comparison_mode = self.comparison_mode;
        app_state.comparison_base_study = self.comparison_base_study;

        let had_filter = !self.filter_ranges.is_empty();
        app_state.filter_ranges = self.filter_ranges;
        if had_filter {
            app_state.apply_filters();
        } else {
            app_state.selected_indices = keep(self.selected_indices);
        }

        self.comparison_study_ids
    }
}

impl TunnyApp {
    /// Re-reads the currently open storage and rebuilds the view from it.
    ///
    /// This is the toolbar Reload action. It is a no-op while another load is
    /// in flight, so repeated clicks cannot stack up overlapping scans.
    pub(super) fn reload_current(&mut self) {
        if !can_reload(&self.app_state, self.is_loading) {
            return;
        }
        let Some(path) = self.app_state.journal_path.clone() else {
            return;
        };
        let Some(restore) = ReloadRestore::capture(&self.app_state) else {
            return;
        };
        // The comparison views are rebuilt from storage once the base study
        // lands (their study_ids live in `restore`), so drop the stale ones now
        // to keep the three parallel Vecs aligned as they come back in order.
        self.app_state.comparison_studies.clear();
        self.app_state.comparison_colors.clear();
        self.app_state.comparison_convergence_histories.clear();
        self.pending_reload = Some(restore);
        self.is_loading = true;
        self.load_error = None;
        dispatch_scan(path, self.sender());
    }

    /// Reacts to the study list arriving from a reload's re-scan.
    ///
    /// Returns `true` when the reload owns this scan result, in which case the
    /// caller skips the usual post-open handling (auto-selecting a lone study,
    /// opening the CSV import dialog): the study to bring back up is the one
    /// that was on screen, not whatever the fresh-open heuristics would pick.
    pub(super) fn continue_reload_after_scan(&mut self) -> bool {
        let Some(restore) = self.pending_reload.as_mut() else {
            return false;
        };
        if restore.reselect_dispatched {
            return false;
        }
        let Some(meta) = self
            .app_state
            .all_studies
            .iter()
            .find(|s| s.study_id == restore.study_id)
            .cloned()
        else {
            // The study is no longer in the storage (file replaced, study
            // dropped). Abandon the reload and let the normal open handling
            // decide what to show.
            self.pending_reload = None;
            return false;
        };
        restore.reselect_dispatched = true;
        self.is_loading = true;
        crate::io::study_worker::dispatch_select_study(meta, self.sender());
        true
    }

    /// Finishes a reload once the re-selected study has finished loading:
    /// re-applies the captured view state and rebuilds the comparison studies
    /// from storage.
    ///
    /// A load that completes while the reload's own re-scan is still in flight
    /// belongs to whatever was already running, not to the reload, so the
    /// captured state is only applied once the re-selection has been dispatched.
    pub(super) fn finish_reload(&mut self) {
        if !self
            .pending_reload
            .as_ref()
            .is_some_and(|r| r.reselect_dispatched)
        {
            return;
        }
        let Some(restore) = self.pending_reload.take() else {
            return;
        };
        let comparison_study_ids = restore.apply(&mut self.app_state);
        if let Some(artifacts_dir) = self.app_state.artifacts_dir.clone() {
            self.request_artifact_scan(artifacts_dir);
        }
        for study_id in comparison_study_ids {
            let Some(meta) = self
                .app_state
                .all_studies
                .iter()
                .find(|s| s.study_id == study_id)
                .cloned()
            else {
                // A comparison study that vanished from the storage is simply
                // dropped from the session rather than reported as an error.
                continue;
            };
            // force: the shared store still holds this study's pre-reload
            // snapshot, which is exactly the stale data the reload exists to
            // replace.
            crate::io::study_worker::dispatch_load_comparison_study(meta, true, self.sender());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{Direction, StudyContext, StudyMeta};
    use crate::state::messages::AppMessage;
    use std::sync::mpsc;

    fn test_app() -> TunnyApp {
        let (tx, rx) = mpsc::sync_channel(32);
        TunnyApp {
            app_state: AppState::new(),
            layout: LayoutState::default(),
            widget_states: WidgetStates::default(),
            canvas_widgets: HashMap::new(),
            is_loading: false,
            load_error: None,
            tx,
            rx,
            pending_reload: None,
            latest_artifact_scan_id: 0,
            reload_when_idle: false,
            current_window_title: None,
            beta_notice: BetaNoticeState::default(),
            beta_notice_ack: None,
        }
    }

    fn meta(study_id: u32) -> StudyMeta {
        StudyMeta {
            study_id,
            name: format!("study{study_id}"),
            directions: vec![Direction::Minimize],
            completed_trials: 0,
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        }
    }

    /// Builds an AppState whose active study exposes `trial_ids` as its rows.
    fn state_with_trials(trial_ids: &[u32]) -> AppState {
        let mut app_state = AppState::new();
        let mut ctx = StudyContext::from_rows_for_test(meta(1), vec![]);
        ctx.view.trial_ids = trial_ids.to_vec();
        app_state.current_study = Some(ctx);
        app_state
    }

    #[test]
    fn can_reload_requires_a_non_csv_source_and_an_idle_app() {
        let mut app_state = state_with_trials(&[0, 1]);
        app_state.journal_path = Some("study.log".into());
        assert!(can_reload(&app_state, false));
        // A load already in flight blocks a second one.
        assert!(!can_reload(&app_state, true));

        // Flat CSV is a one-time import, so it stays unpressable.
        app_state.journal_path = Some("trials.csv".into());
        assert!(!can_reload(&app_state, false));

        // SQLite and DB URLs are both reloadable.
        app_state.journal_path = Some("study.db".into());
        assert!(can_reload(&app_state, false));
        app_state.journal_path = Some("postgresql://u:p@localhost/db".into());
        assert!(can_reload(&app_state, false));

        // Nothing loaded yet -> nothing to reload.
        app_state.current_study = None;
        assert!(!can_reload(&app_state, false));
    }

    #[test]
    fn capture_returns_none_without_an_active_study() {
        assert!(ReloadRestore::capture(&AppState::new()).is_none());
    }

    #[test]
    fn apply_drops_state_pointing_at_trials_that_disappeared() {
        let mut before = state_with_trials(&[10, 11, 12]);
        before.selected_indices = vec![10, 12];
        before.pinned_trials = vec![11, 12];
        before.highlighted_trial = Some(12);
        before.artifacts_dir = Some("/artifacts".into());
        let restore = ReloadRestore::capture(&before).unwrap();

        // Trial 12 is gone from the reloaded study.
        let mut after = state_with_trials(&[10, 11]);
        restore.apply(&mut after);

        assert_eq!(after.selected_indices, vec![10]);
        assert_eq!(after.pinned_trials, vec![11]);
        assert_eq!(after.highlighted_trial, None);
        assert!(after.artifact_map.is_empty());
        // The artifacts folder survives for the scan after study activation.
        assert_eq!(after.artifacts_dir, Some("/artifacts".into()));
    }

    #[test]
    fn apply_keeps_state_that_still_resolves() {
        let mut before = state_with_trials(&[10, 11]);
        before.selected_indices = vec![10, 11];
        before.pinned_trials = vec![10];
        before.highlighted_trial = Some(11);
        before.hv_ref_point_override = Some(vec![1.0, 2.0]);
        before.comparison_mode = true;
        before.comparison_base_study = Some(1);
        let restore = ReloadRestore::capture(&before).unwrap();

        // Two trials were appended by the run that prompted the reload.
        let mut after = state_with_trials(&[10, 11, 12, 13]);
        restore.apply(&mut after);

        assert_eq!(after.selected_indices, vec![10, 11]);
        assert_eq!(after.pinned_trials, vec![10]);
        assert_eq!(after.highlighted_trial, Some(11));
        assert_eq!(after.hv_ref_point_override, Some(vec![1.0, 2.0]));
        assert!(after.comparison_mode);
        assert_eq!(after.comparison_base_study, Some(1));
    }

    #[test]
    fn apply_reports_comparison_studies_in_their_original_order() {
        let mut before = state_with_trials(&[0]);
        before.comparison_studies = vec![
            StudyContext::from_rows_for_test(meta(7), vec![]),
            StudyContext::from_rows_for_test(meta(3), vec![]),
        ];
        let restore = ReloadRestore::capture(&before).unwrap();

        let mut after = state_with_trials(&[0]);
        // Order decides the assigned comparison color, so it must round-trip.
        assert_eq!(restore.apply(&mut after), vec![7, 3]);
    }

    #[test]
    fn finishing_reload_scans_new_and_existing_trial_artifacts_authoritatively() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["10_old.png", "10_new.png", "12_new.png"] {
            std::fs::write(dir.path().join(name), b"image").unwrap();
        }

        let mut before = state_with_trials(&[10, 11]);
        before.journal_path = Some(dir.path().join("study.log"));
        before.artifacts_dir = Some(dir.path().to_path_buf());
        before.artifact_map.insert(10, vec![]);
        before.artifact_map.insert(11, vec![]); // Removed from the source.
        before.selected_indices = vec![10];
        let mut restore = ReloadRestore::capture(&before).unwrap();
        restore.reselect_dispatched = true;

        let mut app = test_app();
        app.app_state = state_with_trials(&[10, 11, 12]);
        app.app_state.journal_path = before.journal_path;
        app.pending_reload = Some(restore);
        app.finish_reload();

        let scanned = app
            .rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(matches!(
            &scanned,
            AppMessage::ArtifactsDirScanned {
                scan_id: Some(1),
                ..
            }
        ));
        app.tx.send(scanned).unwrap();
        app.poll_messages(&egui::Context::default());

        assert_eq!(app.app_state.selected_indices, vec![10]);
        assert_eq!(app.app_state.artifacts_dir.as_deref(), Some(dir.path()));
        assert_eq!(app.app_state.artifact_map[&10].len(), 2);
        assert_eq!(app.app_state.artifact_map[&12].len(), 1);
        assert!(!app.app_state.artifact_map.contains_key(&11));

        // A previous asynchronous scan completing later must not restore old data.
        app.tx
            .send(AppMessage::ArtifactsDirScanned {
                trial_artifacts: HashMap::from([(11, vec![])]),
                artifacts_dir: dir.path().to_path_buf(),
                scan_id: Some(0),
            })
            .unwrap();
        app.poll_messages(&egui::Context::default());
        assert_eq!(app.app_state.artifact_map[&12].len(), 1);
        assert!(!app.app_state.artifact_map.contains_key(&11));
    }

    #[test]
    fn finishing_reload_without_an_artifacts_folder_does_not_scan() {
        let mut before = state_with_trials(&[10]);
        before.journal_path = Some("study.log".into());
        let mut restore = ReloadRestore::capture(&before).unwrap();
        restore.reselect_dispatched = true;

        let mut app = test_app();
        app.app_state = state_with_trials(&[10, 11]);
        app.app_state.journal_path = before.journal_path;
        app.pending_reload = Some(restore);
        app.finish_reload();

        assert!(app.app_state.artifacts_dir.is_none());
        assert_eq!(app.latest_artifact_scan_id, 0);
        assert!(app.rx.try_recv().is_err());
    }
}
