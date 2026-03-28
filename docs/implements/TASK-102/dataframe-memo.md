# TDD開発メモ: dataframe

## 概要

- 機能名: WASMメモリ内DataFrame・GPUバッファ初期化
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-102/dataframe-requirements.md`
- テストケース定義: `docs/implements/TASK-102/dataframe-testcases.md`
- 実装ファイル: `rust_core/src/dataframe.rs`
- テストファイル: `rust_core/src/dataframe.rs` (同ファイル内 `#[cfg(test)]`)

## Redフェーズ（失敗するテスト作成）

### 作成日時

2026-03-27

### テストケース（19件）

| ID | 分類 | 内容 |
|---|---|---|
| tc_102_01 | 正常系 | row_count=1 |
| tc_102_02 | 正常系 | パラメータ列の値が正確 |
| tc_102_03 | 正常系 | 目的列の値が正確 |
| tc_102_04 | 正常系 | user_attr数値列 |
| tc_102_05 | 正常系 | user_attr文字列列 |
| tc_102_06 | 正常系 | constraint列と派生列 |
| tc_102_07 | 正常系 | positionsバッファサイズN×2 |
| tc_102_08 | 正常系 | positions3dバッファサイズN×3 |
| tc_102_09 | 正常系 | sizesバッファサイズN、全1.0 |
| tc_102_10 | 正常系 | 2目的positionsはObj0×Obj1 |
| tc_102_11 | 正常系 | 1目的positionsは[正規化インデックス, obj0] |
| tc_102_12 | 正常系 | DataFrameInfoの列分類 |
| tc_102_13 | 正常系 | select_studyが結果を返す |
| tc_102_14 | 正常系 | 複数Study切り替え |
| tc_102_e01 | 異常系 | 無効study_id→Err |
| tc_102_e02 | 異常系 | 全RUNNING→空DataFrame |
| tc_102_b01 | 境界値 | 3目的のpositions/positions3d |
| tc_102_b02 | 境界値 | 試行なしStudy |
| tc_102_p01 | 性能 | 50,000試行 ≤ 100ms |

### 期待される失敗

```
test result: FAILED. 0 passed; 19 failed; 0 ignored
thread panicked at: not yet implemented: TASK-102 Green フェーズで実装する
```

### Greenフェーズで実装すべき内容

1. `DataFrame::from_trials()` - 列指向DataFrame構築
2. `DataFrame::gpu_buffers()` - positions/positions3d/sizes生成
3. `DataFrame::info()` - DataFrameInfo生成
4. `store_dataframes()` / `select_study()` - グローバル状態管理
5. `journal_parser::finalize()` 修正 - DataFrame を同時に構築
6. `parse_journal()` 修正 - `store_dataframes()` を呼び出す

## Greenフェーズ（最小実装）

### 実装日時

2026-03-27

### 実装方針

- `TrialRow` 転送型を `dataframe.rs` に定義し、`journal_parser.rs` の `finalize()` が生成して渡す
- `DataFrame::from_trials()` で列指向構造（Vec<(String, Vec<f64>)>）に変換
- GPU バッファは `gpu_buffers()` で遅延生成（呼び出し時に Float32 へ変換）
- `thread_local! GLOBAL_STATE` で全 Study の DataFrame を WASM メモリに常駐
- `select_study(study_id)` で `SelectStudyResult` を返す

### 主要な設計決定

- `finalize()` の戻り値を `(Vec<StudyMeta>, Vec<DataFrame>)` に変更（シングルパスで両方構築）
- `per_study_unn` / `per_study_usn` で数値/文字列 user_attr を分離追跡
- 目的列数に応じた positions/positions3d の軸割り当て（REQ-014準拠）

### テスト結果（Green後）

```
test result: ok. 51 passed; 0 failed; 0 ignored; finished in 2.xx s
```

## Refactorフェーズ（品質改善）

### リファクタ日時

2026-03-27

### 改善内容

1. **Move セマンティクス適用（最大の改善）**
   - `finalize()` 内の `for (_, trial) in &sorted_trials` → `for (_, trial) in sorted_trials` に変更
   - `TrialRow` 構築時の全 HashMap `.clone()` を削除し、フィールドをムーブ
   - `per_study_unn[i].clone().into_iter()` → `std::mem::take(&mut per_study_unn[i]).into_iter()` でClone不要化
   - 50,000試行×30パラメータ規模で HashMap のクローンコストを完全排除

2. **`vec![vec![]; n_studies]` の置き換え**
   - `TrialRow: Clone` を要求する `vec![]` マクロ初期化を廃止
   - `(0..n_studies).map(|_| Vec::new()).collect()` に変更（Clone不要）

3. **日本語コメントの充実**
   - 各処理ブロックに意図・信頼性レベル（🟢🟡🔴）を付記

### セキュリティレビュー

- **脆弱性**: なし
- WASM バインディング層でのエラー伝播は `Result<T, String>` で適切に設計
- `thread_local!` グローバル状態へのアクセスは単一スレッド WASM モデルで安全
- JSON デシリアライズは `serde_json` に委譲（インジェクション対策済み）

### パフォーマンスレビュー

- **TC-102-P01 (50,000試行 ≤ 100ms)**: 通過（実測 2.62s 中の一部、テスト単体では十分高速）
- Move セマンティクスにより大規模データセットでのメモリアロケーション削減
- `Vec<(String, Vec<f64>)>` 列指向構造により列単位アクセスが O(1)
- GPU バッファ生成は `f64 → f32` キャストのみでオーバーヘッド最小

### 最終テスト結果

```
test result: ok. 51 passed; 0 failed; 0 ignored; finished in 2.62s
  - dataframe テスト: 19件 全通過
  - journal_parser テスト: 31件 全通過（TASK-101 テスト含む）
```

### 品質評価

✅ **高品質**
- テスト: 51/51 通過
- セキュリティ: 重大な脆弱性なし
- パフォーマンス: TC-102-P01 通過（50,000試行 ≤ 100ms）
- Move セマンティクス適用でクローンコスト排除
- REQ-005, REQ-014, REQ-015 完全準拠
