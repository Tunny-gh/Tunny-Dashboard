# 高速描画ダウンサンプリング 設計ヒアリング記録

**作成日**: 2026-04-07
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の要件定義・設計文書を確認し、ダウンサンプリング設計に必要な不明点や設計方針を確定するためのヒアリングを実施。

---

## 質問と回答

### Q1: 設計の作業規模

**カテゴリ**: 設計方針
**背景**: フル設計（全ファイル）か軽量設計（最小限）かで出力物の粒度が変わる

**回答**: フル設計（推奨）

**信頼性への影響**:
- architecture.md・dataflow.md・interfaces.ts を全て作成。詳細な型定義も含める

---

### Q2: 対象チャート

**カテゴリ**: 機能スコープ
**背景**: deck.gl（WebGL）は50,000点 60fps が既存目標（NFR-001）だが、視認性の問題で全チャート共通化が必要か確認

**回答**: すべてのプロットに共通。点が多すぎると見づらい

**信頼性への影響**:
- スコープが「全チャート共通」に確定。downsampleStore は全9チャートに適用される
- ScatterMatrix 以外（ParetoScatter2D/3D・ObjectivePairMatrix・PCP・SlicePlot・SurfacePlot3D・ClusterScatter）にも同設計が適用されることが確定

---

### Q3: ダウンサンプリングの主な目的

**カテゴリ**: パフォーマンス
**背景**: 目的によってアルゴリズム選択が変わる（学習高速化 vs 描画点数削減）

**回答**: 描画点数を削減しWebGL負荷軽減

**信頼性への影響**:
- サロゲートモデル学習への適用は対象外に確定
- `downsampleStore` は GPU バッファへの renderIndices 提供が主目的に確定

---

### Q4: ダウンサンプリングの保持点公算（Pareto保持）

**カテゴリ**: アルゴリズム設計
**背景**: Pareto Rank1 は多目的最適化の最重要情報。保持するかどうかで視認性と公平性が変わる

**回答**: Pareto点必須保持（推奨）

**信頼性への影響**:
- `downsample_smart(include_pareto=true)` が主要アルゴリズムに確定
- `compute_pareto_ranks()` 完了前に downsampleStore が動作できないという制約が発生（Architecture に記載）

---

### Q5: 描画点数の上限

**カテゴリ**: 性能要件
**背景**: 一律の上限か、チャート別の上限かで設計複雑度が変わる

**回答**: チャート別に適切な値

**信頼性への影響**:
- `downsampleStore` が 6 種のキャッシュを管理する設計に確定
- 点数上限の具体的な数値は設計チームで決定（architecture.md の表に記載）

---

### Q6: 既存コード分析の必要性

**カテゴリ**: 設計プロセス
**背景**: sampling.rs は現在プレースホルダー（`pub struct Sampler;`）のみ。実装パターンは他のモジュールから踏襲可能

**回答**: 不要（推奨）

**信頼性への影響**:
- 実装パターンは `sensitivity.rs`・`clustering.rs` の既存パターンを参照することで設計可能

---

## ヒアリング結果サマリー

### 確認できた事項
- 全チャートに共通のダウンサンプリング機能が必要（視認性 + WebGL 負荷軽減）
- Pareto Rank1 点は必ず保持する（多目的最適化の主要情報を守る）
- 描画目的（GPU renderIndices）がメインで、サロゲート学習への適用は対象外
- チャート別に異なる上限点数を設定する

### 設計方針の決定事項
1. **sampling.rs** に 4 つの WASM 関数を実装
2. **downsampleStore.ts** で 6 種インデックスキャッシュを管理
3. Study 選択時・フィルタ大幅変化時に再計算
4. 全チャートコンポーネントが `getIndices(key)` で取得

### チャート別上限点数（設計決定値）

| キー | 上限 | 対象チャート |
|---|---|---|
| `scatter` | 10,000 | ParetoScatter2D/3D・ObjectivePairMatrix |
| `thumbnail` | 500 | ScatterMatrix サムネイル |
| `hover` | 3,000 | ScatterMatrix ホバー拡大 |
| `pcp` | 5,000 | ParallelCoordinates |
| `data_points` | 5,000 | SlicePlot・SurfacePlot3D 実測点 |
| `cluster` | 10,000 | ClusterScatter・DimReductionScatter |

### 残課題
- 各チャートの具体的な `getIndices(key)` 統合コードは TASK 分割後に実装
- フィルタ変化時の再計算閾値（±20%）は実装時に調整余地あり
- `downsample_by_cluster` はクラスタラベルがある場合のみ動作（フォールバック実装が必要）

---

### 信頼性レベル分布

**ヒアリング前**（要件定義のみからの推測）:
- 🔵 青信号: 5件
- 🟡 黄信号: 5件
- 🔴 赤信号: 2件

**ヒアリング後**:
- 🔵 青信号: 17件 (+12)
- 🟡 黄信号: 2件 (-3)
- 🔴 赤信号: 0件 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **要件定義**: [requirements.md](../../spec/tunny-dashboard-requirements.md)
