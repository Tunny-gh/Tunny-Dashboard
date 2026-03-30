# 可視化機能有効化 ヒアリング記録

**作成日**: 2026-03-29
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の設計文書・Rust 実装・フロントエンドコードを確認し、4 チャート（Importance / SensitivityHeatmap / ClusterView / UMAP）の有効化に必要な不明点や設計方針を明確化するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: この要件の作業規模について教えてください

**カテゴリ**: 作業規模確認
**背景**: kairo-requirements の標準フローとして作業規模を確認

**回答**: フル機能開発（推奨）

**信頼性への影響**: 詳細 EARS 要件定義・ユーザーストーリー・受け入れ基準を全包含する形で進める

---

### Q2: UMAP の実装アプローチ

**カテゴリ**: 技術方針確認
**背景**: `tunny_core.d.ts` に UMAP 計算関数が存在せず、Rust 側にも実装がないため、JS ライブラリ（umap-js）追加か PCA 代替かを確認

**回答**: 「UMAPの利用は過去の仕様で決定したことです。specを確認しその条件を満たすのであればUMAPである必要はありません。適切に過去の仕様を確認して再提案してください」

**仕様確認結果**:
- `tunny-dashboard-requirements.md` REQ-082: 「前処理として PCA（WASM 実装）を適用し、**オプションで** UMAP（umap-js、WebWorker 非同期）を提供しなければならない」
- Phase 12 スコープに UMAP が配置されている
- 結論: **PCA による 2D 次元削減散布図を UMAP チャートとして実装し、UMAP は Phase 12 で追加**。チャートに「UMAP 選択時: Coming Soon」を表示

**信頼性への影響**: REQ-VE-080〜087 が 🔵 青信号（仕様に明示の根拠あり）

---

### Q3: Importance チャートの計算方法

**カテゴリ**: 既存設計確認
**背景**: 現在の実装が全値 1.0 のダミーで、実際に何を使うか確認が必要

**回答**: 「rustで実装されたものをドロップダウンで変更できる形で実装してください」

**確認結果**:
- Rust 実装済み: Spearman 相関係数（REQ-090）、Ridge β 係数（REQ-091）
- MIC（REQ-093）は WebWorker 非同期で別スコープ
- RF 重要度（REQ-094）は .onnx ファイル必要で別スコープ
- SHAP（REQ-095）は shap_values.json 必要で別スコープ
- **本フェーズ**: Spearman |ρ|（目的平均）と Ridge |β|（目的平均）の 2 指標をドロップダウンで切り替え

**信頼性への影響**: REQ-VE-060〜065 が 🔵 青信号

---

### Q4: ClusterView チャートの表示内容

**カテゴリ**: 未定義部分の詳細化
**背景**: `ClusterPanel`（設定 UI）と `ClusterList`（統計テーブル）は実装済みだが、実際の散布図ビジュアライゼーションが未定義

**回答**: PCA 2D 散布図（推奨）— WASM の PCA で全試行を 2 次元化し、クラスタ色分けで表示

**信頼性への影響**: REQ-VE-070〜077 が 🔵 青信号（仕様 REQ-082 の PCA 必須要件と整合）

---

### Q5: Sensitivity / Importance 状態管理

**カテゴリ**: アーキテクチャ確認
**背景**: 現在 `analysisStore` が Zustand 実装として存在しない。型定義（`index.ts`）のみ

**回答**: 専用ストア新設（推奨）— `analysisStore.ts` と `clusterStore.ts` を Zustand ストアとして新設

**信頼性への影響**: REQ-VE-030〜046 が 🔵 青信号（既存ストア実装パターンと整合）

---

## ヒアリング結果サマリー

### 確認できた事項

1. UMAP チャートは PCA 2D 散布図として実現し、umap-js は Phase 12 で追加する方針
2. Importance チャートは Spearman/Ridge の 2 指標をドロップダウンで切り替える
3. ClusterView は PCA 2D 散布図 + クラスタ色分け
4. 新規 Zustand ストア 2 件（analysisStore, clusterStore）を新設
5. SensitivityHeatmap コンポーネントは既に完全実装済み（TASK-802 で完了）

### 追加/変更要件

- REQ-VE-061: Importance 指標ドロップダウン（Spearman / Ridge β）追加
- REQ-VE-081: UMAP チャートに次元削減方式セレクター追加（PCA / UMAP Coming Soon）

### 残課題

- MIC・RF 重要度・SHAP は本スコープ外（別フェーズ）
- UMAP（umap-js + WebWorker）は Phase 12 で実装
- `LeftPanel.tsx` の ClusterPanel 配線については実装時に既存 LeftPanel コードを確認して調整が必要

### 信頼性レベル分布

**ヒアリング前（初期調査時）**:
- 🔵 青信号: 8 件
- 🟡 黄信号: 12 件
- 🔴 赤信号: 2 件

**ヒアリング後（Q1〜Q5 回答反映後）**:
- 🔵 青信号: 20 件 (+12)
- 🟡 黄信号: 5 件 (-7)
- 🔴 赤信号: 0 件 (-2)

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
