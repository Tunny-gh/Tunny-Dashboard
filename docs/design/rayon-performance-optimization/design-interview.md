# rayon 導入による並列化高速化 設計ヒアリング記録

**作成日**: 2026-05-04  
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

---

## ヒアリング目的

コードベース調査（`sensitivity/`・`pdp/`・`core/random_forest/`）で把握した既存実装の
スレッドセーフ性・設計パターンを踏まえ、各並列化戦略の具体的な実装方針を確定するための
ヒアリングを実施した。

---

## 質問と回答

### Q1: RandomForest 並列化時のシード生成方式

**質問日時**: 2026-05-04  
**カテゴリ**: 技術選択  
**背景**: 現行 `RandomForest::train` は 1 つの `Lcg::new(seed)` を順次更新して木ごとの
bootstrap サンプリングに使用している。並列化では各木が独立した RNG を必要とするため、
シード生成方式を決める必要があった。

**回答**: 「**seed + tree_index**」— `seed.wrapping_add(tree_idx as u64)` で各木の `Lcg` を初期化する。

**信頼性への影響**:
- REQ-102（決定論的シード設計）の信頼性レベルが 🟡 → 🔵 に向上
- `architecture.md` § 3「シード設計」に反映済み

---

### Q2: Permutation 特徴量ループの並列化設計

**質問日時**: 2026-05-04  
**カテゴリ**: アーキテクチャ  
**背景**: 現行実装は `x_eval_work` を 1 つ in-place 更新しながら特徴量ループを回す設計
（`O(1)` アロケーションが目的）。並列化には設計変更が必要で 2 つの選択肢があった。

**回答**: 「**特徴量ごとに x_eval コピー**」— 各スレッドが `x_eval.to_vec()` のローカルコピーを保持する。

**信頼性への影響**:
- REQ-103（コピーによる競合排除）の信頼性レベルが 🟡 → 🔵 に向上
- `architecture.md` §4・`dataflow.md` §4 に反映済み

---

### Q3: Sobol 並列化の範囲（サロゲート構築のみ vs 指標計算まで）

**質問日時**: 2026-05-04  
**カテゴリ**: 未定義部分詳細化  
**背景**: Sobol 計算は 3 段階あり（① サロゲート構築、② f_a/f_b 評価、③ per-param 指標計算）。
どこまで並列化するかでコード変更量と効果が異なる。

**回答**: 「**指標計算まで並列化**」— 全 3 段階の objectives / params ループを並列化する。

**信頼性への影響**:
- REQ-002 の適用範囲が「`build_sobol_surrogate` のみ」→「`f_a`/`f_b` + per-param ループも含む」に拡大
- `architecture.md` §2 に 3 段階の設計を記載
- **追加リファクタリング必要**: Sobol 指標計算ロジックを `compute_first_order` / `compute_total_effect` ヘルパー関数に抽出してから並列化

---

### Q4: criterion ベンチマーク対象関数

**質問日時**: 2026-05-04  
**カテゴリ**: 追加要件  
**背景**: ベンチマーク対象を全 4 関数にするか一部に絞るかを確認した。

**回答**: **全 4 関数**（Sensitivity・Sobol・RandomForest::train・Permutation）を対象とする。

**信頼性への影響**:
- NFR-002 のカバレッジが全 4 関数に確定（信頼性レベル: 🔵）
- `architecture.md` §「ベンチマーク設計」に 4 ファイルの一覧を記載

---

## ヒアリング結果サマリー

### 確認できた事項
1. RandomForest のシードは `seed.wrapping_add(tree_idx as u64)` で決定論的
2. Permutation の in-place 設計を「特徴量ごとの x_eval コピー」に変更することを許可
3. Sobol は 3 段階すべての objectives/params ループを並列化する
4. ベンチマークは全 4 関数対象

### 設計決定事項
- `RandomForest::train`: `(0..n_trees).into_par_iter()` + 木ごとシード
- `Permutation`: `(0..p).into_par_iter()` + スレッドローカル `x_work`
- `Sobol`: `build_sobol_surrogate` 内 Ridge + `f_a`/`f_b` + per-param 指標の 3 段階並列
- ベンチマーク: `benches/sensitivity_bench.rs`, `sobol_bench.rs`, `rf_bench.rs`, `permutation_bench.rs` の 4 ファイル新規追加

### 追加判明した実装上の注意
- `run_tree_metric_for_all_objectives<M>` の where 節に `M: Sync` を追加が必要（rayon の `par_iter` が `Sync` を要求するため）
- Sobol の `compute_first_order` / `compute_total_effect` ヘルパー抽出が並列化前の前提作業として必要
- `mat_a` / `mat_b` の LCG サンプリングは RNG 状態の順次依存があるため直列維持

### 残課題
- `TreeMetric` トレイトが `Sync` を自動実装できるか確認（実装に純粋な `fn` のみなら問題なし）
- Sobol per-param ループ内の `f_a`, `f_b` を `Arc` 等で共有するか、コピーで渡すかを実装時に確認

### 信頼性レベル分布

**ヒアリング前（設計段階）**:
- 🔵 青信号: 12
- 🟡 黄信号: 8
- 🔴 赤信号: 0

**ヒアリング後**:
- 🔵 青信号: 18 (+6)
- 🟡 黄信号: 2 (-6)
- 🔴 赤信号: 0

---

## 関連文書

- **要件定義書**: [requirements.md](../../spec/rayon-performance-optimization/requirements.md)
- **アーキテクチャ**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
