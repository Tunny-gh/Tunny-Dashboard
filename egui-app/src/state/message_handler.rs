use crate::state::app_state::{AppState, Direction, StudyContext, StudyView};
use crate::state::messages::AppMessage;
use crate::state::results::ConvergenceHistory;
use crate::ui::widget_states::WidgetStates;
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
                        Self::refresh_best_trial_history(app_state);
                    }
                    None => {
                        *load_error =
                            Some(format!("study_id {} not found in shared store", study_id));
                        *is_loading = false;
                        return;
                    }
                }
                widget_states.convergence.computing = false;
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
            AppMessage::IndicatorHistoryDone {
                indicator,
                base,
                comparisons,
            } => {
                app_state.convergence_indicator = indicator;
                app_state.convergence_history = Some(base);
                app_state.comparison_convergence_histories = comparisons;
                widget_states.convergence.computing = false;
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
                extras_events,
            } => {
                Self::handle_live_update_done(
                    new_trial_rows,
                    updated_study_counts,
                    extras_events,
                    app_state,
                );
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
                widget_states.pdp_chart.insert_cache(
                    &param,
                    &objective,
                    &model_type,
                    feasible_only,
                    result.clone(),
                );
                widget_states.pdp_chart.result = Some(result);
                widget_states.pdp_chart.computing = false;
            }

            AppMessage::ComparisonStudyLoaded { context } => {
                // 3 つの並行 Vec（studies / colors / hv_histories）を同じ順序で揃える。
                let idx = app_state.comparison_studies.len();
                app_state.comparison_studies.push(*context);
                app_state
                    .comparison_colors
                    .push(crate::theme::color_compute::comparison_color_at(idx));
                // プレースホルダーを追加して並行 Vec の添字を揃える。
                // 実際の指標値は次回 poll_chart が base+全比較を一括再計算して上書きする。
                app_state
                    .comparison_convergence_histories
                    .push(ConvergenceHistory {
                        trial_ids: Vec::new(),
                        values: Vec::new(),
                        sample_step: 1,
                        ref_point: Vec::new(),
                    });
                // 基準 Study の指標を None にして統合再計算をトリガーする。
                app_state.convergence_history = None;
            }
            AppMessage::ArtifactsDirScanned {
                trial_artifacts,
                artifacts_dir,
            } => {
                app_state.artifact_map = trial_artifacts;
                app_state.artifacts_dir = Some(artifacts_dir);
            }
            AppMessage::ComparisonStudyLoadFailed(err) => {
                *load_error = Some(err);
            }
            AppMessage::ObservedContourDone(result) => {
                widget_states.observed_contour.result = Some(result);
                widget_states.observed_contour.computing = false;
                widget_states.observed_contour.error_message = None;
            }
            AppMessage::ObservedContourFailed(err) => {
                widget_states.observed_contour.error_message = Some(err);
                widget_states.observed_contour.computing = false;
            }
            AppMessage::SurrogateFitDone(trained) => {
                widget_states.surrogate_opt.trained = Some(trained);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateFitFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateFitCancelled => {
                // ユーザーがキャンセルした。エラー表示はせず状態だけ戻す。
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateOptDone(result) => {
                widget_states.surrogate_opt.result = Some(result);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.optimizing = false;
            }
            AppMessage::SurrogateMultiFitDone(trained) => {
                widget_states.surrogate_opt.multi_trained = Some(trained);
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateMultiFitFailed(err) => {
                widget_states.surrogate_opt.error_message = Some(err);
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
            }
            AppMessage::SurrogateMultiFitCancelled => {
                widget_states.surrogate_opt.error_message = None;
                widget_states.surrogate_opt.fitting = false;
                widget_states.surrogate_opt.fit_progress = None;
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
            AppMessage::RobustnessFitDone(trained) => {
                widget_states.robustness.trained = Some(trained);
                widget_states.robustness.fit_error = None;
                widget_states.robustness.fitting = false;
            }
            AppMessage::RobustnessFitFailed(err) => {
                widget_states.robustness.fit_error = Some(err);
                widget_states.robustness.fitting = false;
            }
            AppMessage::ResponseSurfaceFitDone(trained) => {
                widget_states.response_surface.trained = Some(trained);
                widget_states.response_surface.fit_error = None;
                widget_states.response_surface.fitting = false;
            }
            AppMessage::ResponseSurfaceFitFailed(err) => {
                widget_states.response_surface.fit_error = Some(err);
                widget_states.response_surface.fitting = false;
            }
        }
    }

    /// 単目的 Study の best-so-far 履歴（trial_number, cumulative best）を構築し
    /// `app_state.best_trial_history` へ格納する。多目的 Study では None のまま
    /// （収束カードは非表示となり、HV 履歴が代わりに多目的の推移を担う）。
    fn refresh_best_trial_history(app_state: &mut AppState) {
        let Some(ctx) = app_state.current_study.as_ref() else {
            app_state.best_trial_history = None;
            return;
        };
        if ctx.meta.directions.len() != 1 || ctx.meta.objective_names.len() != 1 {
            app_state.best_trial_history = None;
            return;
        }
        let Some(values) = ctx.view.numeric_column(&ctx.meta.objective_names[0]) else {
            app_state.best_trial_history = None;
            return;
        };
        let is_minimize = matches!(ctx.meta.directions[0], Direction::Minimize);
        let n = ctx.view.row_count();
        let trial_numbers: Vec<u32> = (0..n)
            .map(|i| ctx.view.df.get_trial_number(i).unwrap_or(i as u32))
            .collect();
        app_state.best_trial_history = Some(tunny_core::convergence::build_best_trial_history(
            &trial_numbers,
            values,
            is_minimize,
        ));
    }

    /// Study 選択時のストリーミングロード 1 バッチを適用する。
    ///
    /// - 最初のバッチ（`is_first`）: 既存状態をクリアし StudyContext を新規生成。
    /// - 以降: 既存 DataFrame の列クローンへ `append_trials` で新規行を追記する。
    ///   行指向への再構築（旧 core_rows_from_df 方式）はチャンクごとに O(ロード済み行数) の
    ///   HashMap/String 生成を伴いロード全体で O(n²) になるため廃止した。
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
        // 以降のバッチは列クローン（memcpy 相当）+ in-place 追記。制約列数の増加は
        // append_trials 側が既存列数と max を取って吸収する。
        let start_fresh = is_first || app_state.current_study.is_none();
        let mut new_df = if start_fresh {
            app_state.clear();
            DataFrame::empty()
        } else {
            app_state
                .current_study
                .as_ref()
                .map(|s| (*s.view.df).clone())
                .unwrap_or_else(DataFrame::empty)
        };
        new_df.append_trials(
            &new_rows,
            &param_names,
            &objective_names,
            &user_attr_numeric_names,
            &user_attr_string_names,
            max_constraints,
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
            widget_states.convergence.computing = false;
            widget_states.cluster_scatter = Default::default();
            widget_states.cluster_scatter_3d.clear_runtime_state();
            widget_states.trial_table.cluster.clear_runtime_state();
            app_state.cluster_cache.clear();
            app_state.mcdm_cache.clear();
            app_state.mcdm_result = None;
            widget_states.reset_infeasible_flags();
        }

        if is_final {
            Self::refresh_best_trial_history(app_state);
            *is_loading = false;
        }
    }

    fn handle_live_update_done(
        new_core_rows: Vec<tunny_core::io::journal::live_update::TrialRow>,
        updated_study_counts: Vec<(u32, usize)>,
        extras_events: tunny_core::io::journal::live_update::ExtrasDiff,
        app_state: &mut AppState,
    ) {
        if let Some(study) = &mut app_state.current_study {
            let study_id = study.meta.study_id;

            // 全 trial（全 state）の付帯情報（extras）へライブ差分をマージする。
            Self::merge_extras_diff(study_id, &extras_events);

            // 既存 DataFrame の列クローンへライブ差分の新試行のみを追記する
            // （全行の行指向再構築は行わない）。
            let added_rows: Vec<CoreTrialRow> = new_core_rows
                .iter()
                .map(|core_row| CoreTrialRow {
                    trial_id: core_row.trial_id,
                    trial_number: core_row.trial_number,
                    param_display: core_row.params.clone(),
                    param_category_label: core_row.param_categories.clone(),
                    objective_values: core_row.objectives.clone(),
                    user_attrs_numeric: core_row.user_attrs_numeric.clone(),
                    user_attrs_string: core_row.user_attrs_string.clone(),
                    constraint_values: core_row.constraint_values.clone(),
                })
                .collect();

            let param_names = study.meta.param_names.clone();
            let obj_names = study.meta.objective_names.clone();
            let un = study.view.df.user_attr_numeric_col_names().to_vec();
            let us = study.view.df.user_attr_string_col_names().to_vec();
            let max_c = study.view.df.constraint_col_names().len();
            let mut new_df = (*study.view.df).clone();
            new_df.append_trials(&added_rows, &param_names, &obj_names, &un, &us, max_c);

            let is_minimize: Vec<bool> = study
                .meta
                .directions
                .iter()
                .map(|d| matches!(d, Direction::Minimize))
                .collect();

            // 先に共有ストアを差し替えてアクティブ化し、列がそろった DataFrame から
            // Pareto を計算する（handle_study_chunk と同じ方式）。all_rows を直接 nd_sort へ
            // 渡すと、ライブ差分が目的本数の異なる行を含む場合にスライス範囲外で panic する。
            // from_trials / compute_pareto_ranks は不足目的を NaN で埋め形状を必ずそろえる。
            let arc = std::sync::Arc::new(new_df);
            tunny_core::dataframe::swap_snapshot(study_id, arc.clone());
            let _ = tunny_core::dataframe::select_study(study_id);
            let pareto = tunny_core::pareto::compute_pareto_ranks(&is_minimize);
            study.view = StudyView::new(arc, pareto.ranks);
            study.pareto_indices = pareto.pareto_indices;
        }
        // ライブ更新で trial 数・best 値が変わるため収束履歴も作り直す。
        Self::refresh_best_trial_history(app_state);

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

    /// ライブ差分の [`ExtrasDiff`] を対象 study の [`StudyExtras`] へマージし、
    /// 共有ストアへ原子的に差し替える。trial_id 昇順を維持する。
    ///
    /// - new_trials: state=Running の [`TrialExtra`] を追加する（既存 trial_id は据え置き）。
    /// - intermediate_values: 対応 trial へ (step, value) を追記する（未知なら placeholder 生成）。
    /// - state_changes: state と datetime_complete を更新する（未知なら placeholder 生成）。
    fn merge_extras_diff(study_id: u32, diff: &tunny_core::io::journal::live_update::ExtrasDiff) {
        use std::collections::HashMap;
        use tunny_core::extras::{StudyExtras, TrialExtra, TrialState};

        if diff.new_trials.is_empty()
            && diff.intermediate_values.is_empty()
            && diff.state_changes.is_empty()
        {
            return;
        }

        // 現行スナップショットを基点に可変コピーを作る（無ければ空）。
        let mut extras: StudyExtras = tunny_core::dataframe::extras_snapshot(study_id)
            .map(|arc| (*arc).clone())
            .unwrap_or_default();

        let mut index_of: HashMap<u32, usize> = extras
            .trials
            .iter()
            .enumerate()
            .map(|(i, t)| (t.trial_id, i))
            .collect();

        // trial_id に対応する index を返す。無ければ Running の placeholder を生成する。
        // （trial_number 不明時は trial_id を暫定採用する。live_update と同じフォールバック。）
        fn ensure_trial(
            extras: &mut StudyExtras,
            index_of: &mut HashMap<u32, usize>,
            trial_id: u32,
            trial_number: u32,
            datetime_start: Option<f64>,
        ) -> usize {
            if let Some(&idx) = index_of.get(&trial_id) {
                return idx;
            }
            let idx = extras.trials.len();
            extras.trials.push(TrialExtra {
                trial_id,
                trial_number,
                state: TrialState::Running,
                datetime_start,
                datetime_complete: None,
                intermediate_values: Vec::new(),
            });
            index_of.insert(trial_id, idx);
            idx
        }

        for &(trial_id, _study, trial_number, datetime_start) in &diff.new_trials {
            let idx = ensure_trial(
                &mut extras,
                &mut index_of,
                trial_id,
                trial_number,
                datetime_start,
            );
            // 既存 trial なら datetime_start のみ補完する。
            if extras.trials[idx].datetime_start.is_none() {
                extras.trials[idx].datetime_start = datetime_start;
            }
        }

        for &(trial_id, step, value) in &diff.intermediate_values {
            let idx = ensure_trial(&mut extras, &mut index_of, trial_id, trial_id, None);
            extras.trials[idx].intermediate_values.push((step, value));
        }

        for &(trial_id, state, datetime_complete) in &diff.state_changes {
            let idx = ensure_trial(&mut extras, &mut index_of, trial_id, trial_id, None);
            extras.trials[idx].state = TrialState::from_journal(state);
            if datetime_complete.is_some() {
                extras.trials[idx].datetime_complete = datetime_complete;
            }
        }

        // trial_id 昇順を維持し、各 trial の中間値を step 昇順にそろえる。
        extras.trials.sort_by_key(|t| t.trial_id);
        for trial in &mut extras.trials {
            trial.intermediate_values.sort_by_key(|(step, _)| *step);
        }

        tunny_core::dataframe::swap_extras(study_id, std::sync::Arc::new(extras));
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
                trial_number: i as u32,
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
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                param_bounds: Default::default(),
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

    /// テスト用: 単目的 Study の StudySelected を、任意の目的値・方向で構築する
    /// （best_trial_history の配線検証用）。
    fn make_study_message_single_objective(values: &[f64], direction: Direction) -> AppMessage {
        let trial_count = values.len();
        let core_rows: Vec<CoreTrialRow> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![v],
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
                directions: vec![direction],
                completed_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                param_bounds: Default::default(),
            },
            study_id: 0,
            pareto_rank: vec![0; trial_count],
            pareto_indices: vec![],
        }
    }

    /// テスト用: 2 目的 Study の StudySelected を構築する（best_trial_history が
    /// 多目的では None のままであることの検証用）。
    fn make_study_message_multi_objective(trial_count: usize) -> AppMessage {
        let core_rows: Vec<CoreTrialRow> = (0..trial_count)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64, (trial_count - i) as f64],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["y1".to_string(), "y2".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);

        AppMessage::StudySelected {
            meta: StudyMeta {
                study_id: 0,
                name: "s".to_string(),
                directions: vec![Direction::Minimize, Direction::Minimize],
                completed_trials: trial_count,
                param_names: vec!["x".to_string()],
                objective_names: vec!["y1".to_string(), "y2".to_string()],
                param_bounds: Default::default(),
            },
            study_id: 0,
            pareto_rank: vec![0; trial_count],
            pareto_indices: vec![],
        }
    }

    #[test]
    fn best_trial_history_set_for_single_objective_minimize() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message_single_objective(&[3.0, 1.0, 2.0], Direction::Minimize),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(
            app_state.best_trial_history,
            Some(vec![(0, 3.0), (1, 1.0), (2, 1.0)])
        );
    }

    #[test]
    fn best_trial_history_set_for_single_objective_maximize() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message_single_objective(&[1.0, 3.0, 2.0], Direction::Maximize),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(
            app_state.best_trial_history,
            Some(vec![(0, 1.0), (1, 3.0), (2, 3.0)])
        );
    }

    #[test]
    fn best_trial_history_none_for_multi_objective() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            make_study_message_multi_objective(3),
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.best_trial_history.is_none());
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
            trial_number: trial_id,
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
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                param_bounds: Default::default(),
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
                extras_events: Default::default(),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert_eq!(app_state.current_study.as_ref().unwrap().trial_count(), 5);
    }

    /// 回帰: ライブ差分が目的本数の異なる行（空の objectives）を含んでも、
    /// 多目的 Pareto 計算がスライス範囲外で panic しないこと。
    /// （次の create/complete 境界をまたぐ Trial が空 objectives 行を生むケースを再現）
    #[test]
    fn live_update_done_handles_ragged_objectives_without_panic() {
        let _g = test_store_guard();
        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        // 2 目的の study を構築する。
        let core_rows: Vec<CoreTrialRow> = (0..3)
            .map(|i| CoreTrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64, (i as f64) * 2.0],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["o1".to_string(), "o2".to_string()],
            &[],
            &[],
            0,
        );
        tunny_core::dataframe::store_dataframes(vec![df]);
        MessageHandler::handle(
            AppMessage::StudySelected {
                meta: StudyMeta {
                    study_id: 0,
                    name: "s".to_string(),
                    directions: vec![Direction::Minimize, Direction::Minimize],
                    completed_trials: 3,
                    param_names: vec!["x".to_string()],
                    objective_names: vec!["o1".to_string(), "o2".to_string()],
                    param_bounds: Default::default(),
                },
                study_id: 0,
                pareto_rank: vec![0; 3],
                pareto_indices: vec![],
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        // 完全な行 1 件 + 目的が空のゴミ行 1 件を混ぜて送る（旧実装ではここで panic）。
        let mut empty_obj_row = make_core_trial_row(4, 0, vec![]);
        empty_obj_row.objectives = vec![];
        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![make_core_trial_row(3, 0, vec![1.0, 2.0]), empty_obj_row],
                updated_study_counts: vec![],
                extras_events: Default::default(),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        // panic せず 5 行になっていること。
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
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        }];
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        MessageHandler::handle(
            AppMessage::LiveUpdateDone {
                new_trial_rows: vec![],
                updated_study_counts: vec![(1, 105)],
                extras_events: Default::default(),
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
                extras_events: Default::default(),
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
                elbow_max_k: 10,
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
                param_names: vec![],
                objective_names: vec![],
                param_bounds: Default::default(),
            },
            vec![],
        );

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoaded {
                context: Box::new(context),
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
        assert_eq!(app_state.comparison_convergence_histories.len(), 1);
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
