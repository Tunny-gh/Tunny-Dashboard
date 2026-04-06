# Kriging 高速化 要件定義書

**作成日**: 2026-04-05

## 概要

現在の Kriging（ガウス過程回帰）サロゲートモデルは、N=1000 サブサンプル後でも
L-BFGS 最大 100 イテレーション × ~6×O(N³) という計算量のため、WASM 主スレッドを
30 秒以上ブロックする可能性がある。

本要件は以下 4 つのアプローチで Kriging 計算を高速化し、UIブロックを解消することを目標とする:
1. Web Worker オフロード（UIブロック完全解消）
2. 計算アルゴリズム最適化（LML/勾配統合・L-BFGS削減・サブサンプル縮小）
3. Sparse Kriging モデル追加（誘導点近似による O(N×M²) 削減）

目標: N≤5000 で **10 秒以内**の応答、UIブロックなし

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)
- **既存 Kriging 実装**: `rust_core/src/kriging.rs`
- **既存理論文書**: `theory/kriging.md`

## 機能要件（EARS 記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・既存実装・設計文書を参考にした確実な要件
- 🟡 **黄信号**: 既存実装・設計文書から妥当な推測による要件
- 🔴 **赤信号**: ヒアリング・実装・設計文書にない推測による要件

---

### REQ-001: Web Worker オフロード

- REQ-001-01: Kriging 計算が開始されたとき、システムは WASM 計算を専用 Web Worker スレッドで実行しなければならない 🔵 *ユーザヒアリング（確実に実装したい）より*
- REQ-001-02: Worker への入力データとして、システムは x_matrix（Float64Array）・y（Float64Array）・param1_idx・param2_idx・objective_idx・n_grid・model_type を postMessage で転送しなければならない 🔵 *WASM全局状態の仕組み（note.md）より*
- REQ-001-03: Worker 内で独立した WASM モジュールインスタンスを初期化し、グローバル状態に依存しない新しいエントリポイント `compute_kriging_direct(x_flat, y, ...)` を呼び出さなければならない 🔵 *WASM全局状態分析より*
- REQ-001-04: 計算完了後、Worker は PdpResult2d 相当のデータを postMessage で返却しなければならない 🔵 *既存 Pdp2dWasmResult 型より*
- REQ-001-05: Worker 計算中、システムはアニメーション付きスピナーを表示し続けなければならない 🔵 *既存 isComputingSurface + spinner 実装より*
- REQ-001-06: viteSingleFile（WASM base64 埋め込み）制約下で動作しなければならない 🟡 *viteSingleFile 制約分析より*

### REQ-002: LML + 勾配の統合計算

- REQ-002-01: システムは L-BFGS 最適化の各イテレーションで、LML 値と勾配を1回の Cholesky 分解・K^{-1}計算で同時に取得しなければならない 🔵 *kriging.rs ボトルネック分析より*
- REQ-002-02: 新関数 `log_ml_with_gradient(x, y, params)` は `(f64, Vec<f64>)` を返し、`neg_lml` と `log_ml_gradient` の個別呼び出しを置き換えなければならない 🔵 *既存 kriging.rs 実装より*
- REQ-002-03: Armijo 線探索での関数評価は引き続き LML のみ（Cholesky + alpha のみ）であることを許容する 🟡 *Armijo 条件定義より（勾配は不要）*

### REQ-003: L-BFGS 反復削減 + 早期停止

- REQ-003-01: システムは `max_iter` をデフォルト 100 から 50 に削減しなければならない 🔵 *ユーザヒアリング（バランス重視）より*
- REQ-003-02: システムは連続 5 イテレーションで LML の変化量が 1e-3 以下のとき L-BFGS を早期停止しなければならない 🟡 *GP最適化の一般的な早期停止基準より*
- REQ-003-03: 既存の勾配ノルム収束条件（`‖∇L‖ < 1e-5`）は維持しなければならない 🔵 *既存 kriging.rs 実装より*

### REQ-004: サブサンプル数削減

- REQ-004-01: システムは N > subsample_n のときのサブサンプル数を 1000 から 500 に変更しなければならない 🔵 *ユーザヒアリング（許容）より*
- REQ-004-02: N ≤ 500 の場合はサブサンプリングを行わずに全点を使用しなければならない 🔵 *既存 train_gp 実装より*

### REQ-005: Sparse Kriging モデル追加

- REQ-005-01: システムは SurrogateModelType に `sparse_kriging` を追加しなければならない 🔵 *ユーザヒアリング（ドロップダウン追加）より*
- REQ-005-02: `sparse_kriging` 選択時、システムは FITC（Fully Independent Training Conditional）近似による Sparse GP を使用しなければならない 🔵 *ユーザヒアリング（Sparse GP 実装）より*
- REQ-005-03: Sparse Kriging は M=50 の誘導点を使用し、計算量を O(N×M²) に削減しなければならない 🟡 *Sparse GP 理論・note.md より妥当な推測*
- REQ-005-04: 誘導点の初期配置は訓練データの K-means または均等グリッドサンプリングで決定しなければならない 🟡 *Sparse GP 一般実装より妥当な推測*
- REQ-005-05: SurfacePlot3D の Model ドロップダウンに "Sparse Kriging" オプションを追加しなければならない 🔵 *ユーザヒアリング（モデル選択ドロップダウンに追加）より*
- REQ-005-06: MODEL_COMPUTE_TIME に `sparse_kriging` の計算時間目安（`< 5s`）を追加しなければならない 🟡 *O(N×M²) 計算量から妥当な推測*

### REQ-006: UI 表示時間の目標値

- REQ-006-01: `kriging` モデルは N=1000 で 10 秒以内に計算結果を返さなければならない 🔵 *ユーザヒアリング（< 10 秒）より*
- REQ-006-02: `sparse_kriging` モデルは N=5000 で 5 秒以内に計算結果を返さなければならない 🟡 *O(N×M²) 計算量試算より妥当な推測*
- REQ-006-03: `ridge` および `random_forest` の既存性能目標（それぞれ < 1s, < 2s）を変更してはならない 🔵 *既存 MODEL_COMPUTE_TIME 定義より*

## 非機能要件

### パフォーマンス

- NFR-001: Kriging (N=1000、REQ-002〜004 適用後)の計算時間は 10 秒以内 🔵 *ユーザヒアリング（< 10 秒）より*
- NFR-002: Sparse Kriging (N=5000) の計算時間は 5 秒以内 🟡 *O(N×M²) 計算量から妥当な推測*
- NFR-003: Web Worker オフロードにより、Kriging 計算中も UI（ドロップダウン操作・スピナー）がフリーズしてはならない 🔵 *ユーザヒアリング（UIブロック解消）より*

### 後方互換性

- NFR-010: `ridge` および `random_forest` の既存機能・テストが全て通ることを確認しなければならない 🔵 *既存テストスイートより*
- NFR-011: 既存の `cacheKey` フォーマット（`{model}_{p1}_{p2}_{obj}_{n}`）は変更してはならない 🔵 *analysisStore.ts 実装より*

### セキュリティ

- NFR-020: Worker への postMessage はオリジンチェックを行い、外部からのメッセージを無視しなければならない 🟡 *Web Worker セキュリティベストプラクティスより妥当な推測*
- NFR-021: 試行データはブラウザ外に送信しない（ブラウザ完結） 🔵 *既存セキュリティ制約より*

## Edge ケース

### エラー処理

- EDGE-001: Worker での WASM 初期化失敗時、システムは Main Thread フォールバック（現在の setTimeout(0) 方式）に切り替え、エラーを `surface3dError` にセットしなければならない 🟡 *既存エラーハンドリングパターンより妥当な推測*
- EDGE-002: Sparse Kriging で K_ZZ が非正定値の場合、ジッターを増加させて再試行しなければならない 🟡 *Cholesky 安定性対策より妥当な推測*
- EDGE-003: N < M（訓練点数が誘導点数より少ない）場合、Sparse Kriging は標準 Kriging にフォールバックしなければならない 🔵 *Sparse GP 制約より*

### 境界値

- EDGE-010: N=3（最小値）で Sparse Kriging が動作しなければならない（標準 Kriging フォールバック） 🟡 *既存 n < 3 チェックより*
- EDGE-011: `max_iter=0` 指定時（テスト用）、L-BFGS を実行せず初期パラメータで予測を行わなければならない 🟡 *既存 optimize_hyperparams ロジックより*
