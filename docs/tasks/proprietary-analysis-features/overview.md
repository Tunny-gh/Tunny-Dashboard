# proprietary-analysis-features — タスク概要

## 関連文書

- **要件定義**: [docs/spec/proprietary-analysis-features/requirements.md](../../spec/proprietary-analysis-features/requirements.md)
- **設計文書**: [docs/design/proprietary-analysis-features/architecture.md](../../design/proprietary-analysis-features/architecture.md)
- **データフロー図**: [docs/design/proprietary-analysis-features/dataflow.md](../../design/proprietary-analysis-features/dataflow.md)

## フェーズ概要

| フェーズ | 内容 | タスク数 | 合計工数 |
|---------|------|---------|--------|
| Phase 1 | State & Message 基盤拡張 | 5 | 40h |
| Phase 2 | IO 層実装 | 3 | 40h |
| Phase 3 | UI 層実装 | 9 | 88h |
| Phase 4 | 統合テスト | 2 | 16h |
| **合計** | | **19** | **184h（約23日）** |

---

## Phase 1: State & Message 基盤拡張

| タスクID | タイトル | 工数 | 関連 REQ |
|---------|---------|------|---------|
| [TASK-2110](TASK-2110.md) | AppState 8フィールド追加 | 8h | REQ-001/006/007/008 |
| [TASK-2111](TASK-2111.md) | LayoutState::Comparison バリアント追加 + serde derive | 8h | REQ-003/006 |
| [TASK-2112](TASK-2112.md) | AppMessage 4バリアント追加 | 8h | REQ-001/005/006/007 |
| [TASK-2113](TASK-2113.md) | WidgetStates 拡張（axis_visibility / show_best_line / log_scale） | 8h | REQ-008/009 |
| [TASK-2114](TASK-2114.md) | MessageHandler バインディング（4バリアント処理アーム + spawn_task） | 8h | 全 REQ |

---

## Phase 2: IO 層実装

| タスクID | タイトル | 工数 | 関連 REQ |
|---------|---------|------|---------|
| [TASK-2115](TASK-2115.md) | SessionSnapshot 拡張 + .tdash 形式 + フィルタ/レイアウト JSON | 16h | REQ-002/003/004 |
| [TASK-2116](TASK-2116.md) | ArtifactsScanner 実装（新規 io/artifacts.rs） | 8h | REQ-007 |
| [TASK-2117](TASK-2117.md) | HtmlReportBuilder 実装（新規 io/html_report.rs） | 16h | REQ-005 |

---

## Phase 3: UI 層実装

| タスクID | タイトル | 工数 | 関連 REQ |
|---------|---------|------|---------|
| [TASK-2118](TASK-2118.md) | Toolbar ボタン群追加（フィルタ/レイアウト/セッション/Artifacts/HTML/比較） | 8h | REQ-002/003/004/005/006/007 |
| [TASK-2119](TASK-2119.md) | Left Panel — Trade-off Navigator UI（重み調整スライダー） | 8h | REQ-001 |
| [TASK-2120](TASK-2120.md) | Left Panel — Convergence Card（単目的 Best 値・改善率） | 8h | REQ-008 |
| [TASK-2121](TASK-2121.md) | Bottom Panel — Artifacts 列追加（ファイルタイプアイコン） | 8h | REQ-007 |
| [TASK-2122](TASK-2122.md) | artifact_modal.rs 新規作成（画像/CSV/汎用プレビュー） | 8h | REQ-007 |
| [TASK-2123](TASK-2123.md) | Bottom Panel — Best 解遷移テーブルタブ（単目的専用） | 8h | REQ-008 |
| [TASK-2124](TASK-2124.md) | comparison_panel.rs 新規作成（Stats/HV/Pareto/KDE 4 ビュー） | 16h | REQ-006 |
| [TASK-2125](TASK-2125.md) | parallel_coords — 軸表示制御・ドラッグ並び替え UI | 8h | REQ-009 |
| [TASK-2126](TASK-2126.md) | optimization_history — Best ライン・半対数 Y 軸 UI 拡張 | 8h | REQ-008 |

---

## Phase 4: 統合テスト

| タスクID | タイトル | 工数 | 関連 REQ |
|---------|---------|------|---------|
| [TASK-2127](TASK-2127.md) | セッション永続化 統合テスト（フィルタ/レイアウト/.tdash 往復） | 8h | REQ-002/003/004 |
| [TASK-2128](TASK-2128.md) | Artifacts + HTML レポート 統合テスト | 8h | REQ-005/007 |

---

## 依存関係図

```
Phase 1 (基盤)
  TASK-2110 ──┬── TASK-2114 ──┬── Phase 3 UI 全般
  TASK-2111 ──┤               └── TASK-2118 (Toolbar)
  TASK-2112 ──┤
  TASK-2113 ──┘

Phase 2 (IO)
  TASK-2115 ──── TASK-2118 (Toolbar) ──── TASK-2127 (統合テスト)
  TASK-2116 ──── TASK-2121 (Artifacts列) ─ TASK-2122 (modal)
               └── TASK-2128 (統合テスト)
  TASK-2117 ──── TASK-2118 (Toolbar)
               └── TASK-2128 (統合テスト)

Phase 3 UI (左パネル)
  TASK-2110 ──── TASK-2119 (Trade-off Navigator)
  TASK-2110 ──── TASK-2120 (Convergence Card) ──── TASK-2123 (Best遷移テーブル)
                                                └── TASK-2126 (opt_history Best ライン)

Phase 3 UI (下パネル)
  TASK-2110 + TASK-2116 ──── TASK-2121 ──── TASK-2122

Phase 3 UI (比較パネル)
  TASK-2110 + TASK-2111 ──── TASK-2124 (comparison_panel)

Phase 3 UI (チャート拡張)
  TASK-2113 ──── TASK-2125 (parallel_coords)
  TASK-2113 ──── TASK-2126 (opt_history)
```

---

## 要件カバレッジ

| 要件ID | 要件名 | 担当タスク |
|--------|-------|-----------|
| REQ-001 | Trade-off Navigator | TASK-2110, TASK-2112, TASK-2114, TASK-2119 |
| REQ-002 | フィルタ JSON 保存/復元 | TASK-2115, TASK-2118, TASK-2127 |
| REQ-003 | レイアウト JSON 保存/復元 | TASK-2111, TASK-2115, TASK-2118, TASK-2127 |
| REQ-004 | .tdash セッション保存/復元 | TASK-2115, TASK-2118, TASK-2127 |
| REQ-005 | HTML レポート エクスポート | TASK-2112, TASK-2117, TASK-2118, TASK-2128 |
| REQ-006 | マルチスタディ比較 | TASK-2110, TASK-2111, TASK-2112, TASK-2114, TASK-2118, TASK-2124 |
| REQ-007 | Artifacts ビューア | TASK-2110, TASK-2112, TASK-2114, TASK-2116, TASK-2118, TASK-2121, TASK-2122, TASK-2128 |
| REQ-008 | 収束診断 / Best 解遷移 | TASK-2110, TASK-2113, TASK-2114, TASK-2120, TASK-2123, TASK-2126 |
| REQ-009 | parallel_coords 軸制御 | TASK-2113, TASK-2125 |
| NFR-201 | パス トラバーサル防御 | TASK-2116, TASK-2128 |
