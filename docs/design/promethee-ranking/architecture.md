# PROMETHEE Ranking アーキテクチャ設計

**作成日**: 2026-04-29
**関連要件定義**: [requirements.md](../../spec/promethee-ranking/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書概要・ユーザヒアリングより*

既存の MCDM 機能（TOPSIS / VIKOR）に PROMETHEE I（部分順位付け）と PROMETHEE II（完全順位付け）を追加する。

- 計算はすべて Rust (`rust_core`) で完結し、egui-app の非同期タスクパターン（`spawn_task`）で実行する
- UI は既存の `McdmRankChart` / `McdmTable` ウィジェットを拡張する
- PROMETHEE I は Φ+（青）/ Φ-（赤）の 2 本バー表示、PROMETHEE II は Φnet（正: 青, 負: オレンジ）の単一バー表示

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 egui-app アーキテクチャ・ユーザヒアリングより*

- **パターン**: 4 層メッセージパッシングアーキテクチャ（既存 TOPSIS/VIKOR パターン踏襲）
- **選択理由**: TOPSIS/VIKOR の実装が同一フローで完成しており、開発コストが低く整合性が高い

```
Layer 1: アルゴリズム層 (rust_core/src/mcdm/promethee.rs)
  ↓ tunny_core::mcdm::promethee::compute_promethee()
Layer 2: 型・状態管理層 (egui-app/src/state/)
  ↓ AppMessage::McdmDone(McdmResult::PrometheeI/II)
Layer 3: タスク起動層 (egui-app/src/ui/chart_registry.rs)
  ↓ spawn_task → pending_compute.take()
Layer 4: UI 描画層 (egui-app/src/ui/widgets/mcdm_chart.rs)
```

---

## コンポーネント構成

### Layer 1: アルゴリズム層 🔵

**信頼性**: 🔵 *REQ-PR-001〜008・既存 topsis.rs / vikor.rs パターンより*

**新規ファイル**: `rust_core/src/mcdm/promethee.rs`

| 要素 | 詳細 |
|---|---|
| 公開関数 | `compute_promethee(values, n_trials, n_objectives, weights, is_minimize) -> Result<PrometheeResult, String>` |
| 選好関数 | Linear のみ（q=0, p=range×0.2 自動設定） |
| 出力型 | `PrometheeResult { phi_plus, phi_minus, phi_net, ranked_indices_i, ranked_indices_ii, duration_ms }` |
| NaN 処理 | `filter_valid_indices` 既存関数を流用 |
| バリデーション | `validate_inputs` 既存関数を流用 |

**変更ファイル**:
- `rust_core/src/mcdm/mod.rs`: `pub mod promethee;` 追加
- `rust_core/src/lib.rs`: `pub use mcdm::promethee;` 追加

### Layer 2: 型・状態管理層 🔵

**信頼性**: 🔵 *REQ-PR-010〜014・既存 results.rs / messages.rs パターンより*

**変更ファイル**: `egui-app/src/state/results.rs`

```
追加型:
  PrometheeResult { phi_plus, phi_minus, phi_net, ranked_indices_i, ranked_indices_ii, duration_ms }

McdmMethod enum 拡張:
  Topsis | Vikor | PrometheeI | PrometheeII

McdmResult enum 拡張:
  Topsis(TopsisResult) | Vikor(VikorResult)
  | PrometheeI(PrometheeResult)   // PROMETHEE I  向け: ranked_indices_i 使用
  | PrometheeII(PrometheeResult)  // PROMETHEE II 向け: ranked_indices_ii 使用

McdmResult メソッド更新:
  primary_scores() — PrometheeI → phi_plus, PrometheeII → phi_net
  ranked_indices() — PrometheeI → ranked_indices_i, PrometheeII → ranked_indices_ii
  duration_ms()   — Promethee* → r.duration_ms
  method_label()  — PrometheeI → "PROMETHEE I", PrometheeII → "PROMETHEE II"
```

**変更ファイル**: `egui-app/src/state/message_handler.rs`

```rust
// AppMessage::McdmDone のハンドラに Promethee 分岐を追加
AppMessage::McdmDone(result) => {
    match &result {
        McdmResult::Topsis(r)    => { widget_states.mcdm_chart.cached_topsis = Some(r.clone()); }
        McdmResult::Vikor(r)     => { widget_states.mcdm_chart.cached_vikor = Some(r.clone()); }
        McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
            widget_states.mcdm_chart.cached_promethee = Some(r.clone());
        }
    }
    app_state.mcdm_result = Some(result);
    widget_states.mcdm_chart.computing = false;
}
```

### Layer 3: タスク起動層 🔵

**信頼性**: 🔵 *既存 chart_registry.rs の pending_compute パターン・ユーザヒアリングより*

**変更ファイル**: `egui-app/src/ui/chart_registry.rs`

```rust
// pending_compute ハンドラに Promethee 分岐を追加
McdmMethod::PrometheeI | McdmMethod::PrometheeII => {
    crate::app::spawn_task(tx, move || {
        match tunny_core::mcdm::promethee::compute_promethee(
            &objectives, n_trials, n_objectives, &weights, &is_minimize,
        ) {
            Ok(r) => {
                let result = PrometheeResult {
                    phi_plus: r.phi_plus, phi_minus: r.phi_minus, phi_net: r.phi_net,
                    ranked_indices_i: r.ranked_indices_i,
                    ranked_indices_ii: r.ranked_indices_ii,
                    duration_ms: r.duration_ms,
                };
                let mcdm = if method == McdmMethod::PrometheeI {
                    McdmResult::PrometheeI(result)
                } else {
                    McdmResult::PrometheeII(result)
                };
                AppMessage::McdmDone(mcdm)
            }
            Err(e) => AppMessage::Error(format!("PROMETHEE computation failed: {e}")),
        }
    });
}
```

### Layer 4: UI 描画層 🔵

**信頼性**: 🔵 *REQ-PR-020〜024・ユーザヒアリング・既存 mcdm_chart.rs パターンより*

**変更ファイル**: `egui-app/src/ui/widgets/mcdm_chart.rs`

| 変更項目 | 詳細 |
|---|---|
| `McdmRankChart` | `cached_promethee: Option<PrometheeResult>` フィールド追加 |
| メソッドコンボ切替 | `PrometheeI` / `PrometheeII` → `cached_promethee` から復元 |
| PROMETHEE I バー | Φ+ 青 (`#0c6ac0`) + Φ- 赤 (`#c02020`) の 2 本バー |
| PROMETHEE II バー | Φnet 正値: 青 (`#0c6ac0`)、負値: オレンジ (`#e07000`)、幅は絶対値 |

---

## システム構成図 🔵

**信頼性**: 🔵 *既存アーキテクチャ・ユーザヒアリングより*

```
ユーザー操作
  └─ McdmRankChart.show()
       ├─ ComboBox: PROMETHEE I / II を選択
       ├─ Weights スライダー調整
       └─ "Run" ボタン押下
            ↓ pending_compute = Some(McdmComputeRequest { method: PrometheeI/II, weights, v })

chart_registry.rs
  └─ if let Some(req) = widgets.mcdm_chart.pending_compute.take()
       └─ spawn_task(tx, move || {
              compute_promethee(objectives, n_trials, n_obj, weights, is_minimize)
              → AppMessage::McdmDone(McdmResult::PrometheeI/II(PrometheeResult))
          })

message_handler.rs
  └─ AppMessage::McdmDone(result)
       ├─ cached_promethee = Some(r.clone())
       ├─ app_state.mcdm_result = Some(result)
       └─ mcdm_chart.computing = false

chart_registry.rs (次フレーム)
  └─ mcdm_chart.show(ui, obj_names, &app_state.mcdm_result, trial_rows)
       ├─ PROMETHEE I:  Φ+ 青バー + Φ- 赤バー (ranked_indices_i 順)
       └─ PROMETHEE II: Φnet バー (正:青 / 負:オレンジ) (ranked_indices_ii 順)
```

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

**新規ファイル**:
```
rust_core/src/mcdm/
└── promethee.rs      ← 新規: アルゴリズム実装 + テスト
```

**変更ファイル**:
```
rust_core/src/mcdm/
├── mod.rs            ← pub mod promethee; 追加
rust_core/src/
└── lib.rs            ← pub use mcdm::promethee; 追加

egui-app/src/state/
├── results.rs        ← PrometheeResult, McdmMethod 拡張, McdmResult 拡張
└── message_handler.rs← Promethee 分岐追加

egui-app/src/ui/
├── chart_registry.rs ← spawn_task に Promethee 分岐追加
└── widgets/
    └── mcdm_chart.rs ← cached_promethee, 2 本バー, Φnet バー追加
```

---

## 非機能要件の実現方法

### パフォーマンス 🟡

**信頼性**: 🟡 *NFR-PR-001〜002・O(n²) 特性から妥当な推測*

- `compute_promethee` の O(n²) ループは行優先（row-major）flat `Vec<f64>` で実装しキャッシュ効率を向上
- 重みつき集約選好行列 π(a,b) を1パスで計算し、正/負フローを別パスで集計
- 50,000 試行 × 4 目的: 200 ms 以内（TOPSIS 100ms の 2 倍目標）
- 10,000 試行 × 4 目的: 20 ms 以内

### エラー耐性 🔵

**信頼性**: 🔵 *NFR-PR-020・既存パターンより*

- `compute_promethee` がエラーを返した場合は `AppMessage::Error(...)` を送信
- UI はエラーメッセージを `egui::Label` で表示し、クラッシュしない

### コード規約 🔵

**信頼性**: 🔵 *NFR-PR-010〜011・既存 topsis.rs / vikor.rs より*

- `promethee.rs` のスタイルは `topsis.rs` / `vikor.rs` に準拠（ドキュメントコメント英語、テスト命名 `tc_pr_XXX_NN`）
- egui-app 側は Tailwind CSS 禁止（インラインスタイルのみ）

---

## 技術的制約

### O(n²) 計算量 🔵

**信頼性**: 🔵 *PROMETHEE アルゴリズム特性・ユーザヒアリングより*

- PROMETHEE は全ペア比較が必要で O(n²) の計算量。TOPSIS/VIKOR の O(n) と比べて大きい
- 50,000 試行では 2.5 × 10⁹ ペアとなりタイムアウトリスクがある
- → **flat Vec による行優先アクセス**と**並列化検討**（ただし初回実装は非並列）でリスク軽減

### McdmResult の 2 バリアント分割 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

- `PrometheeI(PrometheeResult)` と `PrometheeII(PrometheeResult)` を別バリアントとして定義
- `primary_scores()` と `ranked_indices()` がどちらの ranked_indices を使うか明確になる
- `match` で exhaustive チェックが働き、将来のメソッド追加時に漏れを防ぐ

### cached_promethee の共有 🟡

**信頼性**: 🟡 *ユーザヒアリング・既存 cached_topsis / cached_vikor パターンから妥当な推測*

- PROMETHEE I と II は同じ `PrometheeResult`（phi_plus / phi_minus / phi_net をすべて含む）を返すため、キャッシュは 1 フィールド `cached_promethee: Option<PrometheeResult>` で共有できる
- I → II または II → I 切替時はキャッシュから即時復元

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [../../spec/promethee-ranking/requirements.md](../../spec/promethee-ranking/requirements.md)
- **既存 MCDM 設計**: [../proprietary-features/architecture.md](../proprietary-features/architecture.md)
- **理論**: [theory/mcdm/topsis.md](../../../theory/mcdm/topsis.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12 件 (86%)
- 🟡 黄信号: 2 件 (14%)
- 🔴 赤信号: 0 件 (0%)

**品質評価**: 高品質
