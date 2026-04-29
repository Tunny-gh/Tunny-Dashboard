---
name: AHP コンテキストノート
description: AHP (Analytic Hierarchy Process) 実装のための技術スタック・既存実装・注意事項
type: project
---

# AHP — コンテキストノート

## プロジェクト技術スタック

| 層               | 技術                                      |
| ---------------- | ----------------------------------------- |
| アルゴリズム実装 | Rust (`rust_core/src/mcdm/`)              |
| デスクトップ UI  | egui/eframe (`egui-app/src/`)             |
| 状態管理         | `egui-app/src/state/`                     |
| 非同期通信       | `std::sync::mpsc::SyncSender<AppMessage>` |

> **注意**: `frontend/` (TypeScript/WASM) は削除済み。全実装は Rust (`egui-app`) で行う。

---

## 既存 MCDM 実装の構成

### アルゴリズム層 (`rust_core/src/mcdm/`)

| ファイル       | 内容                                                          |
| -------------- | ------------------------------------------------------------- |
| `mod.rs`       | `validate_inputs` / `filter_valid_indices` 共通ユーティリティ |
| `topsis.rs`    | TOPSIS アルゴリズム + テスト                                  |
| `vikor.rs`     | VIKOR アルゴリズム + テスト                                   |
| `promethee.rs` | PROMETHEE I/II アルゴリズム + テスト                          |
| `entropy.rs`   | エントロピー重み計算                                          |

### 状態管理層 (`egui-app/src/state/`)

| ファイル             | 関連型                                                                          |
| -------------------- | ------------------------------------------------------------------------------- |
| `results.rs`         | `McdmMethod` (Topsis/Vikor/PrometheeI/PrometheeII), `McdmResult`, 各Result型    |
| `messages.rs`        | `AppMessage::McdmDone(McdmResult)`, `AppMessage::AhpDone(AhpResult)` (追加予定) |
| `message_handler.rs` | `MessageHandler::handle` — McdmDone/AhpDone ハンドリング                        |
| `layout_state.rs`    | `ChartId` enum — `McdmRankChart`, `McdmTable` など                              |

### UI 層 (`egui-app/src/ui/widgets/`)

| ファイル        | 内容                                                        |
| --------------- | ----------------------------------------------------------- |
| `mcdm_chart.rs` | `McdmRankChart` / `McdmTable` — バーチャート + テーブル表示 |
| `ahp_chart.rs`  | 新規作成予定: 一対比較行列入力 + CR表示 + ランキング表示    |

---

## AHP (Analytic Hierarchy Process) アルゴリズム概要

### 一対比較行列（Pairwise Comparison Matrix）

Saaty 1-9スケールで目的関数間の重要度を比較。

```
A[i][j] = a_ij  (i が j より a_ij 倍重要)
A[j][i] = 1 / a_ij  (逆数の法則)
A[i][i] = 1.0  (対角成分)
```

**Saaty スケール**:

- 1: 同等に重要
- 3: 少し重要
- 5: かなり重要
- 7: 非常に重要
- 9: 極めて重要
- 2, 4, 6, 8: 中間値

### 優先度ベクトル導出（固有ベクトル近似法）

```
1. 各列を列合計で除算して正規化行列 B を作成
2. 各行の平均を計算 → 優先度ベクトル w
```

### 整合性チェック

```
λmax = Σ_j (A のj列合計 × w[j])
CI   = (λmax - n) / (n - 1)     (n = 目的関数数)
RI   (ランダム整合性指標, Saatyのテーブル):
  n=1→0.00, n=2→0.00, n=3→0.58, n=4→0.90, n=5→1.12
CR   = CI / RI                   (CR ≤ 0.10 が許容範囲)
```

### スコア計算（加重和法）

```
1. 各目的関数値を Min-Max 正規化:
   - is_minimize[j] = true:  normalized = (max_j - v) / (max_j - min_j)  → 小さいほど高スコア
   - is_minimize[j] = false: normalized = (v - min_j) / (max_j - min_j)  → 大きいほど高スコア
   - max_j == min_j の場合: normalized = 0.0
2. AHP スコア = Σ_j w[j] × normalized_j
3. AHP スコア降順でランキング
```

---

## AHP が既存MCDM と異なる点

1. **重みの入力形式**: 直接スライダー入力ではなく一対比較行列から導出
2. **整合性チェック**: CR 計算・警告が必要
3. **UI**: 既存 McdmRankChart とは別の新規チャート (`ChartId::AhpRankChart`, `ChartId::AhpTable`)
4. **メッセージ**: `AppMessage::AhpDone(AhpResult)` を新規追加（`McdmDone` と独立）

---

## 新規追加ファイル一覧

| 役割                | パス                                                              |
| ------------------- | ----------------------------------------------------------------- |
| AHP アルゴリズム    | `rust_core/src/mcdm/ahp.rs`                                       |
| mod.rs 追加         | `rust_core/src/mcdm/mod.rs`                                       |
| AHP 結果型          | `egui-app/src/state/results.rs` (`AhpResult` 追加)                |
| AHP メッセージ      | `egui-app/src/state/messages.rs` (`AppMessage::AhpDone` 追加)     |
| メッセージハンドラ  | `egui-app/src/state/message_handler.rs` (`AhpDone` 分岐追加)      |
| ChartId 追加        | `egui-app/src/state/layout_state.rs` (`AhpRankChart`, `AhpTable`) |
| AHP UI ウィジェット | `egui-app/src/ui/widgets/ahp_chart.rs` (新規)                     |
| WidgetStates 追加   | `egui-app/src/ui/widget_states.rs`                                |
| chart_registry 分岐 | `egui-app/src/ui/chart_registry.rs`                               |
| 理論ドキュメント    | `theory/mcdm/ahp.md`                                              |

---

## 次回タスク番号

TASK-2144 から開始
