//! Serializable definition of a generic process-integration objective
//! (ROADMAP Phase 2B/2C, items 13 & 14).
//!
//! A [`ProcessDefinition`] describes how to evaluate one trial by running an
//! external command: how the trial's parameter values reach the command
//! (input file template / CLI args / environment variables / JSON stdin), the
//! command itself (with timeout and retries), how objective and constraint
//! values are extracted from the command's output (regex / JSON path / CSV),
//! and optional pre/post commands. The whole struct is `serde`-serializable so
//! an integration can be saved to and loaded from JSON (or TOML) and edited in
//! the GUI.

use serde::{Deserialize, Serialize};

/// A complete process-integration objective definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessDefinition {
    /// Parameter names, in the order the runner supplies values. Placeholders
    /// (`{name}`) in templates and args refer to these.
    pub param_names: Vec<String>,
    /// How parameter values are delivered to the command.
    pub input: InputSpec,
    /// The command that evaluates one trial.
    pub command: CommandSpec,
    /// One extraction spec per objective (same order as the study's objectives).
    pub objectives: Vec<OutputSpec>,
    /// One extraction spec per constraint (feasible when the value is `<= 0`;
    /// empty means unconstrained).
    #[serde(default)]
    pub constraints: Vec<OutputSpec>,
    /// Optional command run once before the evaluation command (e.g. staging
    /// input files). Runs after input substitution.
    #[serde(default)]
    pub pre_command: Option<CommandSpec>,
    /// Optional command run once after the evaluation command succeeds (e.g.
    /// post-processing raw solver output into the file the extractor reads).
    #[serde(default)]
    pub post_command: Option<CommandSpec>,
}

/// How a trial's parameter values reach the command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputSpec {
    /// Substitute `{param}` placeholders in `template` and write the result to
    /// `path` (relative to the working directory) before running the command.
    Template { template: String, path: String },
    /// Pass each parameter as an environment variable named after it.
    Env,
    /// Pass a JSON object `{param: value}` on the command's stdin.
    JsonStdin,
    /// Append CLI arguments: `arg_template` is expanded per parameter with
    /// `{name}` and `{value}` (e.g. `"--{name}={value}"`). The expansions are
    /// split on whitespace into separate argv entries.
    Args { arg_template: String },
}

/// A command invocation with resource limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandSpec {
    /// The program to run (looked up on `PATH` or an absolute path).
    pub program: String,
    /// Fixed arguments, passed verbatim (parameter args from `InputSpec::Args`
    /// are appended after these). Not templated — a script argument full of
    /// braces is safe; parameters reach the command only through the
    /// definition's `InputSpec`.
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory for the command (default: the current directory).
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Per-invocation timeout in seconds. `0` means no timeout.
    #[serde(default)]
    pub timeout_secs: u64,
    /// Number of extra attempts after a failed invocation (default `0`).
    #[serde(default)]
    pub retries: usize,
}

impl CommandSpec {
    /// A command with no fixed args, no working dir, no timeout, no retries.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            timeout_secs: 0,
            retries: 0,
        }
    }
}

/// Extraction spec for one output value (objective or constraint).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputSpec {
    /// Display name (objective / constraint name).
    pub name: String,
    /// Where the value is read from.
    pub source: OutputSource,
    /// How the value is parsed out of the source text.
    pub extractor: Extractor,
}

/// Where an output value is read from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutputSource {
    /// The command's captured standard output.
    Stdout,
    /// A file written by the command (relative to the working directory).
    File { path: String },
}

/// How a numeric value is extracted from source text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Extractor {
    /// The first capture group of `pattern`, parsed as `f64`. With no capture
    /// group, the whole match is used.
    Regex { pattern: String },
    /// A dotted path into a JSON document, e.g. `results.weight` or
    /// `values.0` (array index). The addressed value must be a number (or a
    /// numeric string).
    JsonPath { path: String },
    /// A cell of a CSV document. `has_header` makes row indexing unambiguous:
    /// when true, line 0 is the header (used for [`CsvColumn::Header`] lookup)
    /// and data rows start at line 1; when false there is no header and every
    /// line is a data row. [`CsvColumn::Header`] requires `has_header = true`.
    Csv {
        row: CsvRow,
        column: CsvColumn,
        #[serde(default)]
        has_header: bool,
    },
}

/// CSV row selector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CsvRow {
    /// 0-based data row index (the header row, if any, is not counted).
    Index { index: usize },
    /// The last data row (useful for a solver that appends per-step rows).
    Last,
}

/// CSV column selector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CsvColumn {
    /// 0-based column index.
    Index { index: usize },
    /// Column identified by its header name (requires a header row).
    Header { name: String },
}

impl ProcessDefinition {
    /// Serializes the definition to pretty JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("failed to serialize: {e}"))
    }

    /// Parses a definition from JSON.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("failed to parse definition: {e}"))
    }

    /// Validates internal consistency (names non-empty, unique parameter names,
    /// at least one objective). Returns a human-readable error on the first
    /// problem found.
    pub fn validate(&self) -> Result<(), String> {
        if self.objectives.is_empty() {
            return Err("at least one objective is required".to_string());
        }
        if self.command.program.trim().is_empty() {
            return Err("command program must not be empty".to_string());
        }
        let mut seen = std::collections::HashSet::new();
        for name in &self.param_names {
            if name.trim().is_empty() {
                return Err("parameter names must not be empty".to_string());
            }
            // Whitespace in a name breaks CLI-arg building (the expansion is
            // split on whitespace into separate argv tokens) and makes a poor
            // env-var / journal column name.
            if name.split_whitespace().count() != 1 {
                return Err(format!(
                    "parameter name \"{name}\" must not contain whitespace"
                ));
            }
            if !seen.insert(name.clone()) {
                return Err(format!("duplicate parameter name \"{name}\""));
            }
        }
        // Objective names become journal columns, so they must be present and
        // unique. (Constraint values are recorded positionally as c1..cN, so
        // their names are display-only and not checked for uniqueness here.)
        let mut obj_seen = std::collections::HashSet::new();
        for obj in &self.objectives {
            if obj.name.trim().is_empty() {
                return Err("objective names must not be empty".to_string());
            }
            if !obj_seen.insert(&obj.name) {
                return Err(format!("duplicate objective name \"{}\"", obj.name));
            }
        }
        if let InputSpec::Template { path, .. } = &self.input {
            if path.trim().is_empty() {
                return Err("input template path must not be empty".to_string());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProcessDefinition {
        ProcessDefinition {
            param_names: vec!["x".to_string(), "y".to_string()],
            input: InputSpec::Args {
                arg_template: "--{name}={value}".to_string(),
            },
            command: CommandSpec {
                program: "solver".to_string(),
                args: vec!["--quiet".to_string()],
                working_dir: Some("/tmp/run".to_string()),
                timeout_secs: 30,
                retries: 2,
            },
            objectives: vec![OutputSpec {
                name: "f".to_string(),
                source: OutputSource::Stdout,
                extractor: Extractor::Regex {
                    pattern: r"f=([-0-9.]+)".to_string(),
                },
            }],
            constraints: vec![OutputSpec {
                name: "g".to_string(),
                source: OutputSource::File {
                    path: "out.json".to_string(),
                },
                extractor: Extractor::JsonPath {
                    path: "constraint".to_string(),
                },
            }],
            pre_command: None,
            post_command: Some(CommandSpec::new("cleanup")),
        }
    }

    #[test]
    fn json_round_trip_preserves_definition() {
        let def = sample();
        let json = def.to_json().unwrap();
        let back = ProcessDefinition::from_json(&json).unwrap();
        assert_eq!(def, back);
    }

    #[test]
    fn validate_accepts_sample_and_rejects_problems() {
        assert!(sample().validate().is_ok());

        let mut no_obj = sample();
        no_obj.objectives.clear();
        assert!(no_obj.validate().unwrap_err().contains("objective"));

        let mut dup = sample();
        dup.param_names = vec!["x".to_string(), "x".to_string()];
        assert!(dup.validate().unwrap_err().contains("duplicate"));

        let mut empty_prog = sample();
        empty_prog.command.program = "  ".to_string();
        assert!(empty_prog.validate().unwrap_err().contains("program"));

        let mut spaced = sample();
        spaced.param_names = vec!["max iter".to_string()];
        assert!(spaced.validate().unwrap_err().contains("whitespace"));

        let mut dup_obj = sample();
        dup_obj.objectives = vec![
            OutputSpec {
                name: "f".to_string(),
                source: OutputSource::Stdout,
                extractor: Extractor::Regex {
                    pattern: "(.+)".to_string(),
                },
            },
            OutputSpec {
                name: "f".to_string(),
                source: OutputSource::Stdout,
                extractor: Extractor::Regex {
                    pattern: "(.+)".to_string(),
                },
            },
        ];
        assert!(dup_obj
            .validate()
            .unwrap_err()
            .contains("duplicate objective"));

        let mut empty_path = sample();
        empty_path.input = InputSpec::Template {
            template: "x={x}".to_string(),
            path: "".to_string(),
        };
        assert!(empty_path.validate().unwrap_err().contains("path"));
    }

    #[test]
    fn optional_fields_default_when_absent() {
        // A minimal definition without constraints/pre/post/args deserializes.
        let json = r#"{
            "param_names": ["x"],
            "input": {"kind": "env"},
            "command": {"program": "run"},
            "objectives": [
                {"name": "f", "source": {"kind": "stdout"},
                 "extractor": {"kind": "regex", "pattern": "([0-9.]+)"}}
            ]
        }"#;
        let def = ProcessDefinition::from_json(json).unwrap();
        assert!(def.constraints.is_empty());
        assert!(def.pre_command.is_none());
        assert!(def.post_command.is_none());
        assert_eq!(def.command.timeout_secs, 0);
        assert_eq!(def.command.retries, 0);
        assert!(def.command.args.is_empty());
        def.validate().unwrap();
    }
}
