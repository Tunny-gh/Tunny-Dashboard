//! Grasshopper 定義に対する最適化ランナー。
//!
//! `GhEvaluator`（Rhino.Compute またはモック）で実目的関数を評価し、
//! 全試行を Optuna 互換 journal に記録する。journal に落とすことで既存の
//! ライブ更新・全分析ウィジェット・レポートが無改修でそのまま機能する。
//!
//! 使い方は 2 段階:
//! 1. `prepare_gh_run` — journal を開き study を作成（同期・軽量）。
//!    呼び出し側はこの直後に journal を開けば study 一覧に現れる。
//! 2. `run_prepared` — 最適化ループ本体（ブロッキング。バックグラウンド
//!    スレッドで呼ぶ）。進捗・キャンセルは `FitProgress` を共有する。
//!
//! 内部の最適化は既存実装を転用する: 正規化空間 [0,1]^d・全目的最小化の
//! 規約に合わせ、実変数範囲との変換と Maximize の符号反転をここで担う。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::data::extras::TrialState;
use crate::io::journal::parser::OptimizationDirection;
use crate::io::journal::writer::{JournalWriter, ParamDistribution};
use crate::math::rng::SeededRng;
use crate::surrogate_opt::optimizers::nsga2::{nsga2_minimize, Nsga2Config};
use crate::surrogate_opt::FitProgress;

use super::compute::GhEvaluator;
use super::problem::{GhProblem, GhVariable};

/// 評価失敗・キャンセル時に最適化アルゴリズムへ返すペナルティ値。
/// 無限大は crowding distance の正規化で NaN を生むため大きな有限値を使う。
const FAIL_PENALTY: f64 = 1e12;

/// サンプラーの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhSampler {
    /// 一様ランダムサンプリング（試行数 = `n_trials`）
    Random,
    /// NSGA-II（試行数 = 偶数化した個体数 ×（世代数 + 1））
    Nsga2,
}

/// 最適化実行の設定。
#[derive(Debug, Clone)]
pub struct GhRunConfig {
    pub study_name: String,
    /// 目的ごとの最適化方向（`GhProblem.objectives` と同数・同順）
    pub directions: Vec<OptimizationDirection>,
    pub sampler: GhSampler,
    /// Random サンプラーの試行数
    pub n_trials: usize,
    /// NSGA-II の個体数
    pub population_size: usize,
    /// NSGA-II の世代数
    pub generations: usize,
    pub seed: u64,
}

impl Default for GhRunConfig {
    fn default() -> Self {
        Self {
            study_name: "gh-optimization".to_string(),
            directions: Vec::new(),
            sampler: GhSampler::Nsga2,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            seed: 42,
        }
    }
}

/// 実行結果の要約。
#[derive(Debug, Clone)]
pub struct GhRunSummary {
    pub study_id: u32,
    /// COMPLETE で記録できた試行数
    pub completed: usize,
    /// 評価失敗（FAIL で記録）の試行数
    pub failed: usize,
    /// キャンセルで打ち切られたか
    pub cancelled: bool,
}

/// `prepare_gh_run` の結果。study 作成済みの journal writer を保持する。
pub struct PreparedGhRun {
    writer: Mutex<JournalWriter>,
    study_id: u32,
}

impl PreparedGhRun {
    /// 作成した study の ID（journal 内での連番）。
    pub fn study_id(&self) -> u32 {
        self.study_id
    }
}

/// journal を開いて study を作成する（同期・軽量）。
///
/// この呼び出しの直後から journal 上に study が存在するため、呼び出し側は
/// `run_prepared` をバックグラウンドで開始する前に journal を開いて
/// ライブ更新に載せることができる。
pub fn prepare_gh_run(
    journal_path: &Path,
    problem: &GhProblem,
    cfg: &GhRunConfig,
) -> Result<PreparedGhRun, String> {
    if cfg.directions.len() != problem.objectives.len() {
        return Err(format!(
            "最適化方向の数（{}）が目的の数（{}）と一致しません",
            cfg.directions.len(),
            problem.objectives.len()
        ));
    }
    if problem.variables.is_empty() {
        return Err("変数がありません".to_string());
    }
    let mut writer = JournalWriter::open(journal_path)?;
    let objective_names: Vec<String> = problem.objectives.iter().map(|o| o.name.clone()).collect();
    let study_id = writer.create_study(&cfg.study_name, &cfg.directions, &objective_names)?;
    Ok(PreparedGhRun {
        writer: Mutex::new(writer),
        study_id,
    })
}

/// 最適化ループ本体（ブロッキング）。バックグラウンドスレッドから呼ぶこと。
///
/// - 進捗は `progress` に反映される（total = 予定評価回数）
/// - `progress.request_cancel()` で以降の評価を打ち切る（実行中の solve は
///   完了を待つ）。キャンセル分は journal に記録しない
/// - 評価エラーの試行は FAIL として記録し、最適化アルゴリズムには
///   ペナルティ値を返して続行する
/// - journal への書き込み自体が失敗した場合は中断して Err を返す
pub fn run_prepared(
    prep: &PreparedGhRun,
    problem: &GhProblem,
    evaluator: &dyn GhEvaluator,
    cfg: &GhRunConfig,
    progress: &FitProgress,
) -> Result<GhRunSummary, String> {
    let recorder = TrialRecorder {
        writer: &prep.writer,
        study_id: prep.study_id,
        problem,
        directions: &cfg.directions,
        evaluator,
        progress,
        completed: AtomicUsize::new(0),
        failed: AtomicUsize::new(0),
        io_error: Mutex::new(None),
    };
    let n_dims = problem.variables.len();
    progress.set_stage("Rhino.Compute で評価中");

    match cfg.sampler {
        GhSampler::Random => {
            let n = cfg.n_trials.max(1);
            progress.set_total(n);
            (0..n).into_par_iter().for_each(|i| {
                // 並列でも決定論的になるよう試行ごとに独立シードを導出する
                let mut rng = SeededRng::from_seed(
                    cfg.seed
                        .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
                );
                let x: Vec<f64> = (0..n_dims).map(|_| rng.next_f64()).collect();
                recorder.eval_signed(&x);
            });
        }
        GhSampler::Nsga2 => {
            // nsga2_minimize は個体数を偶数（最低 4）に切り上げ、
            // 初期集団 + 各世代の子集団を評価する。
            let pop_even = (cfg.population_size.max(4) + 1) & !1;
            progress.set_total(pop_even * (cfg.generations + 1));
            let nsga_cfg = Nsga2Config {
                pop_size: cfg.population_size,
                generations: cfg.generations,
                seed: cfg.seed,
                ..Nsga2Config::for_objectives(cfg.directions.len())
            };
            // 定義保存時点のスライダー値を初期個体としてシードする
            let initial = vec![normalize_current(problem)];
            nsga2_minimize(|x| recorder.eval_signed(x), n_dims, &initial, &nsga_cfg);
        }
    }

    if let Some(e) = recorder
        .io_error
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        return Err(format!(
            "journal への書き込みに失敗したため中断しました: {e}"
        ));
    }
    Ok(GhRunSummary {
        study_id: prep.study_id,
        completed: recorder.completed.load(Ordering::Relaxed),
        failed: recorder.failed.load(Ordering::Relaxed),
        cancelled: progress.is_cancelled(),
    })
}

/// 1 試行の評価と journal 記録。並列評価スレッドから共有される。
struct TrialRecorder<'a> {
    writer: &'a Mutex<JournalWriter>,
    study_id: u32,
    problem: &'a GhProblem,
    directions: &'a [OptimizationDirection],
    evaluator: &'a dyn GhEvaluator,
    progress: &'a FitProgress,
    completed: AtomicUsize,
    failed: AtomicUsize,
    /// journal 書き込みエラー（最初の 1 件）。発生後は新規評価を止める。
    io_error: Mutex<Option<String>>,
}

impl TrialRecorder<'_> {
    /// 正規化点を評価し、最小化規約に符号調整した目的値を返す。
    fn eval_signed(&self, x_norm: &[f64]) -> Vec<f64> {
        let n_obj = self.directions.len();
        if self.progress.is_cancelled() || self.has_io_error() {
            return vec![FAIL_PENALTY; n_obj];
        }
        let values = denormalize(self.problem, x_norm);

        let trial_id = match self.begin_trial(&values) {
            Ok(id) => id,
            Err(e) => {
                self.set_io_error(e);
                return vec![FAIL_PENALTY; n_obj];
            }
        };

        match self.evaluator.evaluate(&values) {
            Ok(objectives) if objectives.len() == n_obj => {
                if let Err(e) = self.finish(trial_id, TrialState::Complete, &objectives) {
                    self.set_io_error(e);
                    return vec![FAIL_PENALTY; n_obj];
                }
                self.completed.fetch_add(1, Ordering::Relaxed);
                self.progress.inc_done();
                objectives
                    .iter()
                    .zip(self.directions)
                    .map(|(v, d)| match d {
                        OptimizationDirection::Minimize => *v,
                        OptimizationDirection::Maximize => -*v,
                    })
                    .collect()
            }
            Ok(objectives) => {
                self.record_failure(
                    trial_id,
                    format!(
                        "目的値の数が一致しません（期待 {n_obj}、実際 {}）",
                        objectives.len()
                    ),
                );
                vec![FAIL_PENALTY; n_obj]
            }
            Err(e) => {
                self.record_failure(trial_id, e);
                vec![FAIL_PENALTY; n_obj]
            }
        }
    }

    /// trial を作成し param を記録する（writer ロックは 1 回で済ませる）。
    fn begin_trial(&self, values: &[f64]) -> Result<u32, String> {
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        let trial_id = writer.create_trial(self.study_id)?;
        for (var, value) in self.problem.variables.iter().zip(values) {
            let dist = if var.is_integer {
                ParamDistribution::Int {
                    low: var.low.round() as i64,
                    high: var.high.round() as i64,
                }
            } else {
                ParamDistribution::Float {
                    low: var.low,
                    high: var.high,
                }
            };
            writer.set_trial_param(trial_id, &var.name, *value, &dist)?;
        }
        Ok(trial_id)
    }

    fn finish(&self, trial_id: u32, state: TrialState, values: &[f64]) -> Result<(), String> {
        self.writer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .finish_trial(trial_id, state, values)
    }

    fn record_failure(&self, trial_id: u32, reason: String) {
        if let Err(e) = self.finish(trial_id, TrialState::Fail, &[]) {
            self.set_io_error(e);
            return;
        }
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.progress.inc_done();
        let short: String = reason.chars().take(120).collect();
        self.progress.set_stage(format!("評価エラーあり: {short}"));
    }

    fn has_io_error(&self) -> bool {
        self.io_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    fn set_io_error(&self, e: String) {
        let mut guard = self.io_error.lock().unwrap_or_else(|e| e.into_inner());
        guard.get_or_insert(e);
    }
}

/// 正規化点 [0,1]^d をスライダーの実値に変換する。
/// スライダーの丸め（整数 / 小数桁数）を適用し、journal に記録する値と
/// Compute に送る値を一致させる。
fn denormalize(problem: &GhProblem, x_norm: &[f64]) -> Vec<f64> {
    problem
        .variables
        .iter()
        .zip(x_norm)
        .map(|(var, x)| {
            let x = x.clamp(0.0, 1.0);
            let raw = var.low + x * (var.high - var.low);
            round_variable(var, raw)
        })
        .collect()
}

/// 現在のスライダー値を正規化空間に写す（NSGA-II の初期個体シード用）。
fn normalize_current(problem: &GhProblem) -> Vec<f64> {
    problem
        .variables
        .iter()
        .map(|var| ((var.value - var.low) / (var.high - var.low)).clamp(0.0, 1.0))
        .collect()
}

fn round_variable(var: &GhVariable, raw: f64) -> f64 {
    if var.is_integer {
        raw.round()
    } else {
        let scale = 10f64.powi(var.digits.min(15) as i32);
        (raw * scale).round() / scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::fixtures::sample_ghx;
    use crate::gh::problem::extract_problem;
    use crate::io::journal::parser::parse_single_study;

    /// クロージャで目的値を計算するモック評価器。
    struct FnEvaluator<F: Fn(&[f64]) -> Result<Vec<f64>, String> + Send + Sync>(F);

    impl<F: Fn(&[f64]) -> Result<Vec<f64>, String> + Send + Sync> GhEvaluator for FnEvaluator<F> {
        fn evaluate(&self, values: &[f64]) -> Result<Vec<f64>, String> {
            (self.0)(values)
        }
    }

    fn test_cfg(sampler: GhSampler) -> GhRunConfig {
        GhRunConfig {
            study_name: "gh-test".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize,
            ],
            sampler,
            n_trials: 6,
            population_size: 4,
            generations: 1,
            seed: 7,
        }
    }

    fn sum_diff_evaluator() -> impl GhEvaluator {
        FnEvaluator(|v: &[f64]| Ok(vec![v[0] + v[1], v[0] - v[1]]))
    }

    #[test]
    fn random_sampler_records_all_trials() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

        assert_eq!(summary.completed, 6);
        assert_eq!(summary.failed, 0);
        assert!(!summary.cancelled);

        let data = std::fs::read(&journal).unwrap();
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.name, "gh-test");
        assert_eq!(meta.completed_trials, 6);
        assert_eq!(meta.objective_names, vec!["weight", "disp"]);
        assert_eq!(
            meta.directions,
            vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize
            ]
        );
        assert_eq!(extras.trials.len(), 6);

        // journal に記録された param と目的値の整合（obj0 = span + count）
        let span = df.get_numeric_column("span").unwrap().to_vec();
        let count = df.get_numeric_column("count").unwrap().to_vec();
        let weight = df.get_numeric_column("weight").unwrap().to_vec();
        for i in 0..df.row_count() {
            assert!((span[i] + count[i] - weight[i]).abs() < 1e-9);
            // 整数スライダーは整数値、実数スライダーは範囲内
            assert_eq!(count[i], count[i].round());
            assert!((1.0..=10.0).contains(&count[i]));
            assert!((3.0..=12.0).contains(&span[i]));
        }
        // param_bounds がスライダー範囲を反映
        assert_eq!(meta.param_bounds.get("span"), Some(&(3.0, 12.0)));
    }

    #[test]
    fn nsga2_sampler_runs_expected_evaluations() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Nsga2);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let summary =
            run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

        // 偶数化した個体数 4 ×（世代 1 + 初期 1）= 8 評価
        assert_eq!(summary.completed, 8);
        let snapshot = progress.snapshot();
        assert_eq!(snapshot.total, 8);
        assert_eq!(snapshot.done, 8);
    }

    #[test]
    fn evaluation_errors_are_recorded_as_fail() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        let failing = FnEvaluator(|_: &[f64]| Err("solve failed".to_string()));
        let summary = run_prepared(&prep, &problem, &failing, &cfg, &progress).unwrap();

        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 6);

        let data = std::fs::read(&journal).unwrap();
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.completed_trials, 0);
        assert_eq!(meta.total_trials, 6);
        assert_eq!(df.row_count(), 0);
        assert!(extras
            .trials
            .iter()
            .all(|t| t.state == crate::data::extras::TrialState::Fail));
    }

    #[test]
    fn cancel_before_run_records_nothing() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let cfg = test_cfg(GhSampler::Random);

        let prep = prepare_gh_run(&journal, &problem, &cfg).unwrap();
        let progress = FitProgress::new();
        progress.request_cancel();
        let summary =
            run_prepared(&prep, &problem, &sum_diff_evaluator(), &cfg, &progress).unwrap();

        assert!(summary.cancelled);
        assert_eq!(summary.completed, 0);
        assert_eq!(summary.failed, 0);
        let data = std::fs::read(&journal).unwrap();
        let (meta, _, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.total_trials, 0);
        assert!(extras.trials.is_empty());
    }

    #[test]
    fn direction_mismatch_is_rejected() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join("run.log");
        let mut cfg = test_cfg(GhSampler::Random);
        cfg.directions.pop();
        assert!(prepare_gh_run(&journal, &problem, &cfg).is_err());
    }
}
