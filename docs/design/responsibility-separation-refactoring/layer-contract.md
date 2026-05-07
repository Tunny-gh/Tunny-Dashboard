# 3層境界契約（Pure Logic/Data・App State・UI View）

## 依存方向

- `Pure Logic/Data` は `App State` と `UI View` を知らない。
- `App State` は `Pure Logic/Data` を利用してよいが、`UI View`（egui/theme）に依存しない。
- `UI View` は `App State` の読み取り結果を表示し、入力は Action/Intent として返す。

## 各層の責務

- `Pure Logic/Data`
  - プレーンなデータ構造体、計算ロジック、集計ロジック
  - UI型（`egui::Color32` など）禁止
- `App State`
  - 画面状態、入力中バッファ、非同期タスクの起動・完了反映
  - UIイベントの集約ポイント（状態遷移の唯一の入口）
- `UI View`
  - `&mut Ui` と描画用データに基づく描画
  - 直接I/O・計算起動を行わず、Action/Intent を発火

## 禁止依存ルール

- `state/*` から `egui` / `theme` への依存を禁止する。
- `show_*` など UI関数が `AppState` を直接書き換える責務を増やさない。
- 描画用の色情報キャッシュは UI 層状態に保持する。
