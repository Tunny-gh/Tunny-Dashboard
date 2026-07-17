//! Extracts an optimization problem definition from a .ghx file.
//!
//! Extraction rules:
//! - Find the Tunny component in the definition (an object name containing "tunny")
//! - Number Sliders connected to its Variables input → variables (name, range, digits)
//! - Parameters connected to its Objectives input → objectives (name and source GUID)
//!
//! The objective direction (minimize/maximize) is not read from the ghx
//! because the serialization format of the Tunny component's internal
//! settings isn't fixed; it defaults to Minimize and is left for the user
//! to edit in the UI.

use super::ghx::{parse_archive, GhxChunk};

/// An optimization variable (originating from a Number Slider).
#[derive(Debug, Clone)]
pub struct GhVariable {
    /// The slider's InstanceGuid (used for RH_IN group injection)
    pub instance_guid: String,
    /// Becomes the journal's param name (the slider's NickName; a sequence number is appended on duplicates)
    pub name: String,
    pub low: f64,
    pub high: f64,
    /// The slider's value at the time the definition was saved
    pub value: f64,
    /// Number of decimal digits (the slider's rounding; evaluated values are also rounded to this)
    pub digits: u32,
    /// Whether it's an integer slider (digits == 0 is treated as integer)
    pub is_integer: bool,
}

/// An optimization objective (a parameter connected to Tunny's Objectives input).
#[derive(Debug, Clone)]
pub struct GhObjective {
    /// The source parameter's InstanceGuid (used for RH_OUT relay injection)
    pub source_guid: String,
    /// Objective name (the source parameter's NickName; a sequence number is appended on duplicates)
    pub name: String,
}

/// Intermediate representation of an optimization problem extracted from a .ghx file.
/// Will be merged into the same type once a future .gh + manifest path is added (ROADMAP item 15).
#[derive(Debug, Clone)]
pub struct GhProblem {
    pub variables: Vec<GhVariable>,
    pub objectives: Vec<GhObjective>,
    /// Display name of the detected Tunny component
    pub tunny_component: String,
    /// Notes on connections etc. ignored during extraction (shown in the UI)
    pub warnings: Vec<String>,
}

/// Intermediate info for an object within the definition.
struct ObjectRecord<'a> {
    /// Object type name ("Number Slider" / "Group" etc.; the Name item directly under Object)
    type_name: &'a str,
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
            "DefinitionObjects が見つかりません。Grasshopper の .ghx ファイルか確認してください"
                .to_string()
        })?;

    // Walk all objects and build the index
    let mut records: Vec<ObjectRecord<'_>> = Vec::new();
    for obj in objects_chunk.chunks_named("Object") {
        let type_name = obj.item_text("Name").unwrap_or("");
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
            "Tunny コンポーネントが見つかりません。Tunny で最適化を構成した定義を \
             .ghx 形式で保存してください"
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
            Some(rec) => warnings.push(format!(
                "変数入力に接続された「{}」（{}）は Number Slider ではないためスキップしました",
                rec.nickname, rec.type_name
            )),
            None => warnings.push(format!(
                "変数入力の接続元 {guid} が定義内に見つかりませんでした"
            )),
        }
    }

    // Objectives input → resolve names of source parameters
    let objective_sources = input_sources(tunny.container, &["objective", "objs"], "o");
    let mut objectives = Vec::new();
    for (i, guid) in objective_sources.iter().enumerate() {
        let name = param_index
            .get(guid.as_str())
            .map(|p| p.nickname.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("objective_{}", i + 1));
        objectives.push(GhObjective {
            source_guid: guid.clone(),
            name,
        });
    }

    if variables.is_empty() {
        return Err(format!(
            "Tunny コンポーネント「{}」の変数入力に接続されたスライダーが見つかりません",
            tunny.nickname
        ));
    }
    if objectives.is_empty() {
        return Err(format!(
            "Tunny コンポーネント「{}」の目的入力に接続されたパラメータが見つかりません",
            tunny.nickname
        ));
    }

    dedupe_names(
        &mut variables
            .iter_mut()
            .map(|v| &mut v.name)
            .collect::<Vec<_>>(),
    );
    dedupe_names(
        &mut objectives
            .iter_mut()
            .map(|o| &mut o.name)
            .collect::<Vec<_>>(),
    );

    Ok(GhProblem {
        variables,
        objectives,
        tunny_component: tunny.nickname.clone(),
        warnings,
    })
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
    let slider = rec.container.find_chunk("Slider").ok_or_else(|| {
        format!(
            "スライダー「{}」の Slider チャンクが見つかりません",
            rec.nickname
        )
    })?;
    let low = slider
        .item_f64("Min")
        .ok_or_else(|| format!("スライダー「{}」の Min が読めません", rec.nickname))?;
    let high = slider
        .item_f64("Max")
        .ok_or_else(|| format!("スライダー「{}」の Max が読めません", rec.nickname))?;
    if !high.is_finite() || !low.is_finite() || high <= low {
        return Err(format!(
            "スライダー「{}」の範囲が不正です（Min={low}, Max={high}）",
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
    })
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
    use crate::gh::fixtures::sample_ghx;

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
