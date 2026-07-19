use std::path::PathBuf;

/// Editable search range for one process-optimization parameter (the modal
/// fills these before a run; converted to `runner::VarRange` on start).
#[derive(Debug, Clone)]
pub struct ParamRangeEdit {
    pub name: String,
    pub low: f64,
    pub high: f64,
    pub digits: u32,
    pub is_integer: bool,
}

/// Setup state for a generic process-integration optimization: a loaded
/// `ProcessDefinition` (the external command + how its I/O maps to parameters
/// and objectives) plus the search ranges, objective directions, sampler
/// settings, and journal output the user configures before running. Shown
/// while `AppState::process_opt_dialog` is `Some`.
#[derive(Debug, Clone)]
pub struct ProcessOptDialogState {
    /// The loaded definition (command / input / objectives / constraints).
    pub def: tunny_core::process::ProcessDefinition,
    /// One editable range per parameter (same order as `def.param_names`).
    pub ranges: Vec<ParamRangeEdit>,
    /// Per-objective maximize flag (same order/length as `def.objectives`).
    pub maximize: Vec<bool>,
    /// true = Random sampler, false = NSGA-II (default).
    pub sampler_is_random: bool,
    /// Number of trials for the Random sampler (default 50).
    pub n_trials: usize,
    /// NSGA-II population size (default 16).
    pub population_size: usize,
    /// Number of NSGA-II generations (default 10).
    pub generations: usize,
    /// Random seed (default 42).
    pub seed: u64,
    /// Study name (default: `<definition stem>-<last 6 unix seconds>`).
    pub study_name: String,
    /// Output journal path (default: `<stem>_optuna.log` beside the definition).
    pub journal_path: String,
    /// Error text for a failed Run (invalid ranges / journal open / study create).
    pub error: Option<String>,
}

impl ProcessOptDialogState {
    /// Builds the setup state with defaults right after loading a definition.
    /// Ranges default to `[0, 1]` (2 decimals, continuous) for the user to edit.
    pub fn new(def: tunny_core::process::ProcessDefinition, def_path: &std::path::Path) -> Self {
        let stem = def_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("process_opt")
            .to_string();
        let secs_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 1_000_000)
            .unwrap_or(0);
        let study_name = format!("{stem}-{secs_suffix:06}");
        let journal_path = def_path
            .parent()
            .map(|dir| dir.join(format!("{stem}_optuna.log")))
            .unwrap_or_else(|| PathBuf::from(format!("{stem}_optuna.log")))
            .to_string_lossy()
            .into_owned();
        let ranges = def
            .param_names
            .iter()
            .map(|name| ParamRangeEdit {
                name: name.clone(),
                low: 0.0,
                high: 1.0,
                digits: 2,
                is_integer: false,
            })
            .collect();
        let maximize = vec![false; def.objectives.len()];
        Self {
            def,
            ranges,
            maximize,
            sampler_is_random: false,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            seed: 42,
            study_name,
            journal_path,
            error: None,
        }
    }
}

// ============================================================
// Process-definition builder (GUI editor for a ProcessDefinition)
// ============================================================

/// Which `InputSpec` variant the builder is authoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// Substitute `{param}` into a template file written to a path.
    Template,
    /// Pass each parameter as an environment variable.
    Env,
    /// Pass a `{param: value}` JSON object on stdin.
    JsonStdin,
    /// Expand `arg_template` per parameter into argv entries.
    Args,
}

/// Which `OutputSource` an objective/constraint reads from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// The command's standard output.
    Stdout,
    /// A file written by the command.
    File,
}

/// Which `Extractor` pulls a number out of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractorKind {
    /// First capture group of a regex, parsed as f64.
    Regex,
    /// Dotted JSON path (e.g. `results.weight`).
    JsonPath,
    /// A cell of a CSV table.
    Csv,
}

/// Editable form of `CommandSpec`. Kept as flat fields (rather than the core enum
/// directly) so toggling controls never drops already-typed text; converted to a
/// `CommandSpec` on save.
#[derive(Debug, Clone, Default)]
pub struct CommandEdit {
    pub program: String,
    /// Fixed args passed verbatim (blank rows are dropped on build).
    pub args: Vec<String>,
    /// Working directory; blank means the current directory (`None`).
    pub working_dir: String,
    /// Timeout in seconds; `0` means no timeout.
    pub timeout_secs: u64,
    /// Extra attempts after a failure.
    pub retries: usize,
}

impl CommandEdit {
    fn from_spec(spec: &tunny_core::process::CommandSpec) -> Self {
        Self {
            program: spec.program.clone(),
            args: spec.args.clone(),
            working_dir: spec.working_dir.clone().unwrap_or_default(),
            timeout_secs: spec.timeout_secs,
            retries: spec.retries,
        }
    }

    fn to_spec(&self) -> tunny_core::process::CommandSpec {
        let working_dir = {
            let t = self.working_dir.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        };
        tunny_core::process::CommandSpec {
            program: self.program.trim().to_string(),
            args: self
                .args
                .iter()
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty())
                .collect(),
            working_dir,
            timeout_secs: self.timeout_secs,
            retries: self.retries,
        }
    }
}

/// Editable form of `Extractor` (all variants' fields held at once so switching
/// the kind combo keeps previously typed values).
#[derive(Debug, Clone)]
pub struct ExtractorEdit {
    pub kind: ExtractorKind,
    pub regex_pattern: String,
    pub json_path: String,
    /// true = last data row, false = a specific index.
    pub csv_row_last: bool,
    pub csv_row_index: usize,
    /// true = column by header name, false = by index.
    pub csv_col_by_header: bool,
    pub csv_col_index: usize,
    pub csv_col_header: String,
    pub csv_has_header: bool,
}

impl Default for ExtractorEdit {
    fn default() -> Self {
        Self {
            kind: ExtractorKind::Regex,
            regex_pattern: String::new(),
            json_path: String::new(),
            csv_row_last: true,
            csv_row_index: 0,
            csv_col_by_header: false,
            csv_col_index: 0,
            csv_col_header: String::new(),
            csv_has_header: false,
        }
    }
}

impl ExtractorEdit {
    fn from_extractor(e: &tunny_core::process::Extractor) -> Self {
        use tunny_core::process::{CsvColumn, CsvRow, Extractor};
        let mut out = Self::default();
        match e {
            Extractor::Regex { pattern } => {
                out.kind = ExtractorKind::Regex;
                out.regex_pattern = pattern.clone();
            }
            Extractor::JsonPath { path } => {
                out.kind = ExtractorKind::JsonPath;
                out.json_path = path.clone();
            }
            Extractor::Csv {
                row,
                column,
                has_header,
            } => {
                out.kind = ExtractorKind::Csv;
                match row {
                    CsvRow::Index { index } => {
                        out.csv_row_last = false;
                        out.csv_row_index = *index;
                    }
                    CsvRow::Last => out.csv_row_last = true,
                }
                match column {
                    CsvColumn::Index { index } => {
                        out.csv_col_by_header = false;
                        out.csv_col_index = *index;
                    }
                    CsvColumn::Header { name } => {
                        out.csv_col_by_header = true;
                        out.csv_col_header = name.clone();
                    }
                }
                out.csv_has_header = *has_header;
            }
        }
        out
    }

    fn to_extractor(&self) -> tunny_core::process::Extractor {
        use tunny_core::process::{CsvColumn, CsvRow, Extractor};
        match self.kind {
            ExtractorKind::Regex => Extractor::Regex {
                pattern: self.regex_pattern.clone(),
            },
            ExtractorKind::JsonPath => Extractor::JsonPath {
                path: self.json_path.trim().to_string(),
            },
            ExtractorKind::Csv => {
                let row = if self.csv_row_last {
                    CsvRow::Last
                } else {
                    CsvRow::Index {
                        index: self.csv_row_index,
                    }
                };
                let column = if self.csv_col_by_header {
                    CsvColumn::Header {
                        name: self.csv_col_header.trim().to_string(),
                    }
                } else {
                    CsvColumn::Index {
                        index: self.csv_col_index,
                    }
                };
                // A header-named column requires a header row; keep them consistent.
                let has_header = self.csv_has_header || self.csv_col_by_header;
                Extractor::Csv {
                    row,
                    column,
                    has_header,
                }
            }
        }
    }
}

/// Editable form of an `OutputSpec` (one objective or constraint row).
#[derive(Debug, Clone)]
pub struct OutputSpecEdit {
    pub name: String,
    pub source_kind: SourceKind,
    /// File path when `source_kind == File`.
    pub source_path: String,
    pub extractor: ExtractorEdit,
}

impl Default for OutputSpecEdit {
    fn default() -> Self {
        Self {
            name: String::new(),
            source_kind: SourceKind::Stdout,
            source_path: String::new(),
            extractor: ExtractorEdit::default(),
        }
    }
}

impl OutputSpecEdit {
    fn from_spec(spec: &tunny_core::process::OutputSpec) -> Self {
        use tunny_core::process::OutputSource;
        let (source_kind, source_path) = match &spec.source {
            OutputSource::Stdout => (SourceKind::Stdout, String::new()),
            OutputSource::File { path } => (SourceKind::File, path.clone()),
        };
        Self {
            name: spec.name.clone(),
            source_kind,
            source_path,
            extractor: ExtractorEdit::from_extractor(&spec.extractor),
        }
    }

    fn to_spec(&self) -> tunny_core::process::OutputSpec {
        use tunny_core::process::OutputSource;
        let source = match self.source_kind {
            SourceKind::Stdout => OutputSource::Stdout,
            SourceKind::File => OutputSource::File {
                path: self.source_path.trim().to_string(),
            },
        };
        tunny_core::process::OutputSpec {
            name: self.name.trim().to_string(),
            source,
            extractor: self.extractor.to_extractor(),
        }
    }
}

/// Editable model for authoring a `ProcessDefinition` in the GUI builder. Shown
/// while `AppState::process_def_builder` is `Some`. The whole definition maps to
/// flat editable fields here; `to_definition` rebuilds the core type on save, and
/// `from_definition` populates the form when an existing JSON is loaded.
#[derive(Debug, Clone)]
pub struct ProcessDefBuilderState {
    /// Ordered parameter names (referenced as `{name}` in templates/args).
    pub param_names: Vec<String>,
    pub input_kind: InputKind,
    pub input_template: String,
    pub input_path: String,
    pub input_arg_template: String,
    pub command: CommandEdit,
    pub objectives: Vec<OutputSpecEdit>,
    pub constraints: Vec<OutputSpecEdit>,
    pub pre_command_enabled: bool,
    pub pre_command: CommandEdit,
    pub post_command_enabled: bool,
    pub post_command: CommandEdit,
    /// Where the definition was loaded from / last saved. Seeds the save dialog
    /// and the run modal's study-name / journal defaults.
    pub source_path: Option<PathBuf>,
    /// Error text for a failed validate / save (shown in red).
    pub error: Option<String>,
    /// Non-error status (e.g. "Saved to …"); shown in a muted color.
    pub status: Option<String>,
}

impl Default for ProcessDefBuilderState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessDefBuilderState {
    /// A minimal starter definition: one parameter, args-based input, and one
    /// stdout regex objective. The command program is left blank for the user to
    /// fill (so it fails `validate` until authored), matching the run flow.
    pub fn new() -> Self {
        Self {
            param_names: vec!["x".to_string()],
            input_kind: InputKind::Args,
            input_template: String::new(),
            input_path: String::new(),
            input_arg_template: "--{name}={value}".to_string(),
            command: CommandEdit::default(),
            objectives: vec![OutputSpecEdit {
                name: "f".to_string(),
                source_kind: SourceKind::Stdout,
                source_path: String::new(),
                extractor: ExtractorEdit {
                    kind: ExtractorKind::Regex,
                    regex_pattern: r"([-0-9.eE+]+)".to_string(),
                    ..ExtractorEdit::default()
                },
            }],
            constraints: Vec::new(),
            pre_command_enabled: false,
            pre_command: CommandEdit::default(),
            post_command_enabled: false,
            post_command: CommandEdit::default(),
            source_path: None,
            error: None,
            status: None,
        }
    }

    /// Populates the form from an existing definition (used by "Load…").
    pub fn from_definition(
        def: &tunny_core::process::ProcessDefinition,
        source_path: Option<PathBuf>,
    ) -> Self {
        use tunny_core::process::InputSpec;
        let mut out = Self {
            param_names: def.param_names.clone(),
            input_kind: InputKind::Env,
            input_template: String::new(),
            input_path: String::new(),
            input_arg_template: String::new(),
            command: CommandEdit::from_spec(&def.command),
            objectives: def
                .objectives
                .iter()
                .map(OutputSpecEdit::from_spec)
                .collect(),
            constraints: def
                .constraints
                .iter()
                .map(OutputSpecEdit::from_spec)
                .collect(),
            pre_command_enabled: def.pre_command.is_some(),
            pre_command: def
                .pre_command
                .as_ref()
                .map(CommandEdit::from_spec)
                .unwrap_or_default(),
            post_command_enabled: def.post_command.is_some(),
            post_command: def
                .post_command
                .as_ref()
                .map(CommandEdit::from_spec)
                .unwrap_or_default(),
            source_path,
            error: None,
            status: None,
        };
        match &def.input {
            InputSpec::Template { template, path } => {
                out.input_kind = InputKind::Template;
                out.input_template = template.clone();
                out.input_path = path.clone();
            }
            InputSpec::Env => out.input_kind = InputKind::Env,
            InputSpec::JsonStdin => out.input_kind = InputKind::JsonStdin,
            InputSpec::Args { arg_template } => {
                out.input_kind = InputKind::Args;
                out.input_arg_template = arg_template.clone();
            }
        }
        out
    }

    /// Rebuilds a `ProcessDefinition` from the form. Blank parameter/objective/
    /// constraint names are dropped so a just-added empty row doesn't leak in;
    /// call `def.validate()` afterward for the remaining structural checks.
    pub fn to_definition(&self) -> tunny_core::process::ProcessDefinition {
        use tunny_core::process::InputSpec;
        let input = match self.input_kind {
            InputKind::Template => InputSpec::Template {
                template: self.input_template.clone(),
                path: self.input_path.trim().to_string(),
            },
            InputKind::Env => InputSpec::Env,
            InputKind::JsonStdin => InputSpec::JsonStdin,
            InputKind::Args => InputSpec::Args {
                arg_template: self.input_arg_template.clone(),
            },
        };
        tunny_core::process::ProcessDefinition {
            param_names: self
                .param_names
                .iter()
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect(),
            input,
            command: self.command.to_spec(),
            objectives: self
                .objectives
                .iter()
                .filter(|o| !o.name.trim().is_empty())
                .map(OutputSpecEdit::to_spec)
                .collect(),
            constraints: self
                .constraints
                .iter()
                .filter(|c| !c.name.trim().is_empty())
                .map(OutputSpecEdit::to_spec)
                .collect(),
            pre_command: self.pre_command_enabled.then(|| self.pre_command.to_spec()),
            post_command: self
                .post_command_enabled
                .then(|| self.post_command.to_spec()),
        }
    }
}
