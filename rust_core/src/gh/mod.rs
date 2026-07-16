//! Grasshopper（.ghx）連携。
//!
//! Tunny で最適化を構成した Grasshopper 定義（.ghx）から問題定義を抽出し、
//! Rhino.Compute で実目的関数を評価しながら最適化を実行、試行を Optuna 互換
//! journal に書き込む（ROADMAP フェーズ 2B・項目 15）。
//!
//! パイプライン:
//! 1. `problem::extract_problem` — .ghx から変数（スライダー）と目的を抽出
//! 2. `compute_def::build_compute_definition` — RH_IN / RH_OUT を注入した
//!    Compute 用定義を生成
//! 3. `compute::ComputeEvaluator` — rhino.compute の /grasshopper で 1 試行を評価
//! 4. `runner` — サンプラー（Random / NSGA-II）で最適化ループを回し、
//!    journal に試行を記録（既存のライブ更新・全分析機能がそのまま効く）

mod ghx;

pub mod compute;
pub mod compute_def;
pub mod problem;
pub mod runner;

pub use compute::{ComputeConfig, ComputeEvaluator, GhEvaluator};
pub use compute_def::{build_compute_definition, ComputeDefinition};
pub use problem::{extract_problem, GhObjective, GhProblem, GhVariable};
pub use runner::{
    prepare_gh_run, run_prepared, GhRunConfig, GhRunSummary, GhSampler, PreparedGhRun,
};

/// テスト用の合成 .ghx フィクスチャ。
///
/// 実ファイルの構造（Definition → DefinitionObjects → Object → Container …）を
/// 最小限で再現する: スライダー 2 本（span / count）、出力パラメータ weight を
/// 持つコンポーネント（Beam）、フローティング Number パラメータ disp、
/// それらを Variables / Objectives 入力で受ける Tunny コンポーネント。
#[cfg(test)]
pub(crate) mod fixtures {
    pub fn sample_ghx() -> String {
        r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<Archive name="Root">
  <!--Grasshopper archive-->
  <items count="1">
    <item name="ArchiveVersion" type_name="gh_version" type_code="80">
      <Major>0</Major>
      <Minor>2</Minor>
      <Revision>2</Revision>
    </item>
  </items>
  <chunks count="1">
    <chunk name="Definition">
      <chunks count="2">
        <chunk name="DefinitionHeader">
          <items count="1">
            <item name="Plugin Version" type_name="gh_version" type_code="80">
              <Major>1</Major>
              <Minor>0</Minor>
              <Revision>7</Revision>
            </item>
          </items>
        </chunk>
        <chunk name="DefinitionObjects">
          <items count="1">
            <item name="ObjectCount" type_name="gh_int32" type_code="3">5</item>
          </items>
          <chunks count="5">
            <chunk name="Object" index="0">
              <items count="2">
                <item name="GUID" type_name="gh_guid" type_code="9">57da07bd-ecab-415d-9d86-af36d7073abc</item>
                <item name="Name" type_name="gh_string" type_code="10">Number Slider</item>
              </items>
              <chunks count="1">
                <chunk name="Container">
                  <items count="4">
                    <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-0000000slid1</item>
                    <item name="Name" type_name="gh_string" type_code="10">Number Slider</item>
                    <item name="NickName" type_name="gh_string" type_code="10">span</item>
                    <item name="Optional" type_name="gh_bool" type_code="1">false</item>
                  </items>
                  <chunks count="1">
                    <chunk name="Slider">
                      <items count="4">
                        <item name="Digits" type_name="gh_int32" type_code="3">2</item>
                        <item name="Max" type_name="gh_double" type_code="6">12</item>
                        <item name="Min" type_name="gh_double" type_code="6">3</item>
                        <item name="Value" type_name="gh_double" type_code="6">5.5</item>
                      </items>
                    </chunk>
                  </chunks>
                </chunk>
              </chunks>
            </chunk>
            <chunk name="Object" index="1">
              <items count="2">
                <item name="GUID" type_name="gh_guid" type_code="9">57da07bd-ecab-415d-9d86-af36d7073abc</item>
                <item name="Name" type_name="gh_string" type_code="10">Number Slider</item>
              </items>
              <chunks count="1">
                <chunk name="Container">
                  <items count="4">
                    <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-0000000slid2</item>
                    <item name="Name" type_name="gh_string" type_code="10">Number Slider</item>
                    <item name="NickName" type_name="gh_string" type_code="10">count</item>
                    <item name="Optional" type_name="gh_bool" type_code="1">false</item>
                  </items>
                  <chunks count="1">
                    <chunk name="Slider">
                      <items count="4">
                        <item name="Digits" type_name="gh_int32" type_code="3">0</item>
                        <item name="Max" type_name="gh_double" type_code="6">10</item>
                        <item name="Min" type_name="gh_double" type_code="6">1</item>
                        <item name="Value" type_name="gh_double" type_code="6">3</item>
                      </items>
                    </chunk>
                  </chunks>
                </chunk>
              </chunks>
            </chunk>
            <chunk name="Object" index="2">
              <items count="2">
                <item name="GUID" type_name="gh_guid" type_code="9">11111111-2222-3333-4444-555555555555</item>
                <item name="Name" type_name="gh_string" type_code="10">Beam Analyzer</item>
              </items>
              <chunks count="1">
                <chunk name="Container">
                  <items count="3">
                    <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-000000000cmp</item>
                    <item name="Name" type_name="gh_string" type_code="10">Beam Analyzer</item>
                    <item name="NickName" type_name="gh_string" type_code="10">Beam</item>
                  </items>
                  <chunks count="2">
                    <chunk name="param_input" index="0">
                      <items count="4">
                        <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000bin0</item>
                        <item name="Name" type_name="gh_string" type_code="10">Length</item>
                        <item name="NickName" type_name="gh_string" type_code="10">L</item>
                        <item name="SourceCount" type_name="gh_int32" type_code="3">0</item>
                      </items>
                    </chunk>
                    <chunk name="param_output" index="0">
                      <items count="4">
                        <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000beam</item>
                        <item name="Name" type_name="gh_string" type_code="10">Weight</item>
                        <item name="NickName" type_name="gh_string" type_code="10">weight</item>
                        <item name="SourceCount" type_name="gh_int32" type_code="3">0</item>
                      </items>
                    </chunk>
                  </chunks>
                </chunk>
              </chunks>
            </chunk>
            <chunk name="Object" index="3">
              <items count="2">
                <item name="GUID" type_name="gh_guid" type_code="9">3e8ca6be-fda8-4aaf-b5c0-3c54c8bb7312</item>
                <item name="Name" type_name="gh_string" type_code="10">Number</item>
              </items>
              <chunks count="1">
                <chunk name="Container">
                  <items count="6">
                    <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000disp</item>
                    <item name="Name" type_name="gh_string" type_code="10">Number</item>
                    <item name="NickName" type_name="gh_string" type_code="10">disp</item>
                    <item name="Optional" type_name="gh_bool" type_code="1">true</item>
                    <item name="Source" index="0" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000bem2</item>
                    <item name="SourceCount" type_name="gh_int32" type_code="3">1</item>
                  </items>
                </chunk>
              </chunks>
            </chunk>
            <chunk name="Object" index="4">
              <items count="2">
                <item name="GUID" type_name="gh_guid" type_code="9">99999999-8888-7777-6666-555555555555</item>
                <item name="Name" type_name="gh_string" type_code="10">Tunny</item>
              </items>
              <chunks count="1">
                <chunk name="Container">
                  <items count="3">
                    <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-0000000tunny</item>
                    <item name="Name" type_name="gh_string" type_code="10">Tunny</item>
                    <item name="NickName" type_name="gh_string" type_code="10">Tunny</item>
                  </items>
                  <chunks count="3">
                    <chunk name="param_input" index="0">
                      <items count="6">
                        <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000tin0</item>
                        <item name="Name" type_name="gh_string" type_code="10">Variables</item>
                        <item name="NickName" type_name="gh_string" type_code="10">V</item>
                        <item name="Source" index="0" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-0000000slid1</item>
                        <item name="Source" index="1" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-0000000slid2</item>
                        <item name="SourceCount" type_name="gh_int32" type_code="3">2</item>
                      </items>
                    </chunk>
                    <chunk name="param_input" index="1">
                      <items count="6">
                        <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000tin1</item>
                        <item name="Name" type_name="gh_string" type_code="10">Objectives</item>
                        <item name="NickName" type_name="gh_string" type_code="10">O</item>
                        <item name="Source" index="0" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000beam</item>
                        <item name="Source" index="1" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-00000000disp</item>
                        <item name="SourceCount" type_name="gh_int32" type_code="3">2</item>
                      </items>
                    </chunk>
                    <chunk name="param_output" index="0">
                      <items count="4">
                        <item name="InstanceGuid" type_name="gh_guid" type_code="9">0aaaaaaa-0000-0000-0000-0000000tout0</item>
                        <item name="Name" type_name="gh_string" type_code="10">Trials</item>
                        <item name="NickName" type_name="gh_string" type_code="10">T</item>
                        <item name="SourceCount" type_name="gh_int32" type_code="3">0</item>
                      </items>
                    </chunk>
                  </chunks>
                </chunk>
              </chunks>
            </chunk>
          </chunks>
        </chunk>
      </chunks>
    </chunk>
  </chunks>
</Archive>"#
            .to_string()
    }
}
