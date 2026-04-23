# VIKOR 受け入れ基準

**作成日**: 2026-04-24
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: アルゴリズム仕様・既存実装から妥当な推測による基準
- 🔴 **赤信号**: ヒアリングにない推測による基準

---

## REQ-001: compute_vikor 基本計算 🔵

**信頼性**: 🔵 *VIKORアルゴリズム仕様・ユーザヒアリングより*

### Given
- n_trials >= 1, n_objectives >= 1
- values, weights, is_minimize が正しい長さ
- v ∈ [0.0, 1.0]

### When
- `compute_vikor(&values, n_trials, n_objectives, &weights, &is_minimize, v)` を呼び出す

### Then
- `Ok(VikorResult)` が返る
- `q_values.len() == n_trials`
- `s_values.len() == n_trials`
- `r_values.len() == n_trials`
- `ranked_indices.len() == n_trials`
- Q値はすべて 0.0〜1.0 の範囲

### テストケース

#### 正常系

- [ ] **TC-VIKOR-001**: 2目的・3試行・minimize・v=0.5 🔵
  - **入力**: values=[1,2, 3,1, 2,2], weights=[0.5,0.5], is_minimize=[true,true], v=0.5
  - **期待結果**: Ok、q_values長さ=3、ranked_indices長さ=3、Q値 0〜1
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-002**: maximize目的混在 🔵
  - **入力**: values=[1,1, 5,1, 5,5], weights=[0.7,0.3], is_minimize=[false,true], v=0.5
  - **期待結果**: trial1またはtrial2が最良（Q最小）
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-003**: v=0.0（R方向のみ） 🔵
  - **入力**: v=0.0
  - **期待結果**: Q = R 正規化値（S方向寄与なし）
  - **信頼性**: 🔵 *VIKORアルゴリズム仕様より*

- [ ] **TC-VIKOR-004**: v=1.0（S方向のみ） 🔵
  - **入力**: v=1.0
  - **期待結果**: Q = S 正規化値（R方向寄与なし）
  - **信頼性**: 🔵 *VIKORアルゴリズム仕様より*

- [ ] **TC-VIKOR-005**: 重みが異なる場合のランキング変化 🔵
  - **入力**: values=[1,5, 5,1]（2試行×2目的）, is_minimize=[true,true]
  - **入力A**: weights=[0.9,0.1] → trial0が1位
  - **入力B**: weights=[0.1,0.9] → trial1が1位
  - **期待結果**: 重みが変わるとランキングが逆転する
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-006**: ranked_indices がQ昇順 🔵
  - **期待結果**: ranked_indices[0] の q_values が最小
  - **信頼性**: 🔵

#### 異常系

- [ ] **TC-VIKOR-E01**: n_trials=0 🔵
  - **入力**: n_trials=0
  - **期待結果**: `Err("n_trials must be >= 1")`
  - **信頼性**: 🔵 *TOPSIS実装パターンより*

- [ ] **TC-VIKOR-E02**: values長さ不一致 🔵
  - **入力**: values長 != n_trials * n_objectives
  - **期待結果**: `Err("values length mismatch: ...")`
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-E03**: weights長さ不一致 🔵
  - **入力**: weights長 != n_objectives
  - **期待結果**: `Err("weights length mismatch: ...")`
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-E04**: is_minimize長さ不一致 🔵
  - **入力**: is_minimize長 != n_objectives
  - **期待結果**: `Err("is_minimize length mismatch: ...")`
  - **信頼性**: 🔵

#### 境界値・エッジケース

- [ ] **TC-VIKOR-B01**: 1試行 🟡
  - **入力**: n_trials=1, values=[3.0, 7.0], weights=[0.5,0.5], is_minimize=[true,true]
  - **期待結果**: q_values=[0.0]（または安全な値）、クラッシュなし
  - **信頼性**: 🟡 *VIKORアルゴリズムの自然な帰結から推測*

- [ ] **TC-VIKOR-B02**: 全試行が同一値 🟡
  - **入力**: values=[2,3, 2,3, 2,3]（全同一）
  - **期待結果**: q_values=[0.0, 0.0, 0.0]（クラッシュなし）
  - **信頼性**: 🟡 *ゼロ除算ガード実装から推測*

- [ ] **TC-VIKOR-B03**: NaN含む試行 🔵
  - **入力**: values=[1.0, 1.0, f64::NAN, 1.0]（trial1がNaN）
  - **期待結果**: trial1のq_values=1.0、ranked_indices末尾に配置
  - **信頼性**: 🔵 *ユーザヒアリング（TOPSIS同方針）より*

- [ ] **TC-VIKOR-B04**: 1目的 🟡
  - **入力**: n_objectives=1, values=[3.0,1.0,2.0], weights=[1.0], is_minimize=[true]
  - **期待結果**: Ok、正常計算（最小値 trial1 が最良）
  - **信頼性**: 🟡

---

## REQ-003: primary_scores = 1.0 - Q 🔵

**信頼性**: 🔵 *既存McdmResult.primary_scores()インターフェースより*

### Given
- VikorResult が app_state.mcdm_result に格納されている

### When
- `McdmResult::Vikor(r).primary_scores()` を呼び出す

### Then
- 返り値 = `1.0 - r.q_values`（各要素）
- ranked_indices[0] に対応する primary_score が最大

### テストケース

- [ ] **TC-VIKOR-PS01**: primary_scores の逆変換 🔵
  - q_values=[0.0, 0.5, 1.0]
  - 期待 primary_scores=[1.0, 0.5, 0.0]
  - **信頼性**: 🔵

---

## REQ-007 / REQ-201: v パラメータスライダー 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

### Given
- MCDMランキングチャートが表示されている
- 手法コンボボックスで "VIKOR" を選択している

### When
- Weights セクションを展開する

### Then
- v パラメータのスライダー（0.0〜1.0）が表示される
- デフォルト値は 0.5
- TOPSISを選択している場合はvスライダーは非表示

### テストケース

- [ ] **TC-VIKOR-UI01**: VIKOR選択でvスライダー表示 🔵
  - 手法=VIKOR → v スライダーが存在する
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-UI02**: TOPSIS選択でvスライダー非表示 🔵
  - 手法=TOPSIS → v スライダーが存在しない
  - **信頼性**: 🔵

- [ ] **TC-VIKOR-UI03**: v 変更でpending_computeにv値が反映 🔵
  - v=0.3 設定 → Run クリック → pending_compute内のv=0.3
  - **信頼性**: 🔵

---

## NFR-001: パフォーマンス 🔵

**信頼性**: 🔵 *TOPSIS性能要件と同基準より*

- [ ] **TC-VIKOR-PERF01**: 50,000試行 × 4目的 で 100ms 以内 🔵
  - n_trials=50_000, n_objectives=4, weights=[0.25;4], is_minimize=[true;4], v=0.5
  - 期待: elapsed_ms < 100
  - **信頼性**: 🔵

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| 機能要件 | 7 | 4 | 4 | 15 |
| UI要件 | 3 | 0 | 0 | 3 |
| 非機能要件 | 1 | 0 | 0 | 1 |
| **合計** | 11 | 4 | 4 | 19 |

### 信頼性レベル分布

- 🔵 青信号: 16件 (84%)
- 🟡 黄信号: 3件 (16%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質

### 優先度別テストケース

- **Must Have**: 19件（全件）
- **Should Have**: 0件
- **Could Have**: 0件

---

## テスト実施計画

### Phase 1: rust_core VIKORアルゴリズムテスト
- TC-VIKOR-001〜006（正常系）
- TC-VIKOR-E01〜E04（異常系）
- TC-VIKOR-B01〜B04（境界値）
- TC-VIKOR-PERF01（パフォーマンス）

### Phase 2: egui-app 統合テスト
- TC-VIKOR-PS01（primary_scores変換）
- TC-VIKOR-UI01〜UI03（UIウィジェット）
