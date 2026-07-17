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
    /// Input parameter names (`RH_IN:variable_name`, in the same order as `GhProblem.variables`)
    pub input_params: Vec<String>,
    /// Output parameter names (`RH_OUT:objective_name`, in the same order as `GhProblem.objectives`)
    pub output_params: Vec<String>,
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
    let mut input_params = Vec::with_capacity(problem.variables.len());
    let mut output_params = Vec::with_capacity(problem.objectives.len());

    for var in &problem.variables {
        let nick = format!("RH_IN:{}", var.name);
        let group_guid = synthetic_guid(xml, &mut guid_counter);
        injected.push_str(&group_object_xml(
            next_index,
            &group_guid,
            &nick,
            &var.instance_guid,
        ));
        next_index += 1;
        input_params.push(nick);
    }

    for obj in &problem.objectives {
        // An objective's source is often a component's output parameter, which is not a
        // document object and so cannot be placed directly into a group. We always create
        // a new relay Number parameter to receive from the source, and put the relay into
        // the group instead (this works the same way even when the source is already a
        // floating parameter, so no branching is needed).
        let nick = format!("RH_OUT:{}", obj.name);
        let relay_guid = synthetic_guid(xml, &mut guid_counter);
        injected.push_str(&relay_param_xml(
            next_index,
            &relay_guid,
            &obj.name,
            &obj.source_guid,
        ));
        next_index += 1;
        let group_guid = synthetic_guid(xml, &mut guid_counter);
        injected.push_str(&group_object_xml(
            next_index,
            &group_guid,
            &nick,
            &relay_guid,
        ));
        next_index += 1;
        output_params.push(nick);
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
        output_params,
    })
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
    use crate::gh::fixtures::sample_ghx;
    use crate::gh::problem::extract_problem;

    #[test]
    fn injects_groups_and_relays() {
        let xml = sample_ghx();
        let problem = extract_problem(&xml).unwrap();
        let def = build_compute_definition(&xml, &problem).unwrap();

        assert_eq!(def.input_params, vec!["RH_IN:span", "RH_IN:count"]);
        assert_eq!(def.output_params, vec!["RH_OUT:weight", "RH_OUT:disp"]);

        // Still well-formed after injection, and the object count is updated
        // (original 5 + 2 RH_IN groups + 2×2 relay+group per objective = 11).
        let root = crate::gh::ghx::parse_archive(&def.ghx).unwrap();
        let objects = root.find_chunk_recursive("DefinitionObjects").unwrap();
        assert_eq!(objects.item_i64("ObjectCount"), Some(11));
        assert_eq!(objects.chunks_named("Object").count(), 11);
        assert!(def.ghx.contains(r#"<chunks count="11">"#));

        // The RH_IN group has the slider's InstanceGuid as its member
        let groups: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Group"))
            .collect();
        assert_eq!(groups.len(), 4);
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

        // The original definition body (Tunny component, etc.) is preserved
        assert!(def.ghx.contains("Tunny"));
        assert!(def.ghx.contains("Beam Analyzer"));
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
