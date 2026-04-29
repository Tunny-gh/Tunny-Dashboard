# PROMETHEE Ranking 受け入れ基準

**作成日**: 2026-04-29
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングにない推測による基準

---

## REQ-PR-005: Linear 選好関数 🔵

**信頼性**: 🔵 *ユーザヒアリング（Linear のみ・自動閾値）より*

### Given（前提条件）
- 3 試行 × 2 目的関数のデータ
- is_minimize = [true, true]
- weights = [0.5, 0.5]

### When（実行条件）
- `compute_promethee` を呼び出す

### Then（期待結果）
- q_j = 0.0、p_j = 0.2 × range_j が自動設定される
- 各ペア (a,b) の P(a,b) が Linear 式に従う

### テストケース

#### 正常系

- [ ] **TC-PR-005-01**: Linear 選好: d > p → P = 1.0 🔵
  - **入力**: values = [1.0, 5.0, 5.0, 1.0]（2試行×2目的）, is_minimize=[true,true], weights=[0.5,0.5]
  - **期待結果**: 試行0 vs 試行1 の目的0 で d=|1-5|=4, range=4, p=0.8, d>p → P=1.0
  - **信頼性**: 🔵

- [ ] **TC-PR-005-02**: Linear 選好: d ≤ q → P = 0.0 🔵
  - **入力**: values が全て同値
  - **期待結果**: 全 P = 0.0、全フロー = 0.0
  - **信頼性**: 🔵

- [ ] **TC-PR-005-03**: Linear 選好: q < d ≤ p → P = (d-q)/(p-q) 🔵
  - **入力**: 差分が p の半分のデータ
  - **期待結果**: P = 0.5
  - **信頼性**: 🔵

#### 境界値

- [ ] **TC-PR-005-B01**: range_j = 0 の場合 p_j = 0.0、全 P = 0.0 🔵
  - **入力**: 全試行で目的 j の値が同一
  - **期待結果**: P_j = 0 for all pairs、クラッシュなし
  - **信頼性**: 🔵

---

## REQ-PR-006: フロー計算 🔵

**信頼性**: 🔵 *PROMETHEE アルゴリズム標準定義・ユーザヒアリングより*

### Given
- 有効試行数 n_valid ≥ 2

### When
- `compute_promethee` を呼び出す

### Then
- Φ+(i)、Φ-(i) が [0, 1] の範囲に収まる
- Φ(i) = Φ+(i) - Φ-(i) が成立

### テストケース

#### 正常系

- [ ] **TC-PR-006-01**: 3 試行 × 2 目的 — フロー値範囲チェック 🔵
  - **入力**: values=[1,4, 4,1, 2,2], weights=[0.5,0.5], is_minimize=[true,true]
  - **期待結果**: 全 phi_plus ∈ [0,1]、全 phi_minus ∈ [0,1]
  - **信頼性**: 🔵

- [ ] **TC-PR-006-02**: Φnet = Φ+ - Φ- の検証 🔵
  - **入力**: 任意の有効データ
  - **期待結果**: phi_net[i] == phi_plus[i] - phi_minus[i] (誤差 1e-9 以内)
  - **信頼性**: 🔵

- [ ] **TC-PR-006-03**: 試行が多ければ多いほど高い Φ+ を持つ試行が首位 🔵
  - **入力**: 明確に優れた試行（全目的で最小値）を含む 4 試行データ
  - **期待結果**: 最優秀試行が `ranked_indices_ii[0]` に来る
  - **信頼性**: 🔵

- [ ] **TC-PR-006-04**: n_valid = 1 の場合 全フロー = 0.0 🔵
  - **入力**: 1 試行のデータ（またはNaN含む2試行で有効1）
  - **期待結果**: phi_plus[0]=0, phi_minus[0]=0, phi_net[0]=0
  - **信頼性**: 🔵

#### 境界値

- [ ] **TC-PR-006-B01**: n_valid = 0（全 NaN）— 全フロー 0.0、正常終了 🔵
  - **入力**: 全試行が NaN を含む
  - **期待結果**: Ok(PrometheeResult { phi_plus: all 0.0, ... })
  - **信頼性**: 🔵

---

## REQ-PR-007: PROMETHEE I ランキング 🔵

**信頼性**: 🔵 *ユーザヒアリング（Φ+ 降順・Φ- 昇順タイブレーク）より*

### Given
- 有効試行が複数存在する

### When
- `compute_promethee` 呼び出し後に `ranked_indices_i` を参照

### Then
- `ranked_indices_i` は Φ+ 降順でソートされている
- 同 Φ+ の場合は Φ- 昇順でタイブレーク

### テストケース

#### 正常系

- [ ] **TC-PR-007-01**: 基本 3 試行 — Φ+ 降順検証 🔵
  - **入力**: 明確に Φ+ 差がある 3 試行
  - **期待結果**: ranked_indices_i[0] が最大 Φ+ の試行
  - **信頼性**: 🔵

- [ ] **TC-PR-007-02**: Φ+ タイブレーク — Φ- 昇順 🟡
  - **入力**: 2 試行の Φ+ が同値になるよう設計されたデータ
  - **期待結果**: Φ- が小さい試行が上位
  - **信頼性**: 🟡 *設計データの作成が難しいが妥当なケース*

- [ ] **TC-PR-007-03**: NaN 試行は末尾 🔵
  - **入力**: 1 試行が NaN を含む 3 試行データ
  - **期待結果**: NaN 試行のインデックスが `ranked_indices_i` の末尾
  - **信頼性**: 🔵

---

## REQ-PR-008: PROMETHEE II ランキング 🔵

**信頼性**: 🔵 *ユーザヒアリング（Φnet 降順）より*

### テストケース

#### 正常系

- [ ] **TC-PR-008-01**: 基本ランキング — Φnet 降順 🔵
  - **入力**: 3 試行 × 2 目的（trial0 が明確に最優）
  - **期待結果**: ranked_indices_ii[0] が最大 Φnet の試行
  - **信頼性**: 🔵

- [ ] **TC-PR-008-02**: ranked_indices_ii の長さ = n_trials 🔵
  - **入力**: 任意の有効データ
  - **期待結果**: ranked_indices_ii.len() == n_trials
  - **信頼性**: 🔵

- [ ] **TC-PR-008-03**: NaN 試行は末尾 🔵
  - **入力**: NaN 含有試行を含む 4 試行データ
  - **期待結果**: NaN 試行が `ranked_indices_ii` の末尾
  - **信頼性**: 🔵

---

## REQ-PR-003: バリデーション 🔵

**信頼性**: 🔵 *既存 validate_inputs パターンより*

### テストケース

#### 異常系

- [ ] **TC-PR-003-E01**: n_trials = 0 → Err 🔵
  - **入力**: n_trials=0
  - **期待結果**: Err("n_trials must be >= 1" を含む)
  - **信頼性**: 🔵

- [ ] **TC-PR-003-E02**: values 長さ不一致 → Err 🔵
  - **入力**: values.len() ≠ n_trials × n_objectives
  - **期待結果**: Err(...)
  - **信頼性**: 🔵

- [ ] **TC-PR-003-E03**: weights 長さ不一致 → Err 🔵
  - **入力**: weights.len() ≠ n_objectives
  - **期待結果**: Err(...)
  - **信頼性**: 🔵

- [ ] **TC-PR-003-E04**: is_minimize 長さ不一致 → Err 🔵
  - **入力**: is_minimize.len() ≠ n_objectives
  - **期待結果**: Err(...)
  - **信頼性**: 🔵

---

## REQ-PR-011: McdmMethod enum 更新 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 McdmMethod パターンより*

### テストケース

- [ ] **TC-PR-011-01**: PrometheeI ラベル = "PROMETHEE I" 🔵
  - **期待結果**: `McdmMethod::PrometheeI.label() == "PROMETHEE I"`
  - **信頼性**: 🔵

- [ ] **TC-PR-011-02**: PrometheeII ラベル = "PROMETHEE II" 🔵
  - **期待結果**: `McdmMethod::PrometheeII.label() == "PROMETHEE II"`
  - **信頼性**: 🔵

- [ ] **TC-PR-011-03**: all() に PrometheeI / PrometheeII が含まれる 🔵
  - **期待結果**: `McdmMethod::all()` に 4 要素存在
  - **信頼性**: 🔵

---

## NFR-PR-001: パフォーマンス 🟡

**信頼性**: 🟡 *O(n²) アルゴリズムの特性から妥当な推測*

### テストケース

- [ ] **TC-PR-NFR-001-01**: 50,000 試行 × 4 目的 — 200 ms 以内 🟡
  - **測定項目**: `compute_promethee` の実行時間
  - **目標値**: < 200 ms
  - **測定条件**: Release ビルド、weights=[0.25;4], is_minimize=[true;4]
  - **備考**: O(n²) のため TOPSIS (< 100 ms) より緩い目標値
  - **信頼性**: 🟡

- [ ] **TC-PR-NFR-001-02**: 10,000 試行 × 4 目的 — 20 ms 以内 🟡
  - **目標値**: < 20 ms
  - **信頼性**: 🟡

---

## Edge ケーステスト

### EDGE-PR-001〜005: アルゴリズム境界値

- [ ] **TC-EDGE-PR-001**: n_trials = 1 — 全フロー 0.0、クラッシュなし 🔵
  - **信頼性**: 🔵

- [ ] **TC-EDGE-PR-002**: 全同値データ（range=0） — 全フロー 0.0、クラッシュなし 🔵
  - **信頼性**: 🔵

- [ ] **TC-EDGE-PR-003**: n_objectives = 1 — 正常動作 🔵
  - **信頼性**: 🔵

- [ ] **TC-EDGE-PR-004**: 全 NaN — Ok(全フロー 0.0) 🔵
  - **信頼性**: 🔵

- [ ] **TC-EDGE-PR-005**: Φnet 負値が存在する場合の ranked_indices_ii クラッシュなし 🟡
  - **信頼性**: 🟡

### EDGE-PR-010: Study 変更時のキャッシュクリア

- [ ] **TC-EDGE-PR-010**: Study 変更後に mcdm_result が None になる 🔵
  - **信頼性**: 🔵

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| アルゴリズム (PR-005〜008) | 9 | 4 | 5 | 18 |
| 状態管理 (PR-011) | 3 | 0 | 0 | 3 |
| パフォーマンス | 2 | 0 | 0 | 2 |
| Edge ケース | 4 | 0 | 2 | 6 |
| **合計** | **18** | **4** | **7** | **29** |

### 信頼性レベル分布

- 🔵 青信号: 25 件 (86%)
- 🟡 黄信号: 4 件 (14%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 25 件（アルゴリズム正確性・バリデーション・NaN 処理）
- **Should Have**: 4 件（パフォーマンス・タイブレーク・Φnet 負値）

---

## テスト実施計画

### Phase 1: アルゴリズム単体テスト（rust_core）
- TC-PR-005-01〜B01, TC-PR-006-01〜B01, TC-PR-007-01〜03, TC-PR-008-01〜03
- TC-PR-003-E01〜E04
- TC-EDGE-PR-001〜005
- 実施方法: `cargo test -p tunny-core`

### Phase 2: UI・状態管理テスト（egui-app）
- TC-PR-011-01〜03
- TC-EDGE-PR-010
- 実施方法: `cargo test -p egui-app`

### Phase 3: パフォーマンステスト
- TC-PR-NFR-001-01〜02
- 実施方法: `cargo test -p tunny-core -- --ignored` (release モード)
