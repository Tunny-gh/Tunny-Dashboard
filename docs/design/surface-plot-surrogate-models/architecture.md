# 3D Surface Plot サロゲートモデル拡張 アーキテクチャ設計

**作成日**: 2026-04-05
**関連設計（前フェーズ）**: [mode-frontier-features/architecture.md](../mode-frontier-features/architecture.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:

- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 _ユーザヒアリング・design-interview.md より_

`SurfacePlot3D` チャートが現在サポートする Ridge 回帰サロゲートモデルに加え、**Random Forest（CART + Bagging）** および **Kriging（ARD Matérn 5/2 カーネルによる完全ガウス過程回帰）** の 2 モデルを追加する。

既存の `computePdp2d` WASM 関数に `model_type` パラメータを追加し、モデル種別に応じてアルゴリズムをディスパッチする設計とする。サーバー不要・ブラウザ完結の制約は維持する。

### 現状と変更範囲

| 項目                | 現状                               | 変更後                                            |
| ------------------- | ---------------------------------- | ------------------------------------------------- |
| WASM 関数シグネチャ | `computePdp2d(p1, p2, obj, nGrid)` | `computePdp2d(p1, p2, obj, nGrid, modelType)`     |
| 利用可能モデル      | Ridge のみ                         | Ridge / Random Forest / Kriging                   |
| Rust ファイル       | `pdp.rs`（Ridge のみ）             | `pdp.rs` + `rf.rs`（新規） + `kriging.rs`（新規） |
| UI 状態             | RF/Kriging は `disabled: true`     | `disabled` 解除                                   |

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 _mode-frontier-features アーキテクチャ設計・ユーザヒアリングより_

既存の 5 層レイヤードアーキテクチャを踏襲し、**WASM Core（Rust）層のみに実装を追加**する。

```
WASM Core (rust_core)          ← 実装追加：rf.rs / kriging.rs 新規、pdp.rs / lib.rs 修正
  ↓ wasm-bindgen
wasmLoader.ts                  ← computePdp2d シグネチャ変更（modelType 引数追加）
  ↓
analysisStore.ts               ← surrogateModelType を WASM に渡す修正
  ↓
SurfacePlot3D.tsx              ← RF / Kriging の disabled: true を削除
```

---

## コンポーネント構成

### 層 1: WASM Core（Rust） 🔵

**信頼性**: 🔵 _ユーザヒアリング・既存 pdp.rs パターンより_

#### 1a: `rust_core/src/rf.rs`（新規）

CART（Classification and Regression Tree）+ Bagging によるランダムフォレスト実装。外部クレート不使用・純 Rust。

```
pub struct RandomForest {
    trees: Vec<DecisionTree>,
}

enum TreeNode {
    Leaf(f64),
    Split { feature: usize, threshold: f64, left: Box<TreeNode>, right: Box<TreeNode> },
}
```

**学習アルゴリズム:**

1. N 個の訓練点から Bootstrap サンプリング（復元抽出）で N 点を取得
2. 各分岐で `max(1, p/3)` 個のランダム特徴量を候補に使用
3. MSE（平均二乗誤差）最小化による最良分岐点を探索
4. `max_depth` または `min_samples_leaf` に達したら葉ノードとし、平均値を格納
5. 上記を `n_trees` 本繰り返す

**2D PDP 計算（2次元部分空間射影）:**

- 訓練データから `(x[param1_idx], x[param2_idx])` の 2 列を抽出
- 2D ランダムフォレストとして学習
- 50×50 グリッド上で各格子点を予測し `values[i][j]` に格納

**デフォルトハイパーパラメータ（🟡）:**

| パラメータ         | 値            | 根拠                             |
| ------------------ | ------------- | -------------------------------- |
| `n_trees`          | 100           | 🟡 RF の一般的な推奨値           |
| `max_depth`        | 10            | 🟡 過学習抑制のため適度な深さ    |
| `min_samples_leaf` | 2             | 🟡 少サンプル Study にも対応     |
| 特徴量数（2D）     | 2（全特徴量） | 🔵 2D 部分空間のため全特徴量使用 |

#### 1b: `rust_core/src/kriging.rs`（新規）

ARD（Automatic Relevance Determination）RBF カーネルによるガウス過程回帰（Kriging）。外部クレート不使用・純 Rust。行列演算（Cholesky 分解）を自前実装。

**カーネル関数（ARD Matérn 5/2）:**

```
k(x1, x2) = σ_f² · (1 + √5·r + 5r²/3) · exp(−√5·r)

  where  r² = Σ_d ((x1_d − x2_d) / l_d)²   (ARD 距離)
```

- `σ_f`: シグナル分散（signal variance）
- `l_d`: 特徴量 d の長さスケール（ARD: 次元ごとに独立）
- `σ_n`: 観測ノイズ標準偏差

> **RBF ではなく Matérn 5/2 を採用した理由（🔵）**: Optuna の目的関数（NN ハイパーパラメータ・工学設計等）は C∞ではなく C² 程度の滑らかさが現実的。RBF（C∞ 仮定）はデータから遠い領域で不確実性を過小評価しやすい。BoTorch・Spearmint がデフォルトで ARD Matérn 5/2 を採用しており業界標準。Bessel 関数不要の閉形式のため実装コストは RBF と同等。

**L-BFGS 用解析的勾配（Matérn 5/2）:**

```
∂k/∂log(l_d) = σ_f² · 5/3 · (x1_d−x2_d)² / l_d² · (1 + √5·r) · exp(−√5·r)
```

**学習アルゴリズム:**

1. n > 1000 のとき、訓練データから無作為に 1000 点をサブサンプリング（🔵 n≤5000 ターゲット・ヒアリングより）
2. カーネル行列 K（N×N）を構築: `K[i,j] = k(x_i, x_j)`, `K[i,i] += σ_n²`
3. Cholesky 分解: `L = cholesky(K)` → 安定な線形システム求解
4. `alpha = L^T \ (L \ y)`（`K^{-1} y` の安定計算）
5. 対数周辺尤度を最大化:
   ```
   L = −½ y^T α − Σ_i log(L_ii) − n/2 log(2π)
   ```
6. **L-BFGS**（Limited-memory BFGS、m=5 履歴ベクトル）でハイパーパラメータを最適化:
   - 最適化変数: `log(l₁), log(l₂), log(σ_f), log(σ_n)`（対数空間）
   - 解析的勾配 `∂L/∂θⱼ = ½ tr((αα^T − K⁻¹) · ∂K/∂θⱼ)` を毎ステップ計算（O(N²)）
   - Armijo バックトラッキング線探索（c₁=1e-4）でステップ幅を決定
   - 最大 100 イテレーション、収束判定 `‖∇L‖ < 1e-5`
   - 純 Rust 実装: Two-loop L-BFGS recursion + 線探索 で約 150 行

> **L-BFGS を選択した理由**: scikit-learn の `GaussianProcessRegressor` と同じ業界標準手法。4 次元の小問題で 30〜100 イテレーションで収束（単純な勾配降下では 1000+ 必要）。解析的勾配を使うため数値微分（O(N³) × p 回）より大幅に高速。

**2D PDP 計算（2次元部分空間射影）:**

- 訓練データから `(x[param1_idx], x[param2_idx])` の 2 列を抽出
- 2D GP モデルとして学習（ARD 長さスケールは 2 次元分）
- 予測: `μ(x*) = k(x*, X) · alpha`
- 50×50 グリッド上で予測値を計算

**パフォーマンス計算量（🔵）:**

| n（訓練点数） | 学習                       | グリッド予測（2500 点）   |
| ------------- | -------------------------- | ------------------------- |
| ≤ 1000        | O(1000³) ≈ 10⁹ ops         | O(2500 × 1000) ≈ 2.5M ops |
| 1000〜5000    | O(1000³)（サブサンプル後） | 同上                      |

#### 1c: `rust_core/src/pdp.rs`（修正）

`compute_pdp_2d()` に `model_type: &str` 引数を追加し、モデルをディスパッチする。

```rust
pub fn compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: usize,
    model_type: &str,   // "ridge" | "random_forest" | "kriging"
) -> Option<PdpResult2d>
```

ディスパッチ:

- `"ridge"` → 既存の `compute_pdp_2d_from_matrix()` をそのまま使用
- `"random_forest"` → `rf::compute_pdp_2d_rf(x_matrix, y, param_names, obj, p1, p2, n_grid)`
- `"kriging"` → `kriging::compute_pdp_2d_kriging(x_matrix, y, param_names, obj, p1, p2, n_grid)`
- 未知の値 → `"ridge"` にフォールバック（安全のため）

#### 1d: `rust_core/src/lib.rs`（修正）

`wasm_compute_pdp_2d` に `model_type: &str` 引数を追加。

```rust
#[wasm_bindgen(js_name = "computePdp2d")]
pub fn wasm_compute_pdp_2d(
    param1_name: &str,
    param2_name: &str,
    objective_name: &str,
    n_grid: u32,
    model_type: &str,   // 追加
) -> Result<JsValue, JsValue>
```

### 層 2: TypeScript 型宣言（wasmLoader.ts） 🔵

**信頼性**: 🔵 _既存 wasmLoader.ts パターン・ユーザヒアリングより_

`computePdp2d` メソッドシグネチャに `modelType: string` を追加。`Pdp2dWasmResult` 型は変更なし（戻り値構造は同一）。

```typescript
computePdp2d!: (
    param1Name: string,
    param2Name: string,
    objectiveName: string,
    nGrid: number,
    modelType: string,   // 追加
) => Pdp2dWasmResult
```

### 層 3: Zustand Store（analysisStore.ts） 🔵

**信頼性**: 🔵 _既存 analysisStore.ts の computeSurface3d 実装・ユーザヒアリングより_

`computeSurface3d` 内で `surrogateModelType` を `wasm.computePdp2d` に渡す。現状は渡していないため常に Ridge が使われるバグを修正。

```typescript
// 変更前
const result = wasm.computePdp2d(param1, param2, objective, nGrid);

// 変更後
const result = wasm.computePdp2d(
  param1,
  param2,
  objective,
  nGrid,
  surrogateModelType,
);
```

### 層 4: React コンポーネント（SurfacePlot3D.tsx） 🔵

**信頼性**: 🔵 _既存 SurfacePlot3D.tsx の MODEL_OPTIONS 定義・ユーザヒアリングより_

RF / Kriging オプションの `disabled: true` フラグを削除する。

```typescript
// 変更前
{ value: 'random_forest', label: 'Random Forest (coming soon)', disabled: true },
{ value: 'kriging', label: 'Kriging (coming soon)', disabled: true },

// 変更後
{ value: 'random_forest', label: 'Random Forest' },
{ value: 'kriging', label: 'Kriging' },
```

---

## システム構成図 🔵

**信頼性**: 🔵 _既存 mode-frontier-features アーキテクチャ・ユーザヒアリングより_

```
SurfacePlot3D.tsx
  │ モデル選択（Ridge / RF / Kriging）
  │ パラメータ/目的選択
  ↓
analysisStore.computeSurface3d(param1, param2, obj, nGrid)
  │ surrogateModelType を引数に渡す（修正箇所）
  │ cacheKey = `${surrogateModelType}_${param1}_${param2}_${obj}_${nGrid}`
  ↓
WasmLoader.computePdp2d(p1, p2, obj, nGrid, modelType)
  ↓
rust_core wasm_compute_pdp_2d(p1, p2, obj, nGrid, model_type)
  │
  ├── model_type == "ridge"         → pdp::compute_pdp_2d_from_matrix()   【既存】
  ├── model_type == "random_forest" → rf::compute_pdp_2d_rf()              【新規】
  └── model_type == "kriging"       → kriging::compute_pdp_2d_kriging()    【新規】
        ↓
  PdpResult2d { param1_name, param2_name, objective_name,
                grid1, grid2, values, r_squared }
```

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 _既存プロジェクト構造より_

**新規ファイル:**

```
rust_core/src/
├── rf.rs           ← 新規: CART + Bagging ランダムフォレスト
└── kriging.rs      ← 新規: GP with ARD matérn5/2 カーネル・Cholesky 分解
```

**変更ファイル:**

```
rust_core/src/
├── pdp.rs          ← compute_pdp_2d() に model_type 引数追加・ディスパッチ追加
└── lib.rs          ← wasm_compute_pdp_2d() に model_type 引数追加

frontend/src/
├── wasm/wasmLoader.ts                    ← computePdp2d に modelType 追加
├── stores/analysisStore.ts               ← surrogateModelType を WASM に渡す
└── components/charts/SurfacePlot3D.tsx   ← RF/Kriging の disabled 解除
```

**WASMリビルド必要:**

```
wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg
```

---

## 非機能要件

### パフォーマンス 🔵

**信頼性**: 🔵 _ユーザヒアリング（n≤5000 ターゲット）・既存 NFR より_

| モデル        | 学習               | 推論（50×50 グリッド） | 合計目標          |
| ------------- | ------------------ | ---------------------- | ----------------- |
| Ridge         | O(N·p²) ≈ 無視可能 | 解析的（0ms）          | 50ms 以内（既存） |
| Random Forest | O(N_boot·p·d·T)    | O(50²·T·d)             | 2000ms 以内       |
| Kriging       | O(min(N,1000)³)    | O(2500·min(N,1000))    | 3000ms 以内       |

- N: 試行数、p: パラメータ数、d: 木の深さ、T: 木の本数（100）

Kriging はn > 1000 で自動サブサンプリングし O(1000³) ≈ 10⁹ 演算に抑える。WASM は JavaScript より高速だが、ユーザーに対してローディングスピナーを表示する（既存の `isComputingSurface` を活用）。

### セキュリティ 🔵

**信頼性**: 🔵 _既存 NFR-020・021 より_

- ファイルデータはブラウザ外に送信しない（ブラウザ完結）
- 既存制約を変更しない

### スタイル制約 🔵

**信頼性**: 🔵 _既存 NFR-201（Tailwind 禁止）より_

- UI の変更は `SurfacePlot3D.tsx` の `disabled` フラグ削除のみ
- Tailwind CSS クラス追加なし

---

## 技術的制約と注意事項

### Kriging の Cholesky 分解実装 🟡

**信頼性**: 🟡 _ユーザヒアリングと一般的な GP 実装知識から妥当な推測_

数値的安定性のため、ジッター（`jitter = 1e-6`）を対角成分に加算する。n≤5000 ターゲットだが n>1000 ではサブサンプリングを行う。

### RF の R² 値 🟡

**信頼性**: 🟡 _既存 PdpResult2d 構造・一般的な実装から妥当な推測_

`r_squared` フィールドはランダムフォレストの out-of-bag (OOB) R² で計算する。OOB サンプルが少ない場合は訓練データ上の R² を使用する（フォールバック）。

### `compute_pdp_2d()` 関数シグネチャ変更の後方互換性 🔵

**信頼性**: 🔵 _Rust 関数シグネチャ変更の影響分析より_

`pdp.rs` の `compute_pdp_2d()` と `lib.rs` の `wasm_compute_pdp_2d()` のシグネチャを変更するため、WASM バイナリの再ビルドと `tunny_core.d.ts` の手動更新が必要。

---

## 実装フェーズ 🟡

**信頼性**: 🟡 _ユーザヒアリング・既存実装パターンから妥当な推測_

| Phase   | 内容                                                        | 主要ファイル                |
| ------- | ----------------------------------------------------------- | --------------------------- |
| Phase 1 | `rf.rs` 実装（CART + Bagging）+ Rust テスト                 | `rf.rs`, `pdp.rs`           |
| Phase 2 | `kriging.rs` 実装（GP + ARD + Cholesky）+ Rust テスト       | `kriging.rs`, `pdp.rs`      |
| Phase 3 | `lib.rs` WASM シグネチャ更新 + WASM リビルド                | `lib.rs`, `tunny_core.d.ts` |
| Phase 4 | TypeScript 更新（wasmLoader, analysisStore, SurfacePlot3D） | TS 3 ファイル               |
| Phase 5 | フロントエンドテスト更新・E2E 確認                          | テストファイル              |

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義差分**: [interfaces.ts](interfaces.ts)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **前フェーズ アーキテクチャ**: [../mode-frontier-features/architecture.md](../mode-frontier-features/architecture.md)
- **既存 PDP 実装**: `rust_core/src/pdp.rs`
- **既存 WASM エクスポート**: `rust_core/src/lib.rs`

## 信頼性レベルサマリー

- 🔵 青信号: 13件 (76%)
- 🟡 黄信号: 4件 (24%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
