//! .ghx から最適化問題の定義を抽出する。
//!
//! 抽出規則:
//! - 定義内の Tunny コンポーネント（オブジェクト名に "tunny" を含む）を探す
//! - その Variables 入力に接続された Number Slider 群 → 変数（名前・範囲・桁数）
//! - その Objectives 入力に接続されたパラメータ群 → 目的（名前と接続元 GUID）
//!
//! 目的の方向（minimize/maximize）は Tunny コンポーネントの内部設定の
//! シリアライズ形式が固定でないため ghx からは読まず、既定 Minimize として
//! UI 側で編集させる。

use super::ghx::{parse_archive, GhxChunk};

/// 最適化変数（Number Slider 由来）。
#[derive(Debug, Clone)]
pub struct GhVariable {
    /// スライダーの InstanceGuid（RH_IN グループ注入で使う）
    pub instance_guid: String,
    /// journal の param 名になる（スライダーの NickName、重複時は連番付与）
    pub name: String,
    pub low: f64,
    pub high: f64,
    /// 定義保存時点のスライダー値
    pub value: f64,
    /// 小数桁数（スライダーの丸め。評価値もこの桁に丸める）
    pub digits: u32,
    /// 整数スライダーか（digits == 0 を整数とみなす）
    pub is_integer: bool,
}

/// 最適化目的（Tunny の Objectives 入力に接続されたパラメータ）。
#[derive(Debug, Clone)]
pub struct GhObjective {
    /// 接続元パラメータの InstanceGuid（RH_OUT 用リレー注入で使う）
    pub source_guid: String,
    /// 目的名（接続元パラメータの NickName、重複時は連番付与)
    pub name: String,
}

/// .ghx から抽出した最適化問題の中間表現。
/// 将来 .gh + マニフェスト経路が入っても同じ型に合流させる（ROADMAP 項目 15）。
#[derive(Debug, Clone)]
pub struct GhProblem {
    pub variables: Vec<GhVariable>,
    pub objectives: Vec<GhObjective>,
    /// 検出した Tunny コンポーネントの表示名
    pub tunny_component: String,
    /// 抽出時に無視した接続などの注意事項（UI に表示する）
    pub warnings: Vec<String>,
}

/// 定義内オブジェクトの中間情報。
struct ObjectRecord<'a> {
    /// オブジェクト型名（"Number Slider" / "Group" など。Object 直下の Name item）
    type_name: &'a str,
    container: &'a GhxChunk,
    instance_guid: &'a str,
    nickname: String,
}

/// パラメータ GUID → 表示名 の索引エントリ。
struct ParamEntry {
    nickname: String,
}

/// .ghx テキストから最適化問題を抽出する。
pub fn extract_problem(xml: &str) -> Result<GhProblem, String> {
    let root = parse_archive(xml)?;
    let objects_chunk = root
        .find_chunk_recursive("DefinitionObjects")
        .ok_or_else(|| {
            "DefinitionObjects が見つかりません。Grasshopper の .ghx ファイルか確認してください"
                .to_string()
        })?;

    // 全オブジェクトの走査と索引作り
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

    // パラメータ GUID 索引: フローティングパラメータ自身と、
    // コンポーネントの入出力パラメータ（param_input / param_output）を登録する。
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

    // Tunny コンポーネント検出
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

    // Variables 入力 → スライダー解決
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

    // Objectives 入力 → 接続元パラメータの名前解決
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

/// コンポーネント Container 配下の入出力パラメータ chunk（param_input /
/// param_output）を再帰的に集める。GH のバージョンによりネスト位置が異なる
/// ことがあるため、直下に限定しない。
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

/// Tunny コンポーネントの指定入力（名前の部分一致 or ニックネーム完全一致）に
/// 接続された Source GUID を文書順で返す。
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

/// スライダーオブジェクトから変数情報を読む。
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

/// 名前の重複に連番サフィックスを付けて一意化する（journal の param 名 /
/// 目的名は一意である必要がある）。
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
        // コンポーネント出力パラメータ経由の目的
        assert_eq!(problem.objectives[0].name, "weight");
        assert_eq!(
            problem.objectives[0].source_guid,
            "0aaaaaaa-0000-0000-0000-00000000beam"
        );
        // フローティングパラメータ経由の目的
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
        // フィクスチャの count スライダーの NickName を span に変えて重複させる
        let xml = sample_ghx().replace(
            r#"<item name="NickName" type_name="gh_string" type_code="10">count</item>"#,
            r#"<item name="NickName" type_name="gh_string" type_code="10">span</item>"#,
        );
        let problem = extract_problem(&xml).unwrap();
        assert_eq!(problem.variables[0].name, "span");
        assert_eq!(problem.variables[1].name, "span_2");
    }
}
