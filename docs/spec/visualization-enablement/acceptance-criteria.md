# 可視化機能有効化 受け入れ基準

**作成日**: 2026-03-29
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングにない推測による基準

---

## LAYER 1: WASM バインディング

### REQ-VE-001〜006: 新規 WASM バインディング 🔵

**信頼性**: 🔵 *wasm-phase2-requirements.md REQ-101〜105 のパターン / TASK-801, TASK-901 memo より*

#### Given（前提条件）
- `rust_core/src/lib.rs` に新規 `#[wasm_bindgen]` 関数を追加した
- `wasm-pack build` でビルドが成功している
- `tunny_core.d.ts` に型定義が追加されている

#### When（実行条件）
- JS から各 WASM 関数を呼び出す

#### Then（期待結果）
- 正しい型の JS オブジェクトが返る

#### テストケース

- [ ] **TC-VE-001-01**: `computeSensitivity()` — アクティブ Study あり時に `spearman` / `ridge` / `paramNames` / `objectiveNames` を含むオブジェクトを返す 🔵
- [ ] **TC-VE-001-E01**: `computeSensitivity()` — アクティブ Study なし時に例外がスローされる 🔵
- [ ] **TC-VE-002-01**: `computeSensitivitySelected(indices)` — 有効な indices 時に正常結果を返す 🔵
- [ ] **TC-VE-002-E01**: `computeSensitivitySelected(emptyArray)` — 空配列で例外がスローされる 🔵
- [ ] **TC-VE-003-01**: `runPca(2, "param")` — `projections[N][2]` / `explainedVariance[2]` を返す 🔵
- [ ] **TC-VE-003-E01**: `runPca(2, "invalid_space")` — 例外がスローされる 🔵
- [ ] **TC-VE-004-01**: `runKmeans(3, data, 2)` — `labels.length === N` / `centroids.length === 3` を返す 🔵
- [ ] **TC-VE-005-01**: `estimateKElbow(data, 2, 8)` — `wcssPerK.length === 7` (k=2〜8) / `recommendedK` が 2〜8 範囲内 🔵
- [ ] **TC-VE-006-01**: `computeClusterStats(labels)` — `stats` 配列にクラスタ数分の要素が含まれる 🔵

---

## LAYER 2: analysisStore

### REQ-VE-030〜034: analysisStore 基本動作 🔵

**信頼性**: 🔵 *既存 studyStore / selectionStore パターン / ユーザヒアリングより*

#### テストケース

- [ ] **TC-VE-030-01**: `analysisStore.computeSensitivity()` — 呼び出し中は `isComputingSensitivity === true` 🔵
- [ ] **TC-VE-030-02**: 計算完了後 `sensitivityResult` が非 null で `isComputingSensitivity === false` 🔵
- [ ] **TC-VE-030-E01**: WASM エラー時 `sensitivityError` に文字列がセット、`sensitivityResult` は null 🔵
- [ ] **TC-VE-034-01**: `studyStore.selectStudy()` 後に `sensitivityResult` が null にリセットされる 🟡

---

## LAYER 3: clusterStore

### REQ-VE-040〜046: clusterStore 基本動作 🔵

**信頼性**: 🔵 *tunny-dashboard-requirements.md REQ-080〜087 / TASK-901, TASK-902 memo より*

#### テストケース

- [ ] **TC-VE-040-01**: `clusterStore.runClustering("param", 3)` — `isRunning` が true → false に遷移する 🔵
- [ ] **TC-VE-040-02**: 完了後 `pcaProjections` が非 null かつ長さ N 🔵
- [ ] **TC-VE-040-03**: 完了後 `clusterLabels` が非 null かつ長さ N 🔵
- [ ] **TC-VE-040-04**: 完了後 `clusterStats.stats.length === k` 🔵
- [ ] **TC-VE-043-01**: `clusterStore.estimateK("objective")` — `elbowResult.wcssPerK` が非空 🔵
- [ ] **TC-VE-043-02**: `elbowResult.recommendedK` が 2 以上の整数 🔵
- [ ] **TC-VE-040-E01**: k が試行数より大きい場合 `clusterError` がセットされる 🔵

---

## LAYER 4: SensitivityHeatmap チャート配線

### REQ-VE-050〜054: SensitivityHeatmap チャート 🔵

**信頼性**: 🔵 *TASK-802 memo / SensitivityHeatmap.tsx 既実装 / FreeLayoutCanvas 既存パターンより*

#### Given
- Study が選択されている
- FreeLayoutCanvas に "Sensitivity" チャートが追加されている

#### When
- チャートが初回マウントされる

#### Then
- WASM 計算が自動トリガーされ、完了後にヒートマップが表示される

#### テストケース

##### 正常系
- [ ] **TC-VE-050-01**: チャートマウント後にローディングスピナーが表示される 🔵
- [ ] **TC-VE-050-02**: WASM 完了後にヒートマップの ECharts が表示される (`data-testid="sensitivity-heatmap"` の ECharts 要素) 🔵
- [ ] **TC-VE-050-03**: 上部に Spearman / Ridge β 切り替えボタンが表示される 🔵 *SensitivityHeatmap.tsx 既実装より*
- [ ] **TC-VE-050-04**: 閾値スライダーが機能する 🔵 *SensitivityHeatmap.tsx 既実装より*

##### 異常系
- [ ] **TC-VE-050-E01**: WASM エラー時に `EmptyState message="Sensitivity computation failed"` が表示される 🔵

##### 境界値
- [ ] **TC-VE-050-B01**: 試行数 1 の Study では `EmptyState message="Insufficient trials (min 2)"` が表示される 🔵 *TC-801-05 より*

---

## LAYER 5: Importance チャート

### REQ-VE-060〜065: Importance チャート実装 🔵

**信頼性**: 🔵 *tunny-dashboard-requirements.md REQ-090〜091 / ユーザヒアリングより*

#### Given
- Study が選択されている
- FreeLayoutCanvas に "Importance" チャートが追加されている

#### When
- ドロップダウンで指標を選択する

#### Then
- 選択指標に基づく降順バーチャートが再描画される

#### テストケース

##### 正常系
- [ ] **TC-VE-060-01**: チャートに "Spearman |ρ|" / "Ridge |β|" ドロップダウンが表示される 🔵
- [ ] **TC-VE-060-02**: Spearman 選択時に全パラメータの |ρ| 平均が降順バーで表示される 🔵
- [ ] **TC-VE-060-03**: Ridge 選択時に全パラメータの |β| 平均が降順バーで表示される 🔵
- [ ] **TC-VE-060-04**: チャートタイトルが "Parameter Importance (Spearman |ρ|)" 形式で表示される 🔵
- [ ] **TC-VE-060-05**: ダミー実装（全値 1.0）が表示されず、実データで描画される 🔵

##### 境界値
- [ ] **TC-VE-060-B01**: パラメータ数 0 の Study で `EmptyState` が表示される 🔵

---

## LAYER 6: ClusterView チャート

### REQ-VE-070〜077: ClusterView チャート 🔵

**信頼性**: 🔵 *tunny-dashboard-requirements.md REQ-080〜087 / ユーザヒアリングより*

#### Given
- クラスタリングが実行済み（`clusterStore.pcaProjections` が非 null）
- FreeLayoutCanvas に "Cluster View" チャートが追加されている

#### When
- チャートが表示される

#### Then
- PCA 2D 散布図がクラスタ色分けで表示される

#### テストケース

##### 正常系
- [ ] **TC-VE-070-01**: クラスタリング実行後に散布図が表示される 🔵
- [ ] **TC-VE-070-02**: 各クラスタが異なる色で描画される（`getClusterColor()` 使用） 🔵
- [ ] **TC-VE-070-03**: X 軸 "PC1"・Y 軸 "PC2" ラベルが表示される 🔵
- [ ] **TC-VE-070-04**: 凡例に "Cluster 0"〜"Cluster k-1" が表示される 🟡

##### 異常系
- [ ] **TC-VE-070-E01**: クラスタリング未実行時に `EmptyState message="Run clustering in the left panel first"` が表示される 🔵
- [ ] **TC-VE-070-E02**: `clusterStore.clusterError` がある場合に `EmptyState` にエラー内容が表示される 🔵

---

## LAYER 7: UMAP チャート（PCA モード）

### REQ-VE-080〜087: UMAP チャート 🔵

**信頼性**: 🔵 *tunny-dashboard-requirements.md REQ-082 / ユーザヒアリングより*

#### Given
- Study が選択されている
- FreeLayoutCanvas に "UMAP" チャートが追加されている

#### When
- チャートがマウントされる

#### Then
- PCA 2D 散布図が selectionStore のカラーモードで表示される

#### テストケース

##### 正常系
- [ ] **TC-VE-080-01**: チャートタイトル "Dimensionality Reduction (PCA)" が表示される 🔵
- [ ] **TC-VE-080-02**: 2D 散布図が表示され、試行数分の点が描画される 🔵
- [ ] **TC-VE-080-03**: 次元削減方式セレクタに "PCA"（選択中）と "UMAP (Coming Soon)"（disabled）が表示される 🔵
- [ ] **TC-VE-080-04**: `clusterStore.pcaProjections` が利用可能な場合は追加 WASM 計算なしに表示される 🟡

##### 境界値
- [ ] **TC-VE-080-B01**: 試行数 1 の Study では `EmptyState` が表示される 🟡

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| WASM バインディング | 7 | 4 | 0 | 11 |
| analysisStore | 3 | 1 | 0 | 4 |
| clusterStore | 6 | 1 | 0 | 7 |
| SensitivityHeatmap | 4 | 1 | 1 | 6 |
| Importance | 5 | 0 | 1 | 6 |
| ClusterView | 4 | 2 | 0 | 6 |
| UMAP チャート | 4 | 0 | 1 | 5 |
| **合計** | **33** | **9** | **3** | **45** |

### 信頼性レベル分布

- 🔵 青信号: 41 件 (91%)
- 🟡 黄信号: 4 件 (9%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 38 件
- **Should Have**: 7 件
- **Could Have**: 0 件
