# rust_core 外部ライブラリによる高速化 要件定義書

**作成日**: 2026-05-15

## 概要

`rust_core` クレートは現在 `faer` / `serde` / `serde_json` / `rayon` のみに依存し、
PCA 固有値分解（Jacobi 法）、FITC Sparse GP の Cholesky 分解、Ridge 回帰のガウス消去、
L-BFGS 最適化、K-means クラスタリング、PRNG 等がすべて自前実装されている。
特に `faer` は既に依存しているにもかかわらず、`linear_algebra.rs` のみで使用され、
他のモジュールでは独自のスカラー実装が重複している。

本要件は以下の外部ライブラリ導入により計算処理を高速化する:

1. **faer 活用拡大** — PCA 固有値分解、FITC Cholesky/三角solve、Ridge 回帰を faer に統一
2. **argmin 導入** — 手作り L-BFGS + Armijo を argmin の L-BFGS に置き換え
3. **rand 導入** — 3 種の独自 PRNG を rand + rand_chacha に統一
4. **linfa-clustering 導入** — 2 つの重複 K-means 実装を外部 crate に統合
5. **デッドコード削除** — 未使用の Random Forest 実装（#[allow(dead_code)]）を除去
6. **データレイアウト移行** — `Vec<Vec<f64>>` を `faer::Mat` に全局的に移行し、API も含めたリファクタリングを実施

目標: 計算ヘビーパス（PCA、GP 学習、Ridge、K-means）で **3-50x の高速化** を達成

## 関連文書

- **ヒアリング記録**: [interview-record.md](interview-record.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
- **既存関連要件**:
  - [kriging-performance-optimization](../kriging-performance-optimization/requirements.md)
  - [rayon-performance-optimization](../rayon-performance-optimization/requirements.md)

## 機能要件（EARS 記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コードベース調査・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: コードベース調査から妥当な推測による要件
- 🔴 **赤信号**: コードベース調査にない推測による要件

---

### EP-1: faer 活用拡大（線形代数の高速化）

#### REQ-101: PCA 固有値分解の faer 化

- REQ-101-01: システムは `clustering/pca.rs` の Jacobi 固有値分解（~90 行）を `faer::Mat::selfadjoint_eigenvalue_decomposition()` に置き換えなければならない 🔵 *コードベース調査（Jacobi 法の純スカラー実装確認）より*
- REQ-101-02: 置き換え後の PCA は固有値・固有ベクトルの並び順（降順）を従来と同一に維持しなければならない 🔵 *既存 PCA API（projected データの順序）より*
- REQ-101-03: 置き換え後の PCA は入力データの正規化・中心化の振る舞いを従来と同一に維持しなければならない 🔵 *既存 pca.rs の前処理ロジックより*

#### REQ-102: FITC Sparse GP Cholesky の faer 化

- REQ-102-01: システムは `core/kriging/sparse_fitc.rs` の手作り `cholesky_flat` / `forward_sub_flat` / `backward_sub_flat`（~90 行）を `faer` の Cholesky 分解・三角 solve に置き換えなければならない 🔵 *コードベース調査（重複実装確認）より*
- REQ-102-02: FITC の Woodbury 恒等式に基づく LML 計算は従来と同一の数値結果を返さなければならない 🔵 *sparse_fitc.rs の LML 計算ロジックより*
- REQ-102-03: FITC の予測（predict_mean / predict_var）は従来と同一の数値結果を返さなければならない 🔵 *sparse_fitc.rs の inference 実装より*

#### REQ-103: Ridge 回帰の faer 化

- REQ-103-01: システムは `pdp/ridge_core.rs` および `sensitivity/` 内の Ridge 回帰における手作りガウス消去（~45 行）を `faer` の Cholesky solve に置き換えなければならない 🔵 *コードベース調査より（正則化項 αI > 0 により X'X + αI は常に SPD、Cholesky が最適）*
- REQ-103-02: Ridge 回帰の R² 計算は従来と同一の結果を返さなければならない 🔵 *既存 ridge_core.rs の R² 実装より*

#### REQ-104: データレイアウトの faer::Mat 移行

- REQ-104-01: システムは `Vec<Vec<f64>>` を主要な行列型として `faer::Mat` に全局的に移行しなければならない 🔵 *ユーザヒアリング（全局的移行を選択）より*
- REQ-104-02: 各モジュール間のインターフェース（関数引数・戻り値）を `faer::Mat` ベースに更新しなければならない 🔵 *ユーザヒアリング（API 含むリファクタを選択）より*
- REQ-104-03: 移行により `linear_algebra.rs` の `Vec<Vec<f64>> → faer::Mat` 変換関数は不要となり削除しなければならない 🟡 *移行の結果として妥当な推測*

---

### EP-2: argmin 導入（最適化の高速化）

#### REQ-201: L-BFGS の argmin 化

- REQ-201-01: システムは `core/optimization/lbfgs.rs` の手作り L-BFGS two-loop recursion（~60 行）を `argmin` の `LBFGS` solver に置き換えなければならない 🔵 *コードベース調査・ユーザヒアリングより*
- REQ-201-02: システムは `core/optimization/line_search.rs` の手作り Armijo backtracking（~28 行）を `argmin` の line search に置き換えなければならない 🔵 *コードベース調査より*
- REQ-201-03: argmin 導入範囲は L-BFGS + line search のみとし、他の argmin 機能の導入はスコープ外とする 🔵 *ユーザヒアリング（最適化のみ選択）より*
- REQ-201-04: GP 超パラメータ最適化の収束条件（最大イテレーション数・勾配ノルム閾値）は従来の設定値を維持しなければならない 🔵 *既存 kriging 設定値より*
- REQ-201-05: FITC の超パラメータ最適化に使用される数値勾配（中心差分）は、argmin の `finite_diff` への移行は**スコープ外**とし、既存の中心差分実装を維持しなければならない 🔵 *実装スコープを明確化（「してもよい」は要件として不適切なため「する/しない」を決定）*

---

### EP-3: rand 導入（乱数生成の品質向上）

#### REQ-301: 独自 PRNG の rand 統一

- REQ-301-01: システムは `core/random_forest/rng.rs` の `Lcg`（LCG 実装）を `rand_chacha::ChaCha8Rng` に置き換えなければならない 🔵 *コードベース調査（3 種の独自 PRNG 確認）・ユーザヒアリングより*
- REQ-301-02: システムは `clustering/kmeans.rs` の xorshift64 PRNG を `rand::rngs::StdRng` または `ChaCha8Rng` に置き換えなければならない 🔵 *コードベース調査より*
- REQ-301-03: システムは `sampling/common.rs` の Fisher-Yates シャッフルで使用する LCG を `rand::seq::SliceRandom::shuffle` に置き換えなければならない 🔵 *コードベース調査より*
- REQ-301-04: 各 RNG は決定論的シードによる再現性を維持しなければならない 🔵 *既存 LCG のシード設定・ベンチマーク再現性より*
- REQ-301-05: Sobol 感度分析のモンテカルロサンプリングは引き続き現在の LCG ベースで動作し、低差異列（Sobol 列）の導入はスコープ外とする 🟡 *ユーザーの選択範囲外・将来の改善候補として妥当な推測*

---

### EP-4: linfa-clustering 導入（K-means の統合）

#### REQ-401: K-means の外部 crate 化

- REQ-401-01: システムは `clustering/kmeans.rs`（~302 行）の K-means Lloyd 法実装を `linfa-clustering` の `KMeans` に置き換えなければならない 🔵 *コードベース調査・ユーザヒアリング（linfa-clustering 導入選択）より*
- REQ-401-02: システムは `core/kriging/sparse_fitc.rs` 内の重複 K-means 実装（~135 行）を削除し、linfa-clustering に統一しなければならない 🔵 *コードベース調査（2 箇所の重複確認）より*
- REQ-401-03: K-means++ 初期化は linfa-clustering のデフォルト初期化を使用しなければならない 🔵 *既存 kmeans.rs の k-means++ 実装との互換性より*
- REQ-401-04: エルボー法による最適 k 推定は linfa-clustering の KMeans を用いたループとして維持しなければならない 🔵 *既存 elbow 実装の振る舞いより*
- REQ-401-05: クラスタリング結果（重心・割り当て・WCSS）のデータ型は `clustering/types.rs` の既存型を維持しなければならない 🟡 *API リファクタを考慮しつつ、上位モジュールへの影響を最小化する推測*

---

### EP-5: デッドコード削除

#### REQ-501: Random Forest 実装の削除

- REQ-501-01: システムは `core/random_forest/tree.rs`（~159 行）を削除しなければならない 🔵 *コードベース調査（#[allow(dead_code)]確認）・ユーザヒアリングより*
- REQ-501-02: システムは `core/random_forest/forest.rs`（~84 行）を削除しなければならない 🔵 *コードベース調査より*
- REQ-501-03: システムは `core/random_forest/types.rs`（~24 行）を削除しなければならない 🔵 *コードベース調査より*
- REQ-501-04: `core/random_forest/rng.rs` の LCG は REQ-301 で rand に移行されるまで一時的に別モジュール（例: `core/math/rng.rs`）に移動し、全モジュールの rand 移行完了後に削除しなければならない 🟡 *移行段階の依存関係から妥当な推測*
- REQ-501-05: `core/random_forest/tests.rs` も併せて削除しなければならない 🔵 *テストも dead code の一部として確認*

---

### 制約要件

#### REQ-601: ライセンス互換性

- REQ-601-01: 新規導入するすべての crate（argmin、rand、rand_chacha、linfa-clustering）は MIT または Apache-2.0 ライセンスでなければならない 🔵 *プロジェクトライセンス（MIT）との互換性確認より*

#### REQ-602: 既存ベンチマークの維持

- REQ-602-01: 既存の criterion ベンチマーク（sampling_bench、sensitivity_bench、sobol_bench、rf_bench、permutation_bench）は置き換え後も動作し、性能比較が可能でなければならない 🔵 *既存 Cargo.toml bench 定義より*

#### REQ-603: WASM 非対応

- REQ-603-01: 新規導入 crate は WASM 対応を考慮しなくてよい 🔵 *プロジェクト方針（ネイティブデスクトップアプリ・WASM 廃止）より*

---

## 非機能要件

### パフォーマンス

- NFR-001: PCA 固有値分解は p=20 次元において従来の Jacobi 法より **5x 以上** 高速でなければならない 🟡 *faer SIMD 加速効果から妥当な推測*
- NFR-002: FITC Cholesky 分解は M=100 誘導点において従来のスカラー実装より **3x 以上** 高速でなければならない 🟡 *faer SIMD 加速効果から妥当な推測*
- NFR-003: Ridge 回帰は p=50 特徴量において従来のガウス消去より **3x 以上** 高速でなければならない 🟡 *faer QR/Cholesky 加速効果から妥当な推測*
- NFR-004: Vec<Vec<f64>> → faer::Mat 移行により、行列データのメモリコピーが削減されなければならない 🔵 *連続メモリレイアウトによるキャッシュ効率改善より*

### 品質

- NFR-101: 既存の全テストケースが置き換え後も通過しなければならない 🔵 *既存テストスイートの完全性より*
- NFR-102: 数値精度は既存実装と同等（相対誤差 1e-10 以内）を維持しなければならない 🟡 *faer の数値精度が高いことから妥当な推測*

## Edgeケース

### エラー処理

- EDGE-001: linfa-clustering の KMeans が空クラスタを生成した場合、システムはフォールバックとして再初期化を試みなければならない 🟡 *K-means の既知の収束問題から妥当な推測*
- EDGE-002: argmin の L-BFGS が最大イテレーションに達した場合、最良のパラメータを返さなければならない 🔵 *既存 L-BFGS の振る舞いより*
- EDGE-003: faer の Cholesky 分解が失敗（非正定値行列）した場合、システムは適切なエラーを返さなければならない 🔵 *GP 学習時のカーネル行列条件不良ケースより*

### 境界値

- EDGE-101: データ点数 N=1 の場合、PCA はエラーを返すかnoopでなければならない 🟡 *分散が計算できないことから妥当な推測*
- EDGE-102: クラスタ数 k=1 の場合、K-means は全データの重心を返さなければならない 🔵 *既存 kmeans.rs の振る舞いより*
- EDGE-103: 特徴量数 p=0 の場合、Ridge 回帰は空結果を返さなければならない 🟡 *境界値ケースの推測*

---

## 信頼性レベル分布

- 🔵 青信号: 39 件 (80%)
- 🟡 黄信号: 9 件 (18%)
- 🔴 赤信号: 1 件 (2%)

**品質評価**: ✅ 高品質 — コードベースの詳細調査に基づき、大部分が確実な要件。黄信号は主にパフォーマンス目標値と境界値ケースの推測。REQ-201-05 はスコープ外として確定（🔵 に変更）。

## 導入crate一覧

| Crate | 用途 | ライセンス | 導入対象モジュール |
|-------|------|-----------|-------------------|
| faer (既存) | Cholesky、QR、固有値分解 | MIT | pca.rs, sparse_fitc.rs, ridge_core.rs, 全体データレイアウト |
| argmin 0.11 | L-BFGS 最適化 | MIT/Apache-2.0 | core/optimization/, kriging/ |
| rand 0.9 | 乱数生成インターフェース | MIT/Apache-2.0 | 全モジュールの PRNG 置換 |
| rand_chacha 0.9 | ChaCha8 PRNG 実装 | MIT/Apache-2.0 | 全モジュールの PRNG 置換 |
| linfa-clustering 0.8 | K-means クラスタリング | MIT/Apache-2.0 | clustering/, sparse_fitc.rs |
