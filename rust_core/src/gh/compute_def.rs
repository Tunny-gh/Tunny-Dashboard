//! Generates a solve-ready definition for Rhino.Compute (a .ghx with RH_IN / RH_OUT added).
//!
//! Compute's Grasshopper endpoint assigns input values to groups whose nickname is
//! `RH_IN:name`, and includes the values of parameters inside `RH_OUT:name` groups in the
//! response. Here, for the original .ghx, we inject via XML string manipulation:
//!
//! - An `RH_IN:variable_name` group wrapping each variable slider
//! - A relay Number parameter wired to each objective's source, plus an
//!   `RH_OUT:objective_name` group wrapping it
//!
//! The original definition body is preserved byte-for-byte; only the DefinitionObjects
//! object count and an append at the end are touched.
//!
//! The type GUIDs and the serialization layout of the injected objects (Group /
//! Data parameter) are modeled on, and verified against, GH_Group and floating
//! parameter chunks found in real .ghx files. End-to-end solve behavior against
//! a live Rhino.Compute still needs field verification (ROADMAP item 15).

use super::problem::GhProblem;

/// Type GUID of the Grasshopper Group object (verified against real .ghx files).
const GROUP_TYPE_GUID: &str = "c552a431-af5b-46a9-a8a4-0fcbc27ef596";
/// Type GUID of the floating generic Data parameter, used as the RH_OUT relay
/// (verified against real .ghx files; passes through values of any type).
const DATA_PARAM_TYPE_GUID: &str = "8ec86459-bf01-4409-baee-174d0d2b13d0";

/// A Compute-ready definition with RH_IN / RH_OUT already injected.
#[derive(Debug, Clone)]
pub struct ComputeDefinition {
    /// The injected .ghx text
    pub ghx: String,
    /// Input parameter names, one per RH_IN group: a Number Slider is its own
    /// single-value input; a Gene Pool is one input carrying its whole gene list.
    /// (`RH_IN:name`, in the order the groups appear in `GhProblem.variables`.)
    pub input_params: Vec<String>,
    /// How many consecutive `GhProblem.variables` feed each `input_params` entry
    /// (1 for a slider, N for an N-gene pool). Sums to `variables.len()`; the
    /// evaluator uses it to slice a trial's flat value vector into per-input lists.
    pub input_value_counts: Vec<usize>,
    /// Output parameter names (`RH_OUT:objective_name`, in the same order as `GhProblem.objectives`)
    pub output_params: Vec<String>,
    /// Constraint output parameter names (`RH_OUT:constraint:name`, in the same
    /// order as `GhProblem.constraints`; the prefix keeps them from colliding
    /// with objective outputs of the same name)
    pub constraint_params: Vec<String>,
    /// Per-trial attribute output parameter names (`RH_OUT:attr:name`, in the
    /// same order as `GhProblem.attributes`)
    pub attr_params: Vec<String>,
}

/// Generates a Compute-ready definition from the original .ghx and the extracted problem definition.
pub fn build_compute_definition(
    xml: &str,
    problem: &GhProblem,
) -> Result<ComputeDefinition, String> {
    let anchors = locate_definition_objects(xml)?;

    // ── Generate the injected objects ───────────────────────────────
    let mut guid_counter: u64 = 1;
    let mut injected = String::new();
    let mut next_index = anchors.object_count;
    let mut output_params = Vec::with_capacity(problem.objectives.len());
    let mut constraint_params = Vec::with_capacity(problem.constraints.len());
    let mut attr_params = Vec::with_capacity(problem.attributes.len());

    // Group variables into RH_IN inputs. A Number Slider is its own single-value
    // input; a Gene Pool's genes (contiguous, all carrying the pool's
    // InstanceGuid) are delivered together as one RH_IN list input. See
    // `group_inputs`.
    let groups = group_inputs(&problem.variables);
    let mut input_params = Vec::with_capacity(groups.len());
    let mut input_value_counts = Vec::with_capacity(groups.len());
    for group in &groups {
        let nick = format!("RH_IN:{}", group.name);
        let group_guid = synthetic_guid(xml, &mut guid_counter);
        injected.push_str(&group_object_xml(
            next_index,
            &group_guid,
            &nick,
            group.member_guid,
        ));
        next_index += 1;
        input_params.push(nick);
        input_value_counts.push(group.count);
    }

    // All relay outputs (objectives, constraints, attributes) share the same
    // injection sequence: a relay Data parameter wired to the source, wrapped
    // in an RH_OUT group. An objective's source is often a component's output
    // parameter, which is not a document object and so cannot be placed
    // directly into a group; the relay makes this uniform (it works the same
    // way when the source is already a floating parameter).
    //
    // Kinds: 0 = objective, 1 = constraint, 2 = attribute. The prefixes keep
    // the namespaces apart for ordinary names; `uniquify_nicks` below makes
    // uniqueness structural even for adversarial names (e.g. an objective
    // literally nicknamed "constraint:x" alongside a constraint "x"), since a
    // duplicated ParamName would make the response mapping silently ambiguous.
    let mut relays: Vec<(String, &str, &str, usize)> = Vec::new();
    for obj in &problem.objectives {
        relays.push((
            format!("RH_OUT:{}", obj.name),
            &obj.name,
            &obj.source_guid,
            0,
        ));
    }
    for con in &problem.constraints {
        relays.push((
            format!("RH_OUT:constraint:{}", con.name),
            &con.name,
            &con.source_guid,
            1,
        ));
    }
    for attr in &problem.attributes {
        relays.push((
            format!("RH_OUT:attr:{}", attr.name),
            &attr.name,
            &attr.source_guid,
            2,
        ));
    }
    uniquify_nicks(&mut relays);

    for (nick, name, source_guid, kind) in relays {
        let relay_guid = synthetic_guid(xml, &mut guid_counter);
        injected.push_str(&relay_param_xml(next_index, &relay_guid, name, source_guid));
        next_index += 1;
        let group_guid = synthetic_guid(xml, &mut guid_counter);
        injected.push_str(&group_object_xml(
            next_index,
            &group_guid,
            &nick,
            &relay_guid,
        ));
        next_index += 1;
        match kind {
            0 => output_params.push(nick),
            1 => constraint_params.push(nick),
            _ => attr_params.push(nick),
        }
    }

    // ── 3 splices, in ascending position order: the ObjectCount value, the chunks
    //    count attribute, and the insertion at the end of the object list ───────────
    let new_count = next_index;
    let mut out = String::with_capacity(xml.len() + injected.len() + 64);
    out.push_str(&xml[..anchors.object_count_text.0]);
    out.push_str(&new_count.to_string());
    out.push_str(&xml[anchors.object_count_text.1..anchors.chunks_count_text.0]);
    out.push_str(&new_count.to_string());
    out.push_str(&xml[anchors.chunks_count_text.1..anchors.insertion_pos]);
    out.push_str(&injected);
    out.push_str(&xml[anchors.insertion_pos..]);

    Ok(ComputeDefinition {
        ghx: out,
        input_params,
        input_value_counts,
        output_params,
        constraint_params,
        attr_params,
    })
}

/// One RH_IN input group: a display name, how many consecutive variables it
/// consumes, and the InstanceGuid the injected group wraps.
struct InputGroup<'a> {
    name: String,
    count: usize,
    member_guid: &'a str,
}

/// Partitions the variables into RH_IN input groups (a slider → its own 1-value
/// group; a Gene Pool → one group carrying all its genes). A gene pool declares
/// its own size (`GenePoolSlot::count`); that size is bounded by the run of
/// contiguous variables actually sharing the pool's InstanceGuid, so a
/// truncated/edited variable list can never make a group over-read past its
/// genes. Group names come from the pool nickname for genes, or the variable
/// name for sliders; they are then made pairwise-unique (a pool nickname is not
/// part of the variable-name dedup, so it could otherwise collide).
fn group_inputs(variables: &[super::problem::GhVariable]) -> Vec<InputGroup<'_>> {
    let mut groups: Vec<InputGroup<'_>> = Vec::new();
    let mut i = 0;
    while i < variables.len() {
        let var = &variables[i];
        let want = var.gene_pool.as_ref().map_or(1, |slot| slot.count.max(1));
        let mut span = 1;
        while span < want
            && i + span < variables.len()
            && variables[i + span].instance_guid == var.instance_guid
        {
            span += 1;
        }
        let name = match &var.gene_pool {
            Some(slot) => slot.group_name.clone(),
            None => var.name.clone(),
        };
        groups.push(InputGroup {
            name,
            count: span,
            member_guid: &var.instance_guid,
        });
        i += span;
    }
    uniquify_input_names(&mut groups);
    groups
}

/// Makes RH_IN group names pairwise-unique by appending `_2`, `_3`, … to later
/// duplicates (mirrors `dedupe_names` / `uniquify_nicks`).
fn uniquify_input_names(groups: &mut [InputGroup<'_>]) {
    for i in 1..groups.len() {
        let mut suffix = 2;
        while groups[..i].iter().any(|g| g.name == groups[i].name) {
            let base = &groups[i].name;
            let trimmed = base
                .rfind('_')
                .filter(|&pos| base[pos + 1..].chars().all(|c| c.is_ascii_digit()))
                .map_or(base.as_str(), |pos| &base[..pos]);
            groups[i].name = format!("{trimmed}_{suffix}");
            suffix += 1;
        }
    }
}

/// Edit positions within the DefinitionObjects chunk.
struct Anchors {
    /// Existing object count
    object_count: usize,
    /// Byte range of the ObjectCount item's text
    object_count_text: (usize, usize),
    /// Byte range of the numeric text of the object list's `<chunks count="N">`
    chunks_count_text: (usize, usize),
    /// Position immediately before the object list's closing tag `</chunks>` (the insertion point)
    insertion_pos: usize,
}

/// Locates the DefinitionObjects edit anchors.
///
/// Tags and attributes follow the fixed, machine-generated format produced by GH_IO
/// (double-quoted attributes), and `<` `>` inside item text is XML-escaped, so tag
/// boundaries can be located safely with plain substring search.
fn locate_definition_objects(xml: &str) -> Result<Anchors, String> {
    let err = |msg: &str| format!("Unexpected .ghx structure: {msg}");

    let do_pos = xml
        .find(r#"name="DefinitionObjects""#)
        .ok_or_else(|| err("missing DefinitionObjects"))?;

    // Text range of the ObjectCount item
    let oc_tag = xml[do_pos..]
        .find(r#"<item name="ObjectCount""#)
        .map(|i| do_pos + i)
        .ok_or_else(|| err("missing ObjectCount"))?;
    let oc_text_start = xml[oc_tag..]
        .find('>')
        .map(|i| oc_tag + i + 1)
        .ok_or_else(|| err("unclosed ObjectCount start tag"))?;
    let oc_text_end = xml[oc_text_start..]
        .find("</item>")
        .map(|i| oc_text_start + i)
        .ok_or_else(|| err("unclosed ObjectCount item"))?;
    let object_count: usize = xml[oc_text_start..oc_text_end]
        .trim()
        .parse()
        .map_err(|_| err("ObjectCount is not a number"))?;

    // After the end of the <items> block containing ObjectCount, the first <chunks …>
    // is the container for the object list.
    let items_end = xml[oc_text_end..]
        .find("</items>")
        .map(|i| oc_text_end + i + "</items>".len())
        .ok_or_else(|| err("unclosed items block"))?;
    let chunks_open = xml[items_end..]
        .find("<chunks")
        .map(|i| items_end + i)
        .ok_or_else(|| err("missing object-list chunks"))?;
    let chunks_open_end = xml[chunks_open..]
        .find('>')
        .map(|i| chunks_open + i + 1)
        .ok_or_else(|| err("unclosed chunks start tag"))?;
    let count_attr = xml[chunks_open..chunks_open_end]
        .find(r#"count=""#)
        .map(|i| chunks_open + i + r#"count=""#.len())
        .ok_or_else(|| err("chunks has no count attribute"))?;
    let count_attr_end = xml[count_attr..chunks_open_end]
        .find('"')
        .map(|i| count_attr + i)
        .ok_or_else(|| err("unclosed count attribute"))?;

    // Find the matching </chunks> via a depth traversal (counting and canceling out
    // nested <chunks> inside Object chunks).
    let mut cursor = chunks_open_end;
    let mut depth = 1usize;
    let insertion_pos = loop {
        let next_open = xml[cursor..].find("<chunks").map(|i| cursor + i);
        let next_close = xml[cursor..].find("</chunks>").map(|i| cursor + i);
        match (next_open, next_close) {
            (Some(o), Some(c)) if o < c => {
                depth += 1;
                cursor = o + "<chunks".len();
            }
            (_, Some(c)) => {
                depth -= 1;
                if depth == 0 {
                    break c;
                }
                cursor = c + "</chunks>".len();
            }
            _ => return Err(err("unbalanced chunks")),
        }
    };

    Ok(Anchors {
        object_count,
        object_count_text: (oc_text_start, oc_text_end),
        chunks_count_text: (count_attr, count_attr_end),
        insertion_pos,
    })
}

/// Makes every relay nick unique across the union of the three output kinds by
/// appending `_2`, `_3`, … to later duplicates (matching `dedupe_names` on the
/// problem side). The final nick is what gets pushed into
/// `output_params`/`constraint_params`/`attr_params`, so response mapping by
/// ParamName stays unambiguous.
fn uniquify_nicks(relays: &mut [(String, &str, &str, usize)]) {
    for i in 1..relays.len() {
        let mut suffix = 2;
        while relays[..i].iter().any(|(n, ..)| *n == relays[i].0) {
            let base = &relays[i].0;
            let trimmed = base
                .rfind('_')
                .filter(|&pos| base[pos + 1..].chars().all(|c| c.is_ascii_digit()))
                .map_or(base.as_str(), |pos| &base[..pos]);
            relays[i].0 = format!("{trimmed}_{suffix}");
            suffix += 1;
        }
    }
}

/// Generates a synthetic GUID that does not collide with the existing XML (deterministic).
fn synthetic_guid(xml: &str, counter: &mut u64) -> String {
    loop {
        let guid = format!("7d0acade-0000-4000-8000-{:012x}", *counter);
        *counter += 1;
        if !xml.contains(&guid) {
            return guid;
        }
    }
}

/// Escapes XML text/attribute values.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// XML for a group object containing a single member.
///
/// The layout mirrors GH_Group's actual serialization observed in real .ghx
/// files: an alphabetical item list (Border / Colour / Description / ID /
/// ID_Count / InstanceGuid / Name / NickName) plus an empty Attributes chunk.
fn group_object_xml(
    index: usize,
    instance_guid: &str,
    nickname: &str,
    member_guid: &str,
) -> String {
    format!(
        r#"<chunk name="Object" index="{index}">
  <items count="2">
    <item name="GUID" type_name="gh_guid" type_code="9">{GROUP_TYPE_GUID}</item>
    <item name="Name" type_name="gh_string" type_code="10">Group</item>
  </items>
  <chunks count="1">
    <chunk name="Container">
      <items count="8">
        <item name="Border" type_name="gh_int32" type_code="3">1</item>
        <item name="Colour" type_name="gh_drawing_color" type_code="36">
          <ARGB>150;170;135;255</ARGB>
        </item>
        <item name="Description" type_name="gh_string" type_code="10">A group of Grasshopper objects</item>
        <item name="ID" index="0" type_name="gh_guid" type_code="9">{member_guid}</item>
        <item name="ID_Count" type_name="gh_int32" type_code="3">1</item>
        <item name="InstanceGuid" type_name="gh_guid" type_code="9">{instance_guid}</item>
        <item name="Name" type_name="gh_string" type_code="10">Group</item>
        <item name="NickName" type_name="gh_string" type_code="10">{nick}</item>
      </items>
      <chunks count="1">
        <chunk name="Attributes" />
      </chunks>
    </chunk>
  </chunks>
</chunk>
"#,
        nick = xml_escape(nickname),
    )
}

/// XML for the relay parameter that receives the value from an objective's source.
///
/// Uses the generic Data parameter (pass-through of any type). The layout
/// mirrors a floating Data parameter's actual serialization observed in real
/// .ghx files (Description / InstanceGuid / Name / NickName / Optional /
/// Source / SourceCount plus an Attributes chunk with Bounds and Pivot).
fn relay_param_xml(index: usize, instance_guid: &str, nickname: &str, source_guid: &str) -> String {
    format!(
        r#"<chunk name="Object" index="{index}">
  <items count="2">
    <item name="GUID" type_name="gh_guid" type_code="9">{DATA_PARAM_TYPE_GUID}</item>
    <item name="Name" type_name="gh_string" type_code="10">Data</item>
  </items>
  <chunks count="1">
    <chunk name="Container">
      <items count="7">
        <item name="Description" type_name="gh_string" type_code="10">Contains a collection of generic data</item>
        <item name="InstanceGuid" type_name="gh_guid" type_code="9">{instance_guid}</item>
        <item name="Name" type_name="gh_string" type_code="10">Data</item>
        <item name="NickName" type_name="gh_string" type_code="10">{nick}</item>
        <item name="Optional" type_name="gh_bool" type_code="1">false</item>
        <item name="Source" index="0" type_name="gh_guid" type_code="9">{source_guid}</item>
        <item name="SourceCount" type_name="gh_int32" type_code="3">1</item>
      </items>
      <chunks count="1">
        <chunk name="Attributes">
          <items count="2">
            <item name="Bounds" type_name="gh_drawing_rectanglef" type_code="35">
              <X>0</X>
              <Y>0</Y>
              <W>50</W>
              <H>20</H>
            </item>
            <item name="Pivot" type_name="gh_drawing_pointf" type_code="31">
              <X>0</X>
              <Y>0</Y>
            </item>
          </items>
        </chunk>
      </chunks>
    </chunk>
  </chunks>
</chunk>
"#,
        nick = xml_escape(nickname),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::fixtures::{sample_ghx, sample_ghx_gene_pool};
    use crate::gh::problem::{extract_problem, GhVariable};

    #[test]
    fn injects_groups_and_relays() {
        let xml = sample_ghx();
        let problem = extract_problem(&xml).unwrap();
        let def = build_compute_definition(&xml, &problem).unwrap();

        assert_eq!(def.input_params, vec!["RH_IN:span", "RH_IN:count"]);
        assert_eq!(def.output_params, vec!["RH_OUT:weight", "RH_OUT:disp"]);
        assert_eq!(def.constraint_params, vec!["RH_OUT:constraint:penalty"]);
        assert_eq!(def.attr_params, vec!["RH_OUT:attr:area"]);

        // Still well-formed after injection, and the object count is updated
        // (original 8 + 2 RH_IN groups + 2×2 relay+group per objective
        // + 1×2 for the constraint + 1×2 for the attribute = 18).
        let root = crate::gh::ghx::parse_archive(&def.ghx).unwrap();
        let objects = root.find_chunk_recursive("DefinitionObjects").unwrap();
        assert_eq!(objects.item_i64("ObjectCount"), Some(18));
        assert_eq!(objects.chunks_named("Object").count(), 18);
        assert!(def.ghx.contains(r#"<chunks count="18">"#));

        // The RH_IN group has the slider's InstanceGuid as its member
        let groups: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Group"))
            .collect();
        assert_eq!(groups.len(), 6);
        let rh_in_span = groups
            .iter()
            .map(|g| g.find_chunk("Container").unwrap())
            .find(|c| c.item_text("NickName") == Some("RH_IN:span"))
            .expect("RH_IN:span group");
        assert_eq!(
            rh_in_span.item_text("ID"),
            Some("0aaaaaaa-0000-0000-0000-0000000slid1")
        );

        // RH_OUT goes through a relay Data parameter, and the relay has the
        // objective's source as its Source
        let relays: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Data"))
            .map(|o| o.find_chunk("Container").unwrap())
            .filter(|c| c.item_text("NickName") == Some("weight"))
            .collect();
        assert_eq!(relays.len(), 1);
        assert_eq!(
            relays[0].item_text("Source"),
            Some("0aaaaaaa-0000-0000-0000-00000000beam")
        );
        let relay_guid = relays[0].item_text("InstanceGuid").unwrap();
        let rh_out_weight = groups
            .iter()
            .map(|g| g.find_chunk("Container").unwrap())
            .find(|c| c.item_text("NickName") == Some("RH_OUT:weight"))
            .expect("RH_OUT:weight group");
        assert_eq!(rh_out_weight.item_text("ID"), Some(relay_guid));

        // The constraint relay receives from the constraint's source parameter,
        // and its group carries the prefixed RH_OUT name
        let con_relays: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Data"))
            .map(|o| o.find_chunk("Container").unwrap())
            .filter(|c| c.item_text("NickName") == Some("penalty"))
            .collect();
        assert_eq!(con_relays.len(), 1);
        assert_eq!(
            con_relays[0].item_text("Source"),
            Some("0aaaaaaa-0000-0000-0000-00000000pena")
        );
        let con_relay_guid = con_relays[0].item_text("InstanceGuid").unwrap();
        let rh_out_con = groups
            .iter()
            .map(|g| g.find_chunk("Container").unwrap())
            .find(|c| c.item_text("NickName") == Some("RH_OUT:constraint:penalty"))
            .expect("RH_OUT:constraint:penalty group");
        assert_eq!(rh_out_con.item_text("ID"), Some(con_relay_guid));

        // The attribute relay receives from the attribute's source parameter,
        // and its group carries the prefixed RH_OUT name
        let attr_relays: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Data"))
            .map(|o| o.find_chunk("Container").unwrap())
            .filter(|c| c.item_text("NickName") == Some("area"))
            .collect();
        assert_eq!(attr_relays.len(), 1);
        assert_eq!(
            attr_relays[0].item_text("Source"),
            Some("0aaaaaaa-0000-0000-0000-00000000area")
        );
        let attr_relay_guid = attr_relays[0].item_text("InstanceGuid").unwrap();
        let rh_out_attr = groups
            .iter()
            .map(|g| g.find_chunk("Container").unwrap())
            .find(|c| c.item_text("NickName") == Some("RH_OUT:attr:area"))
            .expect("RH_OUT:attr:area group");
        assert_eq!(rh_out_attr.item_text("ID"), Some(attr_relay_guid));

        // The original definition body (Tunny component, etc.) is preserved
        assert!(def.ghx.contains("Tunny"));
        assert!(def.ghx.contains("Beam Analyzer"));
    }

    #[test]
    fn gene_pool_is_a_single_grouped_input() {
        let xml = sample_ghx_gene_pool();
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.variables.len(), 3);

        let def = build_compute_definition(&xml, &problem).unwrap();
        // The 3 genes collapse to ONE RH_IN input that carries all 3 values.
        assert_eq!(def.input_params, vec!["RH_IN:Genes"]);
        assert_eq!(def.input_value_counts, vec![3]);
        assert_eq!(def.output_params, vec!["RH_OUT:obj"]);

        // Exactly one RH_IN group, wrapping the pool's InstanceGuid (not each gene).
        let root = crate::gh::ghx::parse_archive(&def.ghx).unwrap();
        let objects = root.find_chunk_recursive("DefinitionObjects").unwrap();
        let rh_in: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Group"))
            .map(|o| o.find_chunk("Container").unwrap())
            .filter(|c| {
                c.item_text("NickName")
                    .is_some_and(|n| n.starts_with("RH_IN:"))
            })
            .collect();
        assert_eq!(rh_in.len(), 1);
        assert_eq!(rh_in[0].item_text("NickName"), Some("RH_IN:Genes"));
        assert_eq!(
            rh_in[0].item_text("ID"),
            Some("0aaaaaaa-0000-0000-0000-00000000pool")
        );
    }

    #[test]
    fn mixed_slider_and_gene_pool_inputs_keep_order_and_counts() {
        // A slider variable followed by a 2-gene pool: two RH_IN inputs with
        // value counts [1, 2], in variable order.
        let mut problem = extract_problem(&sample_ghx()).unwrap();
        problem.variables.truncate(1); // keep only the "span" slider
        problem.variables.push(GhVariable {
            instance_guid: "pool-guid".to_string(),
            name: "g0".to_string(),
            low: 0.0,
            high: 1.0,
            value: 0.5,
            digits: 2,
            is_integer: false,
            gene_pool: Some(crate::gh::GenePoolSlot {
                group_name: "pool".to_string(),
                count: 2,
                index: 0,
            }),
        });
        problem.variables.push(GhVariable {
            instance_guid: "pool-guid".to_string(),
            name: "g1".to_string(),
            low: 0.0,
            high: 1.0,
            value: 0.5,
            digits: 2,
            is_integer: false,
            gene_pool: Some(crate::gh::GenePoolSlot {
                group_name: "pool".to_string(),
                count: 2,
                index: 1,
            }),
        });
        let def = build_compute_definition(&sample_ghx(), &problem).unwrap();
        assert_eq!(def.input_params, vec!["RH_IN:span", "RH_IN:pool"]);
        assert_eq!(def.input_value_counts, vec![1, 2]);
    }

    /// An objective whose nickname embeds the constraint prefix must not
    /// produce the same RH_OUT ParamName as a real constraint — the response
    /// mapping would silently read the wrong value for one of them.
    #[test]
    fn colliding_output_nicks_are_uniquified() {
        let xml = sample_ghx();
        let mut problem = extract_problem(&xml).unwrap();
        problem.objectives[0].name = "constraint:penalty".to_string();
        // The fixture's constraint is named "penalty" → prefixed nick collides
        // with the objective's.
        let def = build_compute_definition(&xml, &problem).unwrap();
        assert_eq!(def.output_params[0], "RH_OUT:constraint:penalty");
        assert_eq!(def.constraint_params, vec!["RH_OUT:constraint:penalty_2"]);
        // All output nicks are pairwise distinct
        let mut all: Vec<&String> = def
            .output_params
            .iter()
            .chain(&def.constraint_params)
            .chain(&def.attr_params)
            .collect();
        all.sort();
        let len_before = all.len();
        all.dedup();
        assert_eq!(all.len(), len_before);
    }

    #[test]
    fn escapes_names_in_injected_xml() {
        let xml = sample_ghx().replace(
            r#"<item name="NickName" type_name="gh_string" type_code="10">span</item>"#,
            r#"<item name="NickName" type_name="gh_string" type_code="10">a&amp;b</item>"#,
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.variables[0].name, "a&b");
        let def = build_compute_definition(&xml, &problem).unwrap();
        // Correctly escaped inside the injected XML too, and re-parseable
        assert!(def.ghx.contains("RH_IN:a&amp;b"));
        assert!(crate::gh::ghx::parse_archive(&def.ghx).is_ok());
        assert_eq!(def.input_params[0], "RH_IN:a&b");
    }
}
