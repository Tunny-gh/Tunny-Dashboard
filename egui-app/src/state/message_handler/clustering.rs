use super::*;

impl MessageHandler {
    pub(super) fn handle_clustering_done(
        source: crate::state::messages::ClusterChartSource,
        key: crate::ui::widgets::cluster_scatter::ClusterCacheKey,
        result: crate::state::results::ClusterResult,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
    ) {
        let trial_count = app_state
            .current_study
            .as_ref()
            .map(|c| c.trial_count())
            .unwrap_or(0);
        if result.labels.len() == trial_count {
            // Cache the result per settings key, shared with other charts using the same settings.
            app_state.cluster_cache.insert(key, result);
            // Clear the spinner / pending state of the chart that started the run.
            Self::clear_cluster_runtime(source, widget_states);
        } else {
            let err = crate::state::messages::cluster_ui_error(
                "Cluster result is inconsistent. Please run again.",
                Some(format!(
                    "validation: labels_len({}) != trial_count({})",
                    result.labels.len(),
                    trial_count
                )),
                true,
            );
            Self::set_cluster_error(source, err, widget_states);
        }
    }

    pub(super) fn handle_cluster_failed(
        source: crate::state::messages::ClusterChartSource,
        err: crate::state::messages::ClusterUiError,
        widget_states: &mut WidgetStates,
    ) {
        Self::set_cluster_error(source, err, widget_states);
    }

    /// Clears the execution state of the widget that started clustering.
    fn clear_cluster_runtime(
        source: crate::state::messages::ClusterChartSource,
        widget_states: &mut WidgetStates,
    ) {
        use crate::state::messages::ClusterChartSource;
        match source {
            ClusterChartSource::Scatter2D => widget_states.cluster_scatter.clear_runtime_state(),
            ClusterChartSource::Scatter3D => widget_states.cluster_scatter_3d.clear_runtime_state(),
            ClusterChartSource::Table => widget_states.trial_table.cluster.clear_runtime_state(),
            ClusterChartSource::ArtifactGallery => {
                widget_states.artifact_gallery.clear_cluster_runtime()
            }
        }
    }

    /// Returns a mutable reference to the controls of the chart that started the MCDM computation.
    pub(super) fn mcdm_controls_mut(
        source: crate::state::messages::McdmChartSource,
        widget_states: &mut WidgetStates,
    ) -> &mut crate::ui::widgets::mcdm_chart::McdmControls {
        use crate::state::messages::McdmChartSource;
        match source {
            McdmChartSource::Rank => &mut widget_states.mcdm_chart.controls,
            McdmChartSource::Scatter2D => &mut widget_states.scatter_chart.controls,
            McdmChartSource::Scatter3D => &mut widget_states.mcdm_scatter_3d.controls,
            McdmChartSource::Table => &mut widget_states.trial_table.mcdm.controls,
            McdmChartSource::ArtifactGallery => &mut widget_states.artifact_gallery.mcdm,
        }
    }

    /// Sets an error on the widget that started clustering.
    fn set_cluster_error(
        source: crate::state::messages::ClusterChartSource,
        err: crate::state::messages::ClusterUiError,
        widget_states: &mut WidgetStates,
    ) {
        use crate::state::messages::ClusterChartSource;
        match source {
            ClusterChartSource::Scatter2D => widget_states.cluster_scatter.set_error(err),
            ClusterChartSource::Scatter3D => widget_states.cluster_scatter_3d.set_error(err),
            ClusterChartSource::Table => widget_states.trial_table.cluster.set_error(err),
            ClusterChartSource::ArtifactGallery => {
                widget_states.artifact_gallery.set_cluster_error(err)
            }
        }
    }
}
