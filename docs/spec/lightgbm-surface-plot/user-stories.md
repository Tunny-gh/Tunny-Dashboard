# LightGBM Surface Plot ユーザストーリー

**作成日**: 2026-05-01
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 設計文書・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: 設計文書・ヒアリングから妥当な推測によるストーリー
- 🔴 **赤信号**: 推測によるストーリー

---

## エピック 1: 1D PDP への LightGBM 追加

### ストーリー 1.1: 1D PDP で LightGBM モデルを選択する 🔵

**信頼性**: 🔵 *ユーザヒアリング「1D・2D両方」より*

**私は** ダッシュボードユーザー **として**
**PDP Chart の Model セレクタから "Random Forest (LightGBM)" を選択し Run PDP を実行したい**
**そうすることで** 線形近似（Ridge）では捉えられない非線形なパラメータ影響を PDP として確認できる

**関連要件**: REQ-001, REQ-002, REQ-011, REQ-013, REQ-021

**詳細シナリオ**:
1. PDP Chart を開き、Parameter・Objective を選択する
2. Model セレクタで "Random Forest (LightGBM)" を選択する
3. "Run PDP" ボタンを押す
4. スピナーが表示され計算中であることがわかる
5. 計算完了後、PDP 曲線と R² が表示される

**前提条件**:
- スタディが読み込まれている（trial が 2 件以上）
- LightGBM DLL がシステムに存在する

**制約事項**:
- ICE ライン・95% CI は表示されない

**優先度**: Must Have

---

### ストーリー 1.2: LightGBM 1D PDP の R² で品質を確認する 🔵

**信頼性**: 🔵 *既存 `r2_quality()` 関数・表示パターンより*

**私は** ダッシュボードユーザー **として**
**LightGBM 1D PDP の結果に R² 品質評価（Good / Fair / Poor）を表示してほしい**
**そうすることで** LightGBM モデルがデータを適切に学習できているか即座に判断できる

**関連要件**: REQ-003, REQ-032

**詳細シナリオ**:
1. LightGBM モデルで Run PDP を実行する
2. 計算完了後、R² 値と品質ラベル（例: "R²: 0.91 (Good)"）が表示される

**前提条件**:
- PDP 計算が正常に完了している

**優先度**: Must Have

---

### ストーリー 1.3: LightGBM 学習失敗時のフォールバック 🟡

**信頼性**: 🟡 *既存 Option ベース安全処理パターンから妥当な推測*

**私は** ダッシュボードユーザー **として**
**LightGBM の学習が失敗してもアプリがクラッシュしないことを期待する**
**そうすることで** Ridge フォールバックで結果を確認できる

**関連要件**: REQ-002, EDGE-002

**詳細シナリオ**:
1. LightGBM モデルを選択して Run PDP を実行する
2. DLL 不在などで LightGBM 学習が失敗する
3. Ridge フォールバックで PDP 結果が表示される（UIはクラッシュしない）

**優先度**: Must Have

**備考**: フォールバック時のユーザー通知 UI は今回スコープ外（エラーログのみ）

---

## エピック 2: 2D PDP への LightGBM 追加

### ストーリー 2.1: 2D PDP Surface Plot で LightGBM を使用する 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 `compute_pdp_2d_lgbm` 実装より*

**私は** ダッシュボードユーザー **として**
**2D PDP Chart の Model セレクタから "Random Forest (LightGBM)" を選択して Run 2D PDP を実行したい**
**そうすることで** 2 つのパラメータの非線形な交互作用を Surface Plot（ヒートマップ）として可視化できる

**関連要件**: REQ-011, REQ-014, REQ-022, REQ-031

**詳細シナリオ**:
1. PDP Chart 2D を開き、Parameter 1・Parameter 2・Objective を選択する
2. Model セレクタで "Random Forest (LightGBM)" を選択する（現在は Ridge/Kriging/Sparse Kriging のみ）
3. "Run 2D PDP" ボタンを押す
4. スピナーが表示される
5. 計算完了後、単一ヒートマップ（uncertainties なし）が表示される

**前提条件**:
- スタディが読み込まれている
- Parameter 1 ≠ Parameter 2

**制約事項**:
- LightGBM の 2D PDP は uncertainty（分散）を返さないため、デュアルヒートマップは表示されない

**優先度**: Must Have

---

### ストーリー 2.2: 2D PDP LightGBM のグリッド精度確認 🔵

**信頼性**: 🔵 *ユーザヒアリング「30（高精度）」より*

**私は** ダッシュボードユーザー **として**
**LightGBM の 2D PDP は n_grid=30 で計算されることを期待する**
**そうすることで** 非線形な曲面を細かく可視化できる

**関連要件**: REQ-022

**詳細シナリオ**:
1. LightGBM モデルで Run 2D PDP を実行する
2. ヒートマップが 30×30 グリッドで描画される

**優先度**: Must Have

---

## ストーリーマップ

```
エピック 1: 1D PDP への LightGBM 追加
├── ストーリー 1.1 (🔵 Must Have) - UI で LightGBM 選択・実行
├── ストーリー 1.2 (🔵 Must Have) - R² 品質表示
└── ストーリー 1.3 (🟡 Must Have) - フォールバック

エピック 2: 2D PDP への LightGBM 追加
├── ストーリー 2.1 (🔵 Must Have) - Surface Plot 表示
└── ストーリー 2.2 (🔵 Must Have) - n_grid=30
```

## 信頼性レベルサマリー

- 🔵 青信号: 4件 (80%)
- 🟡 黄信号: 1件 (20%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
