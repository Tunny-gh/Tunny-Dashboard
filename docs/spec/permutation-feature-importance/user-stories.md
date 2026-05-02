# Permutation Feature Importance ユーザストーリー

**作成日**: 2026-05-02
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・設計文書・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: PRD・設計文書・ユーザヒアリングから妥当な推測によるストーリー
- 🔴 **赤信号**: PRD・設計文書・ユーザヒアリングにない推測によるストーリー

---

## エピック1: Permutation メトリクスの追加

### ストーリー 1.1: Permutation 指標の選択と実行 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 ImportanceChart UI パターンより*

**私は** Optuna 最適化結果を分析するエンジニア **として**
**Importance Chart で "Permutation" をメトリクスとして選択し Run ボタンを押したい**
**そうすることで** LightGBM ベースの Permutation Feature Importance（n_repeats=5）でパラメータ重要度を評価できる

**関連要件**: REQ-PFI-005-A, REQ-PFI-005-B, REQ-PFI-005-E

**詳細シナリオ**:
1. Importance Chart ウィジェットを開く
2. メトリクス ドロップダウンの "── Tree-based ──" グループに "Permutation" が表示される
3. "Permutation" を選択する
4. "Run" ボタンをクリックする
5. スピナーと "Computing..." が表示される
6. バックグラウンドで LightGBM RF 学習 + n_repeats=5 シャッフルが実行される
7. 結果として各パラメータの平均MSE増加量（正規化済み）が水平バーチャートで表示される

**前提条件**:
- Study がロード済みであること
- パラメータが 1 つ以上存在すること

**優先度**: Must Have

---

### ストーリー 1.2: R² による信頼性確認 🔵

**信頼性**: 🔵 *既存 ImportanceChart の R² 表示パターンより*

**私は** Optuna 最適化結果を分析するエンジニア **として**
**Permutation 結果の横に R² を確認したい**
**そうすることで** LightGBM モデルのフィット品質を把握しスコアの信頼性を判断できる

**関連要件**: REQ-PFI-005-G

**詳細シナリオ**:
1. Permutation の計算が完了する
2. チャートヘッダー右端に `R² = 0.xxx` が表示される
3. R² < 0.5 の場合は赤色 + "(low fit)" の警告が表示される
4. 0.5 ≤ R² < 0.8 の場合は黄色、R² ≥ 0.8 の場合は緑色で表示される

**前提条件**: Permutation 計算が完了していること

**優先度**: Must Have

---

### ストーリー 1.3: キャッシュによる即時再表示 🔵

**信頼性**: 🔵 *既存 importance_cache パターンより*

**私は** Optuna 最適化結果を分析するエンジニア **として**
**一度計算した Permutation 結果を再計算なしで参照したい**
**そうすることで** 他のメトリクスと切り替えても待ち時間なしで比較できる

**関連要件**: NFR-PFI-003

**詳細シナリオ**:
1. Permutation を Run して結果が表示される
2. メトリクスを Spearman に切り替えて Spearman を計算する
3. メトリクスを Permutation に戻す
4. Run ボタンを押さずとも即座に Permutation の結果が再表示される

**前提条件**: 同 Study 内での操作

**優先度**: Must Have

---

## エピック2: エラーハンドリング・エッジケース対応

### ストーリー 2.1: 計算失敗時のフォールバック表示 🔵

**信頼性**: 🔵 *既存 EmptyState / SensitivityError パターンより*

**私は** Optuna 最適化結果を分析するエンジニア **として**
**計算失敗時にエラーが明示されることを期待する**
**そうすることで** アプリがクラッシュせず、何が問題か把握できる

**関連要件**: EDGE-PFI-001, EDGE-PFI-002, EDGE-PFI-010

**詳細シナリオ**:
1. Study に有効なデータが 2 行未満の場合
2. Permutation 計算が early return で空の結果を返す
3. UI は "No sensitivity data (start the computation first)" を表示する

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: Permutation メトリクスの追加
├── ストーリー 1.1: Permutation 選択と実行 (🔵 Must Have)
├── ストーリー 1.2: R² による信頼性確認 (🔵 Must Have)
└── ストーリー 1.3: キャッシュによる即時再表示 (🔵 Must Have)

エピック2: エラーハンドリング
└── ストーリー 2.1: 計算失敗時のフォールバック (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 4件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
