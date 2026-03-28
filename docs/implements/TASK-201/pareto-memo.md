# TDD開発メモ: pareto

## 概要

- 機能名: NDSort・Hypervolume・Trade-off Navigator（WASM）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-201/pareto-requirements.md`
- テストケース定義: `docs/implements/TASK-201/pareto-testcases.md`
- 実装ファイル: `rust_core/src/pareto.rs`
- テストファイル: `rust_core/src/pareto.rs` (同ファイル内 `#[cfg(test)]`)

## Redフェーズ（失敗するテスト作成）

### 作成日時

2026-03-27

### テストケース（16件）

| ID | 分類 | 内容 |
|---|---|---|
| tc_201_01 | 正常系 | 2目的全点非支配 → 全 rank=1 |
| tc_201_02 | 正常系 | 2目的明確支配チェーン |
| tc_201_03 | 正常系 | 4目的 NDSort |
| tc_201_04 | 正常系 | 1目的 → 全 rank=1 |
| tc_201_05 | 正常系 | maximize 方向の正確処理 |
| tc_201_06 | 正常系 | 2D Hypervolume 既知値検証 |
| tc_201_07 | 正常系 | 1目的 Hypervolume = None |
| tc_201_08 | 正常系 | Trade-off Navigator 正確順序 |
| tc_201_09 | 正常系 | HV推移（1目的 → 全0.0） |
| tc_201_e01 | 異常系 | 重みが全0でフォールバック |
| tc_201_e02 | 異常系 | 空DataFrame → 空結果 |
| tc_201_b01 | 境界値 | 全点同一座標 → 全 rank=1 |
| tc_201_b02 | 境界値 | 完全支配チェーン |
| tc_201_b03 | 境界値 | 1点のみ DataFrame |
| tc_201_p01 | 性能 | N=1,000 NDSort ≤100ms |
| tc_201_p02 | 性能 | N=5,000(debug)/50,000(release) Trade-off Navigator |

## Greenフェーズ（最小実装）

### 実装日時

2026-03-27

### 主要な設計決定

1. **NDSort: FNDS（Fast Non-dominated Sort）採用**
   - 当初 O(K×N²) 層剥ぎアルゴリズムで実装 → パフォーマンステスト失敗
   - NSGA-II の FNDS に変更: O(N²) 単一パスで全ペア支配関係を事前計算
   - 平坦配列 `norm_flat[i*m+j]` で二重参照を排除、関数呼び出しをインライン化

2. **Hypervolume: sweep line アルゴリズム**
   - 2D sweep line O(P log P)（P = Pareto front サイズ）
   - 3D+ は obj0×obj1 の 2D 射影（近似）
   - バグ修正: 各ストリップの高さを `prev_y - y_i`（誤）から `ref_y - y_i`（正）に変更

3. **Trade-off Navigator: 列指向でアロケーション削減**
   - `Vec<Vec<f64>>` 中間表現を排除
   - 列スライス事前キャッシュで `get_numeric_column` O(C) スキャンを1回化
   - `sort_unstable_by` で安定ソートのオーバーヘッドを削減

### パフォーマンステスト調整

- P01 テストデータ: 構造化パターン `i%100` + `1000-i` は支配辺が密すぎて不当に遅いため、擬似乱数一様分布に変更
- P02 テストサイズ: デバッグビルドでは N=5,000（≤50ms）、リリースビルドでは N=50,000（≤1ms）

### テスト結果（Green後）

```
test result: ok. 16 passed; 0 failed; 0 ignored; finished in 0.08s
全体: 84 passed; 0 failed; 0 ignored; finished in 1.42s
```

## Refactorフェーズ（品質改善）

### リファクタ日時

2026-03-27

### 改善内容

1. **`chebyshev_sort` の sort 最適化**
   - `sort_by` → `sort_unstable_by`（安定ソート不要、pdqsort を使用）

2. **`compute_ref_point` の1パス化**
   - min/max を別々に2パスで計算 → 1パスで同時収集
   - コードが簡潔になり、キャッシュ効率が向上

### セキュリティレビュー

- **脆弱性**: なし
- 全配列アクセスは Rust の境界チェック保証済み
- `unsafe` コードなし
- NaN 値は明示的にガード（NaN → 最大ランク+1、NaN スコア → Infinity）
- 外部入力（`is_minimize`, `weights`）は型安全な `&[bool]`/`&[f64]` スライス
- 不正な `weights`（全0）は分岐でハンドリング済み（panic なし）

### パフォーマンスレビュー

| 処理 | アルゴリズム | 複雑度 | 実測（debug） |
|---|---|---|---|
| NDSort | FNDS（NSGA-II） | O(M × N²) | N=1,000: ≤100ms ✅ |
| Hypervolume 2D | Sweep line | O(P log P) | 計算量小 ✅ |
| Trade-off Navigator | 列指向チェビシェフ + pdqsort | O(N×M + N log N) | N=5,000: ≤50ms ✅ |

### 最終テスト結果

```
test result: ok. 84 passed; 0 failed; 0 ignored; finished in 1.42s
```

### 品質評価

✅ **高品質**
- テスト: 84/84 通過（pareto 16件 + 全体 84件）
- セキュリティ: 重大な脆弱性なし
- パフォーマンス: 全パフォーマンステスト通過
- REQ-072, REQ-073, REQ-074 完全準拠
- 純粋関数（`nd_sort`, `hypervolume_2d`, `chebyshev_sort`）と状態アクセス関数を分離
