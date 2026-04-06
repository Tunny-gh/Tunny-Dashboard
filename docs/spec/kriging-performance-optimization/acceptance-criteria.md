# Kriging 高速化 受け入れ基準

**作成日**: 2026-04-05
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: ユーザヒアリング・既存実装より確実な基準
- 🟡 **黄信号**: 既存実装・設計文書から妥当な推測による基準
- 🔴 **赤信号**: ヒアリング・実装にない推測による基準

---

## REQ-002: LML + 勾配統合計算 🔵

**信頼性**: 🔵 *kriging.rs ボトルネック分析・既存コード構造より*

### Given: 既存の `log_marginal_likelihood` と `log_ml_gradient` が実装済み

### When: `log_ml_with_gradient(x, y, params)` を呼び出す

### Then:
- `(lml_value, gradient_vec)` を返す
- `lml_value` は `log_marginal_likelihood(x, y, ...)` と同一値（相対誤差 < 1e-10）
- `gradient_vec` は `log_ml_gradient(x, y, params)` と同一値（相対誤差 < 1e-8）
- 内部での build_kernel_matrix 呼び出しが 1回のみ（現在の 2回から削減）

### テストケース

#### 正常系

- [ ] **TC-002-01**: 既存の LML 値と統合関数の LML 値が一致 🔵
  - **入力**: N=10 のシンセティックデータ、params = [0.0, 0.0, 0.0, -2.0]
  - **期待結果**: |log_ml_with_gradient().0 - log_marginal_likelihood()| < 1e-10

- [ ] **TC-002-02**: 既存の勾配と統合関数の勾配が一致 🔵
  - **入力**: TC-002-01 と同じデータ
  - **期待結果**: 各次元の相対誤差 < 1e-8

- [ ] **TC-002-03**: optimize_hyperparams の収束後 LML が改善 🔵
  - **入力**: N=20 の線形データ
  - **期待結果**: 最終 LML ≥ 初期 LML - 0.1（既存 TC-1635-01 と同等）

---

## REQ-003: L-BFGS 反復削減 + 早期停止 🔵

**信頼性**: 🔵 *ユーザヒアリング（max_iter 削減許容）より*

### Given: `optimize_hyperparams` が n_iter=50 で呼び出される

### When: 10 イテレーション以内に LML 変化が 1e-3 以下になる

### Then: イテレーション 10 + 5 = 15 回目（5 回連続小変化）で早期停止する

### テストケース

#### 正常系

- [ ] **TC-003-01**: max_iter=50 で収束後 LML は合理的 🔵
  - **入力**: N=20、初期値 params=[0,0,0,-2]、max_iter=50
  - **期待結果**: 最終 LML ≥ max_iter=100 の場合の 95%（精度比較）

- [ ] **TC-003-02**: 早期停止が正しく機能する 🟡
  - **入力**: 5 イテレーション後に LML 変化が 1e-4 以下になるデータ
  - **期待結果**: 実行イテレーション数 ≤ 10（早期停止 5 回 + バッファ）

#### 境界値

- [ ] **TC-003-B01**: max_iter=0 で L-BFGS を実行しない 🟡
  - **入力**: N=5、max_iter=0
  - **期待結果**: 初期パラメータのまま返す

---

## REQ-004: サブサンプル数削減 🔵

**信頼性**: 🔵 *ユーザヒアリング（許容）・既存 train_gp より*

### Given: N=600 の訓練データ、subsample_n=500

### When: `train_gp(x, y, 500, seed)` を呼び出す

### Then: モデルの `x_train.len()` が 500 である

### テストケース

#### 正常系

- [ ] **TC-004-01**: N > 500 で 500 点にサブサンプリング 🔵
  - **入力**: N=600、subsample_n=500
  - **期待結果**: `model.x_train.len() == 500`

- [ ] **TC-004-02**: N ≤ 500 でサブサンプリングしない 🔵
  - **入力**: N=300、subsample_n=500
  - **期待結果**: `model.x_train.len() == 300`

- [ ] **TC-004-03**: N=500 ちょうどでサブサンプリングしない 🟡
  - **入力**: N=500、subsample_n=500
  - **期待結果**: `model.x_train.len() == 500`

---

## REQ-005: Sparse Kriging モデル追加 🔵

**信頼性**: 🔵 *ユーザヒアリング（ドロップダウン追加）より*

### Given: SurfacePlot3D コンポーネントが有効な Study でレンダリングされている

### When: Model ドロップダウンで "Sparse Kriging" を選択する

### Then:
- `setSurrogateModelType('sparse_kriging')` が呼ばれる
- `computeSurface3d` が `sparse_kriging` で呼ばれる
- スピナーに "Expected: < 5s" が表示される

### テストケース

#### 正常系

- [ ] **TC-005-01**: sparse_kriging が SurrogateModelType として有効 🔵
  - **入力**: `surrogateModelType = 'sparse_kriging'`
  - **期待結果**: TypeScript 型エラーなし

- [ ] **TC-005-02**: N=100 で compute_pdp_2d_sparse_kriging が値を返す 🔵
  - **入力**: N=100、n_grid=10、M=50 誘導点
  - **期待結果**: grid1.len() == 10、grid2.len() == 10、values[0].len() == 10

- [ ] **TC-005-03**: N < M（N=30）で標準 Kriging にフォールバック 🔵
  - **入力**: N=30（< M=50）、n_grid=10
  - **期待結果**: エラーなし、合理的な結果を返す

- [ ] **TC-005-04**: cacheKey に sparse_kriging が含まれる 🔵
  - **入力**: surrogateModelType='sparse_kriging', p1='x1', p2='x2', obj='obj0', n_grid=50
  - **期待結果**: cacheKey == 'sparse_kriging_x1_x2_obj0_50'

#### 異常系

- [ ] **TC-005-E01**: K_ZZ が非正定値でジッター増加後に成功 🟡
  - **入力**: 分散がほぼ 0 の定数的データ
  - **期待結果**: ジッター増加後に None でなく結果を返す

---

## REQ-001: Web Worker オフロード 🔵

**信頼性**: 🔵 *ユーザヒアリング（確実に実装したい）より*

### Given: Kriging 計算が開始される

### When: Worker への postMessage が送信される

### Then: 計算中も UI スレッドがブロックされない

### テストケース

#### 正常系

- [ ] **TC-001-01**: Worker 計算中にスピナーが表示される 🔵
  - **入力**: isComputingSurface=true（Worker 実行中）
  - **期待結果**: spinner overlay が DOM に存在する

- [ ] **TC-001-02**: Worker 計算完了後に surface3dCache が更新される 🔵
  - **入力**: Worker が PdpResult2d を返す
  - **期待結果**: surface3dCache に cacheKey が追加される

- [ ] **TC-001-03**: Worker 計算完了後にスピナーが消える 🔵
  - **入力**: Worker が結果を返す
  - **期待結果**: isComputingSurface が false になりスピナーが消える

#### 異常系

- [ ] **TC-001-E01**: Worker 初期化失敗時にフォールバック 🟡
  - **入力**: Worker WASM 初期化エラー
  - **期待結果**: surface3dError が設定される or Main Thread フォールバック

---

## NFR テスト

### NFR-001: Kriging N=1000 で 10 秒以内 🔵

**信頼性**: 🔵 *ユーザヒアリング（< 10 秒）より*

- [ ] **TC-NFR-001-01**: Kriging パフォーマンス（REQ-002〜004 適用後） 🔵
  - **測定項目**: train_gp + compute_pdp_2d_kriging の合計時間
  - **目標値**: N=1000、n_grid=50 で < 10,000ms
  - **測定条件**: release ビルド（wasm-pack --release）

### NFR-002: Sparse Kriging N=5000 で 5 秒以内 🟡

**信頼性**: 🟡 *O(N×M²) 計算量から妥当な推測*

- [ ] **TC-NFR-002-01**: Sparse Kriging パフォーマンス 🟡
  - **測定項目**: compute_pdp_2d_sparse_kriging の合計時間
  - **目標値**: N=5000、M=50、n_grid=50 で < 5,000ms
  - **測定条件**: release ビルド

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| REQ-002 (LML統合) | 3 | 0 | 0 | 3 |
| REQ-003 (L-BFGS) | 2 | 0 | 1 | 3 |
| REQ-004 (サブサンプル) | 2 | 0 | 1 | 3 |
| REQ-005 (Sparse Kriging) | 4 | 1 | 0 | 5 |
| REQ-001 (Web Worker) | 3 | 1 | 0 | 4 |
| NFR (パフォーマンス) | 2 | 0 | 0 | 2 |
| **合計** | **16** | **2** | **2** | **20** |

### 信頼性レベル分布

- 🔵 青信号: 16件 (80%)
- 🟡 黄信号: 4件 (20%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 16件
- **Should Have**: 4件
- **Could Have**: 0件

---

## テスト実施計画

### Phase 1: アルゴリズム最適化テスト（Rust unit test）
- REQ-002 (TC-002-*), REQ-003 (TC-003-*), REQ-004 (TC-004-*)
- `cargo test` で実行

### Phase 2: Sparse Kriging テスト（Rust unit test）
- REQ-005 (TC-005-01 〜 TC-005-03)
- `cargo test` で実行

### Phase 3: フロントエンドテスト（Vitest）
- REQ-001 (TC-001-*), REQ-005 (TC-005-04)
- `npm test` で実行

### Phase 4: パフォーマンステスト（Rust bench / WASM 実機計測）
- NFR-001 (TC-NFR-001-01), NFR-002 (TC-NFR-002-01)
- release ビルドで実機計測
