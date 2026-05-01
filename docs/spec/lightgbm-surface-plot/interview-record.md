# LightGBM Surface Plot ヒアリング記録

**作成日**: 2026-05-01
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存コードベース調査（`lgbm.rs`, `pdp/api.rs`, `pdp_chart.rs`, `pdp_2d.rs`, `chart_registry.rs`）で判明したギャップを確認し、スコープ・仕様を明確化する。

## 質問と回答

### Q1: 作業規模

**カテゴリ**: 既存設計確認  
**背景**: kairo-requirements の標準ヒアリング  
**回答**: フル機能開発  
**信頼性への影響**: 全ドキュメントを詳細に作成する方針を確定

---

### Q2: コード詳細分析の必要性

**カテゴリ**: 既存設計確認  
**背景**: 調査で 2D バックエンドが実装済み・1D が未実装と判明しており、追加調査が必要か確認  
**回答**: 必要（網羅的調査を希望）  
**信頼性への影響**: 追加調査を実施し、chart_registry の n_grid ハードコード（2D=20）・1D n_grid 分岐（Ridge=50, other=30）を確認

---

### Q3: 対象スコープ（1D/2D）

**カテゴリ**: 未定義部分詳細化  
**背景**: "surface plot" は 2D のみを指す可能性があったが、UI 一貫性のため 1D も含む可能性を確認  
**回答**: 1D・2D 両方  
**信頼性への影響**: REQ-001〜003（1D 実装）を新規追加、REQ-013 を確定

---

### Q4: LightGBM の n_grid

**カテゴリ**: 未定義部分詳細化  
**背景**: 2D は n_grid=20 ハードコード、1D は Ridge=50/他=30。LightGBM の非線形特性には細かいグリッドが期待される  
**回答**: 30（高精度）  
**信頼性への影響**: REQ-021・REQ-022 を確定（🔵）

---

### Q5: 1D PDP の不確実性表示

**カテゴリ**: 未定義部分詳細化  
**背景**: Kriging は 95% CI を持つが、LightGBM で ICE ライン / 信頼区間が必要か確認  
**回答**: 不要（PDP 曲線のみ）  
**信頼性への影響**: REQ-003 の `y_upper=None / y_lower=None` を確定（🔵）

---

## ヒアリング結果サマリー

### 確認できた事項
- 対象は 1D・2D 両 PDP Chart
- n_grid = 30（2D も現行 20 から変更）
- 1D LightGBM: PDP 曲線 + R² のみ（ICE ライン・CI なし）
- 2D LightGBM: 単一ヒートマップ（uncertainties なし）

### 追加/変更要件
- 1D LightGBM 計算関数 `compute_pdp_1d_lgbm()` の新規実装（REQ-001）
- `compute_pdp_from_data()` へのディスパッチ追加（REQ-002）
- `ModelType::RandomForest` の追加（REQ-011〜012）
- 1D UI ComboBox への追加（REQ-013）
- 2D n_grid を LightGBM のみ 30 に変更（REQ-022）

### 残課題
- LightGBM DLL 存在前提の確認（`lgbm_sys.rs` リンク設定は調査済み、実行環境で DLL が必要）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 4（2D 実装済み関連）
- 🟡 黄信号: 3（n_grid・フォールバック推測）
- 🔴 赤信号: 3（1D スコープ・n_grid・CI 要否）

**ヒアリング後**:
- 🔵 青信号: 10 (+6)
- 🟡 黄信号: 4 (+1)
- 🔴 赤信号: 0 (-3)

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
