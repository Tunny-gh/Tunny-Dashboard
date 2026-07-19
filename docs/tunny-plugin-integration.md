# Tunny Plugin Integration Notes

What Tunny Dashboard reads from Tunny-authored Grasshopper definitions, what the
Tunny plugin side must keep stable for that to work, and what optional
Tunny-side work would unlock next. Audience: the Tunny (Grasshopper plugin)
maintainers.

Status: reflects the Phase 2B/2C implementation as of 2026-07 (items 15 and 16;
.ghx D&D execution via Rhino.Compute, constraints/attributes, and the adaptive
sampler).

## 1. Current integration model — no Tunny changes required

The dashboard parses the **.ghx (GH_Archive XML) serialization directly** and
needs no Tunny release to work. A definition saved as .ghx with a normal Tunny
setup is executable from the dashboard as-is:

1. `extract_problem` reads variables / objectives / constraints / attributes
   from the XML (detection contract below).
2. The dashboard injects `RH_IN:*` groups (one per variable slider; one per
   Gene Pool, carrying the pool's whole gene list) and `RH_OUT:*` relay
   parameters (objectives / constraints / attributes) into a copy of the
   definition and solves it via rhino.compute's `/grasshopper` endpoint.
3. Every trial is written to an Optuna-compatible journal (op codes
   0/3/4/5/6/8/9), so Optuna tooling and the dashboard's own analysis read the
   results without conversion.

### Detection contract

Matching is case-insensitive and name-based (partial match on the parameter
`Name`, or exact match on the `NickName` key listed below). These are the
strings the dashboard depends on:

| What | How it is found |
| --- | --- |
| Tunny component | Object type name or NickName contains `tunny` |
| Variables input | `Name` contains `variable`/`vars`, or NickName equals `v` |
| Objectives input | `Name` contains `objective`/`objs`, or NickName equals `o` |
| Attributes input | `Name` contains `attribute`/`attrs`, or NickName equals `attr` |
| Variable sources | `Number Slider` objects (one variable each) or a `Gene Pool` (type name `Gene Pool`, or type GUID `21553c44-…`; one variable per gene). Slider: name from NickName, range from the `Slider` chunk's `Min`/`Max`, resolution from `Digits` (0 = integer). Gene Pool: genes named `<pool nick><i>`, all sharing the pool's range/`Decimals` from its `GeneData` chunk (`Minimum`/`Maximum`/`Decimals`), each with its own `Value` |
| Objective names | NickName of the source parameter wired into Objectives |
| Attribute component | The component whose output feeds the Attributes input; accepted only when its type name or NickName contains `attr` (e.g. "Construct Fish Attribute") |
| Constraint input (on the attribute component) | `Name` contains `constraint`, or NickName equals `c` |
| Attribute input (on the attribute component) | `Name` contains `attribute`/`attrs`, or NickName equals `attr` |
| Geometry input | Intentionally ignored (no journal representation yet; see §3c) |

Semantics the dashboard implements, matching Tunny's published behavior:

- **Constraints are soft**: a trial is feasible when every constraint value is
  **≤ 0**; the amount above 0 is the violation. Violating trials are still
  evaluated and recorded (objective values kept); feasibility steers NSGA-II
  (constrained domination) and the EI acquisition (feasibility probability),
  and drives the feasibility columns in the analysis.
- **Attributes** become Optuna trial user attributes (numeric values become
  numeric columns, anything else text), visible in every user-attr-aware
  widget.
- Duplicate names are de-duplicated with `_2`, `_3`, … suffixes across all
  categories, and attribute names that collide with the generated constraint
  columns (`c1`…`cN`, `is_feasible`, `constraint_sum`) are renamed with a
  warning.

### Samplers available from the dashboard

- NSGA-II (with constrained domination) and Random.
- **Adaptive (surrogate)**: random bootstrap → fit surrogate (automatic model
  selection) → suggest candidates (Expected Improvement single-objective /
  EHVI multi-objective, feasibility-aware for single-objective) → evaluate via
  Rhino.Compute → refit. This is the dashboard-side realization of the
  "analyze → suggest → run → re-analyze" loop, so no Tunny-side execution or
  enqueue support is required for it.

## 2. Compatibility requests — please keep these stable

Renaming any of the following in a Tunny release silently breaks extraction
for definitions saved with that release (the dashboard fails soft with a
warning, but variables/constraints would go undetected):

1. The Tunny component's type name / NickName containing `tunny`.
2. The `Name`s (or listed NickNames) of the Variables / Objectives /
   Attributes inputs, and of the attribute component's Constraint / Attribute
   inputs.
3. The attribute component's type name containing `attr`
   ("Construct Fish Attribute" qualifies).
4. The Number Slider serialization consumed from GH_Archive: the `Slider`
   chunk's `Min` / `Max` / `Digits` / `Value` items; and the Gene Pool
   serialization: the `GeneData` chunk's `Minimum` / `Maximum` / `Decimals` /
   `Count` and indexed `Value` items.
5. The **≤ 0 = feasible** constraint convention.

If a rename is unavoidable, keeping the old string as part of the new one
(partial match) or telling us one release ahead keeps existing dashboards
working.

## 3. Optional Tunny-side work that would unlock improvements

### a. Embedded problem-definition manifest (enables plain .gh support)

Today the dashboard requires **.ghx** because it parses the XML serialization.
If Tunny embedded a small manifest into the definition when the user configures
an optimization — e.g. JSON stored on the Tunny component (user text /
component data, anything that survives both .gh and .ghx serialization) — the
dashboard could read only that manifest and support binary **.gh** drag & drop
too, while becoming robust to future GH_Archive format changes.

Proposed shape (versioned, additive):

```json
{
  "tunny_manifest": 1,
  "variables": [
    {"guid": "…slider instance guid…", "name": "span", "low": 3.0, "high": 12.0,
     "digits": 2, "integer": false}
  ],
  "objectives": [
    {"guid": "…source param guid…", "name": "weight", "direction": "minimize"}
  ],
  "constraints": [{"guid": "…", "name": "penalty"}],
  "attributes":  [{"guid": "…", "name": "area"}]
}
```

The GUIDs let the dashboard keep injecting RH_IN/RH_OUT against the live
objects; everything else removes the need for name-based detection. The
XML-parsing path stays as the fallback for definitions saved before the
manifest exists.

### b. Objective directions in a readable location

The dashboard cannot currently read minimize/maximize from the definition
(Tunny's per-objective direction, e.g. the FishAttribute direction values, has
no documented stable serialization), so the setup dialog defaults every
objective to Minimize and asks the user. If directions were available —
ideally inside the manifest above — the dialog would prefill them.

### c. Geometry / artifact handoff (later)

The attribute component's Geometry input is ignored: geometry has no
representation in the journal. Once the dashboard grows an artifact store
(ROADMAP prerequisite "Reading artifacts from the RDB"), a shared convention —
e.g. Tunny writing per-trial geometry files and recording their paths as a
trial user attribute or Optuna artifact — would let the dashboard display
geometry per trial. No action needed until then; we will propose a concrete
convention when the artifact store lands.

### d. Explicitly NOT needed

- **Consuming enqueued trials**: dropped in 2026-07. The dashboard executes
  suggested candidates itself (item 15 runner + item 16 adaptive sampler), so
  Tunny does not need to poll shared storage for dashboard-enqueued trials.
- Any Tunny-side execution API for the dashboard: rhino.compute is the
  execution interface.

## 4. Verified environment

The pipeline has been field-verified end-to-end against rhino.compute from
Rhino 8.31 with Hops 0.17, using .ghx files saved by Grasshopper 1.0
(ArchiveVersion 0.2.2). Changes to the GH_Archive version or to compute's
`/grasshopper` request/response schema are the environment-level assumptions
most worth communicating early.
