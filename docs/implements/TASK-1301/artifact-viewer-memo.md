# TDD開発メモ: artifact-viewer

## 概要

- 機能名: アーティファクトビューア（ArtifactViewer / ArtifactGallery）
- 開発開始: 2026-03-28
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル: `frontend/src/stores/artifactStore.ts`
- 実装ファイル: `frontend/src/components/panels/ArtifactViewer.tsx`
- 実装ファイル: `frontend/src/components/panels/ArtifactGallery.tsx`
- テストファイル: `frontend/src/stores/artifactStore.test.ts`
- テストファイル: `frontend/src/components/panels/ArtifactViewer.test.tsx`
- テストファイル: `frontend/src/components/panels/ArtifactGallery.test.tsx`

## Redフェーズ（失敗するテスト作成）

### テストケース

- TC-1301-U01〜U05: getMimeTypeCategory / buildArtifactMeta ユーティリティ
- TC-1301-D01〜D03: pickDirectory（成功/キャンセル/非対応環境）
- TC-1301-L01〜L04: loadArtifactUrl（URL取得/ファイルなし/キャッシュ/dirHandleなし）
- TC-1301-R01: releaseAll
- TC-1301-V01〜V06: ArtifactViewer（非表示/ローディング/画像/ファイルなし/その他）
- TC-1301-G01〜G07: ArtifactGallery（非表示/カード表示/フィルタ/サイズ切替/ページング）

## Greenフェーズ（最小実装）

### 実装方針

- `artifactStore.ts`: Zustand store + ObjectURL キャッシュ + Directory Picker API
- `ArtifactViewer.tsx`: trial 単位のアーティファクト表示（画像/CSV/その他）
- `ArtifactGallery.tsx`: 複数 trial のグリッドギャラリー + ページング

### 主要な設計ポイント

- `getMimeTypeCategory()`: 拡張子から ArtifactType を推定
- `buildArtifactMeta()`: artifactId + filename + trialId → ArtifactMeta
- `pickDirectory()`: DOMException と Error の両方で AbortError をキャッチ
- `isSupported()`: store の静的メソッドとして付与
- PAGE_SIZE = 48 でページング

### テスト結果

- artifactStore.test.ts: 13/13 pass
- ArtifactViewer.test.tsx: 6/6 pass
- ArtifactGallery.test.tsx: 7/7 pass

## Refactorフェーズ（品質改善）

### 改善内容

- ArtifactGallery.test.tsx のモック漏れ修正:
  `vi.clearAllMocks()` は実装をリセットしないため、`beforeEach` 内で `mockReturnValue` を明示的に再設定

### セキュリティレビュー

- ObjectURL は `releaseAll()` で明示的に解放 → メモリリーク防止 🟢
- `_escapeHtml()` 相当の対策は不要（DOM API 経由の表示のみ）

### パフォーマンスレビュー

- ObjectURL キャッシュにより同一ファイルの再読み込みを回避 🟢
- PAGE_SIZE=48 でギャラリーの初期レンダリング負荷を制限 🟢

### 品質評価

- テスト: 241/241 pass（全スイート）
- セキュリティ: 重大な問題なし
- パフォーマンス: キャッシュ・ページング対応済み
