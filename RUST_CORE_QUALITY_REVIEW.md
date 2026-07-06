# rust_core (tunny-core) リリース前品質レビュー

- 実施日: 2026-07-06
- 対象: `rust_core/src/` 全モジュール（約42,000行・156ファイル）
- 観点: 脆弱性・堅牢性 / 保守性 / 重複 / 速度
- 体制: モジュール群ごとの並列レビュー（6担当）+ モジュール横断の重複検出（1担当）+ 統括による重要指摘の実コード裏取り
- 静的解析: `cargo clippy -p tunny-core --all-targets --locked -- -D warnings` は警告ゼロで通過

## 総評

リリースを止める水準の欠陥は少なく、全体の品質は高い。SQL は全経路でプレースホルダ化されておりインジェクションなし、HTML/SVG レポートのエスケープは網羅的、NaN/Inf・空データ・ゼロ除算のガードも大半のモジュールで一貫している。

一方で、**リリース前に修正すべき High が4件**ある。いずれも局所的な修正で対応可能。

| 判定 | 件数 | 内訳 |
|---|---|---|
| High（リリース前に修正推奨） | 4 | メモリ安全性 2、到達可能 panic 1、無効値の静かな伝播 1 |
| Medium | 約20 | 性能（N+1・ホットループ clone・rayon 未活用）、重複、長大関数 |
| Low | 多数 | 一貫性・デッドコード・プレースホルダ doc 等 |

---

## High（リリース前に修正推奨）

### H1. `lgbm_feature_importance` のヒープ外書き込みの可能性 — メモリ安全性
`rust_core/src/lgbm.rs:226-234`

出力バッファを呼び出し側指定の `n_features` で確保するが、`LGBM_BoosterFeatureImportance` はモデルの実特徴量数だけ書き込む。`n_features` がモデルの特徴量数より小さいとヒープ外書き込み（UB）になる。

**修正案**: `LGBM_BoosterGetNumFeature` で実特徴量数を取得してバッファ長に使う（最低でも一致検証を入れる）。

### H2. LightGBM 入力の非矩形行列で OOB リード — メモリ安全性
`rust_core/src/lgbm.rs:99-121, 265-302, 376-388, 451-460`

`ncol` を `x[0].len()` で決める一方、`flat_x` は全行を連結して作るため、行ごとに長さが異なると C API が `nrow*ncol` 要素を読み、確保外メモリを読む。

**修正案**: 入口で全行の長さ一致（および `param_idx < ncol`）を検証し、不一致は `Err`/`None` を返す。

### H3. CMA-ES の固有値分解 `.expect()` による到達可能 panic — 堅牢性
`rust_core/src/surrogate_opt/optimizers/cma_es.rs:210-212`

共分散行列が世代を跨いで発散（NaN/Inf）した場合、`self_adjoint_eigen(...).expect(...)` が panic する。コードベースの他箇所は graceful degradation（GP は `catch_unwind`、他は Option/Result）で統一されているが、この経路のみ未捕捉。

**修正案**: `symmetric_eigen` を `Option`/`Result` 化し、失敗時は現時点の `best` を返して最適化を打ち切る。

### H4. `ward_linkage` が NaN 入力で範囲外 panic — 堅牢性
`rust_core/src/clustering/hierarchical.rs:49, 122-134`

入力に NaN があると標準化で列全体が NaN 化 → 全ペア距離が NaN → 最近傍探索の `dd < best_d` が常に偽で `best = usize::MAX` のままチェーンに積まれ、`dist[c*n+b]` の添字計算がオーバーフロー/範囲外 panic に到達する。現状は UI 側の事前フィルタで回避されているだけで、公開関数として無防備（他の兄弟関数はすべて NaN をフィルタしている）。

**修正案**: 関数冒頭で NaN 行を除外するか、近傍未発見（`best == usize::MAX`）で早期 return する。

### H5. Sobol 感度のみ NaN/Inf フィルタが欠落し無効な指標を返す — 正確性
`rust_core/src/sensitivity/sobol.rs:206-235`

他の感度指標はすべて `prepare_training_data` で NaN/Inf 行を除去するが、`compute_sobol_from_df` だけフィルタせず代理モデルを構築する。失敗/枝刈り trial の NaN 目的値が標準化 → ridge に伝播し、panic せず first_order / total_effect がすべて NaN の「静かに壊れた」結果を返す。

**修正案**: 他手法と同様に有限値の行のみ抽出してから代理モデルを構築する。

---

## Medium（リリース後早期の改善を推奨）

### 性能

| # | 場所 | 問題 | 修正案 |
|---|---|---|---|
| M1 | `surrogate_opt/ehvi.rs:93-99` | `ehvi()` が MC 128 サンプルごとにパレートフロント全体をクローン。L-BFGS マルチスタートで候補あたり 10^5〜10^6 回呼ばれるホットパス | HV 寄与の増分計算 or 前計算済み inclusive HV の利用 |
| M2 | `io/rdb/generic.rs:324-325` | `scan_study_list` が directions / metric_names を study 毎に発行する N+1（リモート PG/MySQL で往復レイテンシが支配的） | 全 study 分を 1 クエリで読んで map 化 |
| M3 | `io/sqlite_backend.rs:59` ほか各バックエンド | クエリ結果全行を `Vec<Vec<SqlValue>>` に一括マテリアライズ。巨大 DB で OOM リスク | 行コールバック/ページング方式へ |
| M4 | `sensitivity/tree/fanova.rs:328` | 区間ごとに不変な葉の周辺重みを区間ループ内で再計算（O(葉数×区間数)） | (leaf, dim) 単位で 1 度だけ計算しキャッシュ |
| M5 | `surrogate_opt` 全般（`validation.rs:189`, `mod.rs:94,1221` 等） | rayon 依存があるのに CV k-fold・Auto 候補評価・目的別 fit が逐次 | 決定性を保てる単位で `par_iter` 化 |
| M6 | `multi_objective/indicators.rs:194-256` | 収束指標系列の計算がステップ毎に全再構築 + 参照集合サイズ無制限で実質 O(n³) | 系列の rayon 並列化 or 参照集合のサブサンプリング |
| M7 | `contour/mod.rs:133-170` | 格子セルごとに全三角形を線形走査（O(grid² × tris)） | 三角形の bbox バケット化 or Delaunay 隣接探索 |
| M8 | `report/svg.rs` 全域 | 描画要素ごとに `format!` の使い捨て String を確保 | `write!(body, ...)`（`std::fmt::Write`）へ |

### 堅牢性・正確性

| # | 場所 | 問題 | 修正案 |
|---|---|---|---|
| M9 | `io/url.rs:138`, `io/rdb/mod.rs:38` | sslmode 無指定の PostgreSQL 接続が警告なく `NoTls`（平文）で資格情報を送信 | 平文接続の明示 opt-in 化 or TLS 実装 |
| M10 | `report/markdown.rs:64-68` | Markdown エスケープが `< > &` を素通し。下流で HTML 化される場合（MCP 表示等）に XSS/構造破壊の余地 | `<` `&` もエスケープ、または「HTML 化しない」前提を明文化 |
| M11 | `lgbm.rs:169-183` | `UpdateOneIter` のエラーを正常終了と同一視 + `lgbm_predict` が失敗を空 Vec に潰し FFI エラーが沈黙 | 学習失敗は `None`、予測は `Result`/`Option` 化 |
| M12 | `report/html.rs:569-576`, `report/markdown.rs:285-295` | 制約違反件数を cap 済みテーブルから数えるため過少表示の可能性 | 未 cap のフロント全体から算出 |

### 保守性・重複

| # | 場所 | 問題 | 修正案 |
|---|---|---|---|
| M13 | `io/parser.rs:32-83, 215-351, 542-760` | journal パースの op_code 分岐が 3 実装に重複、`stream_emit_completed_trial` は引数 15 個 | op ディスパッチの共通化 + 累積状態の struct 化 |
| M14 | `report/html.rs` ↔ `report/markdown.rs` | trial 表・極値表・MCDM 表・集計判定が両レンダラでほぼ同一に再実装 | 集計ロジックをモデル側/共有モジュールへ |
| M15 | `surrogate_opt/acquisition.rs:214-397`, `report/html.rs:415-586`, `report/builder.rs:587-725` | 150〜180 行超の長大関数に責務が混在 | セクション/フェーズ単位のヘルパへ分割 |
| M16 | `pdp/api.rs:46-102` | `gp_fitc`/`gp_vfe`/`gp_moe` の 3 アームが定数以外完全同一（約 18 行 × 3） | `GpMethod` を先に解決して 1 本化 |
| M17 | `mcdm/topsis.rs` ほか多数 | 公開 API の doc が `"Documentation."` 等のプレースホルダのまま、一部テストメッセージが文字化け | 実内容の記述 or 削除 |

### モジュール横断の重複（集約候補）

横断スキャンで「実質同一ロジック」と本体を読み比べて確認したもの:

1. **正規化（合計1化）** — `mcdm/mod.rs:11` / `multi_objective/weights.rs:1` / `sensitivity/tree/common.rs:158` の 3 実装。フォールバック挙動（uniform / zero / 無ガード）が食い違っており、`multi_objective/weights.rs` 版は未使用かつ最も脆弱 → 削除して `mcdm` 版 or `math/` へ集約
2. **パレート支配判定** — `multi_objective/pareto/helpers.rs:4` / `surrogate_opt/ehvi.rs:156` / `surrogate_opt/optimizers/nsga2.rs:152` の 3 実装 → `pub(crate)` で 1 箇所へ
3. **分位点計算（NumPy type-7）** — `statistics/boxplot.rs:104` / `surrogate_opt/robustness.rs:289` → `statistics::quantile` へ一本化
4. **Sturges ビン数** — `report/builder.rs:967` / `statistics/histogram.rs:105`（関数名まで同一） → `compute_histogram(BinRule::Sturges)` の再利用
5. **有限ペア抽出 + 相関委譲** — `statistics/correlation.rs:53` / `report/builder.rs:277` → `pairwise_correlation` の再利用
6. **列 z-score 標準化** — `clustering/hierarchical.rs:69-79` / `clustering/som.rs:75-93` / `clustering/pca.rs:59-76`（pca は分母 n-1 で意味差あり）→ `standardize_columns` ヘルパへ
7. **高速非劣ソート** — `multi_objective/pareto/ranking.rs:11` / `surrogate_opt/optimizers/nsga2.rs:157` → 性能要件が合えば `nd_sort` の `pub(crate)` 化で統一
8. **Box-Muller 正規乱数** — `math/rng.rs` の `next_gaussian` があるのに `cma_es.rs:166-177` / `ehvi.rs:184-195` で再実装
9. **min/max 単一パス計算** — `pdp/utils.rs` / `sensitivity/sobol.rs` / `sensitivity/tree/fanova.rs` の 4 箇所 → `math/` へ

---

## Low（気付き次第の改善で可）

- `io/artifacts.rs:161` — `map_while(Result::ok)` が最初の非 UTF-8 行で走査を打ち切り、以降のメタデータを黙って取りこぼす → `filter_map` へ
- `io/rdb/generic.rs:455-458` ほか — DB の id 列を `as u32` で無検証切り捨て（`artifacts.rs` は `try_from` で保護済みと不整合）
- `io/datetime.rs:30` — 日の妥当性が一律 `1..=31`（2/31 を受理）
- `io/model.rs:250-337` — `append_trials` の列引きが線形探索で 1 バッチ O(C²) → HashMap 化
- `sensitivity/metrics.rs:200` ほか — 列スライス `col[..n]` と `.take(n)` の安全性規約が手法間で不整合
- `pdp/utils.rs:78` vs `sensitivity/ridge.rs:223` — 定数 y の R² 規約が 1.0 / 0.0 で分岐（UI 表示の矛盾リスク）
- `lgbm.rs:99, 128, 265` — 行列寸法の `as i32` 変換が無検証（`i32::try_from` へ）
- `build.rs:16-19` — Linux 分岐に rpath がなくライブラリ名も `lib_lightgbm`（`liblib_lightgbm.so` 期待）で実行時解決に失敗する可能性
- `clustering/kmeans.rs:9-11` — flat データ長が非整除のとき空結果を黙って返す
- `mcdm/vikor.rs:19-26` — 重み `v` の [0,1] 範囲検証なし
- `report/model.rs:204, 233` — `objective_index` / `feasible` / `scatter_axes` が set のみで未使用（デッドコード寄り）
- `gaussian_process.rs:269-280` — `predict_*` の入力次元検査なし（現呼び出しでは到達不能）
- プレースホルダ doc コメント（`/// Documentation.` 等）が io/ / mcdm/ / clustering/ に多数残存

---

## 良好だった点（変更不要と確認済み）

- **SQL インジェクションなし**: 3 バックエンドとも全経路で `?` プレースホルダ + バインドに統一
- **HTML/SVG エスケープ**: `escape_xml` が `& < > " '` を網羅し、ユーザー由来文字列は全て `esc()` 経由
- **FFI のリソース管理**: `Drop` の null ガード、エラー時の手動 Free、CString 生存期間、`unsafe impl Send`（Sync 非実装）の判断はいずれも妥当で、二重解放・リーク・ダングリングなし
- **数値堅牢性の基本線**: `column_mean_std` / `normalize_x_minmax` / `linspace` / `quantile` 等の主要プリミティブはゼロ除算・空・単一点をガード済み。乱数は固定シード（ChaCha8 等）で再現性確保
- **GP 実装の階層**: `gaussian_process.rs` と `surrogate_opt/models.rs` は「薄いラッパ」の健全な関係で重複なし
- **clippy**: `--all-targets -D warnings` で警告ゼロ

## 推奨アクション順序

1. **リリース前必須**: H1〜H5（いずれも局所修正。H1/H2 はメモリ安全性のため最優先）
2. **リリース前推奨**: M9（PostgreSQL 平文接続の opt-in 化）、M10（Markdown エスケープ）、M11（LightGBM エラーの可視化）
3. **リリース後の整理**: 横断重複 9 グループの集約（機械的作業として分担可能）、長大関数の分割、プレースホルダ doc の解消
4. **性能改善**: M1（EHVI クローン）→ M2/M3（RDB 経路）→ M4〜M8 の順で効果が大きい
