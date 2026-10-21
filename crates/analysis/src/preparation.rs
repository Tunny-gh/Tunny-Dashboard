//! Method-specific readiness and derived inputs. These selections deliberately
//! preserve different numerical missingness/fallback policies, rather than
//! imposing one complete-case policy on every method.
use crate::sensitivity::heatmap::HeatmapMethod;
use crate::surrogate_opt::ConstraintData;
use crate::{
    dataframe::DataFrame,
    projection::Projection,
    session::{AnalysisKind, Condition, Readiness},
    training,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum FeatureSpace {
    Parameters,
    Objectives,
    Both,
}
#[derive(Clone, Debug)]
pub enum FitObjectives {
    Name(String),
    Index(usize),
    All,
}
#[derive(Clone)]
pub enum Models {
    Single(Arc<crate::surrogate_opt::TrainedSurrogate>),
    Multi(Arc<Vec<crate::surrogate_opt::TrainedSurrogate>>),
}
impl Models {
    pub(crate) fn objective_names(&self) -> Vec<String> {
        match self {
            Self::Single(m) => vec![m.objective_name.clone()],
            Self::Multi(m) => m.iter().map(|m| m.objective_name.clone()).collect(),
        }
    }
    pub fn single(&self) -> &crate::surrogate_opt::TrainedSurrogate {
        if let Self::Single(m) = self {
            m
        } else {
            panic!("Expected a prepared single model")
        }
    }
    pub fn multi(&self) -> &[crate::surrogate_opt::TrainedSurrogate] {
        if let Self::Multi(m) = self {
            m
        } else {
            panic!("Expected prepared multiple models")
        }
    }
    pub(crate) fn keys(&self) -> Result<Vec<crate::session::ResultKey>, Readiness> {
        let models: Vec<_> = match self {
            Self::Single(m) => vec![m.as_ref()],
            Self::Multi(m) => m.iter().collect(),
        };
        if models.is_empty() {
            return Err(unavailable(
                "No trained model available. Run Fit & Validate first.",
            ));
        }
        models
            .into_iter()
            .map(|m| {
                m.fit_key().cloned().ok_or_else(|| {
                    unavailable("No trained model available. Run Fit & Validate first.")
                })
            })
            .collect()
    }
}
#[derive(Clone, Debug)]
pub enum Preparation {
    Table,
    Entropy,
    Mcdm,
    Cluster {
        space: FeatureSpace,
        manual_k: Option<usize>,
    },
    Fit {
        objectives: FitObjectives,
        constraints: bool,
    },
    Compare {
        objective: usize,
        slice: usize,
    },
    Sensitivity {
        method: HeatmapMethod,
        objective: Option<usize>,
        feasible_only: bool,
        samples: usize,
    },
    Pdp {
        params: Vec<String>,
        objective: String,
        feasible_only: bool,
        two_dimensional: bool,
    },
    Contour {
        x: String,
        y: String,
        value: String,
        feasible_only: bool,
        n_grid: usize,
    },
    Convergence {
        target_points: usize,
    },
    Pca {
        space: crate::clustering::PcaSpace,
        components: usize,
    },
    Som {
        names: Vec<String>,
        spec: crate::clustering::SomSpec,
    },
    Linkage {
        names: Vec<String>,
    },
}

impl Preparation {
    pub(crate) fn default_for(kind: AnalysisKind) -> Result<Self, Readiness> {
        match kind {
            AnalysisKind::Entropy => Ok(Self::Entropy),
            AnalysisKind::Mcdm => Ok(Self::Mcdm),
            AnalysisKind::Convergence => Ok(Self::Convergence { target_points: 50 }),
            AnalysisKind::Pareto
            | AnalysisKind::History
            | AnalysisKind::Report
            | AnalysisKind::Distribution
            | AnalysisKind::Optimize
            | AnalysisKind::Suggest => Ok(Self::Table),
            _ => Err(unavailable(
                "Method-specific preparation settings are required",
            )),
        }
    }
    pub(crate) fn condition(&self) -> Condition {
        Condition::Text(format!("{self:?}"))
    }
    pub(crate) fn matches(&self, kind: AnalysisKind) -> bool {
        match self {
            Self::Table => matches!(
                kind,
                AnalysisKind::Pareto
                    | AnalysisKind::History
                    | AnalysisKind::Report
                    | AnalysisKind::Distribution
                    | AnalysisKind::Optimize
                    | AnalysisKind::Suggest
            ),
            Self::Entropy => kind == AnalysisKind::Entropy,
            Self::Mcdm => kind == AnalysisKind::Mcdm,
            Self::Cluster { .. } => kind == AnalysisKind::Cluster,
            Self::Fit { .. } => matches!(
                kind,
                AnalysisKind::Fit | AnalysisKind::Robustness | AnalysisKind::Surface
            ),
            Self::Compare { .. } => kind == AnalysisKind::Compare,
            Self::Sensitivity { objective, .. } => {
                if objective.is_none() {
                    kind == AnalysisKind::Heatmap
                } else {
                    matches!(kind, AnalysisKind::Importance | AnalysisKind::Sobol)
                }
            }
            Self::Pdp {
                two_dimensional, ..
            } => {
                kind == if *two_dimensional {
                    AnalysisKind::Pdp2d
                } else {
                    AnalysisKind::Pdp1d
                }
            }
            Self::Contour { .. } => kind == AnalysisKind::Contour,
            Self::Convergence { .. } => kind == AnalysisKind::Convergence,
            Self::Pca { .. } => kind == AnalysisKind::Pca,
            Self::Som { .. } => kind == AnalysisKind::Som,
            Self::Linkage { .. } => kind == AnalysisKind::Linkage,
        }
    }
}

pub struct ObjectiveData {
    pub values: Vec<f64>,
    pub names: Vec<String>,
    pub minimize: Vec<bool>,
    pub rows: Vec<usize>,
    pub total_rows: usize,
}
pub struct FeatureData {
    pub names: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
    pub rows: Vec<usize>,
    pub total_rows: usize,
}
pub struct FitData {
    pub names: Vec<String>,
    pub x: Vec<Vec<f64>>,
    pub ys: Vec<Vec<f64>>,
    pub objectives: Vec<String>,
    pub bounds: Vec<Option<(f64, f64)>>,
    pub constraints: Vec<ConstraintData>,
    pub rows: Vec<usize>,
    pub minimize: Vec<bool>,
    pub anchor: Option<Vec<f64>>,
    pub observed: Vec<(f64, f64)>,
}
pub struct SensitivityData {
    pub df: Arc<DataFrame>,
    /// Pairwise maps for Spearman; complete-case maps for fitting methods.
    /// Each index is a row of the original input projection, not a trial ID.
    pub feature_rows: Vec<Vec<usize>>,
    pub rows: Vec<usize>,
}
pub struct PdpData {
    pub names: Vec<String>,
    pub objective: String,
    pub x: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub targets: Vec<usize>,
    pub rows: Vec<usize>,
}
pub struct ContourData {
    pub points: Vec<[f64; 3]>,
    pub trial_ids: Vec<u32>,
    pub rows: Vec<usize>,
    pub geometry: crate::contour::PreparedGeometry,
}
pub struct SeriesData {
    pub trial_ids: Vec<u32>,
    pub objectives: Vec<Vec<f64>>,
    pub step: usize,
    pub rows: Vec<usize>,
}
pub enum PreparedData {
    Table,
    Objectives(ObjectiveData),
    Features(FeatureData),
    Fit(FitData),
    Sensitivity(SensitivityData),
    Pdp(PdpData),
    Contour(ContourData),
    Convergence(Vec<SeriesData>),
}
impl PreparedData {
    pub fn objectives(&self) -> &ObjectiveData {
        if let Self::Objectives(v) = self {
            v
        } else {
            panic!("Expected prepared objectives")
        }
    }
    pub fn features(&self) -> &FeatureData {
        if let Self::Features(v) = self {
            v
        } else {
            panic!("Expected prepared features")
        }
    }
    pub fn fit(&self) -> &FitData {
        if let Self::Fit(v) = self {
            v
        } else {
            panic!("Expected prepared fit")
        }
    }
    pub fn sensitivity(&self) -> &SensitivityData {
        if let Self::Sensitivity(v) = self {
            v
        } else {
            panic!("Expected prepared sensitivity")
        }
    }
    pub fn pdp(&self) -> &PdpData {
        if let Self::Pdp(v) = self {
            v
        } else {
            panic!("Expected prepared PDP")
        }
    }
    pub fn contour(&self) -> &ContourData {
        if let Self::Contour(v) = self {
            v
        } else {
            panic!("Expected prepared contour")
        }
    }
    pub fn series(&self) -> &[SeriesData] {
        if let Self::Convergence(v) = self {
            v
        } else {
            panic!("Expected prepared convergence")
        }
    }
    pub(crate) fn row_maps(
        &self,
        tables: &[Arc<Projection>],
    ) -> Vec<Vec<tdashboard_study::TrialKey>> {
        let selected: Vec<Vec<usize>> = match self {
            Self::Table => return tables.iter().map(|t| t.rows.clone()).collect(),
            Self::Objectives(v) => vec![v.rows.clone()],
            Self::Features(v) => vec![v.rows.clone()],
            Self::Fit(v) => vec![v.rows.clone()],
            Self::Sensitivity(v) => vec![v.rows.clone()],
            Self::Pdp(v) => vec![v.rows.clone()],
            Self::Contour(v) => vec![v.rows.clone()],
            Self::Convergence(v) => v.iter().map(|s| s.rows.clone()).collect(),
        };
        selected
            .into_iter()
            .zip(tables)
            .map(|(rows, table)| rows.into_iter().map(|r| table.rows[r]).collect())
            .collect()
    }
}
fn unavailable(message: impl Into<String>) -> Readiness {
    Readiness::Unavailable(message.into())
}
pub(crate) fn flags(table: &Projection, names: &[String]) -> Vec<bool> {
    // Full-objective requests retain descriptor order. Repeated display labels
    // must not collapse different objective directions to the first match.
    if names == table.meta.objective_names {
        return (0..names.len())
            .map(|i| {
                table
                    .meta
                    .directions
                    .get(i)
                    .is_none_or(|d| matches!(d, tdashboard_study::Direction::Minimize))
            })
            .collect();
    }
    names
        .iter()
        .map(|name| {
            table
                .meta
                .objective_names
                .iter()
                .position(|n| n == name)
                .and_then(|i| table.meta.directions.get(i))
                .is_none_or(|d| matches!(d, tdashboard_study::Direction::Minimize))
        })
        .collect()
}
fn objectives(table: &Projection, rows: Vec<usize>) -> ObjectiveData {
    let names = table.meta.objective_names.clone();
    let values = rows
        .iter()
        .flat_map(|&r| {
            names.iter().map(move |n| {
                table
                    .dataframe
                    .get_numeric_column(n)
                    .and_then(|c| c.get(r))
                    .copied()
                    .unwrap_or(0.0)
            })
        })
        .collect();
    ObjectiveData {
        values,
        minimize: flags(table, &names),
        names,
        rows,
        total_rows: table.dataframe.row_count(),
    }
}

pub(crate) fn prepare(
    tables: &[Arc<Projection>],
    spec: &Preparation,
    front: Option<&crate::pareto::ParetoResult>,
) -> Result<PreparedData, Readiness> {
    let table = &tables[0];
    let df = &table.dataframe;
    let all = || (0..df.row_count()).collect::<Vec<_>>();
    match spec {
        Preparation::Table => Ok(PreparedData::Table),
        Preparation::Entropy => {
            let data = objectives(table, all());
            if data.rows.is_empty() || data.names.is_empty() {
                return Err(unavailable("At least one trial and one objective required"));
            }
            let finite_rows: Vec<_> = data
                .values
                .chunks_exact(data.names.len())
                .enumerate()
                .filter_map(|(row, values)| values.iter().all(|v| v.is_finite()).then_some(row))
                .collect();
            if finite_rows.is_empty() {
                return Err(unavailable(
                    "No valid trials for entropy computation (all NaN)",
                ));
            }
            Ok(PreparedData::Objectives(objectives(table, finite_rows)))
        }
        Preparation::Mcdm => {
            let front = front.ok_or(Readiness::Waiting)?;
            let rows: Vec<_> = front
                .ranks
                .iter()
                .enumerate()
                .filter_map(|(r, &rank)| (rank == 0).then_some(r))
                .collect();
            if rows.is_empty() {
                return Err(unavailable(
                    "MCDM: Pareto front is empty. Run the optimizer first.",
                ));
            }
            Ok(PreparedData::Objectives(objectives(table, rows)))
        }
        Preparation::Cluster { space, manual_k } => {
            let front = front.ok_or(Readiness::Waiting)?;
            let data = cluster_features(
                df,
                &table.meta.param_names,
                &table.meta.objective_names,
                *space,
                &front.ranks,
            );
            if data.rows.len() < 2 || data.names.is_empty() {
                return Err(unavailable(
                    "At least 2 trials and one feature are required.",
                ));
            }
            if manual_k.is_some_and(|k| k < 2 || k > data.rows.len()) {
                return Err(unavailable("k must be in [2, trial_count]."));
            }
            Ok(PreparedData::Features(data))
        }
        Preparation::Fit {
            objectives,
            constraints,
        } => Ok(PreparedData::Fit(prepare_fit(
            table,
            objectives,
            *constraints,
            None,
            true,
        )?)),
        Preparation::Compare { objective, slice } => Ok(PreparedData::Fit(prepare_fit(
            table,
            &FitObjectives::Index(*objective),
            false,
            Some(*slice),
            false,
        )?)),
        Preparation::Sensitivity {
            method,
            objective,
            feasible_only,
            samples,
        } => prepare_sensitivity(table, *method, *objective, *feasible_only, *samples)
            .map(PreparedData::Sensitivity),
        Preparation::Pdp {
            params,
            objective,
            feasible_only,
            two_dimensional,
        } => prepare_pdp(df, params, objective, *feasible_only, *two_dimensional)
            .map(PreparedData::Pdp),
        Preparation::Contour {
            x,
            y,
            value,
            feasible_only,
            n_grid,
        } => {
            let columns: Option<Vec<_>> = [x, y, value]
                .iter()
                .map(|n| df.get_numeric_column(n))
                .collect();
            let columns = columns.ok_or_else(|| unavailable("Selected column not found"))?;
            let (mut points, mut trial_ids, mut rows) = (vec![], vec![], vec![]);
            for r in 0..df.row_count() {
                if *feasible_only && !df.feasibility().is_feasible(r) {
                    continue;
                }
                let values: Option<Vec<_>> = columns
                    .iter()
                    .map(|c| c.get(r).copied().filter(|v| v.is_finite()))
                    .collect();
                if let Some(v) = values {
                    points.push([v[0], v[1], v[2]]);
                    rows.push(r);
                    trial_ids.push(df.get_trial_id(r).unwrap_or(r as u32));
                }
            }
            if rows.len() < 3 {
                return Err(unavailable(
                    "Not enough finite points to interpolate (need >= 3).",
                ));
            }
            let geometry = crate::contour::prepare_geometry(&points)
                .filter(|_| *n_grid > 0)
                .ok_or_else(|| {
                    unavailable("Points are collinear or degenerate; cannot interpolate.")
                })?;
            Ok(PreparedData::Contour(ContourData {
                points,
                trial_ids,
                rows,
                geometry,
            }))
        }
        Preparation::Convergence { target_points } => Ok(PreparedData::Convergence(
            tables
                .iter()
                .map(|t| {
                    let step = (t.dataframe.row_count() / (*target_points).max(1)).max(1);
                    let rows: Vec<_> = (0..t.dataframe.row_count()).step_by(step).collect();
                    let trial_ids = rows
                        .iter()
                        .map(|&r| t.dataframe.get_trial_id(r).unwrap_or(r as u32))
                        .collect();
                    let objectives = rows
                        .iter()
                        .map(|&r| {
                            if !t.dataframe.feasibility().is_feasible(r) {
                                vec![f64::NAN; t.meta.objective_names.len()]
                            } else {
                                t.meta
                                    .objective_names
                                    .iter()
                                    .map(|n| {
                                        t.dataframe
                                            .get_numeric_column(n)
                                            .and_then(|c| c.get(r))
                                            .copied()
                                            .unwrap_or(0.0)
                                    })
                                    .collect()
                            }
                        })
                        .collect();
                    SeriesData {
                        trial_ids,
                        objectives,
                        step,
                        rows,
                    }
                })
                .collect(),
        )),
        Preparation::Pca { space, components } => {
            prepare_pca(df, *space, *components).map(PreparedData::Features)
        }
        Preparation::Som { names, spec } => {
            prepare_som(df, names, spec).map(PreparedData::Features)
        }
        Preparation::Linkage { names } => {
            prepare_features(df, names, 2, false).map(PreparedData::Features)
        }
    }
}

pub fn feature_rows(df: &DataFrame, names: &[String], categorical_zero: bool) -> FeatureData {
    let mut data = FeatureData {
        names: names.to_vec(),
        matrix: vec![],
        rows: vec![],
        total_rows: df.row_count(),
    };
    for r in 0..df.row_count() {
        let row: Option<Vec<_>> = names
            .iter()
            .map(|n| match df.get_numeric_column(n) {
                Some(c) => c.get(r).copied().filter(|v| v.is_finite()),
                None if categorical_zero && df.get_string_column(n).is_some() => Some(0.0),
                None => None,
            })
            .collect();
        if let Some(row) = row {
            data.matrix.push(row);
            data.rows.push(r);
        }
    }
    data
}

/// Existing Pareto-subset/finite-feature policy, shared with explicit table
/// adapters as well as the versioned preparation API.
pub fn cluster_features(
    df: &DataFrame,
    parameters: &[String],
    objectives: &[String],
    space: FeatureSpace,
    ranks: &[u32],
) -> FeatureData {
    let names = match space {
        FeatureSpace::Parameters => parameters.to_vec(),
        FeatureSpace::Objectives => objectives.to_vec(),
        FeatureSpace::Both => parameters.iter().chain(objectives).cloned().collect(),
    };
    let mut data = feature_rows(df, &names, true);
    let retained: Vec<_> = data
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, &r)| (ranks.get(r) == Some(&0)).then_some(i))
        .collect();
    data.matrix = retained.iter().map(|&i| data.matrix[i].clone()).collect();
    data.rows = retained.iter().map(|&i| data.rows[i]).collect();
    data
}

pub fn prepare_features(
    df: &DataFrame,
    names: &[String],
    minimum: usize,
    categorical_zero: bool,
) -> Result<FeatureData, Readiness> {
    let data = feature_rows(df, names, categorical_zero);
    if names.is_empty() || data.rows.len() < minimum {
        return Err(unavailable("Insufficient finite feature rows"));
    }
    Ok(data)
}

pub fn prepare_pca(
    df: &DataFrame,
    space: crate::clustering::PcaSpace,
    components: usize,
) -> Result<FeatureData, Readiness> {
    use crate::clustering::PcaSpace;
    if components == 0 {
        return Err(unavailable("PCA requires a positive component count"));
    }
    let names = match space {
        PcaSpace::Param => df.param_col_names().to_vec(),
        PcaSpace::Objective => df.objective_col_names().to_vec(),
        PcaSpace::All => df
            .param_col_names()
            .iter()
            .chain(df.objective_col_names())
            .chain(df.user_attr_numeric_col_names())
            .cloned()
            .collect(),
    };
    prepare_features(df, &names, 2, true)
}

pub fn prepare_som(
    df: &DataFrame,
    names: &[String],
    spec: &crate::clustering::SomSpec,
) -> Result<FeatureData, Readiness> {
    if spec.grid_w < 2 || spec.grid_h < 2 || spec.n_epochs == 0 {
        return Err(unavailable("Invalid SOM grid or epoch count"));
    }
    prepare_features(df, names, 3, false)
}

fn prepare_fit(
    table: &Projection,
    choice: &FitObjectives,
    constraints: bool,
    slice: Option<usize>,
    validate_fit: bool,
) -> Result<FitData, Readiness> {
    if training::numeric_param_names(&table.dataframe, &table.meta.param_names).is_empty() {
        return Err(unavailable("No numeric parameters available"));
    }
    let objectives = match choice {
        FitObjectives::Name(n) if table.meta.objective_names.contains(n) => vec![n.clone()],
        FitObjectives::Index(i) => vec![table
            .meta
            .objective_names
            .get(*i)
            .ok_or_else(|| unavailable("Invalid objective selection"))?
            .clone()],
        FitObjectives::All if table.meta.objective_names.len() >= 2 => {
            table.meta.objective_names.clone()
        }
        _ => return Err(unavailable("Invalid objective selection")),
    };
    let (names, x, ys, bounds, rows) = training::numeric_fit_xy_multi(
        &table.dataframe,
        &table.meta.param_names,
        &objectives,
        &table.meta.param_bounds,
    )
    .ok_or_else(|| unavailable("No numeric parameters available"))?;
    let mut anchor = None;
    let mut observed = vec![];
    if let Some(slice) = slice {
        if slice >= names.len() {
            return Err(unavailable("Invalid slice parameter selection"));
        }
        let minimize = flags(table, &objectives)[0];
        let values = table
            .dataframe
            .get_numeric_column(&objectives[0])
            .ok_or_else(|| unavailable("Could not resolve an anchor point"))?;
        let mut best = None;
        let mut value = if minimize {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
        for (r, &v) in values.iter().enumerate() {
            if v.is_finite() && (if minimize { v < value } else { v > value }) {
                value = v;
                best = Some(r);
            }
        }
        anchor = best
            .and_then(|best| rows.iter().position(|&r| r == best))
            .map(|i| x[i].clone());
        if anchor.is_none() {
            return Err(unavailable("Could not resolve an anchor point"));
        }
        observed = x.iter().zip(&ys[0]).map(|(x, &y)| (x[slice], y)).collect();
    }
    if validate_fit {
        crate::surrogate_opt::validate_inputs(&x, &ys[0]).map_err(unavailable)?;
    }
    let constraints = if constraints {
        training::collect_constraints(&table.dataframe, &rows)
    } else {
        vec![]
    };
    let minimize = flags(table, &objectives);
    Ok(FitData {
        names,
        x,
        ys,
        objectives,
        bounds,
        constraints,
        rows,
        minimize,
        anchor,
        observed,
    })
}

fn prepare_sensitivity(
    table: &Projection,
    method: HeatmapMethod,
    objective: Option<usize>,
    feasible_only: bool,
    samples: usize,
) -> Result<SensitivityData, Readiness> {
    let df = Arc::new(if feasible_only {
        table.dataframe.filter_feasible()
    } else {
        table.dataframe.clone()
    });
    let source_rows: Vec<_> = (0..table.dataframe.row_count())
        .filter(|&r| !feasible_only || table.dataframe.feasibility().is_feasible(r))
        .collect();
    let mut data = SensitivityData {
        df,
        feature_rows: vec![],
        rows: source_rows.clone(),
    };
    // Heatmaps keep the existing per-cell zero fallback, including empty input.
    let Some(objective) = objective else {
        return Ok(data);
    };
    let df = &data.df;
    let objective_name = df
        .objective_col_names()
        .get(objective)
        .ok_or_else(|| unavailable("Invalid objective selection"))?;
    if df.param_col_names().is_empty()
        || (!matches!(method, HeatmapMethod::Ard) && df.row_count() < 2)
    {
        return Err(unavailable("Insufficient sensitivity data"));
    }
    let y = df
        .get_numeric_column(objective_name)
        .map(|c| c.to_vec())
        .unwrap_or_else(|| vec![0.0; df.row_count()]);
    let params: Vec<_> = df
        .param_col_names()
        .iter()
        .filter(|n| {
            !matches!(method, HeatmapMethod::Spearman | HeatmapMethod::Ridge)
                || df.get_numeric_column(n).is_some()
        })
        .collect();
    let columns: Vec<Vec<f64>> = params
        .iter()
        .map(|n| {
            crate::sensitivity::get_param_numeric_values(df, n, df.row_count())
                .unwrap_or_else(|| vec![0.0; df.row_count()])
        })
        .collect();
    let complete: Vec<_> = (0..df.row_count())
        .filter(|&r| {
            y[r].is_finite()
                && columns.iter().all(|c| c[r].is_finite())
                && (!matches!(
                    method,
                    HeatmapMethod::SobolFirst | HeatmapMethod::SobolTotal
                ) || df
                    .objective_col_names()
                    .iter()
                    .all(|n| df.get_numeric_column(n).is_some_and(|c| c[r].is_finite())))
        })
        .collect();
    if matches!(method, HeatmapMethod::Spearman) {
        data.feature_rows = columns
            .iter()
            .map(|c| {
                (0..df.row_count())
                    .filter(|&r| y[r].is_finite() && c[r].is_finite())
                    .map(|r| source_rows[r])
                    .collect()
            })
            .collect();
    } else {
        data.feature_rows = vec![complete.iter().map(|&r| source_rows[r]).collect()];
    }
    if matches!(
        method,
        HeatmapMethod::RfAnova
            | HeatmapMethod::Mdi
            | HeatmapMethod::Shap
            | HeatmapMethod::Permutation
            | HeatmapMethod::SobolFirst
            | HeatmapMethod::SobolTotal
    ) && complete.len() < 2
    {
        return Err(unavailable("Insufficient finite sensitivity rows"));
    }
    if matches!(
        method,
        HeatmapMethod::SobolFirst | HeatmapMethod::SobolTotal
    ) && samples == 0
    {
        return Err(unavailable("Sobol sample count must be positive"));
    }
    if matches!(method, HeatmapMethod::Ard) {
        let x: Vec<Vec<_>> = (0..df.row_count())
            .map(|r| columns.iter().map(|c| c[r]).collect())
            .collect();
        crate::surrogate_opt::validate_inputs(&x, &y).map_err(unavailable)?;
    }
    Ok(data)
}

pub fn prepare_pdp(
    df: &DataFrame,
    params: &[String],
    objective: &str,
    feasible_only: bool,
    two_dimensional: bool,
) -> Result<PdpData, Readiness> {
    let names = df.param_col_names().to_vec();
    if !df.objective_col_names().iter().any(|n| n == objective) {
        return Err(unavailable("Invalid objective selection"));
    }
    let targets: Option<Vec<_>> = params
        .iter()
        .map(|n| names.iter().position(|p| p == n))
        .collect();
    let targets = targets.ok_or_else(|| unavailable("Invalid parameter selection"))?;
    if targets.is_empty() || (two_dimensional && (targets.len() != 2 || targets[0] == targets[1])) {
        return Err(unavailable("Distinct PDP parameters are required"));
    }
    let (x, y, rows) = training::pdp_rows(df, &names, objective, feasible_only, two_dimensional);
    // Empty/model-dependent numerical fallbacks remain execution results.
    Ok(PdpData {
        names,
        objective: objective.into(),
        x,
        y,
        targets,
        rows,
    })
}
