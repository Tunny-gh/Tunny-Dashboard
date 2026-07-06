use crate::state::app_state::{AppState, McdmResult, StudyContext};
use crate::state::layout_state::ChartId;
use crate::state::results::ClusterResult;
use crate::state::types::Direction;
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::trial_table::TrialTableMode;
use tunny_core::export::{CsvField, CsvWriter};

/// チャート固有のクラスタリング設定キーで、キャッシュからクラスタ結果を解決する。
/// 2D / 3D / Table はそれぞれ独立した設定を持つため、エクスポート対象も各自のキーで引く。
fn cluster_result_for_chart<'a>(
    chart_id: &ChartId,
    app_state: &'a AppState,
    widgets: &WidgetStates,
) -> Option<&'a ClusterResult> {
    let key = match chart_id {
        ChartId::ClusterScatter => widgets.cluster_scatter.cache_key(),
        ChartId::ClusterScatter3D => widgets.cluster_scatter_3d.cache_key(),
        _ => return None,
    };
    app_state.cluster_cache.get(&key)
}

/// チャート固有の MCDM 設定キーで、キャッシュから結果を解決する。
fn mcdm_result_for_chart<'a>(
    chart_id: &ChartId,
    app_state: &'a AppState,
    widgets: &WidgetStates,
) -> Option<&'a McdmResult> {
    let key = match chart_id {
        ChartId::McdmRankChart => widgets.mcdm_chart.controls.cache_key(),
        ChartId::McdmScatterChart => widgets.scatter_chart.controls.cache_key(),
        ChartId::McdmScatterChart3D => widgets.mcdm_scatter_3d.controls.cache_key(),
        _ => return None,
    };
    app_state.mcdm_cache.get(&key)
}

/// 多くの `build_*_csv` 冒頭の定型ガード（current_study を取得し、trial が 1 件以上ある
/// ことを保証する）を集約する。study 未選択・trial 数 0 のいずれかなら `None` を返す。
fn require_study(app_state: &AppState) -> Option<&StudyContext> {
    let study = app_state.current_study.as_ref()?;
    (study.trial_count() > 0).then_some(study)
}

pub fn build_chart_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    // has_csv_data（ボタン活性判定）を唯一のデータ有無判定として使い、両者の乖離を防ぐ。
    // has_csv_data は軽量なので毎エクスポート先頭で呼んでも問題ない。
    if !has_csv_data(chart_id, app_state, widgets) {
        return None;
    }
    match chart_id {
        ChartId::OptimizationHistory => build_optimization_history_csv(app_state, widgets),
        ChartId::ConvergenceIndicators => build_convergence_csv(app_state),
        ChartId::ImportanceChart => build_importance_csv(app_state, widgets),
        ChartId::PdpChart => build_pdp_csv(app_state, widgets),
        ChartId::PdpChart2D => build_pdp_2d_csv(app_state, widgets),
        ChartId::ParallelCoordinates => build_trial_based_csv(app_state),
        ChartId::ScatterMatrix => build_trial_based_csv(app_state),
        ChartId::ClusterScatter => build_cluster_csv(chart_id, app_state, widgets),
        ChartId::SensitivityHeatmap => build_sensitivity_csv(app_state, widgets),
        ChartId::ParetoScatter2D => build_pareto_csv(app_state),
        ChartId::ParetoScatter3D => build_pareto_csv(app_state),
        ChartId::McdmRankChart => mcdm_result_for_chart(chart_id, app_state, widgets)
            .and_then(|r| build_mcdm_rank_csv(r, app_state)),
        ChartId::McdmScatterChart => mcdm_result_for_chart(chart_id, app_state, widgets)
            .and_then(|r| build_mcdm_scatter_csv(r, app_state)),
        ChartId::SliceChart => build_slice_csv(app_state, widgets),
        ChartId::ObservedContour => build_observed_contour_csv(widgets),
        ChartId::SurrogateOpt => build_surrogate_opt_csv(widgets),
        ChartId::Robustness => build_robustness_csv(widgets),
        ChartId::ClusterScatter3D => build_cluster_csv(chart_id, app_state, widgets),
        ChartId::McdmScatterChart3D => mcdm_result_for_chart(chart_id, app_state, widgets)
            .and_then(|r| build_mcdm_scatter_csv(r, app_state)),
        ChartId::Histogram => build_histogram_csv(app_state, widgets),
        ChartId::BoxPlot => build_box_plot_csv(app_state, widgets),
        ChartId::CorrelationMatrix => build_correlation_matrix_csv(app_state, widgets),
        ChartId::ArtifactGallery => None,
        ChartId::RadarComparison => build_radar_comparison_csv(app_state, widgets),
        ChartId::ComparisonTable => build_comparison_table_csv(app_state, widgets),
        ChartId::PcaBiplot => build_pca_biplot_csv(widgets),
        ChartId::SomMap => build_som_csv(app_state, widgets),
        ChartId::Dendrogram => build_dendrogram_csv(widgets),
        ChartId::ResponseSurface3D => build_response_surface_csv(widgets),
        ChartId::IntermediateValues => build_intermediate_values_csv(),
        ChartId::Timeline => build_timeline_csv(),
        ChartId::EdfPlot => build_edf_csv(app_state, widgets),
        ChartId::RankPlot => build_rank_plot_csv(app_state, widgets),
        ChartId::SurrogateCompare => build_surrogate_compare_csv(widgets),
    }
}

/// 統合トライアルテーブル（`PanelItem::TrialTable`）の CSV を、現在のモードに応じて組み立てる。
/// All はトライアル一覧、Cluster はクラスタ割当、MCDM はランキングを出力する。
pub fn build_trial_table_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    match widgets.trial_table.mode {
        TrialTableMode::All => build_trial_based_csv(app_state),
        TrialTableMode::Cluster => {
            let key = widgets.trial_table.cluster.cache_key();
            let cr = app_state.cluster_cache.get(&key)?;
            build_cluster_csv_from_result(cr, app_state)
        }
        TrialTableMode::Mcdm => {
            let key = widgets.trial_table.mcdm.controls.cache_key();
            let result = app_state.mcdm_cache.get(&key)?;
            build_mcdm_table_csv(result, app_state)
        }
    }
}

/// 統合トライアルテーブルに、現在のモードでエクスポート可能なデータがあるか判定する。
pub fn has_trial_table_csv(app_state: &AppState, widgets: &WidgetStates) -> bool {
    match widgets.trial_table.mode {
        TrialTableMode::All => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| s.trial_count() > 0),
        TrialTableMode::Cluster => {
            let key = widgets.trial_table.cluster.cache_key();
            app_state
                .current_study
                .as_ref()
                .zip(app_state.cluster_cache.get(&key))
                .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count())
        }
        TrialTableMode::Mcdm => {
            let key = widgets.trial_table.mcdm.controls.cache_key();
            app_state.current_study.is_some() && app_state.mcdm_cache.contains_key(&key)
        }
    }
}

/// 統合トライアルテーブルの CSV ファイル名を、現在のモードに応じて返す。
pub fn trial_table_csv_filename(widgets: &WidgetStates) -> String {
    let name = match widgets.trial_table.mode {
        TrialTableMode::All => "trial_table",
        TrialTableMode::Cluster => "cluster_table",
        TrialTableMode::Mcdm => "mcdm_table",
    };
    format!("{}.csv", name)
}

pub fn has_csv_data(chart_id: &ChartId, app_state: &AppState, widgets: &WidgetStates) -> bool {
    match chart_id {
        ChartId::SurrogateOpt => {
            widgets.surrogate_opt.result.is_some() || widgets.surrogate_opt.multi_result.is_some()
        }
        ChartId::Robustness => widgets.robustness.cached_result().is_some(),
        ChartId::OptimizationHistory | ChartId::ParallelCoordinates | ChartId::ScatterMatrix => {
            app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.trial_count() > 0)
        }
        ChartId::ConvergenceIndicators => app_state.convergence_history.is_some(),
        ChartId::ImportanceChart => {
            if widgets.importance.computing {
                return false;
            }
            let obj_idx = widgets.importance.objective_index;
            let feasible_only = widgets.importance.feasible_only;
            if widgets.importance.metric.is_sobol() {
                app_state
                    .sobol_cache
                    .contains_key(&(obj_idx, feasible_only))
            } else {
                let key = (widgets.importance.metric.cache_id(), obj_idx, feasible_only);
                app_state.importance_cache.contains_key(&key)
            }
        }
        ChartId::PdpChart => widgets
            .pdp_chart
            .result
            .as_ref()
            .is_some_and(|d| !d.x_values.is_empty()),
        ChartId::PdpChart2D => widgets
            .pdp_2d
            .result
            .as_ref()
            .is_some_and(|r| !r.x_values.is_empty() && !r.y_values.is_empty()),
        ChartId::ClusterScatter => app_state
            .current_study
            .as_ref()
            .zip(cluster_result_for_chart(chart_id, app_state, widgets))
            .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count()),
        ChartId::SensitivityHeatmap => app_state
            .sensitivity_heatmap_cache
            .get(&(
                widgets.sensitivity_heatmap.metric.cache_id(),
                widgets.sensitivity_heatmap.feasible_only,
            ))
            .is_some_and(|m| m.is_well_formed()),
        ChartId::ParetoScatter2D | ChartId::ParetoScatter3D => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| !s.pareto_indices.is_empty()),
        ChartId::McdmRankChart | ChartId::McdmScatterChart => {
            app_state.current_study.is_some()
                && mcdm_result_for_chart(chart_id, app_state, widgets).is_some()
        }
        ChartId::SliceChart => app_state.current_study.as_ref().is_some_and(|s| {
            s.trial_count() > 0
                && s.meta
                    .param_names
                    .get(widgets.slice_chart.selected_param_idx)
                    .is_some()
                && s.meta
                    .objective_names
                    .get(widgets.slice_chart.selected_obj_idx)
                    .is_some()
        }),
        ChartId::ObservedContour => widgets
            .observed_contour
            .result
            .as_ref()
            .is_some_and(|r| !r.surface.x_values.is_empty()),
        ChartId::ClusterScatter3D => app_state
            .current_study
            .as_ref()
            .zip(cluster_result_for_chart(chart_id, app_state, widgets))
            .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count()),
        ChartId::McdmScatterChart3D => {
            app_state.current_study.is_some()
                && mcdm_result_for_chart(chart_id, app_state, widgets).is_some()
        }
        ChartId::Histogram | ChartId::BoxPlot => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| s.trial_count() > 0),
        ChartId::CorrelationMatrix => {
            app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.trial_count() > 0)
                && (widgets.correlation_matrix.include_params
                    || widgets.correlation_matrix.include_objectives)
        }
        ChartId::ArtifactGallery => false,
        ChartId::RadarComparison => app_state.current_study.as_ref().is_some_and(|s| {
            !app_state.pinned_trials.is_empty()
                && !crate::ui::widgets::radar_comparison::build_axes(
                    &s.view,
                    &s.meta.param_names,
                    &s.meta.objective_names,
                    widgets.radar_comparison.include_params,
                )
                .is_empty()
        }),
        ChartId::ComparisonTable => app_state.current_study.as_ref().is_some_and(|s| {
            !crate::ui::widgets::comparison_table::resolve_pinned_rows(
                &s.view,
                &app_state.pinned_trials,
            )
            .is_empty()
                && !crate::ui::widgets::comparison_table::build_rows(
                    &s.view,
                    &s.meta.param_names,
                    &s.meta.objective_names,
                    widgets.comparison_table.show_params,
                    widgets.comparison_table.show_user_attrs,
                )
                .is_empty()
        }),
        ChartId::PcaBiplot => widgets
            .pca_biplot
            .cached_result()
            .is_some_and(|r| !r.projections.is_empty()),
        ChartId::SomMap => app_state.current_study.as_ref().is_some_and(|s| {
            widgets
                .som_map
                .current_grid(&s.meta.param_names, &s.meta.objective_names)
                .is_some()
        }),
        ChartId::Dendrogram => widgets
            .dendrogram
            .leaf_assignments()
            .is_some_and(|a| !a.is_empty()),
        ChartId::ResponseSurface3D => widgets
            .response_surface
            .cached_slice()
            .is_some_and(|s| !s.x_values.is_empty() && !s.y_values.is_empty()),
        ChartId::IntermediateValues => {
            tunny_core::dataframe::active_extras_snapshot().is_some_and(|e| e.has_intermediate())
        }
        ChartId::Timeline => {
            tunny_core::dataframe::active_extras_snapshot().is_some_and(|e| e.has_datetimes())
        }
        ChartId::EdfPlot => app_state.current_study.as_ref().is_some_and(|s| {
            s.meta
                .objective_names
                .get(widgets.edf_plot.obj_idx)
                .is_some_and(|name| s.view.numeric_column(name).is_some_and(|c| !c.is_empty()))
        }),
        ChartId::RankPlot => app_state.current_study.as_ref().is_some_and(|s| {
            s.trial_count() > 0
                && s.meta
                    .param_names
                    .get(widgets.rank_plot.x_param_idx)
                    .is_some()
                && s.meta
                    .param_names
                    .get(widgets.rank_plot.y_param_idx)
                    .is_some()
                && s.meta
                    .objective_names
                    .get(widgets.rank_plot.obj_idx)
                    .is_some()
        }),
        ChartId::SurrogateCompare => widgets
            .surrogate_compare
            .result
            .as_ref()
            .is_some_and(|r| !r.rows.is_empty()),
    }
}

pub fn csv_export_filename(chart_id: &ChartId) -> String {
    let name = match chart_id {
        ChartId::OptimizationHistory => "optimization_history",
        ChartId::ConvergenceIndicators => "convergence_indicators",
        ChartId::ImportanceChart => "importance_chart",
        ChartId::PdpChart => "pdp_chart",
        ChartId::PdpChart2D => "pdp_chart_2d",
        ChartId::ParallelCoordinates => "parallel_coordinates",
        ChartId::ScatterMatrix => "scatter_matrix",
        ChartId::ClusterScatter => "cluster_scatter",
        ChartId::SensitivityHeatmap => "sensitivity_heatmap",
        ChartId::ParetoScatter2D => "pareto_scatter_2d",
        ChartId::ParetoScatter3D => "pareto_scatter_3d",
        ChartId::McdmRankChart => "mcdm_rank_chart",
        ChartId::McdmScatterChart => "mcdm_scatter_chart",
        ChartId::SliceChart => "slice_chart",
        ChartId::ObservedContour => "observed_contour",
        ChartId::SurrogateOpt => "surrogate_optimizer",
        ChartId::Robustness => "robustness",
        ChartId::ClusterScatter3D => "cluster_scatter_3d",
        ChartId::McdmScatterChart3D => "mcdm_scatter_chart_3d",
        ChartId::Histogram => "histogram",
        ChartId::BoxPlot => "box_plot",
        ChartId::CorrelationMatrix => "correlation_matrix",
        ChartId::ArtifactGallery => "artifact_gallery",
        ChartId::RadarComparison => "radar_comparison",
        ChartId::ComparisonTable => "comparison_table",
        ChartId::PcaBiplot => "pca_biplot",
        ChartId::SomMap => "som_map",
        ChartId::Dendrogram => "dendrogram",
        ChartId::ResponseSurface3D => "response_surface_3d",
        ChartId::IntermediateValues => "intermediate_values",
        ChartId::Timeline => "timeline",
        ChartId::EdfPlot => "edf_plot",
        ChartId::RankPlot => "rank_plot",
        ChartId::SurrogateCompare => "surrogate_compare",
    };
    format!("{}.csv", name)
}

/// Observed Contour の補間格子を long 形式で出力する（マスクされたセルは除外）。
fn build_observed_contour_csv(widgets: &WidgetStates) -> Option<String> {
    let r = widgets.observed_contour.result.as_ref()?;
    let surf = &r.surface;
    if surf.x_values.is_empty() || surf.y_values.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([r.x_name.as_str(), r.y_name.as_str(), r.value_name.as_str()]);
    for (i, &x) in surf.x_values.iter().enumerate() {
        for (j, &y) in surf.y_values.iter().enumerate() {
            if let Some(Some(v)) = surf.z.get(i).map(|col| col[j]) {
                w.row([CsvField::Num(x), CsvField::Num(y), CsvField::Num(v)]);
            }
        }
    }
    Some(w.finish())
}

/// 現在の列選択・ビン設定でヒストグラムを再計算して CSV にする。
/// ウィジェット表示時と同じフォールバック（目的関数→パラメータの最初の数値列）を適用する。
fn build_histogram_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let obj_names = &study.meta.objective_names;
    let param_names = &study.meta.param_names;
    let candidates: Vec<&String> = obj_names
        .iter()
        .chain(param_names.iter())
        .filter(|n| study.view.numeric_column(n).is_some())
        .collect();
    let selected = widgets.histogram.selected_col.as_str();
    let col = if candidates.iter().any(|c| c.as_str() == selected) {
        selected
    } else {
        candidates.first()?.as_str()
    };
    let values = study.view.numeric_column(col)?;
    let rule = widgets
        .histogram
        .rule
        .to_core(widgets.histogram.manual_bins);
    let hist = tunny_core::statistics::compute_histogram(values, rule)?;

    let mut w = CsvWriter::new();
    w.header(["bin_start", "bin_end", "count"]);
    for (edge, &count) in hist.bin_edges.windows(2).zip(&hist.counts) {
        w.row([
            CsvField::Num(edge[0]),
            CsvField::Num(edge[1]),
            CsvField::UInt(count as u64),
        ]);
    }
    Some(w.finish())
}

/// 現在の Source/Normalize 設定で各列の箱ひげ統計を再計算して CSV にする（1列1行）。
fn build_box_plot_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    use crate::ui::widgets::box_plot::{normalize_minmax, BoxPlotSource};

    let study = require_study(app_state)?;
    let names: &[String] = match widgets.box_plot.source {
        BoxPlotSource::Objectives => &study.meta.objective_names,
        BoxPlotSource::Parameters => &study.meta.param_names,
    };
    let normalize = widgets.box_plot.normalize;
    let mut w = CsvWriter::new();
    w.header([
        "column",
        "n",
        "mean",
        "min",
        "q1",
        "median",
        "q3",
        "max",
        "whisker_low",
        "whisker_high",
        "n_outliers",
    ]);
    let mut any = false;
    for name in names {
        let Some(raw) = study.view.numeric_column(name) else {
            continue;
        };
        let values = if normalize {
            normalize_minmax(raw)
        } else {
            raw.to_vec()
        };
        let Some(s) = tunny_core::statistics::compute_boxplot(&values) else {
            continue;
        };
        any = true;
        w.row([
            CsvField::Text(name),
            CsvField::UInt(s.n as u64),
            CsvField::Num(s.mean),
            CsvField::Num(s.min),
            CsvField::Num(s.q1),
            CsvField::Num(s.median),
            CsvField::Num(s.q3),
            CsvField::Num(s.max),
            CsvField::Num(s.whisker_low),
            CsvField::Num(s.whisker_high),
            CsvField::UInt(s.outliers.len() as u64),
        ]);
    }
    any.then(|| w.finish())
}

/// 現在の Method/列グループ設定で相関行列を再計算し、ワイド形式で CSV にする。
/// NaN セルは空文字として出力する。
fn build_correlation_matrix_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    if !widgets.correlation_matrix.include_params && !widgets.correlation_matrix.include_objectives
    {
        return None;
    }
    let mut names: Vec<&String> = Vec::new();
    if widgets.correlation_matrix.include_params {
        names.extend(study.meta.param_names.iter());
    }
    if widgets.correlation_matrix.include_objectives {
        names.extend(study.meta.objective_names.iter());
    }
    let columns: Vec<(String, Vec<f64>)> = names
        .into_iter()
        .filter_map(|name| {
            study
                .view
                .numeric_column(name)
                .map(|c| (name.clone(), c.to_vec()))
        })
        .collect();
    if columns.is_empty() {
        return None;
    }
    let matrix = tunny_core::statistics::compute_correlation_matrix(
        &columns,
        widgets.correlation_matrix.method,
    )?;

    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec![""];
    header.extend(matrix.labels.iter().map(String::as_str));
    w.header(header);
    for (i, label) in matrix.labels.iter().enumerate() {
        let mut fields = vec![CsvField::Text(label)];
        for &val in &matrix.values[i] {
            fields.push(if val.is_nan() {
                CsvField::Empty
            } else {
                CsvField::Num(val)
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// レーダー比較の現在の軸設定（Include parameters）でピン留めトライアルの生値を
/// ワイド形式（1 軸 1 行、列 = ピン留めトライアル）で CSV にする。正規化前の生値を出力する。
fn build_radar_comparison_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    if app_state.pinned_trials.is_empty() {
        return None;
    }
    let axes = crate::ui::widgets::radar_comparison::build_axes(
        &study.view,
        &study.meta.param_names,
        &study.meta.objective_names,
        widgets.radar_comparison.include_params,
    );
    if axes.is_empty() {
        return None;
    }
    let pinned_rows: Vec<(u32, usize)> = app_state
        .pinned_trials
        .iter()
        .filter_map(|&trial_id| {
            study
                .view
                .trial_ids
                .iter()
                .position(|&t| t == trial_id)
                .map(|row| (trial_id, row))
        })
        .collect();
    if pinned_rows.is_empty() {
        return None;
    }

    let column_labels: Vec<String> = pinned_rows
        .iter()
        .map(|&(trial_id, row)| {
            let number = study.view.df.get_trial_number(row).unwrap_or(trial_id);
            format!("Trial #{number}")
        })
        .collect();
    let mut header: Vec<&str> = vec!["axis"];
    header.extend(column_labels.iter().map(String::as_str));

    let mut w = CsvWriter::new();
    w.header(header);
    for axis in &axes {
        let mut fields = vec![CsvField::Text(axis.name)];
        for &(_, row) in &pinned_rows {
            fields.push(match axis.col.get(row) {
                Some(&v) if v.is_finite() => CsvField::Num(v),
                _ => CsvField::Empty,
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// 比較表の現在の行設定（Parameters / User attrs）でピン留めトライアルの生値を
/// ワイド形式（1 行 1 行、列 = ピン留めトライアル）で CSV にする。
fn build_comparison_table_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    if app_state.pinned_trials.is_empty() {
        return None;
    }
    let pinned_rows = crate::ui::widgets::comparison_table::resolve_pinned_rows(
        &study.view,
        &app_state.pinned_trials,
    );
    if pinned_rows.is_empty() {
        return None;
    }
    let rows = crate::ui::widgets::comparison_table::build_rows(
        &study.view,
        &study.meta.param_names,
        &study.meta.objective_names,
        widgets.comparison_table.show_params,
        widgets.comparison_table.show_user_attrs,
    );
    if rows.is_empty() {
        return None;
    }

    let column_labels: Vec<String> = pinned_rows
        .iter()
        .map(|&(trial_id, row)| {
            let number = study.view.df.get_trial_number(row).unwrap_or(trial_id);
            format!("Trial #{number}")
        })
        .collect();
    let mut header: Vec<&str> = vec![""];
    header.extend(column_labels.iter().map(String::as_str));

    let mut w = CsvWriter::new();
    w.header(header);
    for info in &rows {
        let mut fields = vec![CsvField::Text(info.label)];
        for &(_, row) in &pinned_rows {
            fields.push(match info.col.get(row) {
                Some(&v) if v.is_finite() => CsvField::Num(v),
                _ => CsvField::Empty,
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// キャッシュ済みの PCA 結果を `pc1,pc2` の 2 列 CSV にする。
fn build_pca_biplot_csv(widgets: &WidgetStates) -> Option<String> {
    let result = widgets.pca_biplot.cached_result()?;
    if result.projections.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["pc1", "pc2"]);
    for row in &result.projections {
        let pc1 = row.first().copied().unwrap_or(0.0);
        let pc2 = row.get(1).copied().unwrap_or(0.0);
        w.row([CsvField::Num(pc1), CsvField::Num(pc2)]);
    }
    Some(w.finish())
}

/// SOM の現在の表示モード（U-matrix / Component Plane / Hits）に対応するノード値
/// グリッドを、行 = y・列 = x のワイド形式で CSV にする。
fn build_som_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let (grid_w, grid_h, values, _label) = widgets
        .som_map
        .current_grid(&study.meta.param_names, &study.meta.objective_names)?;
    if grid_w == 0 || grid_h == 0 || values.len() != grid_w * grid_h {
        return None;
    }
    let mut w = CsvWriter::new();
    let mut header: Vec<String> = vec!["y".to_string()];
    header.extend((0..grid_w).map(|x| format!("x{x}")));
    w.header(header.iter().map(String::as_str));
    for y in 0..grid_h {
        let mut fields = vec![CsvField::UInt(y as u64)];
        for x in 0..grid_w {
            let v = values[y * grid_w + x];
            fields.push(if v.is_finite() {
                CsvField::Num(v)
            } else {
                CsvField::Empty
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// デンドログラムの葉順に (元 view の行インデックス, カット後クラスタラベル) を CSV にする。
fn build_dendrogram_csv(widgets: &WidgetStates) -> Option<String> {
    let assignments = widgets.dendrogram.leaf_assignments()?;
    if assignments.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["row_index", "cluster"]);
    for (row_index, cluster) in assignments {
        w.row([
            CsvField::UInt(row_index as u64),
            CsvField::UInt(cluster as u64),
        ]);
    }
    Some(w.finish())
}

/// 応答曲面スライスの z グリッドを CSV にする（列ヘッダーに x 値、各行の先頭に y 値）。
fn build_response_surface_csv(widgets: &WidgetStates) -> Option<String> {
    let slice = widgets.response_surface.cached_slice()?;
    let nx = slice.x_values.len();
    let ny = slice.y_values.len();
    if nx == 0 || ny == 0 {
        return None;
    }
    let mut w = CsvWriter::new();
    let mut header: Vec<String> = vec!["y\\x".to_string()];
    header.extend(slice.x_values.iter().map(|x| x.to_string()));
    w.header(header.iter().map(String::as_str));
    for yi in 0..ny {
        let mut fields = vec![CsvField::Num(slice.y_values[yi])];
        for xi in 0..nx {
            let z = slice
                .z_values
                .get(xi)
                .and_then(|row| row.get(yi))
                .copied()
                .unwrap_or(f64::NAN);
            fields.push(if z.is_finite() {
                CsvField::Num(z)
            } else {
                CsvField::Empty
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

fn build_optimization_history_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let obj_idx = widgets.opt_history.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let obj_col = study.view.numeric_column(obj_name)?;
    if obj_col.is_empty() {
        return None;
    }
    let is_minimize = !matches!(
        study.meta.directions.get(obj_idx),
        Some(Direction::Maximize)
    );
    let mut w = CsvWriter::new();
    w.header(["trial_index", "objective_value", "best_value"]);
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for (i, &val) in obj_col.iter().enumerate() {
        if val.is_finite() {
            best = if is_minimize {
                best.min(val)
            } else {
                best.max(val)
            };
        }
        w.row([
            CsvField::UInt(i as u64),
            CsvField::Num(val),
            CsvField::Num(best),
        ]);
    }
    Some(w.finish())
}

/// Intermediate Values の全 trial・全ステップを long 形式で出力する（間引きなし）。
fn build_intermediate_values_csv() -> Option<String> {
    let extras = tunny_core::dataframe::active_extras_snapshot()?;
    if !extras.has_intermediate() {
        return None;
    }
    // CSV エクスポートは表示用の間引き（MAX_CURVES）を適用せず全 trial を出す。
    let (curves, _total) = crate::ui::widgets::intermediate_values::build_intermediate_curves(
        &extras.trials,
        false,
        usize::MAX,
    );
    if curves.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["trial_number", "state", "step", "value"]);
    for c in &curves {
        for &[step, value] in &c.points {
            w.row([
                CsvField::UInt(c.trial_number as u64),
                CsvField::Text(c.state.label()),
                CsvField::Num(step),
                CsvField::Num(value),
            ]);
        }
    }
    Some(w.finish())
}

/// Timeline の全 trial の開始/終了（経過秒）を出力する。
fn build_timeline_csv() -> Option<String> {
    let extras = tunny_core::dataframe::active_extras_snapshot()?;
    if !extras.has_datetimes() {
        return None;
    }
    let bars = crate::ui::widgets::timeline::build_timeline_bars(&extras.trials);
    if bars.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([
        "trial_number",
        "state",
        "start_elapsed_s",
        "end_elapsed_s",
        "duration_s",
    ]);
    for b in &bars {
        w.row([
            CsvField::UInt(b.trial_number as u64),
            CsvField::Text(b.state.label()),
            CsvField::Num(b.start),
            CsvField::Num(b.end),
            CsvField::Num(b.end - b.start),
        ]);
    }
    Some(w.finish())
}

fn build_convergence_csv(app_state: &AppState) -> Option<String> {
    let history = app_state.convergence_history.as_ref()?;
    let label = app_state.convergence_indicator.label();
    let mut w = CsvWriter::new();
    w.header(["trial_index", label]);
    for (i, &val) in history.values.iter().enumerate() {
        let trial_idx = i * history.sample_step;
        w.row([CsvField::UInt(trial_idx as u64), CsvField::Num(val)]);
    }
    Some(w.finish())
}
fn build_importance_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    if widgets.importance.computing {
        return None;
    }
    use crate::ui::widgets::importance_chart::{compute_sorted_importance, compute_sorted_sobol};
    let metric = &widgets.importance.metric;
    let obj_idx = widgets.importance.objective_index;
    let feasible_only = widgets.importance.feasible_only;
    let method_name = metric.label();
    let pairs: Vec<(String, f64)> = if metric.is_sobol() {
        let sobol = app_state.sobol_cache.get(&(obj_idx, feasible_only))?;
        compute_sorted_sobol(sobol, obj_idx, metric)
    } else {
        let key = (metric.cache_id(), obj_idx, feasible_only);
        let sensitivity = app_state.importance_cache.get(&key)?;
        compute_sorted_importance(sensitivity, metric, obj_idx)
    };
    if pairs.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["variable", "importance_score", "method"]);
    for (name, score) in &pairs {
        w.row([
            CsvField::Text(name),
            CsvField::Num(*score),
            CsvField::Text(method_name),
        ]);
    }
    Some(w.finish())
}
fn build_pdp_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let r = widgets.pdp_chart.result.as_ref()?;
    if r.x_values.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([
        "variable",
        "variable_value",
        "predicted_objective",
        "lower_ci",
        "upper_ci",
    ]);
    for (i, (&x, &y)) in r.x_values.iter().zip(r.y_values.iter()).enumerate() {
        let lower = r.y_lower.as_ref().and_then(|v| v.get(i)).copied();
        let upper = r.y_upper.as_ref().and_then(|v| v.get(i)).copied();
        w.row([
            CsvField::Text(&r.param_name),
            CsvField::Num(x),
            CsvField::Num(y),
            lower.map(CsvField::Num).unwrap_or(CsvField::Empty),
            upper.map(CsvField::Num).unwrap_or(CsvField::Empty),
        ]);
    }
    Some(w.finish())
}

fn build_pdp_2d_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let result = widgets.pdp_2d.result.as_ref()?;
    if result.x_values.is_empty() || result.y_values.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([
        "param1_name",
        "param1_value",
        "param2_name",
        "param2_value",
        "predicted_objective",
    ]);
    for (xi, &x) in result.x_values.iter().enumerate() {
        for (yi, &y) in result.y_values.iter().enumerate() {
            let z = result
                .z_values
                .get(xi)
                .and_then(|row| row.get(yi))
                .copied()
                .unwrap_or(f64::NAN);
            w.row([
                CsvField::Text(&result.param1_name),
                CsvField::Num(x),
                CsvField::Text(&result.param2_name),
                CsvField::Num(y),
                CsvField::Num(z),
            ]);
        }
    }
    Some(w.finish())
}
fn build_trial_based_csv(app_state: &AppState) -> Option<String> {
    let study = require_study(app_state)?;
    let n = study.trial_count();
    let row_indices: Vec<usize> = (0..n).collect();
    Some(crate::io::export::build_csv_string_from_view(
        &study.view,
        &row_indices,
        &study.meta.param_names,
        &study.meta.objective_names,
    ))
}

fn build_cluster_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let cr = cluster_result_for_chart(chart_id, app_state, widgets)?;
    build_cluster_csv_from_result(cr, app_state)
}

/// クラスタ結果を直接受け取って CSV を組み立てる（チャート ID 非依存）。
/// 統合トライアルテーブルなど、ChartId を持たない呼び出し元から使う。
fn build_cluster_csv_from_result(cr: &ClusterResult, app_state: &AppState) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let n = study.trial_count();
    if cr.labels.len() != n {
        return None;
    }
    let param_names = &study.meta.param_names;
    let obj_names = &study.meta.objective_names;
    let param_cols = study.view.numeric_columns(param_names);
    let obj_cols = study.view.numeric_columns(obj_names);
    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec!["trial_id", "trial_number"];
    header.extend(param_names.iter().map(String::as_str));
    header.extend(obj_names.iter().map(String::as_str));
    header.push("cluster_id");
    w.header(header);
    for i in 0..n {
        let trial_id = study.view.trial_ids.get(i).copied().unwrap_or(i as u32);
        let trial_number = study.view.df.get_trial_number(i).unwrap_or(i as u32);
        let mut fields = vec![
            CsvField::UInt(trial_id as u64),
            CsvField::UInt(trial_number as u64),
        ];
        for col in param_cols.iter().chain(&obj_cols) {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            fields.push(CsvField::Num(v));
        }
        let label = cr.labels.get(i).copied().unwrap_or(-1);
        fields.push(CsvField::Int(label as i64));
        w.row(fields);
    }
    Some(w.finish())
}
fn build_sensitivity_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let m = app_state.sensitivity_heatmap_cache.get(&(
        widgets.sensitivity_heatmap.metric.cache_id(),
        widgets.sensitivity_heatmap.feasible_only,
    ))?;
    if !m.is_well_formed() {
        return None;
    }
    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec!["variable"];
    header.extend(m.objective_names.iter().map(String::as_str));
    w.header(header);
    for (i, param_name) in m.param_names.iter().enumerate() {
        let mut fields = vec![CsvField::Text(param_name)];
        for &val in &m.values[i] {
            fields.push(CsvField::Num(val));
        }
        w.row(fields);
    }
    Some(w.finish())
}
fn build_pareto_csv(app_state: &AppState) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    if study.pareto_indices.is_empty() {
        return None;
    }
    // Export every individual with its Pareto rank (rank 0 = Pareto front).
    // This matches the chart, which plots all trials and colors the front.
    // `StudyView::new` guarantees `pareto_rank` is row-aligned (length == row
    // count), so rank lookups never go out of bounds.
    let row_indices: Vec<usize> = (0..study.view.trial_ids.len()).collect();
    Some(crate::io::export::build_trial_csv_from_view(
        &study.view,
        &row_indices,
        &study.meta.param_names,
        &study.meta.objective_names,
        crate::io::export::TrialCsvColumns {
            pareto_rank: true,
            cluster_id: false,
        },
    ))
}
fn build_mcdm_rank_csv(result: &McdmResult, app_state: &AppState) -> Option<String> {
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let method_name = result.method_label();
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut w = CsvWriter::new();
    w.header(["trial_id", "rank", "score", "method"]);
    for (rank, &idx) in ranked.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = scores.get(i).copied().unwrap_or(f64::NAN);
        w.row([
            CsvField::UInt(trial_id as u64),
            CsvField::UInt((rank + 1) as u64),
            CsvField::Num(score),
            CsvField::Text(method_name),
        ]);
    }
    Some(w.finish())
}

fn build_mcdm_scatter_csv(result: &McdmResult, app_state: &AppState) -> Option<String> {
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut w = CsvWriter::new();
    w.header(["trial_id", "rank", "primary_score"]);
    for (rank, &idx) in ranked.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = scores.get(i).copied().unwrap_or(f64::NAN);
        w.row([
            CsvField::UInt(trial_id as u64),
            CsvField::UInt((rank + 1) as u64),
            CsvField::Num(score),
        ]);
    }
    Some(w.finish())
}

fn build_mcdm_table_csv(result: &McdmResult, app_state: &AppState) -> Option<String> {
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let tid = |idx: u32| trial_ids.get(idx as usize).copied().unwrap_or(idx);
    match result {
        McdmResult::Topsis(r) => {
            let mut w = CsvWriter::new();
            w.header(["trial_id", "rank", "topsis_score"]);
            for (rank, &idx) in r.ranked_indices.iter().enumerate() {
                let score = r.scores.get(idx as usize).copied().unwrap_or(f64::NAN);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(score),
                ]);
            }
            Some(w.finish())
        }
        McdmResult::Vikor(r) => {
            let mut w = CsvWriter::new();
            w.header(["trial_id", "rank", "s_value", "r_value", "q_value"]);
            for (rank, &idx) in r.ranked_indices.iter().enumerate() {
                let i = idx as usize;
                let s = r.s_values.get(i).copied().unwrap_or(f64::NAN);
                let rv = r.r_values.get(i).copied().unwrap_or(f64::NAN);
                let q = r.q_values.get(i).copied().unwrap_or(f64::NAN);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(s),
                    CsvField::Num(rv),
                    CsvField::Num(q),
                ]);
            }
            Some(w.finish())
        }
        McdmResult::PrometheeI(r) => {
            let mut w = CsvWriter::new();
            w.header([
                "trial_id",
                "rank",
                "phi_plus",
                "phi_minus",
                "incomparable_count",
            ]);
            for (rank, &idx) in r.ranked_indices_i.iter().enumerate() {
                let i = idx as usize;
                let phi_plus = r.phi_plus.get(i).copied().unwrap_or(f64::NAN);
                let phi_minus = r.phi_minus.get(i).copied().unwrap_or(f64::NAN);
                let incomparable_count = r.incomparable_counts.get(i).copied().unwrap_or(0);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(phi_plus),
                    CsvField::Num(phi_minus),
                    CsvField::UInt(incomparable_count as u64),
                ]);
            }
            Some(w.finish())
        }
        McdmResult::PrometheeII(r) => {
            let mut w = CsvWriter::new();
            w.header(["trial_id", "rank", "phi_net"]);
            for (rank, &idx) in r.ranked_indices_ii.iter().enumerate() {
                let phi_net = r.phi_net.get(idx as usize).copied().unwrap_or(f64::NAN);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(phi_net),
                ]);
            }
            Some(w.finish())
        }
    }
}

fn build_slice_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let param_idx = widgets.slice_chart.selected_param_idx;
    let obj_idx = widgets.slice_chart.selected_obj_idx;
    let param_name = study.meta.param_names.get(param_idx)?;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let param_col = study.view.numeric_column(param_name);
    let obj_col = study.view.numeric_column(obj_name);
    // Pareto membership is the per-row rank == 0 in the view (row-aligned).
    let mut w = CsvWriter::new();
    w.header(["trial_id", param_name, obj_name, "is_pareto"]);
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        let param_val = param_col
            .and_then(|c| c.get(i))
            .copied()
            .unwrap_or(f64::NAN);
        let obj_val = obj_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let is_pareto = study.view.pareto_rank.get(i).copied() == Some(0);
        w.row([
            CsvField::UInt(tid as u64),
            CsvField::Num(param_val),
            CsvField::Num(obj_val),
            CsvField::Text(if is_pareto { "true" } else { "false" }),
        ]);
    }
    Some(w.finish())
}

/// EDF（経験分布関数）の全 trial 分の点列を出力する（表示上の対数フィルタは適用しない、間引きなし）。
fn build_edf_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let obj_idx = widgets.edf_plot.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let values: Vec<f64> = study.view.numeric_column(obj_name)?.to_vec();
    let points = crate::ui::widgets::edf_plot::build_edf_points(&values, false);
    if points.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([obj_name.as_str(), "cumulative_fraction"]);
    for &[x, y] in &points {
        w.row([CsvField::Num(x), CsvField::Num(y)]);
    }
    Some(w.finish())
}

/// Rank Plot の全 trial 分（NaN/欠損を含む）を出力する。
fn build_rank_plot_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let x_name = study.meta.param_names.get(widgets.rank_plot.x_param_idx)?;
    let y_name = study.meta.param_names.get(widgets.rank_plot.y_param_idx)?;
    let obj_idx = widgets.rank_plot.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let minimize = !matches!(
        study.meta.directions.get(obj_idx),
        Some(Direction::Maximize)
    );
    let x_col = study.view.numeric_column(x_name);
    let y_col = study.view.numeric_column(y_name);
    let obj_values: Vec<f64> = study
        .view
        .numeric_column(obj_name)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let ranks = crate::ui::widgets::rank_plot::compute_rank_percentiles(&obj_values, minimize);
    let mut w = CsvWriter::new();
    w.header(["trial_id", x_name, y_name, obj_name, "rank_percentile"]);
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        let x_val = x_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let y_val = y_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let obj_val = obj_values.get(i).copied().unwrap_or(f64::NAN);
        let rank = ranks.get(i).copied().unwrap_or(f64::NAN);
        w.row([
            CsvField::UInt(tid as u64),
            CsvField::Num(x_val),
            CsvField::Num(y_val),
            CsvField::Num(obj_val),
            CsvField::Num(rank),
        ]);
    }
    Some(w.finish())
}

/// サロゲート最適化の推定最適点を CSV にする。
/// 多目的結果がある場合はフロント点テーブルを優先出力する。
/// 単目的の場合はパラメータ行＋予測値サマリ行を出力する。
fn build_surrogate_opt_csv(widgets: &WidgetStates) -> Option<String> {
    // 多目的結果を優先する。
    if let Some(ref multi) = widgets.surrogate_opt.multi_result {
        return Some(build_surrogate_multi_opt_csv(multi));
    }

    let result = widgets.surrogate_opt.result.as_ref()?;
    let mut w = CsvWriter::new();
    w.header(["name", "value"]);
    for (name, value) in &result.best_params {
        w.row([CsvField::Text(name), CsvField::Num(*value)]);
    }
    let direction = if result.minimize {
        "minimize"
    } else {
        "maximize"
    };
    let predicted_label = format!("predicted_{}({})", direction, result.objective_name);
    w.row([
        CsvField::Text(&predicted_label),
        CsvField::Num(result.best_value),
    ]);
    if let Some(std) = result.predicted_std {
        w.row([CsvField::Text("predicted_std"), CsvField::Num(std)]);
    }
    w.row([CsvField::Text("r_squared"), CsvField::Num(result.r_squared)]);

    // 検証指標を追記する（学習済みモデルが保持されている場合）。
    if let Some(ref trained) = widgets.surrogate_opt.trained {
        let v = &trained.validation;
        w.row([CsvField::Text("train_r2"), CsvField::Num(v.train_r2)]);
        w.row([CsvField::Text("holdout_r2"), CsvField::Num(v.holdout_r2)]);
        w.row([
            CsvField::Text("holdout_rmse"),
            CsvField::Num(v.holdout_rmse),
        ]);
        w.row([CsvField::Text("cv_r2_mean"), CsvField::Num(v.cv_r2_mean)]);
        w.row([CsvField::Text("cv_r2_std"), CsvField::Num(v.cv_r2_std)]);
        w.row([
            CsvField::Text("cv_rmse_mean"),
            CsvField::Num(v.cv_rmse_mean),
        ]);
        w.row([CsvField::Text("cv_rmse_std"), CsvField::Num(v.cv_rmse_std)]);
    }

    Some(w.finish())
}

/// ロバスト性解析の出力サンプルを 1 列 CSV にする。キャッシュが無ければヘッダのみ返す。
fn build_robustness_csv(widgets: &WidgetStates) -> Option<String> {
    let mut w = CsvWriter::new();
    w.header(["sample"]);
    if let Some(result) = widgets.robustness.cached_result() {
        for &v in &result.samples {
            w.row([CsvField::Num(v)]);
        }
    }
    Some(w.finish())
}

/// Compare Surrogates の CV 指標比較表を CSV にする。フィットに失敗したモデルは
/// 数値欄を空欄にする。結果が無ければ None。
fn build_surrogate_compare_csv(widgets: &WidgetStates) -> Option<String> {
    let result = widgets.surrogate_compare.result.as_ref()?;
    let mut w = CsvWriter::new();
    w.header([
        "model",
        "cv_r2_mean",
        "cv_r2_std",
        "holdout_r2",
        "holdout_rmse",
        "train_r2",
    ]);
    for row in &result.rows {
        let model_name = crate::ui::widgets::surrogate_opt::model_label(row.kind);
        if row.error.is_some() {
            w.row([
                CsvField::Text(model_name),
                CsvField::Empty,
                CsvField::Empty,
                CsvField::Empty,
                CsvField::Empty,
                CsvField::Empty,
            ]);
        } else {
            w.row([
                CsvField::Text(model_name),
                CsvField::Num(row.cv_r2_mean),
                CsvField::Num(row.cv_r2_std),
                CsvField::Num(row.holdout_r2),
                CsvField::Num(row.holdout_rmse),
                CsvField::Num(row.train_r2),
            ]);
        }
    }
    Some(w.finish())
}

/// 多目的サロゲート最適化のフロント点テーブルを CSV にする。
/// ヘッダ行 = 目的名 + パラメータ名、1 行 = 1 フロント点。
fn build_surrogate_multi_opt_csv(
    result: &crate::state::messages::SurrogateMultiOptUiResult,
) -> String {
    let mut w = CsvWriter::new();
    // ヘッダ行
    let headers: Vec<&str> = result
        .objective_names
        .iter()
        .map(|s| s.as_str())
        .chain(result.param_names.iter().map(|s| s.as_str()))
        .collect();
    w.header(headers);
    // データ行（1 フロント点 = 1 行）
    for pt in &result.front {
        let fields: Vec<CsvField> = pt
            .values
            .iter()
            .chain(pt.params.iter())
            .map(|&v| CsvField::Num(v))
            .collect();
        w.row(fields);
    }
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::AppState;
    use crate::state::results::ConvergenceHistory;
    use crate::state::types::{Direction, StudyContext, StudyMeta, TrialRow};
    use crate::ui::widget_states::WidgetStates;
    use std::collections::HashMap;

    fn make_study(
        param_names: Vec<String>,
        obj_names: Vec<String>,
        directions: Vec<Direction>,
    ) -> StudyContext {
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions,
            completed_trials: 0,
            param_names,
            objective_names: obj_names,
            param_bounds: Default::default(),
        };
        StudyContext::from_rows_for_test(meta, vec![])
    }

    fn make_trial(id: u32, params: HashMap<String, f64>, objectives: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params,
            objectives,
            ..Default::default()
        }
    }

    fn make_trial_ranked(
        id: u32,
        params: HashMap<String, f64>,
        objectives: Vec<f64>,
        pareto_rank: u32,
    ) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params,
            objectives,
            pareto_rank,
            ..Default::default()
        }
    }

    #[test]
    fn csv_export_filename_optimization_history() {
        assert_eq!(
            csv_export_filename(&ChartId::OptimizationHistory),
            "optimization_history.csv"
        );
    }

    #[test]
    fn csv_export_filename_all_end_with_csv() {
        let ids = vec![
            ChartId::OptimizationHistory,
            ChartId::ConvergenceIndicators,
            ChartId::ImportanceChart,
            ChartId::PdpChart,
            ChartId::PdpChart2D,
            ChartId::ParallelCoordinates,
            ChartId::ScatterMatrix,
            ChartId::ClusterScatter,
            ChartId::SensitivityHeatmap,
            ChartId::ParetoScatter2D,
            ChartId::ParetoScatter3D,
            ChartId::McdmRankChart,
            ChartId::McdmScatterChart,
            ChartId::SliceChart,
            ChartId::SurrogateOpt,
        ];
        for id in &ids {
            assert!(
                csv_export_filename(id).ends_with(".csv"),
                "{:?} filename does not end with .csv",
                id
            );
        }
    }

    #[test]
    fn opt_history_csv_minimize_tracks_cumulative_min() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        study.set_rows_for_test(vec![
            make_trial(0, HashMap::new(), vec![3.0]),
            make_trial(1, HashMap::new(), vec![1.0]),
            make_trial(2, HashMap::new(), vec![2.0]),
        ]);
        state.current_study = Some(study);
        let widgets = WidgetStates::default();

        let csv = build_optimization_history_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_index,objective_value,best_value");
        assert_eq!(lines[1], "0,3,3");
        assert_eq!(lines[2], "1,1,1");
        assert_eq!(lines[3], "2,2,1");
    }

    #[test]
    fn opt_history_csv_nan_objective_becomes_empty_field() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![f64::NAN])]);
        state.current_study = Some(study);
        let widgets = WidgetStates::default();

        let csv = build_optimization_history_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        // NaN objective and the still-infinite running best both serialize as empty fields.
        assert_eq!(lines[1], "0,,");
    }

    #[test]
    fn opt_history_csv_returns_none_when_no_study() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        assert!(build_optimization_history_csv(&state, &widgets).is_none());
    }

    #[test]
    fn convergence_csv_uses_index_times_step() {
        let state = AppState {
            convergence_history: Some(ConvergenceHistory {
                trial_ids: vec![10, 20, 30],
                values: vec![0.1, 0.5, 0.8],
                sample_step: 5,
                ref_point: vec![],
            }),
            // convergence_indicator は AppState::default() で Hypervolume に初期化される。
            ..AppState::default()
        };
        let csv = build_convergence_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_index,Hypervolume");
        assert_eq!(lines[1], "0,0.1");
        assert_eq!(lines[2], "5,0.5");
        assert_eq!(lines[3], "10,0.8");
    }

    #[test]
    fn convergence_csv_returns_none_when_missing() {
        let state = AppState::default();
        assert!(build_convergence_csv(&state).is_none());
    }

    #[test]
    fn importance_csv_returns_none_when_computing() {
        let state = AppState::default();
        let mut widgets = WidgetStates::default();
        widgets.importance.computing = true;
        assert!(build_importance_csv(&state, &widgets).is_none());
    }

    #[test]
    fn importance_csv_returns_none_when_no_cache() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        // importance_cache is empty, should return None
        assert!(build_importance_csv(&state, &widgets).is_none());
    }

    #[test]
    fn importance_csv_has_expected_columns() {
        use crate::state::app_state::SensitivityResult;
        use crate::state::results::RidgeResult;
        let mut state = AppState::default();
        let result = SensitivityResult {
            param_names: vec!["x".into(), "y".into()],
            spearman: vec![vec![0.9, 0.3]],
            ridge: vec![RidgeResult {
                beta: vec![0.8, 0.2],
                r_squared: 0.95,
            }],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        };
        // Spearman is cache_id=0
        state.importance_cache.insert((0u8, 0, false), result);
        let widgets = WidgetStates::default(); // metric=Spearman, obj_idx=0
        let csv = build_importance_csv(&state, &widgets).unwrap();
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "variable,importance_score,method");
        // 2 params → 2 data rows + header
        assert_eq!(csv.lines().count(), 3);
    }

    #[test]
    fn importance_csv_quotes_param_name_with_comma() {
        use crate::state::app_state::SensitivityResult;
        use crate::state::results::RidgeResult;
        let mut state = AppState::default();
        let result = SensitivityResult {
            param_names: vec!["x,y".into()],
            spearman: vec![vec![0.9]],
            ridge: vec![RidgeResult {
                beta: vec![0.8],
                r_squared: 0.95,
            }],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        };
        state.importance_cache.insert((0u8, 0, false), result);
        let widgets = WidgetStates::default();
        let csv = build_importance_csv(&state, &widgets).unwrap();
        assert_eq!(csv.lines().nth(1).unwrap(), "\"x,y\",0.9,Spearman");
    }

    #[test]
    fn importance_csv_guards_param_name_starting_with_equals() {
        use crate::state::app_state::SensitivityResult;
        use crate::state::results::RidgeResult;
        let mut state = AppState::default();
        let result = SensitivityResult {
            param_names: vec!["=SUM(A1)".into()],
            spearman: vec![vec![0.9]],
            ridge: vec![RidgeResult {
                beta: vec![0.8],
                r_squared: 0.95,
            }],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        };
        state.importance_cache.insert((0u8, 0, false), result);
        let widgets = WidgetStates::default();
        let csv = build_importance_csv(&state, &widgets).unwrap();
        assert_eq!(csv.lines().nth(1).unwrap(), "'=SUM(A1),0.9,Spearman");
    }

    #[test]
    fn sensitivity_csv_has_objective_columns_in_header() {
        use crate::state::app_state::HeatmapMatrix;
        let widgets = WidgetStates::default(); // default metric = Spearman (id 0)
        let mut state = AppState::default();
        state.sensitivity_heatmap_cache.insert(
            (widgets.sensitivity_heatmap.metric.cache_id(), false),
            HeatmapMatrix {
                param_names: vec!["x".into(), "y".into()],
                objective_names: vec!["f1".into(), "f2".into()],
                values: vec![vec![0.9, 0.3], vec![0.5, 0.7]],
                signed: true,
            },
        );
        let csv = build_sensitivity_csv(&state, &widgets).unwrap();
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "variable,f1,f2");
        assert_eq!(csv.lines().count(), 3); // header + 2 params
    }

    #[test]
    fn sensitivity_csv_returns_none_when_no_result() {
        let state = AppState::default(); // sensitivity_heatmap_cache is empty
        let widgets = WidgetStates::default();
        assert!(build_sensitivity_csv(&state, &widgets).is_none());
    }

    #[test]
    fn trial_based_csv_has_trial_id_header() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.0_f64);
        study.set_rows_for_test(vec![make_trial(0, p, vec![0.5])]);
        state.current_study = Some(study);
        let csv = build_trial_based_csv(&state).unwrap();
        assert!(csv.lines().next().unwrap().contains("trial_id"));
    }

    #[test]
    fn trial_based_csv_returns_none_when_no_study() {
        let state = AppState::default();
        assert!(build_trial_based_csv(&state).is_none());
    }

    #[test]
    fn cluster_csv_returns_none_when_no_cluster_result() {
        let mut state = AppState::default();
        let widgets = WidgetStates::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        // no cluster result cached
        assert!(build_cluster_csv(&ChartId::ClusterScatter, &state, &widgets).is_none());
    }

    #[test]
    fn cluster_csv_includes_cluster_id_column() {
        use crate::state::results::ClusterResult;
        let mut state = AppState::default();
        let widgets = WidgetStates::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.0_f64);
        study.set_rows_for_test(vec![
            make_trial(0, p.clone(), vec![0.5]),
            make_trial(1, p.clone(), vec![1.0]),
        ]);
        state.current_study = Some(study);
        // 2D チャートの設定キーで結果をキャッシュに登録する。
        let key = widgets.cluster_scatter.cache_key();
        state.cluster_cache.insert(
            key,
            ClusterResult {
                labels: vec![0, 1],
                n_clusters: 2,
            },
        );
        let csv = build_cluster_csv(&ChartId::ClusterScatter, &state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].ends_with(",cluster_id"), "header: {}", lines[0]);
        assert!(lines[1].ends_with(",0"), "row0: {}", lines[1]);
        assert!(lines[2].ends_with(",1"), "row1: {}", lines[2]);
    }

    #[test]
    fn pareto_csv_includes_all_trials_with_rank() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![
            make_trial_ranked(0, HashMap::new(), vec![1.0], 0),
            make_trial_ranked(1, HashMap::new(), vec![2.0], 1),
            make_trial_ranked(2, HashMap::new(), vec![3.0], 2),
        ]);
        study.pareto_indices = vec![0];
        state.current_study = Some(study);
        let csv = build_pareto_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        // Header + every trial, each tagged with its Pareto rank.
        assert_eq!(lines.len(), 4, "header + 3 rows: {:?}", lines);
        assert!(lines[0].contains("pareto_rank"));
        assert!(lines[1].ends_with(",0"), "row0 rank: {}", lines[1]);
        assert!(lines[2].ends_with(",1"), "row1 rank: {}", lines[2]);
        assert!(lines[3].ends_with(",2"), "row2 rank: {}", lines[3]);
    }

    #[test]
    fn pareto_csv_uses_row_rank_not_trial_id() {
        // Regression: rank must be read per row, not by matching trial ids
        // against pareto_indices. With non-contiguous trial ids (100/200/300)
        // the per-row rank must still be emitted correctly for every row.
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![
            make_trial_ranked(100, HashMap::new(), vec![1.0], 0),
            make_trial_ranked(200, HashMap::new(), vec![2.0], 1),
            make_trial_ranked(300, HashMap::new(), vec![3.0], 2),
        ]);
        study.pareto_indices = vec![0];
        state.current_study = Some(study);
        let csv = build_pareto_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 4, "header + 3 rows: {:?}", lines);
        // First data row: trial id 100, trial.number 100 (= test trial_number), rank 0.
        assert!(lines[1].starts_with("100,100,"), "row: {}", lines[1]);
        assert!(lines[1].ends_with(",0"), "row0 rank: {}", lines[1]);
        assert!(lines[3].starts_with("300,300,"), "row: {}", lines[3]);
        assert!(lines[3].ends_with(",2"), "row2 rank: {}", lines[3]);
    }

    #[test]
    fn pareto_csv_returns_none_when_no_pareto() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        // pareto_indices is empty
        state.current_study = Some(study);
        assert!(build_pareto_csv(&state).is_none());
    }

    // ── TASK-2325: PDP tests ──────────────────────────────────────

    #[test]
    fn pdp_csv_returns_none_when_no_result() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        assert!(build_pdp_csv(&state, &widgets).is_none());
    }

    #[test]
    fn pdp_csv_has_correct_header() {
        use crate::state::messages::PdpResult1d;
        let mut widgets = WidgetStates::default();
        widgets.pdp_chart.result = Some(PdpResult1d {
            x_values: vec![0.0, 1.0],
            y_values: vec![0.5, 0.8],
            y_upper: Some(vec![0.6, 0.9]),
            y_lower: Some(vec![0.4, 0.7]),
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
        });
        let state = AppState::default();
        let csv = build_pdp_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "variable,variable_value,predicted_objective,lower_ci,upper_ci"
        );
        assert_eq!(lines.len(), 3); // header + 2 points
        assert_eq!(lines[1], "x,0,0.5,0.4,0.6");
    }

    #[test]
    fn pdp_csv_handles_missing_ci() {
        use crate::state::messages::PdpResult1d;
        let mut widgets = WidgetStates::default();
        widgets.pdp_chart.result = Some(PdpResult1d {
            x_values: vec![0.0],
            y_values: vec![0.5],
            y_upper: None,
            y_lower: None,
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
        });
        let state = AppState::default();
        let csv = build_pdp_csv(&state, &widgets).unwrap();
        // lower_ci and upper_ci should be empty strings
        assert_eq!(csv.lines().nth(1).unwrap(), "x,0,0.5,,");
    }

    #[test]
    fn pdp_2d_csv_returns_none_when_no_result() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        assert!(build_pdp_2d_csv(&state, &widgets).is_none());
    }

    #[test]
    fn pdp_2d_csv_has_correct_header_and_grid() {
        use crate::state::messages::PdpResult2d;
        let mut widgets = WidgetStates::default();
        widgets.pdp_2d.result = Some(PdpResult2d {
            x_values: vec![0.0, 1.0],
            y_values: vec![2.0, 3.0],
            z_values: vec![vec![0.1, 0.2], vec![0.3, 0.4]],
            param1_name: "x".to_string(),
            param2_name: "y".to_string(),
            objective_name: "f".to_string(),
            uncertainties: None,
        });
        let state = AppState::default();
        let csv = build_pdp_2d_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "param1_name,param1_value,param2_name,param2_value,predicted_objective"
        );
        // 2x2 grid → 4 data rows
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1], "x,0,y,2,0.1");
        assert_eq!(lines[2], "x,0,y,3,0.2");
    }

    // ── TASK-2324: MCDM/AHP tests ─────────────────────────────────

    fn make_topsis_mcdm(trial_rows_len: usize) -> crate::state::app_state::McdmResult {
        use crate::state::results::TopsisResult;
        McdmResult::Topsis(TopsisResult {
            scores: (0..trial_rows_len).map(|i| i as f64 * 0.1 + 0.5).collect(),
            ranked_indices: (0..trial_rows_len as u32).rev().collect(),
            duration_ms: 1.0,
        })
    }

    #[test]
    fn mcdm_rank_csv_has_correct_header_and_method_topsis() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![
            make_trial(10, HashMap::new(), vec![1.0]),
            make_trial(11, HashMap::new(), vec![2.0]),
        ]);
        state.current_study = Some(study);
        let result = make_topsis_mcdm(2);
        let csv = build_mcdm_rank_csv(&result, &state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_id,rank,score,method");
        assert!(lines[1].ends_with(",TOPSIS"), "method column: {}", lines[1]);
        assert_eq!(lines.len(), 3); // header + 2 rows
    }

    #[test]
    fn mcdm_rank_csv_returns_none_when_no_study() {
        let state = AppState::default();
        let result = make_topsis_mcdm(1);
        // current_study が無い場合は None
        assert!(build_mcdm_rank_csv(&result, &state).is_none());
    }

    #[test]
    fn mcdm_scatter_csv_has_correct_header() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        let result = make_topsis_mcdm(1);
        let csv = build_mcdm_scatter_csv(&result, &state).unwrap();
        assert_eq!(csv.lines().next().unwrap(), "trial_id,rank,primary_score");
    }

    #[test]
    fn mcdm_table_csv_topsis_header() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        let result = make_topsis_mcdm(1);
        let csv = build_mcdm_table_csv(&result, &state).unwrap();
        assert_eq!(csv.lines().next().unwrap(), "trial_id,rank,topsis_score");
    }

    #[test]
    fn mcdm_table_csv_vikor_header() {
        use crate::state::results::VikorResult;
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        let result = McdmResult::Vikor(VikorResult {
            s_values: vec![0.3],
            r_values: vec![0.2],
            q_values: vec![0.1],
            display_scores: vec![0.4],
            ranked_indices: vec![0],
            compromise_indices: vec![0],
            duration_ms: 1.0,
        });
        let csv = build_mcdm_table_csv(&result, &state).unwrap();
        assert_eq!(
            csv.lines().next().unwrap(),
            "trial_id,rank,s_value,r_value,q_value"
        );
    }

    #[test]
    fn mcdm_table_csv_returns_none_when_no_study() {
        let state = AppState::default();
        let result = make_topsis_mcdm(1);
        assert!(build_mcdm_table_csv(&result, &state).is_none());
    }

    #[test]
    fn slice_csv_includes_param_obj_and_pareto() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.5_f64);
        study.set_rows_for_test(vec![make_trial(0, p, vec![0.5])]);
        study.pareto_indices = vec![0];
        state.current_study = Some(study);
        let widgets = WidgetStates::default();

        let csv = build_slice_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_id,x,f,is_pareto");
        assert_eq!(lines[1], "0,1.5,0.5,true");
    }

    // ── 多目的サロゲート最適化の CSV テスト ──────────────────────

    fn make_multi_opt_result() -> crate::state::messages::SurrogateMultiOptUiResult {
        use tunny_core::surrogate_opt::ParetoFrontPoint;
        crate::state::messages::SurrogateMultiOptUiResult {
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_names: vec!["f0".to_string(), "f1".to_string()],
            front: vec![
                ParetoFrontPoint {
                    params: vec![0.1, 0.2],
                    values: vec![1.0, 4.0],
                },
                ParetoFrontPoint {
                    params: vec![0.3, 0.4],
                    values: vec![2.0, 3.0],
                },
            ],
            r_squared: vec![0.9, 0.85],
        }
    }

    #[test]
    fn multi_opt_csv_header_is_objectives_then_params() {
        let result = make_multi_opt_result();
        let csv = build_surrogate_multi_opt_csv(&result);
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "f0,f1,x,y");
    }

    #[test]
    fn multi_opt_csv_row_count_equals_front_size() {
        let result = make_multi_opt_result();
        let csv = build_surrogate_multi_opt_csv(&result);
        // ヘッダ 1 行 + フロント点 2 行 = 合計 3 行
        assert_eq!(csv.lines().count(), 3);
    }

    #[test]
    fn has_csv_data_true_when_multi_result_present() {
        let mut widgets = WidgetStates::default();
        let state = AppState::default();
        widgets.surrogate_opt.multi_result = Some(make_multi_opt_result());
        assert!(has_csv_data(&ChartId::SurrogateOpt, &state, &widgets));
    }

    #[test]
    fn build_surrogate_opt_csv_prefers_multi_result() {
        let mut widgets = WidgetStates::default();
        widgets.surrogate_opt.multi_result = Some(make_multi_opt_result());
        // 単目的結果も入れておく（多目的が優先されること）。
        widgets.surrogate_opt.result = Some(crate::state::messages::SurrogateOptUiResult {
            best_params: vec![("x".to_string(), 0.5)],
            best_value: 1.0,
            predicted_std: None,
            r_squared: 0.9,
            objective_name: "f".to_string(),
            minimize: true,
            best_observed_value: 1.5,
            predicted_constraints: vec![],
            feasibility_probability: None,
        });
        let state = AppState::default();
        let csv = build_chart_csv(&ChartId::SurrogateOpt, &state, &widgets).unwrap();
        // 多目的 CSV のヘッダには目的名が含まれる
        let header = csv.lines().next().unwrap();
        assert!(
            header.contains("f0") && header.contains("f1"),
            "header: {}",
            header
        );
    }
}
