//! Evaluates one trial by running the external command described by a
//! [`ProcessDefinition`]: substitute parameters → optional pre-command → run →
//! optional post-command → extract objective and constraint values. Handles
//! per-command timeouts and retries.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use super::definition::{CommandSpec, InputSpec, OutputSource, OutputSpec, ProcessDefinition};
use super::extract::extract_value;
use super::substitute::{build_args, build_env, build_json_stdin, render_template};

/// Result of evaluating one trial.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessEvaluation {
    /// Objective values, in the definition's objective order.
    pub objectives: Vec<f64>,
    /// Constraint values, in the definition's constraint order (feasible when
    /// every value is `<= 0`; empty when unconstrained).
    pub constraints: Vec<f64>,
}

/// Evaluates trials by running the command from a [`ProcessDefinition`].
pub struct ProcessEvaluator {
    def: ProcessDefinition,
}

impl ProcessEvaluator {
    /// Builds an evaluator, validating the definition up front.
    pub fn new(def: ProcessDefinition) -> Result<Self, String> {
        def.validate()?;
        Ok(Self { def })
    }

    /// Evaluates one trial. `values` are the parameter values in
    /// `def.param_names` order. Retries the whole substitute→run→extract cycle
    /// up to `command.retries` extra times; the last error is returned.
    pub fn evaluate(&self, values: &[f64]) -> Result<ProcessEvaluation, String> {
        if values.len() != self.def.param_names.len() {
            return Err(format!(
                "expected {} parameter values, got {}",
                self.def.param_names.len(),
                values.len()
            ));
        }
        let attempts = self.def.command.retries + 1;
        let mut last_err = String::new();
        for attempt in 0..attempts {
            match self.evaluate_once(values) {
                Ok(eval) => return Ok(eval),
                Err(e) => {
                    last_err = if attempt + 1 < attempts {
                        format!("attempt {}/{attempts} failed: {e}", attempt + 1)
                    } else {
                        e
                    };
                }
            }
        }
        Err(last_err)
    }

    fn evaluate_once(&self, values: &[f64]) -> Result<ProcessEvaluation, String> {
        let work_dir = self
            .def
            .command
            .working_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        // ── Input substitution ──────────────────────────────────────────────
        let named: HashMap<&str, f64> = self
            .def
            .param_names
            .iter()
            .map(String::as_str)
            .zip(values.iter().copied())
            .collect();

        let mut extra_args: Vec<String> = Vec::new();
        let mut env: Vec<(String, String)> = Vec::new();
        let mut stdin_data: Option<String> = None;
        match &self.def.input {
            InputSpec::Template { template, path } => {
                let rendered = render_template(template, &named)?;
                let target = work_dir.join(path);
                std::fs::write(&target, rendered)
                    .map_err(|e| format!("failed to write input file {}: {e}", target.display()))?;
            }
            InputSpec::Env => {
                env = build_env(&self.def.param_names, values)?;
            }
            InputSpec::JsonStdin => {
                stdin_data = Some(build_json_stdin(&self.def.param_names, values)?);
            }
            InputSpec::Args { arg_template } => {
                extra_args = build_args(arg_template, &self.def.param_names, values)?;
            }
        }

        // ── Pre-command ─────────────────────────────────────────────────────
        if let Some(pre) = &self.def.pre_command {
            let out = run_command(pre, &work_dir, &[], &[], None)?;
            fail_on_nonzero(&pre.program, &out)?;
        }

        // ── Evaluation command ──────────────────────────────────────────────
        let out = run_command(
            &self.def.command,
            &work_dir,
            &extra_args,
            &env,
            stdin_data.as_deref(),
        )?;
        fail_on_nonzero(&self.def.command.program, &out)?;
        let stdout = out.stdout;

        // ── Post-command ────────────────────────────────────────────────────
        if let Some(post) = &self.def.post_command {
            let out = run_command(post, &work_dir, &[], &[], None)?;
            fail_on_nonzero(&post.program, &out)?;
        }

        // ── Extraction ──────────────────────────────────────────────────────
        let objectives = self.extract_all(&self.def.objectives, &stdout, &work_dir)?;
        let constraints = self.extract_all(&self.def.constraints, &stdout, &work_dir)?;
        Ok(ProcessEvaluation {
            objectives,
            constraints,
        })
    }

    /// Reads and extracts every spec in `specs`, caching file contents so a file
    /// with several extractors is read once.
    fn extract_all(
        &self,
        specs: &[OutputSpec],
        stdout: &str,
        work_dir: &Path,
    ) -> Result<Vec<f64>, String> {
        let mut file_cache: HashMap<String, String> = HashMap::new();
        let mut out = Vec::with_capacity(specs.len());
        for spec in specs {
            let text = match &spec.source {
                OutputSource::Stdout => stdout,
                OutputSource::File { path } => {
                    if !file_cache.contains_key(path) {
                        let full = work_dir.join(path);
                        let content = std::fs::read_to_string(&full).map_err(|e| {
                            format!("failed to read output file {}: {e}", full.display())
                        })?;
                        file_cache.insert(path.clone(), content);
                    }
                    file_cache.get(path).expect("just inserted")
                }
            };
            let value = extract_value(&spec.extractor, text)
                .map_err(|e| format!("extracting \"{}\": {e}", spec.name))?;
            out.push(value);
        }
        Ok(out)
    }
}

/// Captured output of a command invocation.
struct CommandOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// Returns an error when the command exited non-zero, including a stderr tail.
fn fail_on_nonzero(program: &str, out: &CommandOutput) -> Result<(), String> {
    if out.status.success() {
        return Ok(());
    }
    let tail: String = out.stderr.chars().rev().take(400).collect::<String>();
    let tail: String = tail.chars().rev().collect();
    Err(format!(
        "command \"{program}\" exited with {} (stderr: {})",
        out.status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string()),
        tail.trim()
    ))
}

/// Runs a command with its literal fixed args plus `extra_args`/`env`/
/// `stdin_data`, enforcing `spec.timeout_secs`. Fixed args are passed verbatim
/// (parameters reach the command only through the chosen `InputSpec`), so a
/// script argument full of braces — awk, shell — is never mistaken for a
/// parameter template.
fn run_command(
    spec: &CommandSpec,
    work_dir: &Path,
    extra_args: &[String],
    env: &[(String, String)],
    stdin_data: Option<&str>,
) -> Result<CommandOutput, String> {
    let mut cmd = Command::new(&spec.program);
    cmd.current_dir(work_dir);
    for arg in &spec.args {
        cmd.arg(arg);
    }
    for arg in extra_args {
        cmd.arg(arg);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    run_with_timeout(cmd, stdin_data, spec.timeout_secs)
}

/// Spawns `cmd`, feeds `stdin_data`, drains stdout/stderr on separate threads
/// (so a full pipe buffer can't deadlock the wait), and enforces `timeout_secs`
/// (0 = unlimited) by polling, killing the child on timeout.
fn run_with_timeout(
    mut cmd: Command,
    stdin_data: Option<&str>,
    timeout_secs: u64,
) -> Result<CommandOutput, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if stdin_data.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });

    let program = format!("{:?}", cmd.get_program());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start {program}: {e}"))?;

    // Write stdin from a thread so a large payload can't deadlock against the
    // child's stdout.
    if let (Some(data), Some(mut sin)) = (stdin_data.map(str::to_owned), child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = sin.write_all(data.as_bytes());
            // `sin` drops here, closing the child's stdin.
        });
    }

    let mut stdout_pipe = child.stdout.take().expect("stdout piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr piped");
    let out_handle = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout_pipe.read_to_string(&mut s);
        s
    });
    let err_handle = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr_pipe.read_to_string(&mut s);
        s
    });

    let status = if timeout_secs == 0 {
        child
            .wait()
            .map_err(|e| format!("failed to wait for {program}: {e}"))?
    } else {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!("command {program} timed out after {timeout_secs}s"));
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(format!("failed to wait for {program}: {e}")),
            }
        }
    };

    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    Ok(CommandOutput {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::definition::{Extractor, OutputSource, OutputSpec};

    /// A `CommandSpec` running a shell snippet. The subprocess end-to-end tests
    /// are unix-gated (portable `cmd.exe` scripts are error-prone); the pure
    /// substitution / extraction / definition logic is tested cross-platform.
    #[cfg(unix)]
    fn shell(script: &str) -> CommandSpec {
        CommandSpec {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), script.to_string()],
            working_dir: None,
            timeout_secs: 0,
            retries: 0,
        }
    }

    fn stdout_regex(name: &str, pattern: &str) -> OutputSpec {
        OutputSpec {
            name: name.to_string(),
            source: OutputSource::Stdout,
            extractor: Extractor::Regex {
                pattern: pattern.to_string(),
            },
        }
    }

    #[cfg(unix)]
    #[test]
    fn evaluates_via_env_and_stdout_regex() {
        // sh reads params from the environment; awk sums them for the objective.
        let command = shell("awk \"BEGIN{print \\\"f=\\\" ($x + $y)}\"");
        let def = ProcessDefinition {
            param_names: vec!["x".to_string(), "y".to_string()],
            input: InputSpec::Env,
            command,
            objectives: vec![stdout_regex("f", r"f=([-0-9.]+)")],
            constraints: vec![],
            pre_command: None,
            post_command: None,
        };
        let eval = ProcessEvaluator::new(def).unwrap();
        let out = eval.evaluate(&[2.0, 3.5]).unwrap();
        assert_eq!(out.objectives, vec![5.5]);
    }

    #[cfg(unix)]
    #[test]
    fn evaluates_via_args_stdout_and_file_constraint() {
        let dir = tempfile::tempdir().unwrap();
        // Command echoes an objective to stdout and writes a constraint file.
        let script = "echo obj=$1 $2; echo '{\"g\": -0.5}' > out.json";
        let command = CommandSpec {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), script.to_string(), "sh".to_string()],
            working_dir: Some(dir.path().to_string_lossy().into_owned()),
            timeout_secs: 10,
            retries: 0,
        };
        let def = ProcessDefinition {
            param_names: vec!["a".to_string(), "b".to_string()],
            input: InputSpec::Args {
                arg_template: "{value}".to_string(),
            },
            command,
            objectives: vec![stdout_regex("obj", r"obj=([-0-9.]+)")],
            constraints: vec![OutputSpec {
                name: "g".to_string(),
                source: OutputSource::File {
                    path: "out.json".to_string(),
                },
                extractor: Extractor::JsonPath {
                    path: "g".to_string(),
                },
            }],
            pre_command: None,
            post_command: None,
        };
        let eval = ProcessEvaluator::new(def).unwrap();
        let out = eval.evaluate(&[7.0, 9.0]).unwrap();
        assert_eq!(out.objectives, vec![7.0]);
        assert_eq!(out.constraints, vec![-0.5]);
    }

    #[cfg(unix)]
    #[test]
    fn template_input_is_written_and_readable_by_command() {
        let dir = tempfile::tempdir().unwrap();
        let command = CommandSpec {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "cat input.txt".to_string()],
            working_dir: Some(dir.path().to_string_lossy().into_owned()),
            timeout_secs: 10,
            retries: 0,
        };
        let def = ProcessDefinition {
            param_names: vec!["span".to_string()],
            input: InputSpec::Template {
                template: "value={span}".to_string(),
                path: "input.txt".to_string(),
            },
            command,
            objectives: vec![stdout_regex("v", r"value=([-0-9.]+)")],
            constraints: vec![],
            pre_command: None,
            post_command: None,
        };
        let eval = ProcessEvaluator::new(def).unwrap();
        assert_eq!(eval.evaluate(&[4.25]).unwrap().objectives, vec![4.25]);
    }

    #[cfg(unix)]
    #[test]
    fn nonzero_exit_retries_then_fails() {
        // Always exits 1; with retries=2 that's 3 attempts, all failing.
        let command = CommandSpec {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "echo boom >&2; exit 1".to_string()],
            working_dir: None,
            timeout_secs: 10,
            retries: 2,
        };
        let def = ProcessDefinition {
            param_names: vec![],
            input: InputSpec::Env,
            command,
            objectives: vec![stdout_regex("f", r"f=([0-9.]+)")],
            constraints: vec![],
            pre_command: None,
            post_command: None,
        };
        let eval = ProcessEvaluator::new(def).unwrap();
        let err = eval.evaluate(&[]).unwrap_err();
        assert!(err.contains("exited with 1"), "unexpected error: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_a_hanging_command() {
        let command = CommandSpec {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            working_dir: None,
            timeout_secs: 1,
            retries: 0,
        };
        let def = ProcessDefinition {
            param_names: vec![],
            input: InputSpec::Env,
            command,
            objectives: vec![stdout_regex("f", r"f=([0-9.]+)")],
            constraints: vec![],
            pre_command: None,
            post_command: None,
        };
        let eval = ProcessEvaluator::new(def).unwrap();
        let start = Instant::now();
        let err = eval.evaluate(&[]).unwrap_err();
        assert!(err.contains("timed out"), "unexpected error: {err}");
        assert!(
            start.elapsed() < Duration::from_secs(4),
            "kill was too slow"
        );
    }

    #[cfg(unix)]
    #[test]
    fn post_command_transforms_output_before_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let wd = dir.path().to_string_lossy().into_owned();
        // Main writes a raw value; post-command converts it into the CSV the
        // extractor reads.
        let def = ProcessDefinition {
            param_names: vec![],
            input: InputSpec::Env,
            command: CommandSpec {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "echo 3.5 > raw.txt".to_string()],
                working_dir: Some(wd.clone()),
                timeout_secs: 10,
                retries: 0,
            },
            objectives: vec![OutputSpec {
                name: "f".to_string(),
                source: OutputSource::File {
                    path: "final.csv".to_string(),
                },
                extractor: Extractor::Csv {
                    row: crate::process::definition::CsvRow::Last,
                    column: crate::process::definition::CsvColumn::Index { index: 0 },
                },
            }],
            constraints: vec![],
            pre_command: None,
            post_command: Some(CommandSpec {
                program: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    "echo v > final.csv; cat raw.txt >> final.csv".to_string(),
                ],
                working_dir: Some(wd),
                timeout_secs: 10,
                retries: 0,
            }),
        };
        let eval = ProcessEvaluator::new(def).unwrap();
        assert_eq!(eval.evaluate(&[]).unwrap().objectives, vec![3.5]);
    }
}
