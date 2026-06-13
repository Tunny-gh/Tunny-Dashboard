use crate::state::app_state::{AppState, Direction, StudyContext, StudyView};
use crate::state::messages::{AppMessage, DownsampleKey};
use crate::state::results::HvHistory;
use crate::ui::widget_states::WidgetStates;
use std::collections::HashMap;
use tunny_core::dataframe::{DataFrame, TrialRow as CoreTrialRow};

/// バックグラウンドタスクからのメッセージを処理するハンドラー
pub struct MessageHandler;

impl MessageHandler {
    /// 単一メッセージを処理し、AppState と WidgetStates を更新する
    pub fn handle(
        msg: AppMessage,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
        load_error: &mut Option<String>,
    ) {
        match msg {
            AppMessage::JournalParsed { studies, path } => {
                app_state.all_studies = studies;
                app_state.journal_path = Some(path);
                *is_loading = false;
            }
            AppMessage::StudySelected {
                meta,
                study_id,
                pareto_rank,
                pareto_indices,
            } => {
                app_state.clear();
                match tunny_core::dataframe::snapshot(study_id) {
                    Some(df) => {
                        let view = StudyView::new(df, pareto_rank);
                        // Phase 2 の完全 meta で all_studies のエントリを同期する
                        // （Phase 1 では completed_trials 等が 0 のため）
                        if let Some(existing) = app_state
                            .all_studies
                            .iter_mut()
                            .find(|s| s.study_id == meta.study_id)
                        {
                            *existing = meta.clone();
                        }
                        app_state.current_study = Some(StudyContext {
                            meta,
                            view,
                            pareto_indices,
                        });
                    }
                    None => {
                        *load_error =
                            Some(format!("study_id {} not found in shared store", study_id));
                        *is_loading = false;
                        return;
                    }
                }
                widget_states.hv_history.computing = false;
                widget_states.cluster_scatter = Default::default();
                widget_states.reset_infeasible_flags();
                *is_loading = false;
            }
            AppMessage::StudyChunkLoaded {
                study_id,
                meta,
                new_rows,
                param_names,
                objective_names,
                user_attr_numeric_names,
                user_attr_string_names,
                max_constraints,
                is_first,
                is_final,
            } => {
                Self::handle_study_chunk(
                    study_id,
                    meta,
                    new_rows,
                    param_names,
                    objective_names,
                    user_attr_numeric_names,
                    user_attr_string_names,
                    max_constraints,
                    is_first,
                    is_final,
                    app_state,
                    widget_states,
                    is_loading,
                );
            }
            AppMessage::SensitivityDone { key, result } => {
                app_state.importance_cache.insert(key, result);
                widget_states.importance.computing = false;
            }
            AppMessage::SensitivityHeatmapDone {
                metric,
                feasible_only,
                result,
            } => {
                app_state
                    .sensitivity_heatmap_cache
                    .insert((metric.cache_id(), feasible_only), result);
                widget_states.sensitivity_heatmap.computing = false;
            }
            AppMessage::SobolDone { key, result } => {
                app_state.sobol_cache.insert(key, result);
                widget_states.importance.computing = false;
            }
            AppMessage::ClusteringDone {
                source,
                key,
                result,
            } => {
                Self::handle_clustering_done(source, key, result, app_state, widget_states);
            }
            AppMessage::ClusterFailed { source, err } => {
                Self::handle_cluster_failed(source, err, widget_states);
            }
            AppMessage::McdmDone {
                source,
                key,
                result,
            } => {
                // 設定キーごとにキャッシュし、同じ設定の他チャートと共有する。
                app_state.mcdm_cache.insert(key, result.clone());
                // 最後に計算した結果は McdmScore カラーモードの基準として保持する。
                app_state.mcdm_result = Some(result);
                // 計算を開始したチャートの実行状態のみ解除する。
                Self::mcdm_controls_mut(source, widget_states).computing = false;
            }
            AppMessage::McdmFailed { source, message } => {
                let controls = Self::mcdm_controls_mut(source, widget_states);
                controls.computing = false;
                controls.pending_entropy = false;
                *load_error = Some(message);
            }
            AppMessage::EntropyDone { source, result } => {
                let controls = Self::mcdm_controls_mut(source, widget_states);
                controls.weights = result.weights.clone();
                controls.entropy_result = Some(result);
                controls.pending_entropy = false;
                controls.computing = false;
            }
            AppMessage::DownsampleDone { key, indices } => match key {
                DownsampleKey::Scatter => app_state.downsample_cache.scatter = Some(indices),
                DownsampleKey::Pcp => app_state.downsample_cache.pcp = Some(indices),
                DownsampleKey::Thumbnail => app_state.downsample_cache.thumbnail = Some(indices),
                DownsampleKey::Hover => app_state.downsample_cache.hover = Some(indices),
            },
            AppMessage::HvHistoryDone {
                trial_ids,
                hv_values,
                sample_step,
                ref_point,
            } => {
                app_state.hv_history = Some(HvHistory {
                    trial_ids,
                    hv_values,
                    sample_step,
                    ref_point,
                });
                widget_states.hv_history.computing = false;
            }
            AppMessage::Pdp2dDone(result) => {
                widget_states.pdp_2d.result = Some(result);
                widget_states.pdp_2d.computing = false;
            }
            AppMessage::Error(e) => {
                *load_error = Some(e);
                *is_loading = false;
            }
            AppMessage::SensitivityError(_e) => {
                widget_states.importance.computing = false;
            }
            AppMessage::LiveUpdateDone {
                new_trial_rows,
                updated_study_counts,
            } => {
                Self::handle_live_update_done(new_trial_rows, updated_study_counts, app_state);
            }
            AppMessage::LiveUpdateError(msg) => {
                app_state.live_update.poller_active = false;
                *load_error = Some(msg);
            }
            AppMessage::LiveUpdateMaybeComplete => {
                app_state.live_update.showing_completion_hint = true;
            }
            AppMessage::PdpDone {
                param,
                objective,
                model_type,
                feasible_only,
                result,
            } => {
                // キャッシュに挿入してから result を設定
                if let crate::state::messages::PdpResult::OneDim(ref r1d) = result {
                    widget_states.pdp_chart.insert_cache(
                        &param,
                        &objective,
                        &model_type,
                        feasible_only,
                        r1d.clone(),
                    );
                }
                widget_states.pdp_chart.result = Some(result);
                widget_states.pdp_chart.computing = false;
            }

            AppMessage::ComparisonStudyLoaded {
                study_idx: _, // studies arrive in dispatch order; sequential append is correct
                context,
                hv_history,
            } => {
                // 3 つの並行 Vec（studies / colors / hv_histories）を同じ順序で揃える。
                let idx = app_state.comparison_studies.len();
                app_state.comparison_studies.push(*context);
                app_state
                    .comparison_colors
                    .push(crate::theme::color_compute::comparison_color_at(idx));
                if let Some(hv) = hv_history {
                    app_state.comparison_hv_histories.push(hv);
                } else {
                    // HV を計算できない Study でも色・studies と添字を揃えるため空履歴を入れる。
                    app_state.comparison_hv_histories.push(HvHistory {
                        trial_ids: Vec::new(),
                        hv_values: Vec::new(),
                        sample_step: 1,
                        ref_point: Vec::new(),
                    });
                }
            }
            AppMessage::ArtifactsDirScanned {
                trial_artifacts,
                artifacts_dir,
            } => {
                app_state.artifact_map = trial_artifacts;
                app_state.artifacts_dir = Some(artifacts_dir);
            }
            AppMessage::HtmlReportDone { .. } => {
                // TASK-2117/2123 で実装
            }
            AppMessage::ComparisonStudyLoadFailed(err) => {
                *load_error = Some(err);
            }
            AppMessage::SurfacePlotDone(result) => {
                widget_states.surface_plot.result = Some(result);
                widget_states.surface_plot.computing = false;
            }
            AppMessage::SurfacePlotFailed(err) => {
                widget_states.surface_plot.error_message = Some(err);
                widget_states.surface_plot.computing = false;
            }
            AppMessage::SurrogateFitDone(trained) => {
                widget_states.surrogate_opt.trained = Some(trained);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
            }
            AppMessage::SurrogateFitFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.fitting = false;
            }
            AppMessage::SurrogateOptDone(result) => {
                widget_states.surrogate_opt.result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateOptFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateMultiFitDone(trained) => {
                widget_states.surrogate_opt.multi_trained = Some(trained);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
            }
            AppMessage::SurrogateMultiFitFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.fitting = false;
            }
            AppMessage::SurrogateMultiOptDone(result) => {
                widget_states.surrogate_opt.multi_result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateMultiOptFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::ChartCaptureFailed(err) => {
                widget_states.capture.last_error = Some(err);
            }
            AppMessage::SurrogateSuggestDone(result) => {
                widget_states.surrogate_opt.suggest_result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.suggesting = false;
            }
            AppMessage::SurrogateSuggestFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.suggesting = false;
            }
            AppMessage::SurrogateMultiSuggestDone(result) => {
                widget_states.surrogate_opt.multi_suggest_result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.multi_suggesting = false;
            }
            AppMessage::SurrogateMultiSuggestFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.multi_suggesting = false;
            }
        }
    }

    /// 現在の DataFrame スナップショットから core TrialRow 群を再構築する。
    /// ライブ更新で新試行を加えた DataFrame を作り直すための入力に用いる。
    fn core_rows_from_df(df: &DataFrame) -> Vec<CoreTrialRow> {
        let n = df.row_count();
        let param_names = df.param_col_names().to_vec();
        let obj_names = df.objective_col_names().to_vec();
        let un = df.user_attr_numeric_col_names().to_vec();
        let us = df.user_attr_string_col_names().to_vec();
        let cn = df.constraint_col_names().to_vec();
        (0..n)
            .map(|i| {
                let mut param_display = HashMap::new();
                let mut param_category_label = HashMap::new();
                for name in &param_names {
                    if let Some(col) = df.get_numeric_column(name) {
                        if let Some(v) = col.get(i) {
                            param_display.insert(name.clone(), *v);
                        }
                    } else if let Some(col) = df.get_string_column(name) {
                        if let Some(v) = col.get(i) {
                            param_category_label.insert(name.clone(), v.clone());
                        }
                    }
                }
                let objective_values = obj_names
                    .iter()
                    .filter_map(|o| df.get_numeric_column(o).and_then(|c| c.get(i).copied()))
                    .collect();
                let mut user_attrs_numeric = HashMap::new();
                for name in &un {
                    if let Some(c) = df.get_numeric_column(name) {
                        if let Some(v) = c.get(i) {
                            user_attrs_numeric.insert(name.clone(), *v);
                        }
                    }
                }
                let mut user_attrs_string = HashMap::new();
                for name in &us {
                    if let Some(c) = df.get_string_column(name) {
                        if let Some(v) = c.get(i) {
                            user_attrs_string.insert(name.clone(), v.clone());
                        }
                    }
                }
                let constraint_values = cn
                    .iter()
                    .filter_map(|c| df.get_numeric_column(c).and_then(|col| col.get(i).copied()))
                    .collect();
                CoreTrialRow {
                    trial_id: df.get_trial_id(i).unwrap_or(i as u32),
                    param_display,
                    param_category_label,
                    objective_values,
                    user_attrs_numeric,
                    user_attrs_string,
                    constraint_values,
                }
            })
            .collect()
    }

    /// Study 選択時のストリーミングロード 1 バッチを適用する。
    ///
    /// - 最初のバッチ（`is_first`）: 既存状態をクリアし StudyContext を新規生成。
    /// - 以降: 既存 DataFrame から行を再構築 → 新規行を追記 → 列を含め DataFrame を作り直す。
    /// - Pareto は重い（多目的 nd_sort が O(N²)）ため**ストリーミング中は計算せず**、
    ///   `is_final` のバッチで一度だけ確定計算する（読み込み中は rank 0 表示）。
    #[allow(clippy::too_many_arguments)]
    fn handle_study_chunk(
        study_id: u32,
        meta: crate::state::app_state::StudyMeta,
        new_rows: Vec<CoreTrialRow>,
        param_names: Vec<String>,
        objective_names: Vec<String>,
        user_attr_numeric_names: Vec<String>,
        user_attr_string_names: Vec<String>,
        max_constraints: usize,
        is_first: bool,
        is_final: bool,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
    ) {
        // 最初のバッチは Study 切り替えとして既存状態をリセットする。
        let start_fresh = is_first || app_state.current_study.is_none();
        let mut all_rows: Vec<CoreTrialRow> = if start_fresh {
            app_state.clear();
            Vec::with_capacity(new_rows.len())
        } else {
            app_state
                .current_study
                .as_ref()
                .map(|s| Self::core_rows_from_df(&s.view.df))
                .unwrap_or_default()
        };
        // 既存スナップショットの制約列数も考慮（streaming 中に制約列が増えても保持）。
        let max_c = max_constraints.max(
            app_state
                .current_study
                .as_ref()
                .map(|s| s.view.df.constraint_col_names().len())
                .unwrap_or(0),
        );
        all_rows.extend(new_rows);

        let new_df = DataFrame::from_trials(
            &all_rows,
            &param_names,
            &objective_names,
            &user_attr_numeric_names,
            &user_attr_string_names,
            max_c,
        );
        let arc = std::sync::Arc::new(new_df);
        tunny_core::dataframe::swap_snapshot(study_id, arc.clone());

        // Pareto は最終バッチでのみ確定。アクティブ DataFrame を読むため select_study で活性化する。
        let (ranks, pareto_indices) = if is_final {
            let _ = tunny_core::dataframe::select_study(study_id);
            let is_minimize: Vec<bool> = meta
                .directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            let pareto = tunny_core::pareto::compute_pareto_ranks(&is_minimize);
            (pareto.ranks, pareto.pareto_indices)
        } else {
            (Vec::new(), Vec::new())
        };

        let view = StudyView::new(arc, ranks);
        if let Some(study) = &mut app_state.current_study {
            study.meta = meta.clone();
            study.view = view;
            study.pareto_indices = pareto_indices;
        } else {
            app_state.current_study = Some(StudyContext {
                meta: meta.clone(),
                view,
                pareto_indices,
            });
        }

        // Phase 2 の累積 meta で all_studies のエントリを同期する。
        if let Some(existing) = app_state
            .all_studies
            .iter_mut()
            .find(|s| s.study_id == study_id)
        {
            *existing = meta;
        }

        if start_fresh {
            // 後続機能がアクティブ DataFrame を参照できるよう早期に活性化する。
            let _ = tunny_core::dataframe::select_study(study_id);
            widget_states.hv_history.computing = false;
            widget_states.cluster_scatter = Default::default();
            widget_states.cluster_scatter_3d.clear_runtime_state();
            widget_states.trial_table.cluster.clear_runtime_state();
            app_state.cluster_cache.clear();
            app_state.mcdm_cache.clear();
            app_state.mcdm_result = None;
            widget_states.reset_infeasible_flags();
        }

        if is_final {
            *is_loading = false;
        }
    }

    fn handle_live_update_done(
        new_core_rows: Vec<tunny_core::io::journal::live_update::TrialRow>,
        updated_study_counts: Vec<(u32, usize)>,
        app_state: &mut AppState,
    ) {
        if let Some(study) = &mut app_state.current_study {
            let study_id = study.meta.study_id;

            // 既存 DataFrame から core 行を再構築し、新試行を追加して DataFrame を作り直す。
            let mut all_rows = Self::core_rows_from_df(&study.view.df);
            for core_row in &new_core_rows {
                all_rows.push(CoreTrialRow {
                    trial_id: core_row.trial_id,
                    param_display: core_row.params.clone(),
                    param_category_label: core_row.param_categories.clone(),
                    objective_values: core_row.objectives.clone(),
                    user_attrs_numeric: core_row.user_attrs_numeric.clone(),
                    user_attrs_string: core_row.user_attrs_string.clone(),
                    constraint_values: core_row.constraint_values.clone(),
                });
            }

            let param_names = study.meta.param_names.clone();
            let obj_names = study.meta.objective_names.clone();
            let un = study.view.df.user_attr_numeric_col_names().to_vec();
            let us = study.view.df.user_attr_string_col_names().to_vec();
            let max_c = study.view.df.constraint_col_names().len();
            let new_df =
                DataFrame::from_trials(&all_rows, &param_names, &obj_names, &un, &us, max_c);

            // Pareto ランク再計算
            let is_minimize: Vec<bool> = study
                .meta
                .directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();
            let objectives: Vec<Vec<f64>> = all_rows
                .iter()
                .map(|r| r.objective_values.clone())
                .collect();
            let ranks = tunny_core::pareto::nd_sort(&objectives, &is_minimize);
            let pareto_indices: Vec<u32> = ranks
                .iter()
                .enumerate()
                .filter_map(|(i, &r)| if r == 0 { Some(i as u32) } else { None })
                .collect();

            // ArcSwap で共有ストアのスナップショットを差し替え、view を作り直す。
            let arc = std::sync::Arc::new(new_df);
            tunny_core::dataframe::swap_snapshot(study_id, arc.clone());
            study.view = StudyView::new(arc, ranks);
            study.pareto_indices = pareto_indices;
        }

        // トライアル数が変わるとキャッシュ済み結果の行数が合わなくなるため破棄する。
        app_state.cluster_cache.clear();
        app_state.mcdm_cache.clear();
        app_state.mcdm_result = None;

        // Update all_studies completed_trials
        for (study_id, new_count) in updated_study_counts {
            if let Some(meta) = app_state
                .all_studies
                .iter_mut()
                .find(|m| m.study_id == study_id)
            {
                meta.completed_trials = new_count;
            }
        }
    }

    fn handle_clustering_done(
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
            // 結果は設定キーごとにキャッシュし、同じ設定の他チャートと共有する。
            app_state.cluster_cache.insert(key, result);
            // 実行を開始したチャートの spinner / pending を解除する。
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

    fn handle_cluster_failed(
        source: crate::state::messages::ClusterChartSource,
        err: crate::state::messages::ClusterUiError,
        widget_states: &mut WidgetStates,
    ) {
        Self::set_cluster_error(source, err, widget_states);
    }

    /// クラスタリング開始元のウィジェットの実行状態を解除する。
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

    /// MCDM 計算開始元チャートの controls への可変参照を返す。
    fn mcdm_controls_mut(
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

    /// クラスタリング開始元のウィジェットにエラーを設定する。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{Direction, StudyMeta};

    /// テスト用: 共有ストア（テストビルドでは thread_local）に DataFrame を格納し、
    /// 新しい StudySelected ペイロード（study_id + pareto_rank）を返す。
    fn make_study_message(trial_count: usize) -> AppMessage {
        let core_rows: Vec<CoreTrialRow> = (0..trial_count)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["y".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);

        AppMessage::StudySelected {
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: trial_count,
                total_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                user_attr_names: vec![],
                has_constraints: false,
            },
            study_id: 0,
            pareto_rank: vec![0; trial_count],
            pareto_indices: vec![],
        }
    }

    /// 共有ストア（本番ビルドではプロセスグローバル）を使うテストを直列化するガード。
    /// tunny-desktop のテストは tunny-core を通常リンクするため store は全テスト共有。
    /// store_dataframes + snapshot を使うテストはこのガードで直列化して競合を防ぐ。
    fn test_store_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    fn clustering_done_updates_state_when_lengths_match() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        widgets.cluster_scatter.computing = true;
        let key = widgets.cluster_scatter.cache_key();
        MessageHandler::handle(
            AppMessage::ClusteringDone {
                source: crate::state::messages::ClusterChartSource::Scatter2D,
                key: key.clone(),
                result: crate::state::results::ClusterResult {
                    labels: vec![0, 1, 0],
                    n_clusters: 2,
                },
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.cluster_cache.contains_key(&key));
        assert!(!widgets.cluster_scatter.computing);
        assert!(widgets.cluster_scatter.last_error.is_none());
    }

    #[test]
    fn clustering_done_rejects_mismatched_label_length() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        let key = widgets.cluster_scatter.cache_key();
        MessageHandler::handle(
            AppMessage::ClusteringDone {
                source: crate::state::messages::ClusterChartSource::Scatter2D,
                key: key.clone(),
                result: crate::state::results::ClusterResult {
                    labels: vec![0, 1],
                    n_clusters: 2,
                },
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.cluster_cache.is_empty());
        assert!(widgets.cluster_scatter.last_error.is_some());
    }

    fn make_core_trial_row(
        trial_id: u32,
        study_id: u32,
        objectives: Vec<f64>,
    ) -> tunny_core::io::journal::live_update::TrialRow {
        tunny_core::io::journal::live_update::TrialRow {
            trial_id,
            trial_number: trial_id,
            params: std::collections::HashMap::new(),
            param_categories: std::collections::HashMap::new(),
            objectives,
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
            study_id,
        }
    }

    fn make_chunk_row(trial_id: u32, x: f64, obj: f64) -> CoreTrialRow {
        CoreTrialRow {
            trial_id,
            param_display: std::collections::HashMap::from([("x".to_string(), x)]),
            param_category_label: std::collections::HashMap::new(),
            objective_values: vec![obj],
            user_attrs_numeric: std::collections::HashMap::new(),
            user_attrs_string: std::collections::HashMap::new(),
            constraint_values: vec![],
        }
    }

    fn chunk_message(rows: Vec<CoreTrialRow>, is_first: bool, is_final: bool) -> AppMessage {
        AppMessage::StudyChunkLoaded {
            study_id: 0,
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                total_trials: 0,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                user_attr_names: vec![],
                has_constraints: false,
            },
            new_rows: rows,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_numeric_names: vec![],
            user_attr_string_names: vec![],
            max_constraints: 0,
            is_first,
            is_final,
        }
    }

    #[test]
    fn study_chunks_accumulate_rows_across_batches() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = true;
        let mut load_error = None;

        // 1st batch: establishes study, still loading.
        MessageHandler::handle(
            chunk_message(
                vec![make_chunk_row(0, 0.1, 1.0), make_chunk_row(1, 0.2, 2.0)],
                true,
                false,
            ),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 2);
        assert!(is_loading, "still loading mid-stream");

        // 2nd (final) batch: appends and finalizes.
        MessageHandler::handle(
            chunk_message(vec![make_chunk_row(2, 0.3, 3.0)], false, true),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 3);
        assert!(!is_loading, "loading cleared on final batch");

        // 列データが結合されている
        let xs = app_state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .numeric_column("x")
            .unwrap()
            .to_vec();
        assert_eq!(xs, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn live_update_done_appends_trial_rows() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 3);

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![
                    make_core_trial_row(3, 1, vec![1.0]),
                    make_core_trial_row(4, 1, vec![2.0]),
                ],
                updated_study_counts: vec![(1, 5)],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 5);
    }

    #[test]
    fn live_update_done_updates_all_studies_counts() {
        let mut app_state = AppState::new();
        app_state.all_studies = vec![crate::state::app_state::StudyMeta {
            study_id: 1,
            name: "s".to_string(),
            directions: vec![],
            completed_trials: 100,
            total_trials: 100,
            param_names: vec![],
            objective_names: vec![],
            user_attr_names: vec![],
            has_constraints: false,
        }];
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![],
                updated_study_counts: vec![(1, 105)],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.all_studies[0].completed_trials, 105);
    }

    #[test]
    fn live_update_done_preserves_filter_ranges() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );
        app_state.filter_ranges.insert("x".to_string(), (0.0, 1.0));
        app_state.selected_indices = vec![0, 1];

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![make_core_trial_row(3, 1, vec![1.0])],
                updated_study_counts: vec![],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.filter_ranges.contains_key("x"));
        assert_eq!(app_state.selected_indices, vec![0, 1]);
    }

    #[test]
    fn live_update_error_sets_poller_inactive() {
        let mut app_state = AppState::new();
        app_state.live_update.poller_active = true;
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateError("test error".to_string()),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(!app_state.live_update.poller_active);
        assert!(load_error.is_some());
    }

    #[test]
    fn live_update_maybe_complete_sets_hint() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateMaybeComplete,
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.live_update.showing_completion_hint);
    }

    #[test]
    fn study_selected_resets_cluster_widget_runtime_state() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        widgets.cluster_scatter.computing = true;
        widgets.cluster_scatter.pending_compute =
            Some(crate::ui::widgets::cluster_scatter::ClusterComputeRequest {
                k: 3,
                target_space: crate::ui::widgets::cluster_scatter::ClusterSpace::Objective,
                k_mode: crate::ui::widgets::cluster_scatter::KSelectionMode::Manual,
                init_strategy:
                    crate::ui::widgets::cluster_scatter::KMeansInitStrategy::KMeansPlusPlus,
            });

        MessageHandler::handle(
            make_study_message(4),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(!widgets.cluster_scatter.computing);
        assert!(widgets.cluster_scatter.pending_compute.is_none());
        assert!(widgets.cluster_scatter.last_error.is_none());
    }

    // ── TASK-2230: 比較ロードメッセージのテスト ──────────────────

    #[test]
    fn comparison_load_message_updates_state_entrypoint() {
        use crate::state::app_state::StudyContext;
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        let context = StudyContext::from_rows_for_test(
            StudyMeta {
                study_id: 99,
                name: "compare".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                total_trials: 0,
                param_names: vec![],
                objective_names: vec![],
                user_attr_names: vec![],
                has_constraints: false,
            },
            vec![],
        );

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoaded {
                study_idx: 0,
                context: Box::new(context),
                hv_history: None,
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.comparison_studies.len(), 1);
        assert_eq!(app_state.comparison_studies[0].meta.study_id, 99);
        // 並行 Vec が同じ長さに揃うこと
        assert_eq!(app_state.comparison_colors.len(), 1);
        assert_eq!(app_state.comparison_hv_histories.len(), 1);
    }

    #[test]
    fn comparison_load_failed_message_sets_load_error() {
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error: Option<String> = None;

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoadFailed("file not found".to_string()),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(load_error.as_deref(), Some("file not found"));
    }
}
