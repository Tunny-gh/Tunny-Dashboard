//! Generic process-integration objective (ROADMAP Phase 2B/2C, items 13 & 14).
//!
//! Evaluates a trial by running an external command — the "Optuna is the
//! workflow engine; Dashboard is the cockpit" runner for solvers that are not
//! Grasshopper. A [`ProcessDefinition`] (serde-serializable, so it can be saved
//! and edited in the GUI) describes:
//!
//! 1. How the trial's parameter values reach the command
//!    ([`InputSpec`]: input-file template substitution, CLI args, environment
//!    variables, or a JSON stdin payload).
//! 2. The command to run ([`CommandSpec`], with a per-invocation timeout and
//!    retries), plus optional pre/post commands.
//! 3. How objective and constraint values are extracted from the command's
//!    output ([`OutputSpec`] via [`Extractor`]: regex, JSON path, or CSV cell).
//!
//! [`ProcessEvaluator`] runs one trial through this pipeline and returns the
//! objective/constraint values. Wiring the evaluator into the sampler loop and
//! journal recording (so process runs stream into the live view like the
//! Grasshopper runner) and the GUI for building definitions are separate
//! follow-ups.

mod definition;
mod evaluator;
mod extract;
mod substitute;

pub use definition::{
    CommandSpec, CsvColumn, CsvRow, Extractor, InputSpec, OutputSource, OutputSpec,
    ProcessDefinition,
};
pub use evaluator::{ProcessEvaluation, ProcessEvaluator, VarRange};
