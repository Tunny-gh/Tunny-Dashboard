# memory-efficiency データフロー図

**作成日**: 2026-05-29
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/memory-efficiency/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・コード調査・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: それらから妥当に推測したフロー
- 🔴 **赤信号**: 根拠資料にない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *コード調査・ヒアリングより*

```mermaid
flowchart TD
    File[Journal ログファイル] --> Parser[パーサ finalize_state<br/>ワーカースレッド]
    Parser -->|全 study DataFrame| Store["共有 Arc ストア<br/>study_id → ArcSwap&lt;DataFrame&gt;"]
    Store -->|Arc クローン| View[StudyView<br/>列参照 + 並行配列]
    View --> Ctx[StudyContext]
    Ctx --> Widgets[各ウィジェット]
    Widgets -->|列スライス借用| View
    Live[ライブ更新 batch] -->|新スナップショット store| Store
```

## 主要機能のデータフロー

### フロー1: study 選択（MEM-001 / 現行複製の排除） 🔵

**信頼性**: 🔵 *ユーザストーリー1.1・`io/study.rs`・`message_handler.rs:25-43` より*

**関連要件**: REQ-001, REQ-002, REQ-101

**現行（Before）**:
```mermaid
sequenceDiagram
    participant UI as UIスレッド
    participant Wk as ワーカースレッド
    participant TL as thread_local DataFrame

    UI->>Wk: dispatch_select_study(meta)
    Wk->>TL: select_study(id) → active_study_id 設定
    Wk->>TL: extract_trial_rows()（列→行+HashMap 複製）
    Wk-->>UI: StudySelected { trial_rows, gpu_data, ... }
    UI->>UI: StudyContext に Vec<TrialRow> 永続保持
    Note over UI,TL: 列(TL) + 行(UI) の二重保持
```

**改修後（After）**:
```mermaid
sequenceDiagram
    participant UI as UIスレッド
    participant Wk as ワーカースレッド
    participant St as 共有Arcストア

    UI->>Wk: dispatch_select_study(study_id)
    Wk->>St: get(study_id) → Arc<DataFrame> クローン
    Wk->>Wk: pareto_rank 等の派生属性を算出（並行配列）
    Wk-->>UI: StudySelected { study_id, derived_attrs, pareto_indices }
    UI->>UI: StudyView 構築（Arc<DataFrame> + 並行配列）
    Note over UI,St: 列データは単一実体を Arc 共有（複製なし）
```

**詳細ステップ**:
1. study 選択は `study_id` のみ受け渡し（行データを運ばない）。
2. ワーカーは共有ストアから `Arc<DataFrame>` を取得し、Pareto ランク等の派生属性のみ算出。
3. UI は `Arc<DataFrame>`（参照）＋並行配列で `StudyView` を構成。永続 `Vec<TrialRow>` を作らない（REQ-101）。

### フロー2: 比較 study 追加（MEM-005） 🔵

**信頼性**: 🔵 *ユーザストーリー3.1・`study_worker.rs:74-129`・ヒアリングQ2 より*

**関連要件**: REQ-010, REQ-011, REQ-103, REQ-201

```mermaid
sequenceDiagram
    participant UI as UIスレッド
    participant St as 共有Arcストア

    UI->>St: get(comparison_study_id) → Arc<DataFrame> クローン
    Note over UI,St: 再パースなし（初回常駐済み・ヒアリングQ2）
    UI->>UI: 軽量 StudyView を遅延構築（メタ + Arc 参照）
    UI->>UI: comparison に study_id + 軽量ビューを保持
    Note over UI: フル StudyContext 複製を持たない（REQ-010）

    UI->>UI: 比較削除 → StudyView ドロップ（Arc 参照解放）
```

**詳細ステップ**:
1. 比較追加はジャーナル再パース（現行 `load_comparison_study_task`）を行わず、共有ストアから `study_id` で `Arc<DataFrame>` を参照。
2. 比較対象は軽量メタ＋遅延 `StudyView` のみ保持。メモリ増分はフル複製比例にならない（REQ-103）。
3. 比較削除時は `StudyView` をドロップし `Arc` 参照を解放（REQ-201）。

### フロー3: ライブ更新（ArcSwap スナップショット差替え） 🔵

**信頼性**: 🔵 *ヒアリングQ1・`message_handler.rs:209-268` より*

**関連要件**: REQ-003, EDGE-102

```mermaid
sequenceDiagram
    participant Poll as ポーラ（ワーカー）
    participant St as ArcSwap<DataFrame>
    participant UI as UIスレッド（描画）

    Poll->>Poll: 新 batch を取り込んだ DataFrame を再構築
    Poll->>St: store(Arc::new(new_df))（原子的差替え）
    UI->>St: load() → 最新スナップショット Arc 取得
    UI->>UI: 派生属性（rank/cluster/state）を再算出
    Note over UI,St: 描画中の旧スナップショットは Arc で安全に生存
```

**詳細ステップ**:
1. ポーラは新試行を取り込んだ新 `DataFrame` を構築し、`ArcSwap::store` で差し替える。
2. UI は次フレームの `load()` で最新スナップショットを取得、`StudyView` の並行配列（pareto_rank 等）を再算出。
3. 描画中フレームが握る旧 `Arc<DataFrame>` は参照が残る限り生存（ロックフリー・安全。EDGE-102）。

### フロー4: 分析実行（MEM-004 一時行列の排除） 🔵

**信頼性**: 🔵 *ユーザストーリー3.2・`poll_chart.rs:16-22` より*

**関連要件**: REQ-008, REQ-009, REQ-102

```mermaid
flowchart TD
    A[PDP/Surface/Sensitivity 実行要求] --> B{入力変化あり?}
    B -->|なし| C[既存結果キャッシュ利用<br/>大行列を再生成しない]
    B -->|あり| D[StudyView/DataFrame の<br/>列スライスを借用]
    D --> E[フラットバッファ/共有行列で分析]
    E --> F[結果をキャッシュ]
    C --> G[描画]
    F --> G
```

**詳細ステップ**:
1. 現行は `r.params.get(p)` の HashMap 参照から実行ごとに `Vec<Vec<f64>>` を再構築（`poll_chart.rs:21`）。
2. 改修後は列スライス（`get_numeric_column`）を借用し、ネスト Vec 再構築を回避（REQ-009）。
3. 入力不変なら再実行で同等の大行列を再生成しない（REQ-102）。

## ロード時データフロー（MEM-006） 🔵

**信頼性**: 🔵 *`io/journal/parser/finalize.rs:9-115` より*

**関連要件**: REQ-012, NFR-003

```mermaid
flowchart TD
    A[trial_builders: HashMap<TrialBuilder>] --> B[sorted_trials: Vec]
    B --> C{現行: per_study_rows<br/>Vec<Vec<TrialRow>> 全保持}
    C --> D[DataFrame::from_trials<br/>※ per_study_rows と共存=ピーク]
    B -.改修後.-> E[列ビルダへ直接書き込み<br/>/ study 単位で逐次解放]
    E --> F[DataFrame 構築後、中間を即解放]
```

**詳細ステップ**:
1. 現行は `per_study_rows`（全行）と確定 `DataFrame` がピーク時共存（`finalize.rs:20,104`）。
2. 改修後は study 単位で行を列ビルダへ流し込み、DataFrame 化後に中間を逐次解放してピークを下げる（REQ-012）。
3. パース結果（study/目的/user attr/制約）の等価性を維持（受け入れ基準 TC-012-02）。

## 状態管理フロー

### StudyContext 状態遷移 🔵

**信頼性**: 🔵 *`app_state.rs:128-167`（clear/reset）より*

```mermaid
stateDiagram-v2
    [*] --> 未選択
    未選択 --> 選択済み: study 選択（StudyView 構築）
    選択済み --> 選択済み: ライブ更新（ArcSwap 差替え）
    選択済み --> 未選択: 別 study 選択（旧 StudyView 解放）
    選択済み --> 比較中: 比較追加（Arc 参照のみ）
    比較中 --> 選択済み: 比較リセット（StudyView ドロップ）
```

## データ整合性の保証 🔵

**信頼性**: 🔵 *REQ-402, REQ-403・ヒアリングより*

- **スナップショット一貫性**: 1 フレームの描画は単一の `Arc<DataFrame>` スナップショットと、それに対応する並行配列を使用（混在しない）。
- **無効化ルール**: study 切替＝旧 `StudyView` ドロップ。ライブ更新＝`ArcSwap` 差替え＋並行配列再算出。比較削除＝`StudyView` ドロップ。
- **データ損失防止**: 列の数値変換・カテゴリ列分岐（`model.rs:76-99`）の現行挙動を保つ（REQ-402, EDGE-002）。

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [types.rs](types.rs)
- **要件定義**: [requirements.md](../../spec/memory-efficiency/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 12件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
