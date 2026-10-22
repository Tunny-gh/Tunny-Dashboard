//! Compute launch and lifetime around the existing Grasshopper optimization loop.
use std::path::Path;

use tdashboard_analysis::surrogate_opt::FitProgress;

use super::{
    build_compute_definition, prepare_gh_run, run_prepared, start_compute_server_tracked,
    ComputeConfig, ComputeDefinition, ComputeEvaluator, ComputeTarget, GhProblem, GhRunConfig,
    GhRunSummary, PreparedGhRun,
};

/// Connection and local-launch settings confirmed by the caller.
pub struct ComputeLaunchConfig {
    pub target: ComputeTarget,
    pub port: u16,
    pub startup_timeout_secs: u64,
    pub api_key: Option<String>,
    pub max_parallel: usize,
}

/// A synchronous preparation: the study already exists before the worker starts.
pub struct PreparedComputeRun {
    run: PreparedGhRun,
    definition: ComputeDefinition,
    definition_pointer: Option<String>,
}

/// Builds the injected definition, persists it beside the journal (best-effort),
/// then creates the study. Validation and error ordering match the GUI launch path.
pub fn prepare_compute_run(
    journal_path: &Path,
    ghx: &str,
    problem: &GhProblem,
    config: &GhRunConfig,
) -> Result<PreparedComputeRun, String> {
    let definition = build_compute_definition(ghx, problem)?;
    let compute_path = journal_path.with_extension("compute.ghx");
    let absolute = std::path::absolute(&compute_path).unwrap_or(compute_path);
    let definition_pointer = std::fs::write(&absolute, &definition.ghx)
        .ok()
        .map(|()| absolute.to_string_lossy().into_owned());
    let run = prepare_gh_run(journal_path, problem, config)?;
    Ok(PreparedComputeRun {
        run,
        definition,
        definition_pointer,
    })
}

impl PreparedComputeRun {
    /// Runs on a worker, retaining a locally launched Compute server until the
    /// optimization ends. Remote servers receive the embedded definition only;
    /// a file pointer is valid only for the local EXE target.
    pub fn run(
        &self,
        problem: &GhProblem,
        config: &GhRunConfig,
        launch: ComputeLaunchConfig,
        progress: &FitProgress,
    ) -> Result<GhRunSummary, String> {
        let _server;
        let mut use_pointer = false;
        let server_url = match launch.target {
            ComputeTarget::Url(url) => url,
            ComputeTarget::Exe(path) => {
                let handle = start_compute_server_tracked(
                    &path,
                    launch.port,
                    launch.startup_timeout_secs,
                    progress,
                )?;
                let url = handle.url().to_string();
                _server = handle;
                use_pointer = true;
                url
            }
        };
        let compute_config = ComputeConfig {
            server_url,
            api_key: launch.api_key,
            max_parallel: launch.max_parallel,
            ..ComputeConfig::default()
        };
        let mut evaluator = ComputeEvaluator::new(&compute_config, &self.definition);
        if use_pointer {
            if let Some(pointer) = &self.definition_pointer {
                evaluator = evaluator.with_definition_pointer(pointer.clone());
            }
        }
        run_prepared(&self.run, problem, &evaluator, config, progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::{extract_problem, fixtures::sample_ghx, GhSampler};
    use tdashboard_io::journal::parser::OptimizationDirection;

    fn config() -> GhRunConfig {
        GhRunConfig {
            directions: vec![OptimizationDirection::Minimize; 2],
            sampler: GhSampler::Random,
            n_trials: 1,
            ..Default::default()
        }
    }

    #[test]
    fn preparation_preserves_error_order_and_best_effort_definition_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");
        let problem = extract_problem(&sample_ghx()).unwrap();
        assert!(prepare_compute_run(&path, "broken", &problem, &config()).is_err());
        assert!(!path.exists());
        let mut invalid = config();
        invalid.directions.clear();
        assert!(prepare_compute_run(&path, &sample_ghx(), &problem, &invalid).is_err());
        assert!(!path.exists());
        let definition_path = path.with_extension("compute.ghx");
        assert!(std::fs::read_to_string(&definition_path)
            .unwrap()
            .contains("RH_IN:span"));
        // A directory at the sidecar destination forces a write failure without
        // relying on platform-specific permissions or elevated-user behavior.
        std::fs::remove_file(&definition_path).unwrap();
        std::fs::create_dir(&definition_path).unwrap();
        let prep = prepare_compute_run(&path, &sample_ghx(), &problem, &config()).unwrap();
        assert!(prep.definition_pointer.is_none());
        let progress = FitProgress::new();
        progress.request_cancel();
        let summary = prep
            .run(
                &problem,
                &config(),
                ComputeLaunchConfig {
                    target: ComputeTarget::Url("http://127.0.0.1:1".into()),
                    port: 8081,
                    startup_timeout_secs: 1,
                    api_key: None,
                    max_parallel: 1,
                },
                &progress,
            )
            .unwrap();
        assert!(summary.cancelled);
        assert_eq!(summary.completed, 0);
    }

    #[test]
    fn local_launch_failure_preserves_prepared_journal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");
        let problem = extract_problem(&sample_ghx()).unwrap();
        let prep = prepare_compute_run(&path, &sample_ghx(), &problem, &config()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let error = prep
            .run(
                &problem,
                &config(),
                ComputeLaunchConfig {
                    target: ComputeTarget::Exe(dir.path().join("missing.exe")),
                    port: 8081,
                    startup_timeout_secs: 1,
                    api_key: None,
                    max_parallel: 1,
                },
                &FitProgress::new(),
            )
            .unwrap_err();
        assert!(error.contains("missing.exe"), "{error}");
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
