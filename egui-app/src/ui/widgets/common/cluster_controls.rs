//! Common logic for the clustering settings UI and compute-queue submission (D-3).
//!
//! The 4 widgets — 2D / 3D cluster scatter plots, the cluster table, and the Artifact
//! gallery — all share the same clustering settings (k / Max k / target space /
//! k-selection mode / Init strategy) and execution state (computing / pending / error),
//! and used to duplicate the same control-row UI and "Run clicked -> validation ->
//! queue submission" flow.
//!
//! Each widget needs to keep its settings and execution state as flat fields (since
//! they're referenced and taken directly from the outside, e.g. by tests or workers),
//! so instead of owning the values together, this module takes a bundle of mutable
//! references to those fields ([`ClusterControls`]) and shares only the UI drawing and
//! queue-submission logic.

use crate::state::messages::ClusterUiError;
use crate::ui::widgets::cluster_scatter::{
    validate_cluster_request, ClusterComputeRequest, ClusterSpace, KMeansInitStrategy,
    KSelectionMode,
};

/// A bundle of mutable references to the settings and execution state that the
/// cluster control UI edits.
///
/// The 4 widgets share the same field layout, but fields are kept flat because tests
/// and workers reference and `take` individual fields directly. Each widget builds
/// this bundle from its own fields on every call and delegates to `show_controls` /
/// `try_queue_compute`.
pub struct ClusterControls<'a> {
    pub k: &'a mut usize,
    pub target_space: &'a mut ClusterSpace,
    pub k_mode: &'a mut KSelectionMode,
    pub init_strategy: &'a mut KMeansInitStrategy,
    /// Upper bound on k explored in Elbow (automatic) mode.
    pub elbow_max_k: &'a mut usize,
    pub computing: &'a mut bool,
    pub pending_compute: &'a mut Option<ClusterComputeRequest>,
    pub last_error: &'a mut Option<ClusterUiError>,
}

impl ClusterControls<'_> {
    /// Draws the clustering settings UI (k / Max k / mode / space / Init / Run) in a
    /// single row.
    ///
    /// - `count` is the target point count (often the Pareto front point count) used
    ///   as the basis for k / Max k and run eligibility.
    /// - `id_prefix` is used to avoid `id_salt` collisions across the 3 ComboBoxes
    ///   (unique per widget); assigns `"{id_prefix}_k_mode"` / `"{id_prefix}_space"` /
    ///   `"{id_prefix}_init"`.
    /// - When `inline_spinner` is true, a spinner is shown to the right of Run while
    ///   running (2D scatter plots show their own spinner separately, so they pass
    ///   false).
    ///
    /// On Run click, calls `try_queue_compute`, which pushes to pending if validation
    /// passes.
    pub fn show_controls(
        &mut self,
        ui: &mut egui::Ui,
        count: usize,
        id_prefix: &str,
        inline_spinner: bool,
    ) {
        ui.horizontal(|ui| {
            let k_editable = !*self.computing && *self.k_mode == KSelectionMode::Manual;
            ui.label("k:");
            ui.add_enabled(
                k_editable,
                egui::DragValue::new(&mut *self.k).range(2..=count.max(2)),
            );

            let elbow_max_k_editable =
                !*self.computing && *self.k_mode == KSelectionMode::ElbowDefault;
            ui.label("Max k:");
            ui.add_enabled(
                elbow_max_k_editable,
                egui::DragValue::new(&mut *self.elbow_max_k).range(2..=50),
            );

            egui::ComboBox::from_id_salt(format!("{id_prefix}_k_mode"))
                .selected_text(self.k_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        self.k_mode,
                        KSelectionMode::ElbowDefault,
                        KSelectionMode::ElbowDefault.label(),
                    );
                    ui.selectable_value(
                        self.k_mode,
                        KSelectionMode::Manual,
                        KSelectionMode::Manual.label(),
                    );
                });

            egui::ComboBox::from_id_salt(format!("{id_prefix}_space"))
                .selected_text(self.target_space.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        self.target_space,
                        ClusterSpace::Objective,
                        ClusterSpace::Objective.label(),
                    );
                    ui.selectable_value(
                        self.target_space,
                        ClusterSpace::Variable,
                        ClusterSpace::Variable.label(),
                    );
                    ui.selectable_value(
                        self.target_space,
                        ClusterSpace::Combined,
                        ClusterSpace::Combined.label(),
                    );
                });

            ui.label("Init:");
            egui::ComboBox::from_id_salt(format!("{id_prefix}_init"))
                .selected_text(self.init_strategy.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        self.init_strategy,
                        KMeansInitStrategy::KMeansPlusPlus,
                        KMeansInitStrategy::KMeansPlusPlus.label(),
                    );
                    ui.selectable_value(
                        self.init_strategy,
                        KMeansInitStrategy::Deterministic,
                        KMeansInitStrategy::Deterministic.label(),
                    );
                });

            if ui
                .add_enabled(!*self.computing, egui::Button::new("Run"))
                .clicked()
            {
                self.try_queue_compute(count);
            }

            if inline_spinner && *self.computing {
                ui.spinner();
                ui.label("Running clustering...");
            }
        });
    }

    /// Builds a clustering request from the current settings and validates it,
    /// pushing to pending on success. On validation failure, clears pending and sets
    /// the error (same behavior as the previous per-widget implementations).
    pub fn try_queue_compute(&mut self, count: usize) {
        let request = ClusterComputeRequest {
            k: *self.k,
            target_space: *self.target_space,
            k_mode: *self.k_mode,
            init_strategy: *self.init_strategy,
            elbow_max_k: *self.elbow_max_k,
        };

        match validate_cluster_request(&request, count) {
            Ok(()) => {
                *self.pending_compute = Some(request);
                *self.computing = true;
                *self.last_error = None;
            }
            Err(err) => {
                *self.pending_compute = None;
                *self.last_error = Some(err);
            }
        }
    }
}
