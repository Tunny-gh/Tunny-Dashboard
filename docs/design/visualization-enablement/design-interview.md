# 可視化機能有効化 設計ヒアリング記録

**作成日**: 2026-03-29
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存のコードベース（`comparisonStore.ts`・`studyStore.ts`・`selectionStore.ts`）と要件定義書を確認し、`analysisStore`・`clusterStore` の設計方針および新規チャートコンポーネントの実装方針を明確化するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: Study 変更リセットパターン

**カテゴリ**: アーキテクチャ確認
**背景**: 既存コードで 2 種類のリセットパターンが混在している。
- `studyStore.selectStudy` 内で `selectionStore` を直接リセット（`useSelectionStore.getState().clearSelection()` 相当）
- `comparisonStore` は `useStudyStore.subscribe` で Study 変更を購読しリアクティブにリセット

`analysisStore` / `clusterStore` でどちらを採用するか確認が必要だった。

**回答**: subscribe パターン（推奨）

**信頼性への影響**:
- REQ-VE-034・REQ-VE-045（Study 変更リセット）の信頼性レベルが 🟡 → 🔵 に向上
- 設計文書に `useStudyStore.subscribe` パターンを明記
- `studyStore.ts` への変更不要（循環依存リスク回避）

---

### Q2: ImportanceChart・ClusterScatter・DimReductionScatter のコンポーネント化

**カテゴリ**: 設計方針確認
**背景**: 要件定義書の note.md では独立ファイル化を想定しているが、`importance` チャートは現在 `FreeLayoutCanvas.tsx` 内インラインで実装されているため、既存のダミー実装を独立コンポーネントに切り出すか確認が必要だった。

**回答**: 独立コンポーネント（推奨）— SensitivityHeatmap.tsx と同パターン

**信頼性への影響**:
- コンポーネントレイアウト設計が 🟡 → 🔵 に向上
- 各チャートを `frontend/src/components/charts/` 配下に配置
- テスト可能な独立コンポーネントとして実装

---

## ヒアリング結果サマリー

### 確認できた事項

1. **リセットパターン**: `useStudyStore.subscribe` パターンを採用。`studyStore.ts` への変更不要
2. **コンポーネント分割**: `ImportanceChart.tsx`・`ClusterScatter.tsx`・`DimReductionScatter.tsx` を独立ファイルとして新規作成
3. **FreeLayoutCanvas**: 4 ケースを追加し、各独立コンポーネントを呼び出す形にする

### 設計方針の決定事項

| 項目 | 決定内容 |
|---|---|
| Study 変更リセット | `useStudyStore.subscribe` (subscribeWithSelector が必要な場合は追加) |
| コンポーネント配置 | `frontend/src/components/charts/` 配下に独立ファイル |
| UMAP チャート PCA 取得 | `clusterStore.pcaProjections` 再利用 → なければ `wasm.runPca(2, 'all')` 直接呼び出し（clusterStore 経由せず） |
| Importance 指標 | Spearman `|ρ|` / Ridge `|β|` の 2 択ドロップダウン |
| Int32Array 変換 | `i32 as usize`（k-means は常に ≥ 0）。HDBSCAN 追加時に `-1` フィルタ追加 |

### 残課題

- `selectionStore.colorMode` と `DimReductionScatter` の連携詳細は実装時に確認（`colormaps.ts` の `getColorForValue` 等）
- `ClusterPanel.tsx` への `estimateK` コールバック接続は `LeftPanel.tsx` 修正時に確認

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 14件
- 🟡 黄信号: 4件
- 🔴 赤信号: 0件

**ヒアリング後**:
- 🔵 青信号: 18件 (+4)
- 🟡 黄信号: 0件 (-4)
- 🔴 赤信号: 0件

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **要件定義**: [requirements.md](../../spec/visualization-enablement/requirements.md)
