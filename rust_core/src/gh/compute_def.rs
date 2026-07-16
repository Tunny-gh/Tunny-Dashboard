//! Rhino.Compute で solve 可能な定義（RH_IN / RH_OUT 付き .ghx）の生成。
//!
//! Compute の Grasshopper エンドポイントは、`RH_IN:名前` のニックネームを持つ
//! グループに入力値を割り当て、`RH_OUT:名前` グループ内パラメータの値を応答に
//! 含める。ここでは元の .ghx に対して:
//!
//! - 各変数スライダーを包む `RH_IN:変数名` グループ
//! - 各目的の接続元に配線したリレー Number パラメータ + それを包む
//!   `RH_OUT:目的名` グループ
//!
//! を XML 文字列操作で注入する。元の定義本体はバイト単位で保持し、
//! DefinitionObjects のオブジェクト数と末尾への追加のみを行う。
//!
//! 注意: 注入するオブジェクトの型 GUID（Group / Param_Number）と最小
//! シリアライズ形式は実環境の Rhino.Compute での E2E 検証が必要
//! （ROADMAP 項目 15 の後段タスク）。

use super::problem::GhProblem;

/// Grasshopper の Group オブジェクトの型 GUID。
const GROUP_TYPE_GUID: &str = "c552a431-af5b-46a9-a8a4-0fcbc27ef596";
/// フローティング Number パラメータ（Param_Number）の型 GUID。
const PARAM_NUMBER_TYPE_GUID: &str = "3e8ca6be-fda8-4aaf-b5c0-3c54c8bb7312";

/// RH_IN / RH_OUT 注入済みの Compute 用定義。
#[derive(Debug, Clone)]
pub struct ComputeDefinition {
    /// 注入済み .ghx テキスト
    pub ghx: String,
    /// 入力パラメータ名（`RH_IN:変数名`、`GhProblem.variables` と同順）
    pub input_params: Vec<String>,
    /// 出力パラメータ名（`RH_OUT:目的名`、`GhProblem.objectives` と同順）
    pub output_params: Vec<String>,
}

/// 元の .ghx と抽出済み問題定義から Compute 用定義を生成する。
pub fn build_compute_definition(
    xml: &str,
    problem: &GhProblem,
) -> Result<ComputeDefinition, String> {
    let anchors = locate_definition_objects(xml)?;

    // ── 注入オブジェクトの生成 ──────────────────────────────────────
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
        // 目的の接続元はコンポーネントの出力パラメータであることが多く、
        // ドキュメントオブジェクトではないためグループに直接入れられない。
        // 常にリレー Number パラメータを新設して接続元から受け、リレーを
        // グループに入れる（接続元がフローティングパラメータでも同じ経路で
        // 動くため分岐しない）。
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

    // ── 3 箇所のスプライス（位置昇順）: ObjectCount 値・chunks count 属性・
    //    オブジェクト列末尾への挿入 ────────────────────────────────────
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

/// DefinitionObjects チャンク内の編集位置。
struct Anchors {
    /// 既存のオブジェクト数
    object_count: usize,
    /// ObjectCount item のテキスト範囲（バイト）
    object_count_text: (usize, usize),
    /// オブジェクト列 `<chunks count="N">` の数値テキスト範囲（バイト）
    chunks_count_text: (usize, usize),
    /// オブジェクト列の閉じタグ `</chunks>` の直前位置（挿入点）
    insertion_pos: usize,
}

/// DefinitionObjects の編集アンカーを特定する。
///
/// タグ・属性は GH_IO が機械生成する固定形式（ダブルクォート属性）で、
/// item テキスト内の `<` `>` は XML エスケープされるため、素朴な部分文字列
/// 探索でタグ境界を安全に特定できる。
fn locate_definition_objects(xml: &str) -> Result<Anchors, String> {
    let err = |msg: &str| format!(".ghx の構造が想定と異なります: {msg}");

    let do_pos = xml
        .find(r#"name="DefinitionObjects""#)
        .ok_or_else(|| err("DefinitionObjects がありません"))?;

    // ObjectCount item のテキスト範囲
    let oc_tag = xml[do_pos..]
        .find(r#"<item name="ObjectCount""#)
        .map(|i| do_pos + i)
        .ok_or_else(|| err("ObjectCount がありません"))?;
    let oc_text_start = xml[oc_tag..]
        .find('>')
        .map(|i| oc_tag + i + 1)
        .ok_or_else(|| err("ObjectCount の開始タグが閉じていません"))?;
    let oc_text_end = xml[oc_text_start..]
        .find("</item>")
        .map(|i| oc_text_start + i)
        .ok_or_else(|| err("ObjectCount が閉じていません"))?;
    let object_count: usize = xml[oc_text_start..oc_text_end]
        .trim()
        .parse()
        .map_err(|_| err("ObjectCount が数値ではありません"))?;

    // ObjectCount を含む <items> ブロックの終端後、最初の <chunks …> が
    // オブジェクト列のコンテナ。
    let items_end = xml[oc_text_end..]
        .find("</items>")
        .map(|i| oc_text_end + i + "</items>".len())
        .ok_or_else(|| err("items ブロックが閉じていません"))?;
    let chunks_open = xml[items_end..]
        .find("<chunks")
        .map(|i| items_end + i)
        .ok_or_else(|| err("オブジェクト列の chunks がありません"))?;
    let chunks_open_end = xml[chunks_open..]
        .find('>')
        .map(|i| chunks_open + i + 1)
        .ok_or_else(|| err("chunks の開始タグが閉じていません"))?;
    let count_attr = xml[chunks_open..chunks_open_end]
        .find(r#"count=""#)
        .map(|i| chunks_open + i + r#"count=""#.len())
        .ok_or_else(|| err("chunks に count 属性がありません"))?;
    let count_attr_end = xml[count_attr..chunks_open_end]
        .find('"')
        .map(|i| count_attr + i)
        .ok_or_else(|| err("count 属性が閉じていません"))?;

    // 対応する </chunks> を深さ走査で探す（Object チャンク内の入れ子の
    // <chunks> を数えて相殺する）。
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
            _ => return Err(err("chunks の対応が取れません")),
        }
    };

    Ok(Anchors {
        object_count,
        object_count_text: (oc_text_start, oc_text_end),
        chunks_count_text: (count_attr, count_attr_end),
        insertion_pos,
    })
}

/// 既存 XML と衝突しない合成 GUID を生成する（決定論的）。
fn synthetic_guid(xml: &str, counter: &mut u64) -> String {
    loop {
        let guid = format!("7d0acade-0000-4000-8000-{:012x}", *counter);
        *counter += 1;
        if !xml.contains(&guid) {
            return guid;
        }
    }
}

/// XML テキスト/属性値のエスケープ。
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

/// メンバー 1 つを含むグループオブジェクトの XML。
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
      <items count="5">
        <item name="ID" index="0" type_name="gh_guid" type_code="9">{member_guid}</item>
        <item name="IDCount" type_name="gh_int32" type_code="3">1</item>
        <item name="InstanceGuid" type_name="gh_guid" type_code="9">{instance_guid}</item>
        <item name="Name" type_name="gh_string" type_code="10">Group</item>
        <item name="NickName" type_name="gh_string" type_code="10">{nick}</item>
      </items>
    </chunk>
  </chunks>
</chunk>
"#,
        nick = xml_escape(nickname),
    )
}

/// 目的の接続元から値を受けるリレー Number パラメータの XML。
fn relay_param_xml(index: usize, instance_guid: &str, nickname: &str, source_guid: &str) -> String {
    format!(
        r#"<chunk name="Object" index="{index}">
  <items count="2">
    <item name="GUID" type_name="gh_guid" type_code="9">{PARAM_NUMBER_TYPE_GUID}</item>
    <item name="Name" type_name="gh_string" type_code="10">Number</item>
  </items>
  <chunks count="1">
    <chunk name="Container">
      <items count="6">
        <item name="InstanceGuid" type_name="gh_guid" type_code="9">{instance_guid}</item>
        <item name="Name" type_name="gh_string" type_code="10">Number</item>
        <item name="NickName" type_name="gh_string" type_code="10">{nick}</item>
        <item name="Optional" type_name="gh_bool" type_code="1">true</item>
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

        // 注入後も整形式で、オブジェクト数が更新されている
        // （元 5 + RH_IN グループ 2 + 目的ごとにリレー+グループ 2×2 = 11）。
        let root = crate::gh::ghx::parse_archive(&def.ghx).unwrap();
        let objects = root.find_chunk_recursive("DefinitionObjects").unwrap();
        assert_eq!(objects.item_i64("ObjectCount"), Some(11));
        assert_eq!(objects.chunks_named("Object").count(), 11);
        assert!(def.ghx.contains(r#"<chunks count="11">"#));

        // RH_IN グループがスライダーの InstanceGuid をメンバーに持つ
        let groups: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Group"))
            .collect();
        assert_eq!(groups.len(), 4);
        let rh_in_span = groups
            .iter()
            .map(|g| g.find_chunk("Container").unwrap())
            .find(|c| c.item_text("NickName") == Some("RH_IN:span"))
            .expect("RH_IN:span グループ");
        assert_eq!(
            rh_in_span.item_text("ID"),
            Some("0aaaaaaa-0000-0000-0000-0000000slid1")
        );

        // RH_OUT はリレー Number パラメータを経由し、リレーが目的の接続元を
        // Source に持つ
        let relays: Vec<_> = objects
            .chunks_named("Object")
            .filter(|o| o.item_text("Name") == Some("Number"))
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
            .expect("RH_OUT:weight グループ");
        assert_eq!(rh_out_weight.item_text("ID"), Some(relay_guid));

        // 元の定義本体（Tunny コンポーネント等）は保持される
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
        // 注入 XML 内でも正しくエスケープされ、再パース可能
        assert!(def.ghx.contains("RH_IN:a&amp;b"));
        assert!(crate::gh::ghx::parse_archive(&def.ghx).is_ok());
        assert_eq!(def.input_params[0], "RH_IN:a&b");
    }
}
