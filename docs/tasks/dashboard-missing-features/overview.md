# dashboard-missing-features 実装タスク概要

**作成日**: 2026-05-12  
**対象機能**: ダッシュボード不足機能 8 件（CSV Export / Comparison UI / Pinning / PDP Overlay / Surface Plot / Brushing & Linking / Comparison Diff / PNG Export）

## 関連文書

- **要件定義**: [📋 requirements.md](../../spec/dashboard-missing-features/requirements.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](../../spec/dashboard-missing-features/acceptance-criteria.md)
- **設計文書**: [📐 architecture.md](../../design/dashboard-missing-features/architecture.md)
- **データフロー**: [🔄 dataflow.md](../../design/dashboard-missing-features/dataflow.md)
- **型設計**: [📝 interfaces.rs](../../design/dashboard-missing-features/interfaces.rs)
- **タスク分割ヒアリング**: [💬 interview-record.md](interview-record.md)

## 前提条件

- 実装レイヤー順は「基盤 → 状態管理 → UI → 統合」で進める
- タスク粒度は 1 日 = 8 時間を基準とする
- テスト方針は標準レベル（主要ロジック単体テスト + 代表統合テスト）とする
- Surface Plot は Heatmap/Contour 実装に加え、3D 描画調査タスクを含める

## 補助文書の有無

- **API仕様書**: 該当なし
- **DBスキーマ**: 該当なし

---

## 全体サマリー

| 項目 | 値 |
|------|----|
| プロジェクト期間 | 2026-05-13 - 2026-06-09（20営業日想定） |
| 総タスク数 | 20 |
| 総工数 | 160時間 |
| 平均工数/タスク | 8時間 |
| 実装方式 | TDD 18件 / DIRECT 2件 |
| 信頼性分布 | 🔵 15件 / 🟡 5件 / 🔴 0件 |

## フェーズ別計画

| Phase | 期間 | 工数 | タスク数 | 主目的 |
|-------|------|------|---------|--------|
| Phase 1 | Day 1-5 | 40時間 | 5 | 共通コマンド・状態・保存/比較基盤を先に固定する |
| Phase 2 | Day 6-10 | 40時間 | 5 | Toolbar / Trial Table / Comparison / PDP の操作 UI を揃える |
| Phase 3 | Day 11-16 | 48時間 | 6 | Surface Plot と Brushing の高度可視化を実装する |
| Phase 4 | Day 17-20 | 32時間 | 4 | PNG Export と横断検証でリリース品質へ仕上げる |

---

## Phase 詳細

### Phase 1: 基盤・状態管理（40時間）

- [TASK-2228](TASK-2228.md): 共通コマンド・状態・メッセージ基盤拡張
- [TASK-2229](TASK-2229.md): CSVエクスポート純粋ロジックと保存ヘルパー実装
- [TASK-2230](TASK-2230.md): Comparison Studyローダーとメッセージ経路実装
- [TASK-2231](TASK-2231.md): ピン留め状態管理とセッション保存/復元統合
- [TASK-2232](TASK-2232.md): 選択＋ピン留め表示ヘルパーと共通描画導線整備

### Phase 2: 操作UIと比較機能（40時間）

- [TASK-2233](TASK-2233.md): ツールバーCSV Exportメニュー実装
- [TASK-2234](TASK-2234.md): ツールバーComparison追加/削除UI実装
- [TASK-2235](TASK-2235.md): Trial Tableピン留めUI実装
- [TASK-2236](TASK-2236.md): Comparison Diffタブと差分メトリクス実装
- [TASK-2237](TASK-2237.md): PDP observed overlayの選択連動仕上げ

### Phase 3: 高度可視化とリンク（48時間）

- [TASK-2238](TASK-2238.md): Surface Plot ChartId・状態・ルーティング基盤実装
- [TASK-2239](TASK-2239.md): Surface Plot Heatmap/Contour計算と描画実装
- [TASK-2240](TASK-2240.md): Surface Plot 3D描画調査と最小プロトタイプ
- [TASK-2241](TASK-2241.md): Pareto Scatter 2D矩形ブラッシング実装
- [TASK-2242](TASK-2242.md): Parallel Coordinates軸ブラッシング実装
- [TASK-2243](TASK-2243.md): Brushing & Linking横断最適化と表示整合

### Phase 4: エクスポート統合と検証（32時間）

- [TASK-2244](TASK-2244.md): セルツールバー⋯メニューとキャプチャ状態実装
- [TASK-2245](TASK-2245.md): チャートPNGキャプチャ・crop・保存実装
- [TASK-2246](TASK-2246.md): 主要機能回帰テスト整備
- [TASK-2247](TASK-2247.md): 統合検証とリリース前バグ修正

---

## 依存関係サマリー

### 主経路

1. `TASK-2228 → TASK-2231 → TASK-2232 → TASK-2241 / TASK-2242 → TASK-2243 → TASK-2246 → TASK-2247`
2. `TASK-2228 → TASK-2238 → TASK-2239 → TASK-2246 → TASK-2247`
3. `TASK-2228 → TASK-2229 → TASK-2233 → TASK-2246 → TASK-2247`
4. `TASK-2228 → TASK-2230 → TASK-2234 → TASK-2236 → TASK-2246 → TASK-2247`

### クリティカルパス

- 最長かつリスクが高い経路は `TASK-2228 → TASK-2231 → TASK-2232 → TASK-2241 → TASK-2243 → TASK-2246 → TASK-2247`
- Surface Plot は別枝で進められるが、`TASK-2239` の計算 API と `TASK-2245` の PNG 保存方式は Phase 4 の統合時に確認が必要

---

## リスクと重点確認事項

### 高リスク（🟡）

- `TASK-2236`: Comparison Diff の見せ方と差分テーブルの情報量調整
- `TASK-2239`: Surface Plot の Heatmap/Contour 実装と非同期計算 API 整理
- `TASK-2240`: 3D 描画方式の調査結果次第で次期設計が変わる
- `TASK-2243`: 50,000 試行での Brushing 応答時間確認
- `TASK-2245`: egui/eframe の capture API と PNG 品質の検証

### 重点確認事項

- `selected_indices ∪ pinned_trials` を全チャートで同一解釈に統一すること
- Comparison base Study 変更時のリセット規約を UI 上でも分かるようにすること
- PNG 保存は「cell 単位」で行い、viewport 全体保存に逃げないこと

---

## 信頼性レベル集計

| レベル | タスク数 | 割合 |
|--------|---------|------|
| 🔵 青信号 | 15 | 75% |
| 🟡 黄信号 | 5 | 25% |
| 🔴 赤信号 | 0 | 0% |

**品質評価**: ✅ 高品質

---

## 完了判定

- 20 タスクすべてが完了している
- [../../spec/dashboard-missing-features/acceptance-criteria.md](../../spec/dashboard-missing-features/acceptance-criteria.md) の主要ケースに対する自動/手動確認が揃っている
- `cargo build` と `cargo test -p egui-app` が通る
- 3D Surface Plot は本開発スコープ外として調査結果と次期判断材料が残っている

## 次の推奨アクション

- `/tsumiki:kairo-implement` でタスク実装を開始する
- 特定タスクから始める場合は `/tsumiki:kairo-implement TASK-2228` のように指定する
