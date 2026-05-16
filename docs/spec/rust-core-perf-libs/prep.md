# rust_core 外部ライブラリ高速化 準備タスク（ユーザー作業）

> **仕様**: [requirements.md](requirements.md)
> **生成日**: 2026-05-15

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義で明確に必要と判明したタスク
- 🟡 **黄信号**: 要件定義から妥当に推測されるタスク
- 🔴 **赤信号**: 推測による予防的タスク

---

## 確認済み（設計に反映済み）

- [x] **argmin-math の faer backend 非対応を確認** 🔵 *REQ-201 より*
  - argmin-math の faer バックエンドは faer v0.21 までのサポートで、v0.24 と非互換
  - **決定**: `argmin-math = { version = "0.5", features = ["vec"] }` の vec backend を採用
  - argmin バージョン: 0.11（0.10 は古い）
  - 関連要件: REQ-201

- [x] **linfa-clustering の ndarray 依存を確認** 🔵 *REQ-401 より*
  - linfa-clustering 0.8 は ndarray 0.16 に依存
  - faer::Mat ↔ ndarray::Array2 変換コストは K-means の O(N·K·iter) に対して O(N·M) で無視できる
  - **決定**: `linfa-clustering = "0.8"`、`ndarray = "0.16"` を明示的依存に追加
  - 関連要件: REQ-401, REQ-104

- [x] **faer::Mat のメモリレイアウトと ndarray 変換の整合性** 🔵 *REQ-401, REQ-104 より*
  - faer::Mat は **column-major** だが ndarray::Array2 のデフォルトは **row-major**
  - `as_slice()` による直接変換は転置が発生するため使用不可
  - **決定**: `ndarray::Array2::from_shape_fn((rows, cols), |(i, j)| mat[(i, j)])` による element-wise 変換を採用（設計・型定義に反映済み）
  - 関連要件: REQ-401

- [x] **rand / rand_chacha バージョン互換性の確認** 🔵 *REQ-301 より*
  - rand 0.10 系は rand_chacha が未対応（rand_chacha 0.9.0 が最新）
  - **決定**: `rand = "0.9"` + `rand_chacha = "0.9"` で統一（互換性確認済み）
  - 関連要件: REQ-301

---

## 実装前に確認が必要な事項

- [ ] **linfa-clustering の WCSS 取得 API** 🟡 *REQ-401-04 より*
  - エルボー法で WCSS（Within-Cluster Sum of Squares）を k 別に取得する必要がある
  - linfa-clustering 0.8 の `KMeans` fitted model に `inertia()` 相当のメソッドが存在するか確認
  - 代替案: linfa が WCSS を返さない場合、`KMeansModel::centroids()` と `predict()` から手動計算する
  - 確認方法: `cargo doc -p linfa-clustering --open` で `KMeans` 構造体の公開 API を確認
  - 影響範囲: REQ-401-04 の実装方針
  - 関連要件: REQ-401

- [ ] **argmin と linfa の依存チェーン確認** 🟡 *REQ-201, REQ-401 より*
  - argmin 0.11 と linfa-clustering 0.8 が ndarray 0.16 を共有する場合のバージョン競合確認
  - 確認コマンド: `cargo tree -d` で重複依存を確認
  - 関連要件: REQ-201, REQ-401

---

## サマリー

| 状態 | 件数 | 🔵 | 🟡 | 🔴 |
|------|------|-----|-----|-----|
| 確認済み | 4 | 4 | 0 | 0 |
| 要確認 | 2 | 0 | 2 | 0 |

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ヒアリング記録**: [interview-record.md](interview-record.md)
- **アーキテクチャ**: [../design/rust-core-perf-libs/architecture.md](../../design/rust-core-perf-libs/architecture.md)
