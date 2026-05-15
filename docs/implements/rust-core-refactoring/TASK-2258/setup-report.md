# TASK-2258 設定作業実行

## 作業概要

- **タスクID**: TASK-2258
- **作業内容**: SensitivityMetric トレイト定義と SensitivityKind リネーム
- **実行日時**: 2026-05-15
- **実行者**: Claude Code (direct-setup)

## 設計文書参照

- **参照文書**:
  - `docs/design/rust-core-refactoring/architecture.md` (A-1 SensitivityMetric トレイト導入)
  - `docs/design/rust-core-refactoring/interfaces.rs` (型定義仕様)
  - `docs/design/rust-core-refactoring/design-interview.md` (Q3 名前衝突解消)
  - `docs/tasks/rust-core-refactoring/TASK-2258.md` (タスク詳細)
- **関連要件**: REQ-A01, REQ-A02, REQ-A03

## 実行した作業

### 1. metric_trait.rs の新規作成

**作成ファイル**: `rust_core/src/sensitivity/metric_trait.rs`

```rust
pub trait SensitivityMetric: Send + Sync {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult>;
    fn name(&self) -> &'static str;
}
```

**設計内容**:
- `Send + Sync` スーパートレイトによりマルチスレッド安全性を保証
- `compute()` は計算失敗時に `None` を返す（パニックなし）
- `name()` は静的文字列を返し、ログ・デバッグ用の識別子として使用

### 2. enum SensitivityMetric → SensitivityKind リネーム

**変更ファイル**: `rust_core/src/sensitivity/types.rs`

- `pub enum SensitivityMetric` → `pub enum SensitivityKind` にリネーム
- ドキュメントコメント追加（リネーム理由を明記）

### 3. mod.rs の更新

**変更ファイル**: `rust_core/src/sensitivity/mod.rs`

```rust
mod metric_trait;  // 追加
pub use metric_trait::SensitivityMetric;  // 追加
pub use types::{
    // SensitivityMetric → SensitivityKind に変更
    MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityKind, SensitivityResult,
    ShapResult, SobolResult, TreeImportanceResult,
};
```

### 4. rust_core 内の参照更新

**変更ファイル**:
- `rust_core/src/sensitivity/analysis/full.rs` — `SensitivityMetric` → `SensitivityKind` (import + 10箇所の match arm)
- `rust_core/src/sensitivity/tests.rs` — `SensitivityMetric::Permutation` → `SensitivityKind::Permutation` (2箇所)

### 5. egui-app 内の参照更新

**変更ファイル**:
- `egui-app/src/ui/poll_chart.rs` — `tunny_core::sensitivity::SensitivityMetric::` → `tunny_core::sensitivity::SensitivityKind::` (6箇所)

## 作業結果

- [x] `rust_core/src/sensitivity/metric_trait.rs` が作成され、`SensitivityMetric` トレイトが定義されている
- [x] `rust_core/src/sensitivity/types.rs` 内の enum が `SensitivityKind` にリネームされている
- [x] egui-app 内の `SensitivityMetric` 参照が `SensitivityKind` に更新されている
- [x] `rust_core/src/sensitivity/mod.rs` に `pub use metric_trait::SensitivityMetric;` が追加されている
- [x] `cargo build -p tunny-core` が通る
- [x] `cargo build -p tunny-desktop` が通る
- [x] `cargo test -p tunny-core` 全 363 テスト通過（4 ignored）

## 遭遇した問題と解決方法

問題は発生しなかった。すべての変更はコンパイルエラーなしで一回で成功した。

## 変更ファイル一覧

| ファイル | 操作 | 変更内容 |
|----------|------|----------|
| `rust_core/src/sensitivity/metric_trait.rs` | 新規作成 | `SensitivityMetric` トレイト定義 |
| `rust_core/src/sensitivity/types.rs` | 変更 | enum 名 `SensitivityMetric` → `SensitivityKind` |
| `rust_core/src/sensitivity/mod.rs` | 変更 | `mod metric_trait` 追加、再エクスポート更新 |
| `rust_core/src/sensitivity/analysis/full.rs` | 変更 | import + match arm の enum 名更新 |
| `rust_core/src/sensitivity/tests.rs` | 変更 | テスト内の enum 名更新 |
| `egui-app/src/ui/poll_chart.rs` | 変更 | 6箇所の enum パス更新 |

## 次のステップ

- `/tsumiki:direct-verify rust-core-refactoring TASK-2258` を実行して設定を確認
- 後続タスク TASK-2259 (SpearmanMetric, RidgeMetric の実装) と TASK-2260 (tree-based metrics の実装) を実行可能
