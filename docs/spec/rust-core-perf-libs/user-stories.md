# rust_core 外部ライブラリ高速化 ユーザストーリー

**作成日**: 2026-05-15
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: コードベース調査・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: コードベース調査から妥当な推測によるストーリー
- 🔴 **赤信号**: コードベース調査にない推測によるストーリー

---

## エピック1: faer 活用拡大（線形代数の高速化）

### ストーリー 1.1: PCA 固有値分解の faer 化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

**私は** Tunny Dashboard 開発者 **として**
**PCA の固有値分解を faer の SIMD 加速実装で高速化したい**
**そうすることで** 高次元データ（p=20+）でのクラスタリング分析がスムーズになる

**関連要件**: REQ-101, REQ-104

**詳細シナリオ**:
1. `clustering/pca.rs` の `PcaResult::compute` を開く
2. Jacobi 固有値分解ループ（~90 行）を `faer::Mat::selfadjoint_eigenvalue_decomposition()` に置き換える
3. 共分散行列を `faer::Mat` として構築する
4. テストを実行し、従来と同一の射影結果を確認する

**前提条件**:
- faer が既に依存関係に含まれている
- 共分散行列の計算結果が従来と同一

**制約事項**:
- 固有値の降順ソートを維持
- 投射データの次元数（n_components）指定を維持

**優先度**: Must Have

---

### ストーリー 1.2: FITC Cholesky の faer 化 🔵

**信頼性**: 🔵 *コードベース調査より*

**私は** Tunny Dashboard 開発者 **として**
**Sparse GP（FITC）の Cholesky 分解と三角 solve を faer に置き換えたい**
**そうすることで** Kriging PDP の大規模データ対応時の計算が SIMD 加速される

**関連要件**: REQ-102, REQ-104

**詳細シナリオ**:
1. `sparse_fitc.rs` の `cholesky_flat` を faer の Cholesky に置き換える
2. `forward_sub_flat` / `backward_sub_flat` を faer の三角 solve に置き換える
3. フラット配列ベースの計算を `faer::Mat` ベースに変更する
4. FITC LML・予測値のテストで同一結果を確認する

**前提条件**:
- カーネル行列が正定値であること（失敗時のエラーハンドリングあり）

**優先度**: Must Have

---

### ストーリー 1.3: Ridge 回帰の faer 化 🔵

**信頼性**: 🔵 *コードベース調査より*

**私は** Tunny Dashboard 開発者 **として**
**Ridge 回帰のガウス消去を faer の Cholesky solve に置き換えたい**
**そうすることで** 感度分析（Ridge）と PDP（Ridge）の回帰計算が高速化される

**関連要件**: REQ-103, REQ-104

**詳細シナリオ**:
1. `pdp/ridge_core.rs` のガウス消去を `faer` Cholesky に置き換える（`(X'X + αI)β = X'y` は α > 0 で常に SPD）
2. `sensitivity/` 内の同等の Ridge 実装も更新する
3. 回帰係数と R² が従来と同一であることを確認する

**優先度**: Must Have

---

### ストーリー 1.4: データレイアウトの faer::Mat 移行 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

**私は** Tunny Dashboard 開発者 **として**
**Vec<Vec<f64>> を faer::Mat に全局的に移行したい**
**そうすることで** メモリコピーのオーバーヘッドがなくなり、全モジュールで一貫した線形代数APIが使える

**関連要件**: REQ-104

**詳細シナリオ**:
1. 各モジュールの入出力型を `Vec<Vec<f64>>` から `faer::Mat` に変更する
2. egui-app 側の呼び出しコードを更新する
3. `linear_algebra.rs` の変換関数を削除する
4. 全テストを通して動作確認する

**前提条件**:
- 各モジュールの faer 対応が完了していること

**優先度**: Must Have

---

## エピック2: argmin 導入（最適化の高速化）

### ストーリー 2.1: L-BFGS の argmin 化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

**私は** Tunny Dashboard 開発者 **として**
**手作り L-BFGS を argmin に置き換えたい**
**そうすることで** GP 超パラメータ最適化の収束性が改善し、メンテナンス負荷が下がる

**関連要件**: REQ-201

**詳細シナリオ**:
1. `argmin` と `argmin-math`（faer backend）を Cargo.toml に追加する
2. `core/optimization/lbfgs.rs` の L-BFGS two-loop recursion を argmin の `LBFGS` に置き換える
3. `core/optimization/line_search.rs` の Armijo backtracking を argmin の line search に置き換える
4. GP 学習（kriging/gaussian_process/training.rs）の呼び出しを argmin ベースに更新する
5. 収束パラメータ（max_iter、tolerance）を従来値に設定する
6. GP 予測結果が従来と同一であることを確認する

**前提条件**:
- argmin が faer backend で動作すること（ndarray 依存でないこと）

**制約事項**:
- L-BFGS + line search のみ導入
- 将来の拡張（制約付き最適化等）はスコープ外

**優先度**: Must Have

---

## エピック3: rand 導入（乱数生成の品質向上）

### ストーリー 3.1: PRNG の rand 統一 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

**私は** Tunny Dashboard 開発者 **として**
**3 種の独自 PRNG を rand + rand_chacha に統一したい**
**そうすることで** 乱数品質が向上し、モンテカルロ法の精度が改善する

**関連要件**: REQ-301

**詳細シナリオ**:
1. `rand` と `rand_chacha` を Cargo.toml に追加する
2. `core/random_forest/rng.rs` の LCG → `ChaCha8Rng` に移行（一時的に core/math/ に配置）
3. `clustering/kmeans.rs` の xorshift64 → `StdRng` に移行（その後 linfa-clustering で削除）
4. `sampling/common.rs` の Fisher-Yates → `SliceRandom::shuffle` に移行
5. 各モジュールでシード再現性を確認する

**前提条件**:
- 各 RNG が決定論的シードで初期化可能であること

**優先度**: Must Have

---

## エピック4: linfa-clustering 導入（K-means 統合）

### ストーリー 4.1: K-means の linfa-clustering 化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

**私は** Tunny Dashboard 開発者 **として**
**2 つの重複 K-means 実装を linfa-clustering に統合したい**
**そうすることで** 重複コードがなくなり、保守性と品質が向上する

**関連要件**: REQ-401

**詳細シナリオ**:
1. `linfa-clustering` を Cargo.toml に追加する
2. `clustering/kmeans.rs` の公開 API を linfa-clustering バックエンドに置き換える
3. `sparse_fitc.rs` 内の K-means を clustering モジュールの統一 API を呼び出すように変更する
4. エルボー法が linfa-clustering で動作することを確認する
5. クラスタリング結果の型（`clustering/types.rs`）を維持する

**前提条件**:
- linfa-clustering が K-means++ 初期化をサポートしていること
- 結果型の互換性が維持されること

**優先度**: Must Have

---

## エピック5: デッドコード削除

### ストーリー 5.1: Random Forest 実装の削除 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

**私は** Tunny Dashboard 開発者 **として**
**未使用の Random Forest 実装を削除したい**
**そうすることで** コードベースが整理され、コンパイル時間が短縮される

**関連要件**: REQ-501

**詳細シナリオ**:
1. `core/random_forest/` ディレクトリ内の tree.rs, forest.rs, types.rs, tests.rs を削除する
2. rng.rs の LCG を core/math/ に一時移動する（rand 移行完了まで）
3. `core/mod.rs` から random_forest モジュールの宣言を削除する
4. 他モジュールからの参照（LCG のみ）が新配置で動作することを確認する

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: faer 活用拡大
├── ストーリー 1.1 PCA 固有値分解 (🔵 Must Have)
├── ストーリー 1.2 FITC Cholesky (🔵 Must Have)
├── ストーリー 1.3 Ridge 回帰 (🔵 Must Have)
└── ストーリー 1.4 データレイアウト移行 (🔵 Must Have)

エピック2: argmin 導入
└── ストーリー 2.1 L-BFGS 置き換え (🔵 Must Have)

エピック3: rand 導入
└── ストーリー 3.1 PRNG 統一 (🔵 Must Have)

エピック4: linfa-clustering 導入
└── ストーリー 4.1 K-means 統合 (🔵 Must Have)

エピック5: デッドコード削除
└── ストーリー 5.1 Random Forest 削除 (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 8 件 (100%)
- 🟡 黄信号: 0 件 (0%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: ✅ 高品質 — すべてのストーリーがコードベース調査とユーザヒアリングに基づいている。
