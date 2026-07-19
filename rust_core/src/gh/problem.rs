//! Extracts an optimization problem definition from a .ghx file.
//!
//! Extraction rules:
//! - Find the Tunny component in the definition (an object name containing "tunny")
//! - Number Sliders connected to its Variables input → one variable each
//!   (name, range, digits); a Gene Pool → one variable per gene (the pool's
//!   shared range/digits, each gene's own value)
//! - Parameters connected to its Objectives input → objectives (name and source GUID)
//!
//! The objective direction (minimize/maximize) is not read from the ghx
//! because the serialization format of the Tunny component's internal
//! settings isn't fixed; it defaults to Minimize and is left for the user
//! to edit in the UI.

use super::ghx::{parse_archive, GhxChunk};

/// Type GUID of the Galapagos Gene Pool component (the `<item name="GUID">`
/// directly under its `Object` chunk). Matched alongside the type name so a
/// renamed/localized display name still resolves.
const GENE_POOL_TYPE_GUID: &str = "21553c44-ea62-475e-a8bb-62b2a3ee5ca5";

/// Locates one gene within its Gene Pool. All genes of a pool share the pool's
/// `instance_guid` and are delivered to Rhino.Compute as a single RH_IN list
/// input (one value per gene), rather than one input per gene.
#[derive(Debug, Clone, PartialEq)]
pub struct GenePoolSlot {
    /// The pool's RH_IN input nickname (its NickName). Shared by every gene.
    pub group_name: String,
    /// Number of genes in the pool (= length of the RH_IN value list).
    pub count: usize,
    /// This gene's 0-based index within the pool.
    pub index: usize,
}

/// An optimization variable (originating from a Number Slider, or one gene of a
/// Gene Pool).
#[derive(Debug, Clone)]
pub struct GhVariable {
    /// The source object's InstanceGuid (used for RH_IN group injection). For a
    /// Gene Pool gene this is the pool's guid — shared by all of the pool's genes.
    pub instance_guid: String,
    /// Becomes the journal's param name (the slider's NickName, or `<pool nick><i>`
    /// for a gene; a sequence number is appended on duplicates)
    pub name: String,
    pub low: f64,
    pub high: f64,
    /// The value at the time the definition was saved
    pub value: f64,
    /// Number of decimal digits (the rounding; evaluated values are also rounded to this)
    pub digits: u32,
    /// Whether it's an integer variable (digits == 0 is treated as integer)
    pub is_integer: bool,
    /// Set when this variable is one gene of a Gene Pool (see [`GenePoolSlot`]).
    /// `None` for a standalone Number Slider (its own single-value RH_IN input).
    pub gene_pool: Option<GenePoolSlot>,
}

/// An optimization objective (a parameter connected to Tunny's Objectives input).
#[derive(Debug, Clone)]
pub struct GhObjective {
    /// The source parameter's InstanceGuid (used for RH_OUT relay injection)
    pub source_guid: String,
    /// Objective name (the source parameter's NickName; a sequence number is appended on duplicates)
    pub name: String,
}

/// An optimization constraint (a parameter connected to the Constraint input of
/// the attribute component wired to Tunny's Attributes input).
///
/// Tunny's convention: a trial is feasible when every constraint value is <= 0.
/// Constraints are "soft" — infeasible trials are still evaluated and recorded;
/// feasibility only steers the sampler and the analysis.
#[derive(Debug, Clone)]
pub struct GhConstraint {
    /// The source parameter's InstanceGuid (used for RH_OUT relay injection)
    pub source_guid: String,
    /// Constraint name (the source parameter's NickName; a sequence number is appended on duplicates)
    pub name: String,
}

/// A per-trial attribute (a parameter connected to the Attribute input of the
/// attribute component wired to Tunny's Attributes input).
///
/// Recorded to the journal as an Optuna trial user attribute under this name,
/// so it shows up in the dashboard's user-attribute columns. The Geometry
/// input is not captured (geometry has no journal representation; it belongs
/// to a future artifact store).
#[derive(Debug, Clone)]
pub struct GhAttribute {
    /// The source parameter's InstanceGuid (used for RH_OUT relay injection)
    pub source_guid: String,
    /// Attribute name (the source parameter's NickName; a sequence number is appended on duplicates)
    pub name: String,
}

/// Intermediate representation of an optimization problem extracted from a .ghx file.
/// Will be merged into the same type once a future .gh + manifest path is added (ROADMAP item 15).
#[derive(Debug, Clone)]
pub struct GhProblem {
    pub variables: Vec<GhVariable>,
    pub objectives: Vec<GhObjective>,
    /// Constraints wired via the attribute component (empty when none are set up)
    pub constraints: Vec<GhConstraint>,
    /// Per-trial attributes wired via the attribute component (empty when none are set up)
    pub attributes: Vec<GhAttribute>,
    /// Display name of the detected Tunny component
    pub tunny_component: String,
    /// Notes on connections etc. ignored during extraction (shown in the UI)
    pub warnings: Vec<String>,
}

/// Intermediate info for an object within the definition.
struct ObjectRecord<'a> {
    /// Object type name ("Number Slider" / "Gene Pool" / "Group" etc.; the Name
    /// item directly under Object)
    type_name: &'a str,
    /// Object type GUID (the `GUID` item directly under Object; identifies the
    /// component type independently of its display name)
    type_guid: &'a str,
    container: &'a GhxChunk,
    instance_guid: &'a str,
    nickname: String,
}

/// Index entry mapping a parameter GUID → its display name.
struct ParamEntry {
    nickname: String,
}

/// Extracts an optimization problem from .ghx text.
pub fn extract_problem(xml: &str) -> Result<GhProblem, String> {
    let root = parse_archive(xml)?;
    let objects_chunk = root
        .find_chunk_recursive("DefinitionObjects")
        .ok_or_else(|| {
            "DefinitionObjects not found; make sure this is a Grasshopper .ghx file".to_string()
        })?;

    // Walk all objects and build the index
    let mut records: Vec<ObjectRecord<'_>> = Vec::new();
    for obj in objects_chunk.chunks_named("Object") {
        let type_name = obj.item_text("Name").unwrap_or("");
        let type_guid = obj.item_text("GUID").unwrap_or("");
        let Some(container) = obj.find_chunk("Container") else {
            continue;
        };
        let Some(instance_guid) = container.item_text("InstanceGuid") else {
            continue;
        };
        let nickname = container
            .item_text("NickName")
            .filter(|s| !s.is_empty())
            .or_else(|| container.item_text("Name"))
            .unwrap_or("")
            .to_string();
        records.push(ObjectRecord {
            type_name,
            type_guid,
            container,
            instance_guid,
            nickname,
        });
    }

    // Parameter GUID index: registers both floating parameters themselves
    // and component input/output parameters (param_input / param_output).
    let mut param_index: std::collections::HashMap<&str, ParamEntry> =
        std::collections::HashMap::new();
    for rec in &records {
        param_index.insert(
            rec.instance_guid,
            ParamEntry {
                nickname: rec.nickname.clone(),
            },
        );
        for param in component_params(rec.container) {
            if let Some(guid) = param.item_text("InstanceGuid") {
                let nickname = param
                    .item_text("NickName")
                    .filter(|s| !s.is_empty())
                    .or_else(|| param.item_text("Name"))
                    .unwrap_or("")
                    .to_string();
                param_index.insert(guid, ParamEntry { nickname });
            }
        }
    }

    // Detect the Tunny component
    let tunny = records
        .iter()
        .find(|r| {
            r.type_name.to_ascii_lowercase().contains("tunny")
                || r.nickname.to_ascii_lowercase().contains("tunny")
        })
        .ok_or_else(|| {
            "Tunny component not found. Save a definition that contains a Tunny \
             optimization setup as .ghx"
                .to_string()
        })?;

    let mut warnings = Vec::new();

    // Variables input → slider resolution
    let variable_sources = input_sources(tunny.container, &["variable", "vars"], "v");
    let mut variables = Vec::new();
    for guid in &variable_sources {
        match records.iter().find(|r| r.instance_guid == guid) {
            Some(rec) if rec.type_name.eq_ignore_ascii_case("Number Slider") => {
                match read_slider(rec) {
                    Ok(var) => variables.push(var),
                    Err(e) => warnings.push(e),
                }
            }
            Some(rec) if is_gene_pool(rec) => match read_gene_pool(rec) {
                Ok(genes) => variables.extend(genes),
                Err(e) => warnings.push(e),
            },
            Some(rec) => warnings.push(format!(
                "Skipped \"{}\" ({}) on the variables input because it is not a Number Slider or Gene Pool",
                rec.nickname, rec.type_name
            )),
            None => warnings.push(format!(
                "Source {guid} of the variables input was not found in the definition"
            )),
        }
    }

    // Objectives input → resolve names of source parameters
    let objective_sources = input_sources(tunny.container, &["objective", "objs"], "o");
    let mut objectives = Vec::new();
    for (i, guid) in objective_sources.iter().enumerate() {
        objectives.push(GhObjective {
            source_guid: guid.clone(),
            name: resolve_param_name(&param_index, guid, "objective", i),
        });
    }

    // Attributes input → follow to the attribute component (Construct Fish
    // Attribute) and read the sources of its Constraint input as constraints
    // (Tunny's convention: constraint value <= 0 means the trial is feasible)
    // and the sources of its Attribute input as per-trial attributes. The
    // Geometry input is intentionally ignored (no journal representation).
    let attribute_sources = input_sources(tunny.container, &["attribute", "attrs"], "attr");
    let mut constraints = Vec::new();
    let mut attributes = Vec::new();
    for guid in &attribute_sources {
        // The source GUID is the attribute component's output parameter (or the
        // component itself when it is a floating object); resolve the owner.
        let owner = records.iter().find(|r| {
            r.instance_guid == guid.as_str()
                || component_params(r.container)
                    .iter()
                    .any(|p| p.item_text("InstanceGuid") == Some(guid.as_str()))
        });
        let Some(owner) = owner else {
            warnings.push(format!(
                "Source {guid} of the attributes input was not found in the definition"
            ));
            continue;
        };
        // Identity check, mirroring the slider-type check on the variables
        // input: the Constraint/Attribute input nicks ("C", "Attr") are generic
        // enough that harvesting them from an arbitrary component wired into
        // the Attributes input would silently invent constraints.
        let owner_label = format!("{} {}", owner.type_name, owner.nickname).to_ascii_lowercase();
        if !owner_label.contains("attr") {
            warnings.push(format!(
                "Skipped \"{}\" ({}) on the attributes input because it does not look like an attribute component",
                owner.nickname, owner.type_name
            ));
            continue;
        }
        let constraint_sources = input_sources(owner.container, &["constraint"], "c");
        let attr_value_sources = input_sources(owner.container, &["attribute", "attrs"], "attr");
        if constraint_sources.is_empty() && attr_value_sources.is_empty() {
            warnings.push(format!(
                "No constraints or attributes found on attribute component \"{}\" (no sources on its Constraint / Attribute inputs)",
                owner.nickname
            ));
            continue;
        }
        for guid in &constraint_sources {
            constraints.push(GhConstraint {
                source_guid: guid.clone(),
                name: resolve_param_name(&param_index, guid, "constraint", constraints.len()),
            });
        }
        for guid in &attr_value_sources {
            attributes.push(GhAttribute {
                source_guid: guid.clone(),
                name: resolve_param_name(&param_index, guid, "attribute", attributes.len()),
            });
        }
    }

    if variables.is_empty() {
        return Err(format!(
            "No sliders connected to the variables input of Tunny component \"{}\"",
            tunny.nickname
        ));
    }
    if objectives.is_empty() {
        return Err(format!(
            "No parameters connected to the objectives input of Tunny component \"{}\"",
            tunny.nickname
        ));
    }

    // Attribute names become journal user-attr column names, where they would
    // collide with the parser's generated constraint/feasibility columns —
    // colliding columns get silently shadowed or cross-contaminated downstream,
    // so rename them here and tell the user.
    for attr in &mut attributes {
        if is_reserved_column_name(&attr.name) {
            let renamed = format!("{}_attr", attr.name);
            warnings.push(format!(
                "Attribute \"{}\" was renamed to \"{renamed}\" because the name is reserved for constraint columns",
                attr.name
            ));
            attr.name = renamed;
        }
    }

    // Deduplicate across ALL name lists at once: every name ends up as a
    // journal column in a single namespace (params / objectives / user attrs),
    // so a cross-category duplicate (e.g. an attribute named like an objective)
    // is just as harmful as one within a category.
    dedupe_names(
        &mut variables
            .iter_mut()
            .map(|v| &mut v.name)
            .chain(objectives.iter_mut().map(|o| &mut o.name))
            .chain(constraints.iter_mut().map(|c| &mut c.name))
            .chain(attributes.iter_mut().map(|a| &mut a.name))
            .collect::<Vec<_>>(),
    );

    Ok(GhProblem {
        variables,
        objectives,
        constraints,
        attributes,
        tunny_component: tunny.nickname.clone(),
        warnings,
    })
}

/// Resolves the display name for a source parameter: its non-empty NickName
/// from the index, or `"{fallback}_{index+1}"` when unknown. Shared by the
/// objectives / constraints / attributes loops so the naming rule stays in one
/// place.
fn resolve_param_name(
    param_index: &std::collections::HashMap<&str, ParamEntry>,
    guid: &str,
    fallback: &str,
    index: usize,
) -> String {
    param_index
        .get(guid)
        .map(|p| p.nickname.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}_{}", fallback, index + 1))
}

/// Whether a name collides with the journal parser's generated constraint /
/// feasibility columns (`c1`..`cN`, `is_feasible`, `constraint_sum`).
fn is_reserved_column_name(name: &str) -> bool {
    if name == "is_feasible" || name == "constraint_sum" {
        return true;
    }
    let mut chars = name.chars();
    chars.next() == Some('c') && {
        let rest = chars.as_str();
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
    }
}

/// Recursively collects input/output parameter chunks (param_input /
/// param_output) under a component's Container. Not limited to direct
/// children, since the nesting position can vary by GH version.
fn component_params(container: &GhxChunk) -> Vec<&GhxChunk> {
    let mut found = Vec::new();
    collect_params(container, &mut found);
    found
}

fn collect_params<'a>(chunk: &'a GhxChunk, out: &mut Vec<&'a GhxChunk>) {
    for child in &chunk.chunks {
        let name = child.name.to_ascii_lowercase();
        if name == "param_input" || name == "param_output" {
            out.push(child);
        } else {
            collect_params(child, out);
        }
    }
}

/// Returns, in document order, the Source GUIDs connected to a Tunny
/// component's specified input (matched by partial name match or exact
/// nickname match).
fn input_sources(container: &GhxChunk, name_keys: &[&str], nick_key: &str) -> Vec<String> {
    let mut found = Vec::new();
    for param in component_params(container) {
        if !param.name.eq_ignore_ascii_case("param_input") {
            continue;
        }
        let name = param.item_text("Name").unwrap_or("").to_ascii_lowercase();
        let nick = param
            .item_text("NickName")
            .unwrap_or("")
            .to_ascii_lowercase();
        let matches = name_keys
            .iter()
            .any(|k| name.contains(k) || nick.contains(k))
            || nick == nick_key;
        if !matches {
            continue;
        }
        for src in param.items_named("Source") {
            let guid = src.text.trim();
            if !guid.is_empty() {
                found.push(guid.to_string());
            }
        }
    }
    found
}

/// Reads variable info from a slider object.
fn read_slider(rec: &ObjectRecord<'_>) -> Result<GhVariable, String> {
    let slider = rec
        .container
        .find_chunk("Slider")
        .ok_or_else(|| format!("Slider chunk not found for slider \"{}\"", rec.nickname))?;
    let low = slider
        .item_f64("Min")
        .ok_or_else(|| format!("Cannot read Min of slider \"{}\"", rec.nickname))?;
    let high = slider
        .item_f64("Max")
        .ok_or_else(|| format!("Cannot read Max of slider \"{}\"", rec.nickname))?;
    if !high.is_finite() || !low.is_finite() || high <= low {
        return Err(format!(
            "Slider \"{}\" has an invalid range (Min={low}, Max={high})",
            rec.nickname
        ));
    }
    let value = slider.item_f64("Value").unwrap_or(low);
    let digits = slider.item_i64("Digits").unwrap_or(2).max(0) as u32;
    let name = if rec.nickname.is_empty() {
        "x".to_string()
    } else {
        rec.nickname.clone()
    };
    Ok(GhVariable {
        instance_guid: rec.instance_guid.to_string(),
        name,
        low,
        high,
        value,
        digits,
        is_integer: digits == 0,
        gene_pool: None,
    })
}

/// Whether an object is a Galapagos Gene Pool (by type name or type GUID).
fn is_gene_pool(rec: &ObjectRecord<'_>) -> bool {
    rec.type_name.eq_ignore_ascii_case("Gene Pool")
        || rec.type_guid.eq_ignore_ascii_case(GENE_POOL_TYPE_GUID)
}

/// Reads a Gene Pool into one variable per gene. Every gene shares the pool's
/// range/decimals (from the `GeneData` chunk's `Minimum` / `Maximum` /
/// `Decimals`) and carries its own saved value (the indexed `Value` items). Gene
/// names are `<pool nick><i>`; all genes keep the pool's `instance_guid` so they
/// are injected as a single RH_IN list input.
fn read_gene_pool(rec: &ObjectRecord<'_>) -> Result<Vec<GhVariable>, String> {
    let data = rec.container.find_chunk("GeneData").ok_or_else(|| {
        format!(
            "GeneData chunk not found for gene pool \"{}\"",
            rec.nickname
        )
    })?;
    let low = data
        .item_f64("Minimum")
        .ok_or_else(|| format!("Cannot read Minimum of gene pool \"{}\"", rec.nickname))?;
    let high = data
        .item_f64("Maximum")
        .ok_or_else(|| format!("Cannot read Maximum of gene pool \"{}\"", rec.nickname))?;
    if !high.is_finite() || !low.is_finite() || high <= low {
        return Err(format!(
            "Gene pool \"{}\" has an invalid range (Minimum={low}, Maximum={high})",
            rec.nickname
        ));
    }
    let digits = data.item_i64("Decimals").unwrap_or(2).max(0) as u32;
    // Per-gene saved values, kept positionally: a value that fails to parse
    // falls back to the low bound in place rather than being dropped (dropping
    // would shift every later gene's value onto the wrong index).
    let values: Vec<f64> = data
        .items_named("Value")
        .map(|it| it.text.trim().parse().unwrap_or(low))
        .collect();
    // The declared Count is authoritative; never emit fewer genes than there are
    // Value items either (so a missing/short Count can't drop genes).
    let count = (data.item_i64("Count").unwrap_or(0).max(0) as usize).max(values.len());
    if count == 0 {
        return Err(format!("Gene pool \"{}\" has no genes", rec.nickname));
    }
    let base = if rec.nickname.is_empty() {
        "gene".to_string()
    } else {
        rec.nickname.clone()
    };
    let genes = (0..count)
        .map(|i| GhVariable {
            instance_guid: rec.instance_guid.to_string(),
            name: format!("{base}{i}"),
            low,
            high,
            value: values.get(i).copied().unwrap_or(low),
            digits,
            is_integer: digits == 0,
            gene_pool: Some(GenePoolSlot {
                group_name: base.clone(),
                count,
                index: i,
            }),
        })
        .collect();
    Ok(genes)
}

/// Uniquifies duplicate names by appending a sequence-number suffix (journal
/// param names / objective names must be unique).
fn dedupe_names(names: &mut [&mut String]) {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in names.iter_mut() {
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            **name = format!("{}_{}", name, count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::fixtures::{sample_ghx, sample_ghx_without_constraint};

    #[test]
    fn extracts_variables_and_objectives_from_fixture() {
        let problem = extract_problem(&sample_ghx()).unwrap();
        assert_eq!(problem.tunny_component, "Tunny");

        assert_eq!(problem.variables.len(), 2);
        let span = &problem.variables[0];
        assert_eq!(span.name, "span");
        assert_eq!(span.low, 3.0);
        assert_eq!(span.high, 12.0);
        assert_eq!(span.value, 5.5);
        assert_eq!(span.digits, 2);
        assert!(!span.is_integer);
        let count = &problem.variables[1];
        assert_eq!(count.name, "count");
        assert!(count.is_integer);
        assert_eq!(count.low, 1.0);
        assert_eq!(count.high, 10.0);

        assert_eq!(problem.objectives.len(), 2);
        // Objective via a component output parameter
        assert_eq!(problem.objectives[0].name, "weight");
        assert_eq!(
            problem.objectives[0].source_guid,
            "0aaaaaaa-0000-0000-0000-00000000beam"
        );
        // Objective via a floating parameter
        assert_eq!(problem.objectives[1].name, "disp");

        // Constraint via the attribute component's Constraint input
        assert_eq!(problem.constraints.len(), 1);
        assert_eq!(problem.constraints[0].name, "penalty");
        assert_eq!(
            problem.constraints[0].source_guid,
            "0aaaaaaa-0000-0000-0000-00000000pena"
        );

        // Per-trial attribute via the attribute component's Attribute input
        assert_eq!(problem.attributes.len(), 1);
        assert_eq!(problem.attributes[0].name, "area");
        assert_eq!(
            problem.attributes[0].source_guid,
            "0aaaaaaa-0000-0000-0000-00000000area"
        );
        assert!(problem.warnings.is_empty());
    }

    #[test]
    fn extracts_gene_pool_as_one_variable_per_gene() {
        use crate::gh::fixtures::sample_ghx_gene_pool;
        let problem = extract_problem(&sample_ghx_gene_pool()).unwrap();

        // 3 genes → 3 variables, each with the pool's shared range/decimals and
        // its own saved value.
        assert_eq!(problem.variables.len(), 3);
        for (i, expected_value) in [25.0, 50.0, 75.0].into_iter().enumerate() {
            let v = &problem.variables[i];
            assert_eq!(v.name, format!("Genes{i}"));
            assert_eq!(v.low, 0.0);
            assert_eq!(v.high, 100.0);
            assert_eq!(v.digits, 2);
            assert!(!v.is_integer);
            assert_eq!(v.value, expected_value);
            // All genes share the pool's InstanceGuid (one RH_IN list input).
            assert_eq!(v.instance_guid, "0aaaaaaa-0000-0000-0000-00000000pool");
            let slot = v.gene_pool.as_ref().expect("gene should carry a pool slot");
            assert_eq!(slot.group_name, "Genes");
            assert_eq!(slot.count, 3);
            assert_eq!(slot.index, i);
        }

        assert_eq!(problem.objectives.len(), 1);
        assert_eq!(problem.objectives[0].name, "obj");
        assert!(problem.warnings.is_empty());
    }

    #[test]
    fn gene_pool_unparseable_value_keeps_gene_alignment() {
        // A middle gene's value is corrupted: the gene count must stay 3, the bad
        // value falls back to the low bound, and later genes keep their values
        // (no index shift from dropping the bad item).
        use crate::gh::fixtures::sample_ghx_gene_pool;
        let xml = sample_ghx_gene_pool().replace(
            r#"<item name="Value" index="1" type_name="gh_decimal" type_code="7">50</item>"#,
            r#"<item name="Value" index="1" type_name="gh_decimal" type_code="7">N/A</item>"#,
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.variables.len(), 3);
        assert_eq!(problem.variables[0].value, 25.0);
        assert_eq!(problem.variables[1].value, 0.0); // fell back to low
        assert_eq!(problem.variables[2].value, 75.0); // not shifted
    }

    #[test]
    fn no_attribute_wiring_means_no_constraints_or_attributes() {
        let problem = extract_problem(&sample_ghx_without_constraint()).unwrap();
        assert!(problem.constraints.is_empty());
        assert!(problem.attributes.is_empty());
        // Not an error, and no warning either (an unwired Attributes input is normal)
        assert!(problem.warnings.is_empty());
    }

    #[test]
    fn attribute_component_without_recognized_inputs_warns() {
        // Rename both the Constraint and Attribute inputs so the attribute
        // component has no detectable inputs; extraction succeeds with a warning.
        let xml = sample_ghx()
            .replace(
                r#"<item name="Name" type_name="gh_string" type_code="10">Constraint</item>"#,
                r#"<item name="Name" type_name="gh_string" type_code="10">Other</item>"#,
            )
            .replace(
                r#"<item name="NickName" type_name="gh_string" type_code="10">C</item>"#,
                r#"<item name="NickName" type_name="gh_string" type_code="10">X</item>"#,
            )
            .replace(
                r#"<item name="Name" type_name="gh_string" type_code="10">Attribute</item>"#,
                r#"<item name="Name" type_name="gh_string" type_code="10">Misc</item>"#,
            )
            .replace(
                r#"<item name="NickName" type_name="gh_string" type_code="10">Attr</item>"#,
                r#"<item name="NickName" type_name="gh_string" type_code="10">Y</item>"#,
            );
        let problem = extract_problem(&xml).unwrap();
        assert!(problem.constraints.is_empty());
        assert!(problem.attributes.is_empty());
        assert_eq!(problem.warnings.len(), 1);
        assert!(
            problem.warnings[0].contains("FishAttr"),
            "unexpected warning: {}",
            problem.warnings[0]
        );
    }

    #[test]
    fn attribute_named_like_constraint_column_is_renamed() {
        // Rename the fixture's attribute source ("area") to the reserved "c1".
        let xml = sample_ghx().replace(
            r#"<item name="NickName" type_name="gh_string" type_code="10">area</item>"#,
            r#"<item name="NickName" type_name="gh_string" type_code="10">c1</item>"#,
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.attributes[0].name, "c1_attr");
        assert!(
            problem.warnings.iter().any(|w| w.contains("c1_attr")),
            "expected rename warning, got {:?}",
            problem.warnings
        );
    }

    #[test]
    fn cross_category_duplicate_names_are_uniquified() {
        // Rename the attribute source to "weight" (the first objective's name):
        // the attribute must be renamed, not silently collide.
        let xml = sample_ghx().replace(
            r#"<item name="NickName" type_name="gh_string" type_code="10">area</item>"#,
            r#"<item name="NickName" type_name="gh_string" type_code="10">weight</item>"#,
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.objectives[0].name, "weight");
        assert_eq!(problem.attributes[0].name, "weight_2");
    }

    #[test]
    fn non_attribute_component_on_attributes_input_is_skipped_with_warning() {
        // Rename the attribute component so it no longer looks like one; its
        // "C"/"Attr" inputs must not be harvested as constraints/attributes.
        let xml = sample_ghx()
            .replace(
                r#"<item name="Name" type_name="gh_string" type_code="10">Construct Fish Attribute</item>"#,
                r#"<item name="Name" type_name="gh_string" type_code="10">Custom Cluster</item>"#,
            )
            .replace(
                r#"<item name="NickName" type_name="gh_string" type_code="10">FishAttr</item>"#,
                r#"<item name="NickName" type_name="gh_string" type_code="10">Cluster</item>"#,
            );
        let problem = extract_problem(&xml).unwrap();
        assert!(problem.constraints.is_empty());
        assert!(problem.attributes.is_empty());
        assert_eq!(problem.warnings.len(), 1);
        assert!(
            problem.warnings[0].contains("does not look like an attribute component"),
            "unexpected warning: {}",
            problem.warnings[0]
        );
    }

    #[test]
    fn reserved_column_name_detection() {
        assert!(is_reserved_column_name("c1"));
        assert!(is_reserved_column_name("c42"));
        assert!(is_reserved_column_name("is_feasible"));
        assert!(is_reserved_column_name("constraint_sum"));
        assert!(!is_reserved_column_name("c"));
        assert!(!is_reserved_column_name("c1a"));
        assert!(!is_reserved_column_name("cost"));
        assert!(!is_reserved_column_name("area"));
    }

    #[test]
    fn constraint_only_when_attribute_input_is_unwired() {
        // Remove the Attribute input's source: constraints stay, attributes empty.
        let xml = sample_ghx().replace(
            r#"<item name="Source" index="0" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000area</item>"#,
            "",
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.constraints.len(), 1);
        assert!(problem.attributes.is_empty());
        assert!(problem.warnings.is_empty());
    }

    #[test]
    fn error_without_tunny_component() {
        let xml = sample_ghx().replace("Tunny", "SomethingElse");
        let err = extract_problem(&xml).unwrap_err();
        assert!(err.contains("Tunny"), "unexpected error: {err}");
    }

    #[test]
    fn duplicate_names_are_uniquified() {
        // Change the fixture's count slider NickName to span to create a duplicate
        let xml = sample_ghx().replace(
            r#"<item name="NickName" type_name="gh_string" type_code="10">count</item>"#,
            r#"<item name="NickName" type_name="gh_string" type_code="10">span</item>"#,
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.variables[0].name, "span");
        assert_eq!(problem.variables[1].name, "span_2");
    }
}
