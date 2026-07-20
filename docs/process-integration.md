# Process Integration — Optimizing Any External Tool

Run an optimization where **the Dashboard drives the sampling** (in Rust) and
**your own command-line tool evaluates the objective**. The whole loop needs
only the Dashboard and your tool — **no Python and no Optuna at runtime**. Each
trial is written to a normal Optuna-compatible journal, so every analysis
feature (live update, all widgets, reports, MCP) works on the results exactly as
it does for any other study.

This is the "Optuna is the workflow engine; Dashboard is the cockpit" runner for
solvers that are not Grasshopper. (For Grasshopper, see the drag-and-drop `.ghx`
flow instead.)

> Status: fully available from the Dashboard. **Optimize ▸ New Tool…** on the
> toolbar opens a GUI **builder** for authoring (or importing and editing) a
> process **definition** (below) and saving it back to JSON.
> **Optimize ▸ Optimize Tool…** loads a saved definition, lets you set each
> parameter's search range, the objective directions, and the sampler, and runs
> it with the live monitoring you already know. The same definition and run can also be driven through the `tunny-core`
> library.

## How it works

For each trial the sampler proposes parameter values; the Dashboard then:

1. **Delivers the parameters** to your command — as an input file, environment
   variables, a JSON stdin payload, or CLI arguments.
2. **Runs your command** (with an optional timeout and automatic retries),
   optionally wrapped by a **pre-command** and a **post-command**.
3. **Extracts the objective and constraint values** from what the command
   printed (stdout) or wrote to a file — via a regular expression, a JSON path,
   or a CSV cell.
4. **Records the trial** to the journal (parameters, objectives, constraints,
   COMPLETE / FAIL).

The samplers are **Random** and **NSGA-II** (single- and multi-objective),
implemented in Rust — the same ones the Grasshopper runner uses.

## The process definition

A run is described by a `ProcessDefinition`, which is plain JSON (so it can be
saved, shared, and edited in the GUI builder):

```json
{
  "param_names": ["length", "thickness"],
  "input":   { "kind": "args", "arg_template": "--{name}={value}" },
  "command": { "program": "python3", "args": ["solve.py"], "timeout_secs": 60, "retries": 1 },
  "objectives":  [
    { "name": "mass", "source": { "kind": "stdout" },
      "extractor": { "kind": "regex", "pattern": "mass\\s*=\\s*([0-9.eE+-]+)" } }
  ],
  "constraints": [
    { "name": "stress", "source": { "kind": "file", "path": "out.json" },
      "extractor": { "kind": "json_path", "path": "results.stress_ratio" } }
  ]
}
```

- **`param_names`** — the optimization variables, in the order values are
  delivered. Names must be non-blank, unique, and whitespace-free.
- **`input`** — how parameters reach the command (see below).
- **`command`** — the program, fixed `args` (passed verbatim), optional
  `working_dir`, `timeout_secs` (`0` = no limit), and `retries` (extra attempts
  after a failure).
- **`objectives`** / **`constraints`** — one extraction spec each. A trial is
  **feasible when every constraint value is ≤ 0** (Tunny's convention), and
  constraints are soft: an infeasible trial is still recorded, and feasibility
  steers NSGA-II.
- **`pre_command`** / **`post_command`** (optional) — commands run before and
  after the evaluation, e.g. to stage inputs or post-process raw solver output
  into the file the extractor reads. Each has its own `working_dir`.

### Delivering parameters (`input`)

Pick whichever convention your tool already speaks:

| `kind` | What the command receives | Example |
| --- | --- | --- |
| `args` | CLI arguments — `arg_template` expanded per parameter with `{name}` / `{value}`, split on whitespace | `--length=12.5 --thickness=3` |
| `env` | One environment variable per parameter (named after it) | `length=12.5`, `thickness=3` |
| `json_stdin` | A JSON object on stdin | `{"length": 12.5, "thickness": 3}` |
| `template` | A rendered input file at `path` (`{name}` placeholders substituted, `{{`/`}}` are literal braces) | see below |

`template` example — write a solver input deck before each run:

```json
"input": {
  "kind": "template",
  "path": "case.inp",
  "template": "LENGTH {length}\nTHICKNESS {thickness}\n"
}
```

Integral parameter values render without a trailing `.0` (`3`, not `3.0`), so a
tool that parses integers sees the token it expects.

### Extracting outputs (`extractor`)

Each objective/constraint reads from a `source` (`stdout` or a `file`) and is
parsed by one of:

| `kind` | Reads | Fields |
| --- | --- | --- |
| `regex` | the first capture group (or the whole match) as a number | `pattern` |
| `json_path` | a dotted path into a JSON document — object keys and array indices, e.g. `results.objectives.0` | `path` |
| `csv` | one cell of a CSV | `row` (`{ "kind": "index", "index": N }` or `{ "kind": "last" }`), `column` (`index` or header `name`), and `has_header` |

Notes:

- The extracted value must be a **finite number**. A non-numeric or `inf` / `nan`
  output makes that trial a **failed evaluation** (recorded as FAIL) rather than
  a silently wrong success — so a diverged solver does not poison the results.
- The whole matched token must be a clean number (`12.5`, `1e-3`). Values with a
  trailing unit or separator (`12.5 kg`, `1_000`) are **not** partially parsed —
  use a regex whose capture group isolates the number.
- For CSV, `has_header: true` means line 0 is the header (data rows start at
  line 1) and enables selecting a column by header `name`; row indices always
  count from the first data row.

## Building a definition in the GUI

You don't have to write the JSON by hand. Click **Optimize ▸ New Tool…** on the
toolbar to open the **definition builder**, a form that maps one-to-one to the
fields above:

- **Parameters** — add / remove parameter names (referenced as `{name}`).
- **Input** — pick how parameters reach the command (command-line args,
  environment variables, JSON on stdin, or an input-file template) and fill in
  that scheme's fields.
- **Command** — the program, any fixed args (added / removed as rows), working
  directory, timeout, and retries.
- **Objectives** and **Constraints** — one boxed row each: the name, whether the
  value comes from stdout or a file, and the extractor (regex / JSON path / CSV
  cell) with its settings.
- **Hooks** — optionally enable a pre- and/or post-command.

Buttons at the bottom:

- **Load…** imports an existing definition JSON into the form for editing.
- **Save to File…** validates the form and writes it to a JSON file (the same
  checks as loading: at least one objective, a non-blank program, and unique,
  whitespace-free parameter and objective names). Blank rows are ignored.
- **Optimize →** validates the form and hands the definition straight to the run
  setup dialog below — no need to save first.

## Running a study from the Dashboard

Click **Optimize ▸ Optimize Tool…** on the toolbar and pick a process-definition
JSON (or reach this dialog via **Optimize →** in the builder). The setup dialog
shows the command read-only and lets you fill in the rest:

- a search **range** for each parameter (low / high, decimal digits, or an
  integer flag) — ranges start at `[0, 1]` for you to edit, and a row with
  `low ≥ high` is flagged and blocks the run;
- each objective's **direction** (Minimize / Maximize);
- the **sampler** — Random (with a trial count) or NSGA-II (population and
  generations, with the total evaluation count shown) — plus the seed;
- the **journal path** and **study name** (pre-filled next to the definition).

Press **Run** and the study appears in the study list immediately, with Live
Update on so trials stream in while the run proceeds. A progress overlay shows
the count and a **Cancel** button; on completion it reports how many trials
succeeded or failed. Constraints (if any) are listed read-only — they are part
of the definition, not something you configure here.

## Running a study from the library

A process definition describes *how to evaluate*; the *optimization problem* adds
the search **range** for each parameter and the objective **directions**. Build
them together so the variable order and objective/constraint counts always line
up, then run the sampler loop:

```rust
use std::collections::HashMap;
use tunny_core::process::{ProcessDefinition, ProcessEvaluator, VarRange};
use tunny_core::runner::{prepare_run, run_prepared, RunConfig, Sampler};
use tunny_core::io::journal::parser::OptimizationDirection;
use tunny_core::surrogate_opt::FitProgress;

// 1. Load / build the definition.
let def = ProcessDefinition::from_json(json_text)?;

// 2. Give each parameter a search range (low, high, decimal digits, integer?).
let ranges = HashMap::from([
    ("length".into(),    VarRange { low: 5.0,  high: 30.0, digits: 2, is_integer: false }),
    ("thickness".into(), VarRange { low: 1.0,  high: 10.0, digits: 0, is_integer: true  }),
]);
let problem = def.build_problem(&ranges)?;         // aligned to the definition
let evaluator = ProcessEvaluator::new(def)?;

// 3. Configure and run — the Dashboard drives the sampling, your command evaluates.
let cfg = RunConfig {
    study_name: "beam".into(),
    directions: vec![OptimizationDirection::Minimize], // one per objective
    sampler: Sampler::Nsga2,
    population_size: 16,
    generations: 20,
    ..RunConfig::default()
};
let progress = FitProgress::new();
let prep = prepare_run(&journal_path, &problem, &cfg)?;   // creates the study
let summary = run_prepared(&prep, &problem, &evaluator, &cfg, &progress)?;
```

The study exists in the journal as soon as `prepare_run` returns, so you can open
the journal in the Dashboard and watch trials stream in live while
`run_prepared` runs. `progress` exposes progress and cancellation.

## Behavior notes

- **Parallelism** — trials evaluate concurrently; make sure your command is safe
  to run in parallel (e.g. give each a distinct `working_dir` if it writes fixed
  filenames).
- **Failures** — a non-zero exit, a timeout, a missing/again-non-numeric output,
  or the wrong number of objectives/constraints records the trial as **FAIL**
  (with a penalty fed to the sampler) and the run continues. Only a failure to
  write the journal itself aborts the run.
- **Attributes** — process definitions extract objectives and constraints only;
  per-trial user attributes are not part of the definition (`build_problem`
  declares none).

## Worked example

A two-variable minimization whose objective is printed to stdout by a shell
tool (any language works — this uses `awk` for brevity):

```json
{
  "param_names": ["x", "y"],
  "input":   { "kind": "args", "arg_template": "{value}" },
  "command": { "program": "sh",
               "args": ["-c", "awk \"BEGIN{print \\\"f=\\\" (($1-3)*($1-3) + $2)}\"", "sh"],
               "timeout_secs": 10, "retries": 1 },
  "objectives": [
    { "name": "f", "source": { "kind": "stdout" },
      "extractor": { "kind": "regex", "pattern": "f=([-0-9.]+)" } }
  ]
}
```

With `x ∈ [0, 6]` and `y ∈ [0, 5]` minimized, the Dashboard samples points,
runs the command per trial, records `f = (x−3)² + y`, and writes an ordinary
Optuna study you can then analyze with every Dashboard feature — all without a
Python or Optuna installation.
