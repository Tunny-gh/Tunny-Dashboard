# TASK-1615 テストケース: TOPSISアルゴリズム

**機能名**: topsis
**タスクID**: TASK-1615
**要件名**: mode-frontier-features
**作成日**: 2026-04-04

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 設計文書・要件定義書より直接導出
- 🟡 **黄信号**: 設計文書から妥当な推測
- 🔴 **赤信号**: 設計文書にない推測

---

## 4. 開発言語・フレームワーク

- **プログラミング言語**: Rust (edition 2021)
  - 言語選択の理由: プロジェクト既存の WASM コアが全て Rust で実装されている
  - テストに適した機能: `#[test]` 属性・`assert!`/`assert_eq!` マクロ・cargo test による高速並列実行
- **テストフレームワーク**: 標準Rustテスト（`#[cfg(test)] mod tests`）
  - フレームワーク選択の理由: 既存の pareto.rs / sensitivity.rs が全て標準テストを使用
  - テスト実行コマンド: `cd rust_core && cargo test topsis`
- **テストファイル位置**: `rust_core/src/topsis.rs` 末尾の `#[cfg(test)] mod tests` ブロック内
- 🔵 参照: `docs/implements/mode-frontier-features/TASK-1615/note.md` §5, `rust_core/src/pareto.rs`

---

## 1. 正常系テストケース

### TC-1615-01: 3試行×2目的・minimize両方の基本動作

- **テスト名**: `tc_1615_01_basic_two_obj_minimize`
- **何をテストするか**: minimize方向の2目的に対して、スコアが0〜1の範囲に収まり、ranked_indicesがスコア降順になることを確認する
- **期待される動作**: compute_topsis が Ok(TopsisResult) を返し、スコアが数学的に正しい順序になる
- **入力値**:
  ```rust
  // 試行0: (1.0, 4.0) → obj0小・obj1大（obj0優位）
  // 試行1: (4.0, 1.0) → obj0大・obj1小（obj1優位）
  // 試行2: (2.0, 2.0) → 中間
  values = [1.0, 4.0, 4.0, 1.0, 2.0, 2.0]
  weights = [0.5, 0.5]
  is_minimize = [true, true]
  ```
- **期待される結果**:
  - `scores` は全要素が `0.0 <= score <= 1.0`
  - `ranked_indices.len() == 3`
  - `ranked_indices[0] == 0 または 1`（trial0/trial1が上位。均等重みで試行2が中間）
  - `scores[2] < scores[0]` または `scores[2] < scores[1]`（中間値の試行は最高スコアにならない）
- **テストの目的**: TOPSIS基本動作・スコア範囲・降順ソートの確認
- **確認ポイント**: scores全要素が[0,1]内、ranked_indicesがスコア降順
- 🔵 参照: `docs/tasks/mode-frontier-features/TASK-1615.md` テストケース1, topsis-requirements.md §4.1

---

### TC-1615-02: maximize方向の目的が正しくランク付けされる

- **テスト名**: `tc_1615_02_maximize_direction`
- **何をテストするか**: `is_minimize=[false, true]`（obj0はmaximize）で、obj0の値が大きい試行が高スコアになる
- **期待される動作**: maximize目的では正理想解と負理想解が逆転し、大きい値の試行が有利になる
- **入力値**:
  ```rust
  // 試行0: obj0=1.0(小) obj1=1.0(小) → maximize目的では不利
  // 試行1: obj0=5.0(大) obj1=1.0(小) → maximize目的では有利・minimize目的でも良い
  // 試行2: obj0=5.0(大) obj1=5.0(大) → maximize有利・minimize不利
  values = [1.0, 1.0, 5.0, 1.0, 5.0, 5.0]
  weights = [0.7, 0.3]  // obj0重視
  is_minimize = [false, true]
  ```
- **期待される結果**:
  - `ranked_indices[0] == 1`（trial1がobj0大・obj1小で最優秀）
  - `scores[0] < scores[1]`（trial1 > trial0）
- **テストの目的**: maximize方向フラグの正確な反映確認
- **確認ポイント**: 正理想解/負理想解の方向がis_minimizeに応じて反転している
- 🔵 参照: `docs/tasks/mode-frontier-features/TASK-1615.md` テストケース2

---

### TC-1615-03: 重みが異なると順位が変わる

- **テスト名**: `tc_1615_03_weights_affect_ranking`
- **何をテストするか**: weights=[0.9, 0.1]とweights=[0.1, 0.9]でランキングが変化することを確認
- **期待される動作**: 重みを変えると優先される目的が変わり、最上位試行が入れ替わる
- **入力値**:
  ```rust
  // 試行0: obj0を重視した場合に有利
  // 試行1: obj1を重視した場合に有利
  values = [1.0, 5.0, 5.0, 1.0]  // 2試行×2目的
  weights_a = [0.9, 0.1]
  weights_b = [0.1, 0.9]
  is_minimize = [true, true]
  ```
- **期待される結果**:
  - `weights_a`使用時: `ranked_indices[0] == 0`（trial0が obj0=1.0 で有利）
  - `weights_b`使用時: `ranked_indices[0] == 1`（trial1が obj1=1.0 で有利）
- **テストの目的**: 重みパラメータがスコア計算に正しく反映されることの確認
- 🔵 参照: `docs/design/mode-frontier-features/architecture.md` TOPSIS重み付き正規化ステップ

---

### TC-1615-04: 1試行での動作確認

- **テスト名**: `tc_1615_04_single_trial`
- **何をテストするか**: n_trials=1の場合、スコアが有効値を返しクラッシュしない
- **期待される動作**: 1試行の場合、正理想解=負理想解=その試行自身。D+=D-=0となりscore=0.5
- **入力値**:
  ```rust
  values = [3.0, 7.0]
  n_trials = 1, n_objectives = 2
  weights = [0.5, 0.5]
  is_minimize = [true, true]
  ```
- **期待される結果**:
  - `result.scores.len() == 1`
  - `result.scores[0] == 0.5`（D+=D-=0のフォールバック）
  - `result.ranked_indices == [0]`
- **テストの目的**: 1試行の境界条件でクラッシュしないことの確認
- 🟡 参照: topsis-requirements.md §2.3（D++D-=0フォールバック仕様より推測）

---

### TC-1615-05: 1目的のみでの動作確認

- **テスト名**: `tc_1615_05_single_objective`
- **何をテストするか**: n_objectives=1の場合に正常計算できる
- **期待される動作**: 1次元空間でのTOPSIS計算が正しく実行される
- **入力値**:
  ```rust
  values = [3.0, 1.0, 2.0]  // 3試行×1目的
  n_trials = 3, n_objectives = 1
  weights = [1.0]
  is_minimize = [true]
  ```
- **期待される結果**:
  - `ranked_indices[0] == 1`（値1.0が最小→最良）
  - `ranked_indices[2] == 0`（値3.0が最大→最悪）
- **テストの目的**: 1目的の退化ケースでも正確に動作することの確認
- 🔵 参照: topsis-requirements.md §4.2

---

## 2. 異常系テストケース

### TC-1615-06: n_trials=0でエラーを返す

- **テスト名**: `tc_1615_06_zero_trials_error`
- **エラーケースの概要**: 試行数が0の場合、計算不能として Err を返す
- **エラー処理の重要性**: パニックせず、呼び出し元で適切にエラーハンドリングできる
- **入力値**:
  ```rust
  values = []
  n_trials = 0, n_objectives = 2
  weights = [0.5, 0.5]
  is_minimize = [true, true]
  ```
- **期待される結果**: `Err(msg)` が返り、msg に "n_trials" を含む
- **テストの目的**: 空入力での堅牢性確認
- 🔵 参照: topsis-requirements.md §3.2, TASK-1615.md 完了条件

---

### TC-1615-07: values長さ不一致でエラーを返す

- **テスト名**: `tc_1615_07_values_length_mismatch_error`
- **エラーケースの概要**: `values.len() != n_trials * n_objectives` の場合 Err を返す
- **入力値**:
  ```rust
  values = [1.0, 2.0, 3.0]  // 長さ3 だが n_trials=2, n_objectives=2 → 期待4
  n_trials = 2, n_objectives = 2
  weights = [0.5, 0.5]
  is_minimize = [true, true]
  ```
- **期待される結果**: `Err(msg)` が返り、msg に "length" または "mismatch" を含む
- **テストの目的**: 入力データ整合性チェックの確認
- 🔵 参照: topsis-requirements.md §3.2

---

### TC-1615-08: weights長さ不一致でエラーを返す

- **テスト名**: `tc_1615_08_weights_length_mismatch_error`
- **エラーケースの概要**: `weights.len() != n_objectives` の場合 Err を返す
- **入力値**:
  ```rust
  values = [1.0, 2.0, 3.0, 4.0]
  n_trials = 2, n_objectives = 2
  weights = [1.0]  // 長さ1 だが n_objectives=2
  is_minimize = [true, true]
  ```
- **期待される結果**: `Err(msg)` が返る
- 🔵 参照: topsis-requirements.md §3.2

---

### TC-1615-09: is_minimize長さ不一致でエラーを返す

- **テスト名**: `tc_1615_09_is_minimize_length_mismatch_error`
- **エラーケースの概要**: `is_minimize.len() != n_objectives` の場合 Err を返す
- **入力値**:
  ```rust
  values = [1.0, 2.0, 3.0, 4.0]
  n_trials = 2, n_objectives = 2
  weights = [0.5, 0.5]
  is_minimize = [true]  // 長さ1
  ```
- **期待される結果**: `Err(msg)` が返る
- 🔵 参照: topsis-requirements.md §3.2

---

## 3. 境界値テストケース

### TC-1615-10: 全試行が同一値でスコア0.5を返す（D++D-=0）

- **テスト名**: `tc_1615_10_all_same_values_no_crash`
- **境界値の意味**: 全試行が同一値の場合、ユークリッド距離がゼロになるゼロ除算ケース
- **境界値での動作保証**: score = 0.5 で統一され、クラッシュ・パニックが発生しない
- **入力値**:
  ```rust
  values = [2.0, 3.0, 2.0, 3.0, 2.0, 3.0]  // 3試行が全同一
  n_trials = 3, n_objectives = 2
  weights = [0.5, 0.5]
  is_minimize = [true, true]
  ```
- **期待される結果**:
  - `result.is_ok() == true`（クラッシュなし）
  - `result.scores` の全要素が `0.5`（誤差1e-9以内）
- **テストの目的**: ゼロ除算フォールバックの動作確認
- 🔵 参照: topsis-requirements.md §2.3, TASK-1615.md テストケース3

---

### TC-1615-11: NaN値を含む試行が末尾ランクになる

- **テスト名**: `tc_1615_11_nan_trial_ranked_last`
- **境界値の意味**: 目的関数値にNaN（未評価試行）が含まれる実運用でよく発生するケース
- **境界値での動作保証**: NaN試行はscore=0.0でランク末尾に配置される
- **入力値**:
  ```rust
  // 試行0は優秀な値、試行1にNaN
  values = [1.0, 1.0, f64::NAN, 1.0]
  n_trials = 2, n_objectives = 2
  weights = [0.5, 0.5]
  is_minimize = [true, true]
  ```
- **期待される結果**:
  - `result.scores[1] == 0.0`（NaN試行のスコア）
  - `result.ranked_indices.last() == Some(&1)`（NaN試行が最末尾）
- **テストの目的**: NaN値の安全な処理確認
- 🔵 参照: TASK-1615.md §NaN処理, topsis-requirements.md §3.2

---

### TC-1615-12: パフォーマンス – 50,000試行×4目的で100ms以内

- **テスト名**: `tc_1615_12_performance_50k_trials`
- **境界値の意味**: 大規模最適化実行結果でのリアルタイム処理可能な上限サイズ
- **境界値での動作保証**: 50,000×4目的の計算が100ms以内に完了する
- **入力値**:
  ```rust
  let n_trials = 50_000;
  let n_objectives = 4;
  let values: Vec<f64> = (0..n_trials * n_objectives).map(|i| i as f64 % 100.0).collect();
  let weights = [0.25f64; 4];
  let is_minimize = [true; 4];
  ```
- **期待される結果**:
  - `result.is_ok() == true`
  - `elapsed.as_millis() < 100`
- **テストの目的**: NFR（パフォーマンス要件）の充足確認
- **注意**: `#[test]` の中で `std::time::Instant` を使用して計測
- 🔵 参照: TASK-1615.md 完了条件・パフォーマンス要件

---

### TC-1615-13: ranked_indicesの長さがn_trialsと一致する

- **テスト名**: `tc_1615_13_ranked_indices_length`
- **境界値の意味**: 全試行（NaN含む）がrankingに含まれることの保証
- **入力値**:
  ```rust
  values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]  // 3試行×2目的
  ```
- **期待される結果**:
  - `result.ranked_indices.len() == 3`
  - `result.scores.len() == 3`
  - `result.ranked_indices` に 0,1,2 が各1回ずつ含まれる（置換）
- **テストの目的**: 出力配列のサイズ整合性確認
- 🔵 参照: topsis-requirements.md §2.2

---

### TC-1615-14: positive_ideal と negative_ideal の次元数が n_objectives と一致する

- **テスト名**: `tc_1615_14_ideal_solutions_dimension`
- **境界値の意味**: 理想解の次元が目的数と一致することの保証
- **入力値**:
  ```rust
  n_objectives = 3
  values = 任意の3試行×3目的
  ```
- **期待される結果**:
  - `result.positive_ideal.len() == 3`
  - `result.negative_ideal.len() == 3`
- **テストの目的**: 出力構造体の次元整合性確認
- 🟡 参照: topsis-requirements.md §2.2（出力仕様より推測）

---

## 5. テストケース実装テンプレート（Rust）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // 正常系テストケース
    // ============================================================

    #[test]
    fn tc_1615_01_basic_two_obj_minimize() {
        // 【テスト目的】: 2目的minimize方向でのTOPSIS基本動作確認
        // 【テスト内容】: 3試行×2目的の最適化結果をランキングする
        // 【期待される動作】: スコアが0〜1の範囲、ranked_indicesが降順
        // 🔵 参照: TASK-1615.md テストケース1

        // 【テストデータ準備】: 3試行×2目的（minimize両方）
        let values = [1.0_f64, 4.0, 4.0, 1.0, 2.0, 2.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];

        // 【実際の処理実行】: TOPSISアルゴリズムを実行
        let result = compute_topsis(&values, 3, 2, &weights, &is_minimize);

        // 【結果検証】
        assert!(result.is_ok(), "計算は成功すべき");
        let r = result.unwrap();

        // 【検証項目】: スコアが[0,1]の範囲内
        for &s in &r.scores {
            assert!(s >= 0.0 && s <= 1.0, "スコアは0〜1の範囲: {}", s);
        }
        // 【検証項目】: ranked_indicesの長さ
        assert_eq!(r.ranked_indices.len(), 3);
        // 【検証項目】: 降順ソート確認
        for i in 0..r.ranked_indices.len() - 1 {
            assert!(
                r.scores[r.ranked_indices[i] as usize]
                    >= r.scores[r.ranked_indices[i + 1] as usize],
                "ランキングはスコア降順でなければならない"
            );
        }
    }

    #[test]
    fn tc_1615_10_all_same_values_no_crash() {
        // 【テスト目的】: 全試行が同一値のときD++D-=0となるゼロ除算ケースの安全処理確認
        // 🔵 参照: TASK-1615.md テストケース3

        let values = [2.0_f64, 3.0, 2.0, 3.0, 2.0, 3.0];
        let result = compute_topsis(&values, 3, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok(), "同一値でもエラーにならない");
        let r = result.unwrap();
        for &s in &r.scores {
            // 【検証項目】: フォールバックスコアが0.5
            assert!((s - 0.5).abs() < 1e-9, "全同一値はscore=0.5: {}", s);
        }
    }

    // ... (他のテストケースも同様に実装)
}
```

---

## 6. 要件定義との対応関係

| テストケース | 要件定義参照 |
|------------|------------|
| TC-01〜05  | §1機能概要・§2入出力仕様・§4.1基本使用パターン |
| TC-06〜09  | §3.2セキュリティ・堅牢性要件（入力検証） |
| TC-10      | §2.3スコアの意味（D++D-=0フォールバック） |
| TC-11      | §3.2（NaN処理）・§4.3エッジケース |
| TC-12      | §3.1パフォーマンス要件（50k×4, 100ms） |
| TC-13〜14  | §2.2出力値仕様 |

---

## 品質判定結果

| 観点 | 状態 |
|-----|------|
| テストケース分類 | ✅ 正常系5件・異常系4件・境界値4件 — 網羅的 |
| 期待値定義 | ✅ 全件に具体的な期待値・判定条件あり |
| 技術選択 | ✅ Rust標準テスト（既存コードベースと一致） |
| 実装可能性 | ✅ 確実（外部依存なし） |
| 信頼性レベル | 🔵 青: 12件(86%) / 🟡 黄: 2件(14%) / 🔴 赤: 0件(0%) |

**品質評価**: ✅ 高品質
