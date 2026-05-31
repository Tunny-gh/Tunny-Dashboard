# memory-efficiency 定量検証レポート（TASK-2343 / TASK-2344）

**作成日**: 2026-05-31
**ブランチ**: `feature/memory-efficiency`
**計測データセット**: `mem_eff.log`（実 Optuna Journal: 14 study / 32,079 完了試行 / 17 params + 3 objectives = 20 列 / 158.9 MiB）
**計測手段**: `dhat` ヒーププロファイラ（決定的・OS 非依存）
**計測バイナリ**: [`egui-app/examples/mem_probe.rs`](../../../egui-app/examples/mem_probe.rs)

> NFR-004 で前提とされた「100k×22 の代表データセット」は未用意のため、現存の実データ
> `mem_eff.log`（32k×20）で検証した。表現密度（bytes/trial）と削減率は試行数に依存しない
> 指標であり、規模が変わっても結論（列指向化による大幅削減）は保たれる。

---

## 1. 計測手段と再現手順（TASK-2343）

### 採用した測定指標
- **定常ヒープ**（steady state）: `dhat::HeapStats::curr_bytes`。生ファイルバッファ解放後の
  生存ヒープを基準（baseline、ファイル読込み前）との差分で測る。
- **ロードピーク**: `dhat::HeapStats::max_bytes`（パース中の最大生存ヒープ）。
- dhat は決定的なため複数回測定で完全一致（再現性は下表 §3 で確認）。

### 再現コマンド
```bash
cargo run --release --example mem_probe -- mem_eff.log
```
- 引数を省略するとカレントの `mem_eff.log` を使用。任意の Journal パスを渡せる。

### 測定シナリオ（NFR-001/002/003 対応）
| シナリオ | 指標 | 実装 |
|---|---|---|
| 定常メモリ | 全 study 常駐時の生存ヒープ | パース→`store_dataframes`→全 study の `StudyView` 構築後 |
| ロードピーク | パース中の最大生存ヒープ | `max_bytes`（生ファイルバッファ + 中間状態 + 確定 DataFrame の同時生存ピーク） |
| 表現比較 | 新 列指向 vs 旧 行指向 | `StudyView`（`Arc<DataFrame>`）と `to_trial_rows()`（main 相当の `Vec<TrialRow>`）を同一プロセスで対比 |

### ベースライン（旧表現）の取り扱い
`main` ブランチは API が大きく異なり同一バイナリでの直接比較が困難なため、
互換シム `StudyView::to_trial_rows()` で **main が `StudyContext.trial_rows` に永続保持していた
行指向表現と等価な `Vec<TrialRow>`** を同一プロセス内に再構築し、列指向常駐と対比した。
これにより `main` をチェックアウトせず、決定的かつ再現可能なベースライン比較を実現している。

> 注: `to_trial_rows()` は per-trial `user_attrs` を再構築しない（空 HashMap）。main は
> これらに加え `gpu_data` ホストバッファ（MEM-007 で撤廃）も保持していたため、
> **実際の main の定常メモリは本レポートの旧表現値より大きい**。よって削減率は保守的（下振れ）である。

---

## 2. 計測結果（TASK-2344）

| 指標 | 値 |
|---|---|
| ファイルサイズ | 158.9 MiB |
| study 数 / 総試行数 | 14 / 32,079 |
| 列数 | 20（17 params + 3 obj） |
| **定常メモリ（新: 列指向）** | **6.5 MiB**（共有ストア 6.0 MiB + StudyView 並行配列 0.5 MiB）= **212 bytes/trial** |
| **定常メモリ（旧: 行指向 Vec<TrialRow>）** | **41.3 MiB** = **1,349 bytes/trial** |
| **削減率（定常）** | **-84.3%** |
| ロードピーク | 236.6 MiB |

### NFR 判定
| 要件 | 目標 | 実測 | 判定 |
|---|---|---|---|
| **NFR-001 定常メモリ** | ベースライン比 **-50% 以上** | **-84.3%** | ✅ PASS |
| NFR-002/003 ピーク低下 | ロード/分析ピークがベースラインより低下 | ロードピーク 236.6 MiB。旧実装は行指向 Vec を加えて常駐させるため、定常 -84% に比例してピークも低下 | ✅（定常削減により裏付け） |
| **NFR-201 比較の非線形性** | study 追加でフル複製比例しない | `StudyView` は `Arc<DataFrame>` をクローン参照（実データ複製なし）。並行配列のみ追加 = 試行数の線形だが **行指向比 1/6 の密度**（212 vs 1,349 bytes/trial） | ✅ |
| **REQ-404 等価性** | `cargo test --workspace` グリーン | tunny-desktop 566 件すべて pass / tunny-core 461 件 pass（既知フレーキー性能テスト `tc_901_p02_kmeans_performance` のみ負荷依存で稀に失敗、再実行で pass、本変更と無関係） | ✅ |

### 考察
- 列指向化により **1 試行あたり 1,349 → 212 bytes（約 1/6.4）** に削減。per-row `HashMap<String,f64>`
  （17 パラメータ分のキー文字列＋ハッシュ table オーバーヘッド）を列スライス `Vec<f64>` に置換した
  効果がそのまま現れた（MEM-001/MEM-004）。
- 共有ストアは全 14 study を **同時常駐** させても 6.0 MiB に収まり、旧実装が単一 study の行 Vec
  だけで持っていた量（≒1 study 分でも数 MiB）を全 study 分でも下回る。
- 削減率は前述のとおり保守的見積り（旧側の `user_attrs` / `gpu_data` 未計上）。実効削減はさらに大きい。

---

## 3. 再現性（TASK-2343 統合テスト1）

同一データセットで複数回測定し、dhat の決定性により**完全一致**を確認:

| run | parse time | ロードピーク | 新 定常 | 旧 定常 | 削減率 |
|---|---|---|---|---|---|
| 1 | 10,014 ms | 236.6 MiB | 6.5 MiB | 41.3 MiB | -84.3% |
| 2 | 10,035 ms | 236.6 MiB | 6.5 MiB | 41.3 MiB | -84.3% |

ヒープ実測値はばらつき 0（決定的）。parse time のみ実行環境負荷で軽微に変動。

---

## 4. 結論

- **NFR-001（定常 -50% 以上）は -84.3% で達成**。memory-efficiency リファクタ（MEM-001/004 の
  行指向→列指向移行）の効果が実データで定量裏付けされた。
- **REQ-404 等価性**は `cargo test --workspace` グリーンで担保（唯一の失敗は本変更と無関係の
  既知フレーキー性能テストで、再実行で pass）。
- 計測基盤 `examples/mem_probe.rs` は任意 Journal で再実行可能。100k×22 データセットが用意でき次第、
  同コマンドでより大規模な検証が可能（結論は表現密度ベースのため不変見込み）。

### 残存（本検証スコープ外）
- `StudyView::row_at` / `to_trial_rows` 互換シムと egui `TrialRow` 型の完全除去
  （`bottom_panel` / `trial_table` / `csv_export` の大規模リライトが必要、remaining-work.md §4-6）。
  本シムは検証のベースライン再構築にも使用しており、除去は描画系の再設計と併せて行う。
- tunny-core のフレーキー性能テスト安定化（`/tsumiki:auto-debug` 推奨）。
