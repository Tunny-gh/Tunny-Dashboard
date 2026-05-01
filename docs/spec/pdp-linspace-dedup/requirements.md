# pdp-linspace-dedup 要件定義書（軽量版）

## 概要

`linspace` 関数が `rust_core/src/core/lgbm.rs`（`pdp_linspace`）と `rust_core/src/pdp/utils.rs`（`linspace`）に同一実装で存在する。可視性の境界により `lgbm.rs` から `pdp::utils::linspace` にアクセスできないため重複が発生している。共通配置先に移動して重複を解消する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)

## 主要機能要件

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存実装・コード調査に基づく確実な要件
- 🟡 **黄信号**: 既存実装から妥当な推測による要件
- 🔴 **赤信号**: 既存実装にない推測による要件

### 必須機能（Must Have）

- REQ-001: `linspace` 関数を `core::math` モジュールに新規ファイルとして配置し、`pub(crate)` 公開とする 🔵 *既存 core::math/mod.rs 構造より*
- REQ-002: `pdp::utils::linspace` の定義を `core::math` からのインポートに変更し、`pdp` モジュール内の既存呼び出し元（`ridge_core.rs`, `kriging_core.rs`）は修正不要とする 🔵 *既存 pdp/ridge_core.rs・pdp/kriging_core.rs の import 文より*
- REQ-003: `core::lgbm::pdp_linspace` を削除し、`core::math` の `linspace` を使用する 🔵 *既存 lgbm.rs の使用箇所（3件）より*
- REQ-004: 全既存テストが引き続き通過する 🔵 *現状 704 テスト通過より*

### 基本的な制約

- REQ-401: `pdp` → `core` の既存依存方向を維持し、逆方向の依存を新たに作らない 🔵 *モジュール構造 pdp/mod.rs の use 宣言より*
- REQ-402: `pdp::utils` の `col_mean_std` 関数は移動対象外とする 🔵 *col_mean_std は pdp モジュール内でのみ使用されているため*

## 簡易ユーザーストーリー

### ストーリー1: linspace の共通化

**私は** 開発者 **として**
**linspace を単一の配置先から利用したい**
**そうすることで** 将来の修正時に片方だけ直すバグを防げる

**関連要件**: REQ-001, REQ-002, REQ-003

## 基本的な受け入れ基準

### REQ-001: core::math への配置

**Given**: `core::math` ディレクトリに `linear_algebra.rs` のみが存在する
**When**: 新規ファイル `core::math/grid.rs` に `linspace` を `pub(crate)` で追加する
**Then**: `lgbm.rs` と `pdp::utils` の両方から `crate::core::math::grid::linspace` として呼び出し可能

**テストケース**:
- [ ] 正常系: `linspace(0.0, 1.0, 5)` が `[0.0, 0.25, 0.5, 0.75, 1.0]` を返す
- [ ] 境界値: `linspace(0.0, 1.0, 0)` が `[]` を返す
- [ ] 境界値: `linspace(0.0, 1.0, 1)` が `[0.5]` を返す

### REQ-003: pdp_linspace の削除

**Given**: `lgbm.rs` に `pdp_linspace` 関数が定義されている
**When**: `pdp_linspace` を削除し `crate::core::math::grid::linspace` に置換する
**Then**: コンパイルが通り、既存テストが全て通過する

**テストケース**:
- [ ] `cargo test --workspace` が 704 テスト通過（4 ignored）

## 最小限の非機能要件

- **パフォーマンス**: linspace 自体は純粋関数であり、移動による性能変化なし
- **保守性**: 単一配置により将来の修正コストが半減
