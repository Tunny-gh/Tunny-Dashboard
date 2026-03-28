# TDD開発メモ: csv-export

## 概要

- 機能名: CSVエクスポート・ピン留め（TASK-1101）
- 開発開始: 2026-03-28
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `rust_core/src/export.rs`: `serialize_csv()` / `serialize_csv_from_df()`
  - `frontend/src/stores/exportStore.ts`: ExportStore（対象選択・列選択・ピン留め管理）
  - `frontend/src/components/export/ExportPanel.tsx`: CSV エクスポート UI
- テストファイル:
  - `rust_core/src/export.rs` (inline tests)
  - `frontend/src/stores/exportStore.test.ts`
  - `frontend/src/components/export/ExportPanel.test.tsx`

## テストケース（Rust: 10件, Frontend: 17件 = 計27件）

### Rust（export.rs）

| ID | 内容 |
|---|---|
| TC-1101-01 | ヘッダ行が列名になる |
| TC-1101-02 | trial_id 特殊列が Optuna trial_id を返す |
| TC-1101-03 | 数値列の値が正しくフォーマットされる |
| TC-1101-04 | 指定インデックスのみが出力される |
| TC-1101-05 | 範囲外インデックスはスキップされる |
| TC-1101-06 | カンマ含む文字列は RFC 4180 でクォート |
| TC-1101-07 | 列名 JSON パースが正しく動作する |
| TC-1101-08 | 空インデックスでヘッダのみ |
| TC-1101-09 | format_f64 が整数・小数・NaN を正しく処理 |
| TC-1101-10 | 存在しない列は空文字列セル |

### Frontend（exportStore + ExportPanel）

| ID | 分類 | 内容 |
|---|---|---|
| TC-1101-11〜17 | 正常系 | Store の setCsvTarget, exportCsv, pinTrial, unpinTrial, updatePinMemo |
| TC-1101-18〜22 | 正常系 | ExportPanel レンダリング、ラジオ選択、ダウンロードボタン |
| TC-1101-L01 | ローディング | isExporting=true でボタン disabled |
| TC-1101-E01〜04 | 異常系 | 空 indices エラー、ピン留め上限エラー、UI エラー表示 |

## 主要な設計決定

1. **serialize_csv の JSON パース: serde_json 非依存**
   - 文字列配列のみを対象とした軽量パーサーを実装
   - 依存クレート追加なしで RFC 4180 準拠の CSV 生成

2. **trial_id 特殊列**
   - `"trial_id"` を特殊列として認識し DataFrame の actual trial_id を返す
   - Optuna の連番ではなく実際の trial_id を出力

3. **ExportStore: <a download> フォールバック**
   - File System Access API 非対応ブラウザのために `<a download>` でダウンロード
   - BOM付き UTF-8 で Excel 互換性を確保

4. **ピン留め上限: MAX_PINS=20**
   - 超過時は pinError をセットして追加をキャンセル
   - 重複ピン留めは無視（index 重複チェック）

## 最終テスト結果

```
Rust: 10/10 passed
Frontend: 26 files, 176 tests passed; 0 failed
```

## 品質評価

✅ **高品質**
- Rust テスト: 10/10 通過
- フロントエンド全 176 テスト通過
- セキュリティ: 重大な脆弱性なし
- REQ-150〜REQ-153, REQ-156 準拠
