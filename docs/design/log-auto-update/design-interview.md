# Log Auto Update 設計ヒアリング記録

**作成日**: 2026-05-11
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の要件定義（`docs/spec/log-auto-update/requirements.md`）・実装コード（live_update.rs, study_worker.rs, toolbar.rs）を確認し、アーキテクチャ設計に必要な技術判断を明確化するためのヒアリングを実施しました。

## 質問と回答

### Q1: ポーリングスレッドの実装方式

**質問日時**: 2026-05-11
**カテゴリ**: アーキテクチャ
**背景**: thread_local!制約により永続スレッドが必要。既存のstudy_workerパターン（永続スレッド＋コマンドチャネル）を拡張するか、独立したPollerスレッドを作成するかの判断が必要。study_worker拡張は recv_timeout 導入による複雑化、独立Pollerはthread_local!の分離が懸念。

**回答**: 専用Pollerスレッド（推奨）

**信頼性への影響**:
- この回答により、DD-01「専用Pollerスレッドパターン」の信頼性レベルが 🔴 → 🔵 に向上
- study_workerとは独立して動作する設計が確定
- thread_local!はPollerスレッド内で独自に管理

---

### Q2: 新規トライアル検出後のデータ更新方法

**質問日時**: 2026-05-11
**カテゴリ**: データモデル
**背景**: REQ-LU-404で全件再パースが禁止されているが、Paretoランク計算には全トライアルデータが必要。差分バイトのみからTrialRowを構築するインクリメンタル方式と、ファイル全体のリロード（OSキャッシュ活用）のトレードオフを確認。

**回答**: インクリメンタル更新（推奨）— ファイルの再読み込みなし

**信頼性への影響**:
- この回答により、DD-02「インクリメンタルトライアル構築」の信頼性レベルが 🔴 → 🔵 に向上
- `append_journal_diff_v2` の新規関数設計が必要（TrialRow構築を含む拡張版）
- distribution情報をPollerに渡す `LiveUpdateContext` 構造体が必要

---

### Q3: Paretoランクの再計算タイミング

**質問日時**: 2026-05-11
**カテゴリ**: パフォーマンス
**背景**: 既存要件REQ-135では「クラスタリング結果は自動更新せず、手動の再クラスタリングボタン」としている。Paretoランクも同様に手動再計算とするか、新規トライアルごとに自動再計算するか。自動再計算は50K点で100msのコストがある。

**回答**: 毎回全再計算

**信頼性への影響**:
- この回答により、DD-03「Paretoランク全再計算」の信頼性レベルが 🔴 → 🔵 に向上
- MessageHandler内で `compute_pareto_ranks` を毎回呼び出す設計に確定
- GPUバッファも毎回再構築が必要

---

## ヒアリング結果サマリー

### 確認できた事項
- 専用Pollerスレッドが適切なアーキテクチャ（study_worker拡張ではない）
- インクリメンタル更新でファイル再読み込みなし
- Paretoランクは毎回全再計算（手動ではない）

### 設計方針の決定事項
- 新規モジュール: `egui-app/src/io/live_update_poller.rs`
- 新規関数: `rust_core::append_journal_diff_v2`
- 新規構造体: `LiveUpdateContext`（distribution情報等をPollerに渡す）
- 停止制御: `AtomicBool` + `AtomicU64`

### 残課題
- `append_journal_diff_v2` の具体的なTrialRow構築ロジック
- GPUバッファのインクリメンタル更新 vs 全再構築の最適化
- 大量トライアル（10K+）一括追加時のパフォーマンス検証

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 5
- 🟡 黄信号: 2
- 🔴 赤信号: 3

**ヒアリング後**:
- 🔵 青信号: 10 (+5)
- 🟡 黄信号: 0 (-2)
- 🔴 赤信号: 0 (-3)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/log-auto-update/requirements.md)
