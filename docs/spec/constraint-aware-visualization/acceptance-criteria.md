# 制約条件を考慮した可視化 受け入れ基準

**作成日**: 2026-06-03
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による基準

---

## REQ-CAV-001: 実行可能性の判定 🔵

**信頼性**: 🔵 *ユーザヒアリング 2026-06-03 + 既存実装 `model.rs` より*

### Given（前提条件）
- `system_attrs.constraints` を持つ trial が存在する

### When（実行条件）
- journal ファイルをパース後、DataFrame が構築される

### Then（期待結果）
- `constraints` 配列の全値が `<= 0.0` の trial は `is_feasible = 1.0`
- `constraints` 配列に `> 0.0` の値がある trial は `is_feasible = 0.0`

### テストケース

#### 正常系

- [x] **TC-CAV-001-01**: 全制約 <= 0 の trial は feasible 🔵
  - **入力**: `constraints: [-1.0, 0.0, -0.5]`
  - **期待結果**: `is_feasible = 1.0`
  - **信頼性**: 🔵 *既存 `model.rs` L142–148 の実装より（既実装）*

- [x] **TC-CAV-001-02**: 制約に > 0 の値がある trial は infeasible 🔵
  - **入力**: `constraints: [-1.0, 0.5, -0.5]`
  - **期待結果**: `is_feasible = 0.0`
  - **信頼性**: 🔵 *既存 `model.rs` L142–148 の実装より（既実装）*

- [x] **TC-CAV-001-03**: 制約なし Study は全 trial が feasible 🔵
  - **入力**: `has_constraints = false`（`is_feasible` 列なし）
  - **期待結果**: グレーアウトなし・全 trial 通常表示
  - **信頼性**: 🔵 *REQ-CAV-001-C より*

#### 境界値

- [ ] **TC-CAV-001-B01**: 制約値がちょうど 0.0 の trial は feasible 🔵
  - **入力**: `constraints: [0.0, 0.0]`
  - **期待結果**: `is_feasible = 1.0`
  - **信頼性**: 🔵 *`model.rs` L142 の `c <= 0.0` 条件より*

- [ ] **TC-CAV-001-B02**: 空の constraints 配列は feasible 🟡
  - **入力**: `constraints: []`
  - **期待結果**: `is_feasible = 1.0`（`all()` の空イテレータ動作）
  - **信頼性**: 🟡 *Rust の `Iterator::all()` の空イテレータ動作から推測*

---

## REQ-CAV-010〜014: グレーアウト表示 🔵

**信頼性**: 🔵 *ユーザヒアリング 2026-06-03 より*

### Given（前提条件）
- 制約あり Study が選択されている
- チャートに trial データが表示されている

### When（実行条件）
- チャートが描画される

### Then（期待結果）
- `is_feasible = 0.0` の trial 点は `COLOR_INFEASIBLE` で描画される
- `is_feasible = 1.0` の trial 点は従来の色分けで描画される

### テストケース

#### 正常系

- [ ] **TC-CAV-010-01**: 実行不可能解がグレーアウトで表示される 🔵
  - **入力**: `constraints: [1.0]`（infeasible）の trial を含む Study
  - **期待結果**: 当該 trial が `COLOR_INFEASIBLE`（グレー半透明）で描画される
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-010-02**: 制約なし Study ではグレーアウトなし 🔵
  - **入力**: `has_constraints = false` の Study
  - **期待結果**: 全 trial が従来の色で表示される
  - **信頼性**: 🔵 *REQ-CAV-012 より*

- [ ] **TC-CAV-014-01**: `COLOR_INFEASIBLE` が `chart_colors.rs` に定義されている 🔵
  - **入力**: コンパイル確認
  - **期待結果**: `COLOR_INFEASIBLE: Color32` 定数が存在し、alpha=80 程度の半透明グレー
  - **信頼性**: 🔵 *ユーザヒアリング「alpha=80」より*

---

## REQ-CAV-020〜023: Show Infeasible トグル 🔵

**信頼性**: 🔵 *ユーザヒアリング 2026-06-03 より*

### Given（前提条件）
- 制約あり Study が選択されている

### When（実行条件）
- チャートのツールバー領域が表示される

### Then（期待結果）
- "Show Infeasible" チェックボックスが表示される
- デフォルトはチェック済み（true）

### テストケース

#### 正常系

- [ ] **TC-CAV-020-01**: Show Infeasible トグルが制約あり Study で表示される 🔵
  - **入力**: `has_constraints = true` の Study を選択
  - **期待結果**: チャートのツールバーに "Show Infeasible" チェックボックスが表示される
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-021-01**: デフォルトはチェック済み（表示） 🔵
  - **入力**: Study 選択直後
  - **期待結果**: `show_infeasible = true`（実行不可能解が表示された状態）
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-022-01**: Show Infeasible = false で実行不可能解が非表示になる 🔵
  - **入力**: チェックボックスを外す
  - **期待結果**: `is_feasible = 0.0` の trial 点が即座に消える
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-023-01**: 制約なし Study ではトグルが表示されない 🔵
  - **入力**: `has_constraints = false` の Study を選択
  - **期待結果**: "Show Infeasible" チェックボックスが表示されない
  - **信頼性**: 🔵 *REQ-CAV-023 より*

---

## REQ-CAV-030〜033: Pareto ランク計算からの除外 🔵

**信頼性**: 🔵 *ユーザヒアリング 2026-06-03 より*

### Given（前提条件）
- `has_constraints = true` の Study
- 実行可能解と実行不可能解が混在している

### When（実行条件）
- Study の Pareto ランクが計算される

### Then（期待結果）
- 実行不可能解は Pareto ランク計算に含まれない
- `pareto_indices` に実行不可能解のインデックスが含まれない

### テストケース

#### 正常系

- [ ] **TC-CAV-030-01**: 実行不可能解が Pareto フロントに含まれない 🔵
  - **入力**: infeasible な trial のうち、目的値が Pareto 優位な値を持つものを含む Study
  - **期待結果**: `pareto_indices` に infeasible trial のインデックスが含まれない
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-032-01**: 実行不可能解の pareto_rank が特別値になる 🟡
  - **入力**: infeasible な trial を含む Study
  - **期待結果**: infeasible trial の `pareto_rank` が `u32::MAX` または `999` などの特別値
  - **信頼性**: 🟡 *既存 `pareto_rank: Vec<u32>` 型から推測*

#### 境界値

- [ ] **TC-CAV-030-B01**: 全 trial が実行不可能解の場合 🔵
  - **入力**: 全 trial の `is_feasible = 0.0`
  - **期待結果**: `pareto_indices` が空、Pareto フロントなし
  - **信頼性**: 🔵 *EDGE-CAV-010 より*

---

## REQ-CAV-040〜043: ParetoScatter2D への対応 🔵

**信頼性**: 🔵 *ユーザヒアリング + 既存 `pareto_2d.rs` 実装より*

### テストケース

#### 正常系

- [ ] **TC-CAV-040-01**: ParetoScatter2D で実行不可能解がグレーアウトされる 🔵
  - **入力**: 制約あり Study で ParetoScatter2D を表示
  - **期待結果**: infeasible trial 点が `COLOR_INFEASIBLE` で描画される
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-042-01**: ParetoScatter2D のコントロール行にトグルが追加される 🔵
  - **入力**: `has_constraints = true` の Study で ParetoScatter2D を表示
  - **期待結果**: X/Y 軸選択の行に "Show Infeasible" チェックボックスが表示される
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-043-01**: 実行不可能解が実行可能解より手前に描画されない 🟡
  - **入力**: feasible と infeasible の点が重なる位置
  - **期待結果**: infeasible 点が背面に表示される（feasible 点が前面）
  - **信頼性**: 🟡 *視認性から推測*

---

## REQ-CAV-060〜063: ParallelCoordinates への対応 🔵

### テストケース

- [ ] **TC-CAV-060-01**: 並行座標で実行不可能解の折れ線がグレーアウトされる 🔵
  - **入力**: 制約あり Study で ParallelCoordinates を表示
  - **期待結果**: infeasible trial の折れ線が `COLOR_INFEASIBLE` で描画される
  - **信頼性**: 🔵 *ユーザヒアリングより*

- [ ] **TC-CAV-062-01**: Show Infeasible = false で折れ線が非表示 🔵
  - **入力**: チェックボックスを外す
  - **期待結果**: infeasible trial の折れ線が即座に消える
  - **信頼性**: 🔵 *ユーザヒアリングより*

---

## REQ-CAV-070〜073: ScatterMatrix への対応 🔵

### テストケース

- [ ] **TC-CAV-070-01**: 散布行列で実行不可能解がグレーアウトされる 🔵
  - **入力**: 制約あり Study で ScatterMatrix を表示
  - **期待結果**: infeasible trial 点が全セルで `COLOR_INFEASIBLE` で描画される
  - **信頼性**: 🔵 *ユーザヒアリングより*

---

## REQ-CAV-080〜083: OptimizationHistory への対応 🔵

### テストケース

- [ ] **TC-CAV-080-01**: 最適化履歴で実行不可能解がグレーアウトされる 🔵
  - **入力**: 制約あり Study で OptimizationHistory を表示
  - **期待結果**: infeasible trial 点が `COLOR_INFEASIBLE` で描画される
  - **信頼性**: 🔵 *ユーザヒアリングより*

---

## REQ-CAV-090〜093: ClusterScatter への対応 🔵

### テストケース

- [ ] **TC-CAV-090-01**: クラスター散布図で実行不可能解がグレーアウトされる 🔵
  - **入力**: 制約あり Study で ClusterScatter を表示
  - **期待結果**: infeasible trial 点が `COLOR_INFEASIBLE` で描画される
  - **信頼性**: 🔵 *ユーザヒアリングより*

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| 判定ロジック | 3 | 0 | 2 | 5 |
| グレーアウト表示 | 3 | 0 | 0 | 3 |
| トグル UI | 4 | 0 | 0 | 4 |
| Pareto 計算 | 2 | 0 | 1 | 3 |
| 各チャート対応 | 7 | 0 | 1 | 8 |
| **合計** | **19** | **0** | **4** | **23** |

### 信頼性レベル分布

- 🔵 青信号: 20件（87%）
- 🟡 黄信号: 3件（13%）
- 🔴 赤信号: 0件（0%）

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 23件（全件）
- **Should Have**: 0件
- **Could Have**: 0件

---

## テスト実施計画

### Phase 1: ユニットテスト（rust_core）
- TC-CAV-001-01〜B02（判定ロジック）
- `DataFrame::from_trials` の `is_feasible` 生成テスト

### Phase 2: Pareto 計算テスト
- TC-CAV-030-01〜B01
- `pareto_indices` に infeasible が含まれないことの確認

### Phase 3: UI テスト（手動確認）
- TC-CAV-010〜093（グレーアウト・トグル・各チャート対応）
- 実際の制約付き journal ファイルで動作確認
