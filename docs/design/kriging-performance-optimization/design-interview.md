# Kriging 高速化 設計ヒアリング記録

**作成日**: 2026-04-05
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義書の技術要件（REQ-001〜005）を具体的な設計に落とし込むために、
実装上の重要な技術的判断について確認したヒアリング。
特に viteSingleFile 制約下での Web Worker 実装方針と Sparse GP の設計詳細を確認。

---

## 質問と回答

### Q1: 設計規模

**カテゴリ**: 優先順位
**背景**: 設計の深度を決定するため

**質問**: この設計の作業規模について教えてください
**回答**: フル設計（推奨）

**信頼性への影響**:
- 全設計ドキュメント（architecture.md, dataflow.md, interfaces.ts）を作成する方針確定

---

### Q2: 既存実装の詳細分析

**カテゴリ**: 既存アーキテクチャ確認
**背景**: 既に kriging.rs, pdp.rs, SurfacePlot3D.tsx, analysisStore.ts を読み込み済み。追加調査不要か確認。

**質問**: 既存実装の詳細分析が必要ですか？
**回答**: 不要（推奨）

**信頼性への影響**:
- 既存ファイルの読み込みで十分なコンテキストが確認された

---

### Q3: Web Worker + viteSingleFile の WASM ロード方法

**カテゴリ**: 技術選択（最重要）
**背景**: viteSingleFile は WASM を base64 data URI として単一 HTML に埋め込む。
Worker には独自の WASM インスタンスが必要で、`new Worker(new URL(...))` 構文は
viteSingleFile でバンドル後に問題が生じる可能性がある。

2つの選択肢:
1. **Blob URL 方式**: Worker スクリプトを文字列→Blob→URL として実行時生成
2. **Vite Worker インポート方式**: `new Worker(new URL('./kriging.worker.ts', import.meta.url))`（viteSingleFile 展開時に問題の恐れあり）

**質問**: Web Worker で WASM を実行する際の WASM ロード方法
**回答**: Blob URL 方式（推奨）

**信頼性への影響**:
- architecture.md の Worker 設計が 🔵 に確定
- viteSingleFile 制約下での Worker 起動設計が確定

---

### Q4: Worker 内で呼び出す新 WASM 関数の設計

**カテゴリ**: 技術選択（アーキテクチャ重要）
**背景**: 現在の `computePdp2d` は WASM グローバル状態（`OnceLock<Mutex<GlobalState>>`）から
x_matrix と y を読み取る設計。Worker スレッドはメインスレッドのグローバル状態にアクセスできないため、
データを直接引数で受け取る別関数が必要。

2つの選択肢:
1. **`compute_kriging_raw`**: Kriging 専用。シンプルな引数設計
2. **`compute_pdp_2d_raw`**: 汎用化。model_type も引数に含め将来の全モデル対応可能

**質問**: Worker 内で呼び出す新しい WASM 関数の設計
**回答**: compute_kriging_raw（推奨）

**信頼性への影響**:
- `wasm_compute_kriging_raw(x_flat, y, n_samples, param1_idx, param2_idx, n_grid, model_type)` の
  シグネチャが 🔵 に確定（型定義 interfaces.ts に反映）
- 将来の拡張は `model_type: &str` 引数で対応可能

---

### Q5: Sparse GP 誘導点選択戦略

**カテゴリ**: 未定義部分詳細化
**背景**: Sparse Kriging の品質はM個の誘導点の配置に大きく依存する。
K-means（データ分布を反映）vs 均等グリッド（実装容易だがデータ希薄領域に点を配置）。

**質問**: Sparse Kriging（FITC）の誘導点選択戦略
**回答**: K-means クラスタリング（推奨）

**信頼性への影響**:
- `select_inducing_points_kmeans()` 関数の設計が 🔵 に確定
- Lloyd's アルゴリズム（外部クレート不使用）で実装する方針決定

---

### Q6: 実装フェーズ分け

**カテゴリ**: 優先順位
**背景**: 3 つの最適化アプローチ（アルゴリズム・Worker・Sparse Kriging）の実装順序。
- アルゴリズム先行: 確実に高速化を達成してから Worker を追加
- Worker 先行: UIブロック解消を最優先し、後から高速化

**質問**: 実装フェーズ分け
**回答**: アルゴリズム → Worker → Sparse の順

**信頼性への影響**:
- architecture.md の Phase 1/2/3 の順序が 🔵 に確定
- 各フェーズで独立したテスト・検証が可能な設計を採用

---

## ヒアリング結果サマリー

### 確認できた事項

1. **設計規模**: フル設計（全ドキュメント作成）
2. **Worker 方式**: Blob URL 方式（viteSingleFile 制約対応）
3. **新 WASM 関数**: `compute_kriging_raw`（グローバル状態不要、引数でデータ転送）
4. **Sparse GP 誘導点**: K-means（純 Rust Lloyd's アルゴリズム）
5. **実装順序**: Phase 1（アルゴリズム）→ Phase 2（Worker）→ Phase 3（Sparse）

### 設計方針の決定事項

1. **Worker + viteSingleFile**: Blob URL で Worker ソースを実行時生成。WASM は Worker 内で独立初期化
2. **データ転送**: `Float64Array` Transferable でゼロコピー転送（N=5000 で ~120KB）
3. **Sparse GP**: FITC 近似 + K-means 誘導点（M=50、N < M のとき標準 Kriging フォールバック）
4. **フォールバック**: Worker 初期化失敗 → `surface3dError` セット（Main Thread フォールバックは Phase 2 スコープ外）

### 残課題

1. **Blob URL + vite**: vite-plugin-singlefile でのモジュール Worker 文字列化の具体的な実装方法（実装時に確認）
2. **WASM base64 取得方法**: Worker に渡す WASM base64 文字列のソース（`import wasmUrl from './pkg/tunny_core_bg.wasm?url'` や `?inline` オプション）
3. **FITC vs FSGP**: FITC 以外の Sparse GP 近似（VFE 等）の精度差の検証は Phase 3 実装時に確認

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 8件
- 🟡 黄信号: 8件
- 🔴 赤信号: 4件

**ヒアリング後**:
- 🔵 青信号: 18件 (+10)
- 🟡 黄信号: 11件 (+3)
- 🔴 赤信号: 0件 (-4)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **要件定義**: [requirements.md](../../spec/kriging-performance-optimization/requirements.md)
