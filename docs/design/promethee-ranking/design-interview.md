# PROMETHEE Ranking 設計ヒアリング記録

**作成日**: 2026-04-29
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の TOPSIS / VIKOR 実装パターン（rust_core + egui-app）を確認し、PROMETHEE I / II の技術実装方針を決定するためのヒアリングを実施した。McdmResult のバリアント設計、Φnet 負値の描画方法、非同期パターンの流用可否、実装ガイドの要否を明確化した。

---

## 質問と回答

### Q1: McdmResult の Promethee バリアント設計

**カテゴリ**: 未定義設計の詳細化
**背景**: PROMETHEE I と II はどちらも同じ `PrometheeResult`（phi_plus / phi_minus / phi_net を含む）を使うが、`primary_scores()` と `ranked_indices()` の dispatch 先が異なる。2 種類の設計案があった：
- **案 A**: `Promethee(PrometheeResult, McdmMethod)` — 1 バリアント + method フィールド
- **案 B**: `PrometheeI(PrometheeResult)` + `PrometheeII(PrometheeResult)` — 2 バリアント分割

**回答**: **2 バリアント分割（案 B）**

**信頼性への影響**:
- REQ-PR-014 の設計方針が 🔵 に確定
- `primary_scores()` が PrometheeI → phi_plus、PrometheeII → phi_net を返すことが明確になった
- `ranked_indices()` が PrometheeI → ranked_indices_i、PrometheeII → ranked_indices_ii を返すことが明確になった
- Rust の exhaustive match チェックにより、将来の McdmMethod 追加時に漏れが防止される

---

### Q2: PROMETHEE II の Φnet 負値バーチャート描画方法

**カテゴリ**: 未定義設計の詳細化
**背景**: Φnet ∈ [-1, 1] は負値を取りうる。既存の TOPSIS / VIKOR バーチャートは [0, 1] スコアを幅に直接使用するため、負値の描画方法を決定する必要があった。選択肢：
- 正規化して [0, 1] にマッピング（全バー同方向）
- 負値は別色（オレンジ）で描画、幅は絶対値

**回答**: **負値は別色（オレンジ #e07000）で描画、幅は絶対値**

**信頼性への影響**:
- REQ-PR-023-B の描画仕様が 🔵 に確定
- PROMETHEE II 描画: `幅 = phi_net[idx].abs()`、`色 = if phi_net >= 0 { #0c6ac0 } else { #e07000 }`
- PROMETHEE I の Φ+ 青 / Φ- 赤 と色体系を統一（青 = 良好、赤/オレンジ = 不利）

---

### Q3: 非同期タスクパターン（spawn_task）の流用可否

**カテゴリ**: 既存設計確認
**背景**: egui-app は `crate::app::spawn_task(tx, move || { ... → AppMessage })` パターンで TOPSIS / VIKOR を非同期実行している。PROMETHEE でも同じパターンを使うか、または専用のキャンセル機構や tokio タスクが必要かを確認した。

**回答**: **既存 spawn_task パターンを流用（推奨）**

**信頼性への影響**:
- Layer 3 の chart_registry.rs 設計が 🔵 に確定
- `std::thread::spawn` + `SyncSender<AppMessage>` のパターンがそのまま適用可能
- O(n²) でも別スレッドで実行するため UI はブロックされない
- キャンセル機構は既存同様不要（試行ごとの計算であり、ユーザーが新しい Run を押すまで待つ）

---

### Q4: 実装ガイドの要否

**カテゴリ**: 追加設計の確認
**背景**: 設計文書に `implementation-guide.md` を含めるかどうかを確認した。architecture.md・dataflow.md・interfaces.rs と重複する可能性があるが、実装者向けの手順書として役立つ場合がある。

**回答**: **含める（推奨）**

**信頼性への影響**:
- `docs/design/promethee-ranking/implementation-guide.md` を作成することが確定
- 実装順序（rust_core → results.rs → message_handler.rs → chart_registry.rs → mcdm_chart.rs）を明示することで、実装者が依存関係を把握しやすくなる

---

## ヒアリング結果サマリー

### 確認できた事項

- McdmResult: **2 バリアント分割**（PrometheeI / PrometheeII）
- Φnet 負値描画: **オレンジ色 (#e07000)、幅 = 絶対値**
- 非同期パターン: **既存 spawn_task を流用**
- 実装ガイド: **作成する**

### 残課題

- なし（全項目がヒアリングで確定）

### 信頼性レベル分布

**ヒアリング前（推定）**:
- 🔵 青信号: 8 件
- 🟡 黄信号: 4 件
- 🔴 赤信号: 0 件

**ヒアリング後**:
- 🔵 青信号: 12 件 (+4)
- 🟡 黄信号: 2 件 (-2)
- 🔴 赤信号: 0 件 (0)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [../../spec/promethee-ranking/requirements.md](../../spec/promethee-ranking/requirements.md)
