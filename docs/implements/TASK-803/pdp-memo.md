# TDD開発メモ: pdp

## 概要

- 機能名: 部分依存プロット（PDP）WASM実装（Ridge簡易版）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `rust_core/src/pdp.rs`: Rust PDP計算実装
  - `frontend/src/wasm/workers/onnxWorker.ts`: ONNX Worker スタブ
  - `frontend/src/wasm/workers/OnnxWorkerClient.ts`: ONNX Worker クライアント
- テストファイル:
  - `rust_core/src/pdp.rs` (インラインテスト)
  - `frontend/src/wasm/workers/OnnxWorkerClient.test.ts`

## テストケース（17件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-803-01 | 正常系 | y=x1のとき x1 PDP が単調増加 |
| TC-803-02 | 正常系 | y=-x1 のとき x1 PDP が単調減少 |
| TC-803-03 | 正常系 | 中点 PDP 値 ≈ y_mean（±5%） |
| TC-803-04 | 正常系 | 完全線形データで R² > 0.99 |
| TC-803-05 | 境界値 | n < 2 で空の PdpResult1d を返す |
| TC-803-06 | 正常系 | 2変数PDP の結果行列が n_grid×n_grid |
| TC-803-07 | 境界値 | n < 2 で 2変数PDPも空を返す |
| TC-803-08 | 正常系 | 2変数PDP で R² > 0.95 |
| TC-803-09 | 正常系 | param_name・objective_name が正しく保持 |
| TC-803-P01 | 性能 | 1変数PDP: debug=1k×4、release=50k×10 ≤20ms |
| TC-803-P02 | 性能 | 2変数PDP: debug=1k×4、release=50k×10 ≤100ms |
| TC-803-W01 | 正常系 | load() がスタブ段階で success=false を返す |
| TC-803-W02 | 正常系 | load() が Worker に load メッセージを送信 |
| TC-803-W03 | 正常系 | infer() が空の Float32Array を返す |
| TC-803-W04 | 正常系 | infer() が Worker に infer メッセージを送信 |
| TC-803-W05 | 正常系 | load リクエストのメッセージ構造が正しい |
| TC-803-W06 | 正常系 | infer リクエストのメッセージ構造が正しい |

## 主要な設計決定

1. **Ridge 解析的PDP**
   - 線形モデルのPDPは解析的に計算可能:
     `f̄_j(v) = y_mean + β_j * (v - mean_j) / std_j`
   - N 回の予測計算が不要 → グリッド点数に関わらず O(1) の追加コスト
   - TASK-801 の `compute_ridge()` を再利用

2. **2変数PDPも解析的計算**
   - `f̄_{j1,j2}(v1, v2) = y_mean + β_j1*(v1-mean_j1)/std_j1 + β_j2*(v2-mean_j2)/std_j2`
   - 線形モデルなので真の交互作用項はない（ONNX版で改善予定）

3. **ONNX Worker スタブ**
   - workerFactory 注入パターンで依存性注入
   - スタブは success=false を返してRidgeフォールバックを促す
   - 将来的に `onnxruntime-web` を import して実装

4. **パフォーマンス**
   - ボトルネックは Ridge 拡張 (O(np²)) → TASK-801 の最適化済み実装を使用
   - `#[cfg(debug_assertions)]` で debug 時はデータサイズを縮小

## 最終テスト結果

```
Running pdp tests
test result: ok. 11 passed; 0 failed

Running OnnxWorkerClient tests
test result: ok. 6 passed; 0 failed

All Rust tests: 111 passed; 0 failed
All frontend tests: 119 passed; 0 failed
```

## 品質評価

✅ **高品質**
- テスト: 17/17 通過
- セキュリティ: 重大な脆弱性なし（境界チェック済み）
- パフォーマンス: debug/release 両モードで要件達成
- REQ-100〜REQ-101 準拠
