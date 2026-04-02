# Sobol感度指数による重要度分析 設計ヒアリング記録

**作成日**: 2026-04-01
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の感度分析実装（Spearman相関・Ridge回帰）を基盤として、Sobol指数を追加するための
不明点・設計判断事項を明確化するためのヒアリングを実施した。

---

## 質問と回答

### Q1: 設計の作業規模

**質問日時**: 2026-04-01
**カテゴリ**: プロセス
**背景**: 設計文書の詳細度を決定する必要があった

**回答**: フル設計（推奨）

**信頼性への影響**:
- 全設計項目（architecture.md, dataflow.md, interfaces.ts）を包括的に作成する方針が確定
- 信頼性レベル: 🔵

---

### Q2: 既存実装の詳細分析の要否

**質問日時**: 2026-04-01
**カテゴリ**: コード分析
**背景**: 設計品質を高めるために既存コードを調査すべきか判断が必要だった

**回答**: 必要

**信頼性への影響**:
- 既存の `SensitivityResult`, `SensitivityMetric`, `RidgeResult`, `wasmLoader.ts`, `analysisStore.ts`, `ImportanceChart.tsx` の実装を詳細に把握できた
- 既存の `compute_sensitivity()` の処理フロー（Z スコア標準化 → Ridge → spearman）を確認し、Sobol 設計の基盤とした
- 信頼性レベル: 🔵

---

### Q3: Sobol指数の計算方式

**質問日時**: 2026-04-01
**カテゴリ**: アルゴリズム選択
**背景**: Optuna データは均一サンプリングではないため、真のSobol計算（Saltelli行列法）を直接適用できない。代替手法の選定が必要だった

**選択肢**:
- サロゲート法（Ridge + Saltelli サンプリング）
- 分散分解法（純粋データ・ビン分割）
- RBD-FAST法

**回答**: サロゲート法（推奨）

**信頼性への影響**:
- Ridge サロゲートを構築し、Saltelli 行列でサロゲートを評価する方針が確定
- 既存の `compute_ridge()` 実装を流用できる
- 信頼性レベル: 🔵

---

### Q4: SensitivityMetric への統合方式

**質問日時**: 2026-04-01
**カテゴリ**: アーキテクチャ
**背景**: Sobol を既存メトリクスの一選択肢として追加するか、独立した新機能として設計するか

**選択肢**:
- 既存メトリクスに追加（`SensitivityMetric` 型を拡張）
- 独立した新機能（`SobolResult` 専用 Store・UI）

**回答**: 既存メトリクスに追加（推奨）

**信頼性への影響**:
- `SensitivityMetric` 型に `'sobol_first' | 'sobol_total'` を追加する方針が確定
- `ImportanceChart.tsx` での表示切り替えも既存の分岐に追加
- `AnalysisStore` に `sobolResult` フィールドと `computeSobol()` を追加
- 信頼性レベル: 🔵

---

### Q5: サロゲートモデルの設計（交差項の有無）

**質問日時**: 2026-04-01
**カテゴリ**: アルゴリズム詳細
**背景**: 線形 Ridge のみでは S_i = ST_i となり、一次指数と全効果指数を区別できない。交差項を追加する二次 Ridge を採用するかどうか

**選択肢**:
- 二次 Ridge（x_i², x_i×x_j の交差項付き、S_i ≠ ST_i を実現）
- 線形 Ridge（高速だが S_i = ST_i）

**回答**: 二次 Ridge（推奨）

**信頼性への影響**:
- 二次特徴量の構成: 線形 p 項 + 二乗 p 項 + 交差 p(p-1)/2 項 の設計が確定
- p=30 の場合 495 特徴量。Ridge に追加のメモリが必要だが許容範囲内
- 信頼性レベル: 🔵

---

### Q6: Saltelli サンプリングのデフォルト N 値

**質問日時**: 2026-04-01
**カテゴリ**: パフォーマンス
**背景**: N が大きいほど精度が高いが計算時間が増加する。デフォルト値の決定

**選択肢**:
- N=512（軽量）
- N=1024（推奨）
- N=2048（高精度）

**回答**: N=1024（推奨）

**信頼性への影響**:
- p=30, N=1024 で評価回数 = 1024 × 62 = 63,488 回
- サロゲート評価コスト ≈ 16M ops、目標 2000ms 以内
- 信頼性レベル: 🔵

---

## ヒアリング結果サマリー

### 確認できた事項
- サロゲート法（二次 Ridge + Saltelli）で一次・全効果 Sobol 指数を実現する
- 既存の `SensitivityMetric` 体系に統合し、`ImportanceChart.tsx` で切り替え可能にする
- デフォルト N=1024、PRNG は LCG64 の自前実装（外部クレート不要）
- 新規ファイル不要、既存ファイルへの追加変更のみで実装可能

### 設計方針の決定事項
1. **Rust**: `sensitivity.rs` に `SobolResult`, `SobolSurrogate`, `compute_sobol()` を追加
2. **Rust**: `lib.rs` に `wasm_compute_sobol(n_samples: u32)` バインディングを追加
3. **TS**: `wasmLoader.ts` に `SobolWasmResult` 型と `computeSobol()` メソッドを追加
4. **TS**: `analysisStore.ts` に `sobolResult`, `isComputingSobol`, `computeSobol()` を追加
5. **TS**: `SensitivityMetric` 型を拡張: `'sobol_first' | 'sobol_total'` を追加
6. **TS**: `ImportanceChart.tsx` に `sobol_first` / `sobol_total` の score 計算を追加

### 残課題
- なし（すべての設計判断が確定済み）

---

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 2件（アーキテクチャ方針、既存コード）
- 🟡 黄信号: 8件（アルゴリズム選択、統合方式、パフォーマンス等の未確認項目）
- 🔴 赤信号: 2件（サロゲートモデル詳細、N値）

**ヒアリング後**:
- 🔵 青信号: 12件 (+10)
- 🟡 黄信号: 0件 (-8)
- 🔴 赤信号: 0件 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **既存感度分析 WASM API**: [../tunny-dashboard/wasm-api.md](../tunny-dashboard/wasm-api.md)
- **既存型定義**: [../tunny-dashboard/interfaces.ts](../tunny-dashboard/interfaces.ts)
