# chart-catalog 要件定義書

## 概要

右側に収納可能なチャートカタログパネルを追加する。パネルは全チャート種別の一覧を表示し、ユーザーがチャートをドラッグしてフリーレイアウトキャンバス（Mode D）に配置できるようにする。既存のチャート移動ドラッグ（タイル再配置）との共存が必要であり、AppShell のグリッドレイアウトを3カラム構成に拡張する。

---

## ユーザーストーリー

### ストーリー1: チャートをキャンバスに追加する

- **である** 最適化結果を分析するデータアナリスト **として**
- **私は** 右側パネルからチャートをドラッグしてキャンバスに追加 **したい**
- **そうすることで** 必要な可視化だけを自由に配置した分析ビューを素早く構成できる

### ストーリー2: パネルを折り畳んでキャンバスを広く使う

- **である** 最適化結果を分析するデータアナリスト **として**
- **私は** チャートカタログパネルをボタン1つで開閉 **したい**
- **そうすることで** 配置作業が終わった後はキャンバスの表示領域を最大化できる

### ストーリー3: 既に配置済みのチャートを把握する

- **である** 最適化結果を分析するデータアナリスト **として**
- **私は** カタログパネルで各チャートが配置済みか未配置かを一目で確認 **したい**
- **そうすることで** 重複配置を避けながら必要なチャートを素早く追加できる

### ストーリー4: キャンバス上のチャートを削除する

- **である** 最適化結果を分析するデータアナリスト **として**
- **私は** キャンバス上のチャートタイルを閉じるボタンで削除 **したい**
- **そうすることで** 不要なチャートを取り除いて画面を整理できる

---

## 機能要件（EARS記法）

### 通常要件

- REQ-001: システムは AppShell の右端に ChartCatalogPanel コンポーネントを配置しなければならない
- REQ-002: ChartCatalogPanel は全 ChartId（14種）の一覧をスクロール可能なリストで表示しなければならない
- REQ-003: 各チャートアイテムはチャート名と種別アイコン（絵文字または SVG）を表示しなければならない
- REQ-004: ChartCatalogPanel は開閉トグルボタン（`data-testid="catalog-toggle-btn"`）を持たなければならない
- REQ-005: AppShell のグリッドは閉じた状態で `auto 1fr auto`（ToggleボタンのみのColumn）、開いた状態で `auto 1fr 220px` の3カラム構成に切り替えなければならない

### 条件付き要件

- REQ-101: ユーザーがチャートアイテムをキャンバスにドラッグ開始した場合、システムは `dataTransfer` に `{ type: 'add-chart', chartId }` の JSON 文字列をセットしなければならない
- REQ-102: ユーザーがドラッグをキャンバスのドロップゾーン上で離した場合、システムは `dataTransfer.type` が `'add-chart'` であれば `freeModeLayout.cells` に新しいセルを追加しなければならない
- REQ-103: ユーザーがドラッグをキャンバスのドロップゾーン上で離した場合、システムは `dataTransfer.type` が `'move-chart'` であれば既存セルの位置を更新しなければならない（既存の再配置動作を維持）
- REQ-104: 追加されたチャートのデフォルトサイズは `gridRow: [row, row+2], gridCol: [col, col+2]`（2×2スパン）でなければならない
- REQ-105: ドロップ位置が 4×4 グリッド境界を超える場合、システムはスパンを境界内にクランプしなければならない
- REQ-106: 同一 `chartId` であっても複数インスタンスの重複配置を許容しなければならない（例: スライスプロットを複数並べる用途）
- REQ-107: LayoutMode が `'D'` 以外のとき、ユーザーがカタログからチャートをドロップした場合、システムは自動的に `setLayoutMode('D')` を呼び出して Mode D に切り替えてからチャートを追加しなければならない
- REQ-108: カタログの各チャートアイテムには現在キャンバス上に配置されているインスタンス数を `（N個）` の形式で表示しなければならない（0個のときは非表示）
- REQ-109: 各セルはユニークな `cellId: string`（`crypto.randomUUID()` 等で生成）を持たなければならない。`chartId` ではなく `cellId` でセルを識別・操作する

### 状態要件

- REQ-201: パネルが開いている状態（`isOpen: true`）では、ChartCatalogPanel は幅 220px で表示されなければならない
- REQ-202: パネルが閉じている状態（`isOpen: false`）では、ChartCatalogPanel はトグルボタン幅（28px）のみを表示しなければならない
- REQ-203: `isOpen` の初期値は `false`（折り畳み状態）でなければならない
- REQ-204: `isOpen` はコンポーネントのローカル状態（`useState`）として管理し、セッション間での永続化は行わなくてよい

### 削除要件

- REQ-301: FreeLayoutCanvas の各チャートタイルは削除ボタン（`data-testid="chart-close-btn-{cellId}"`）を持たなければならない
- REQ-302: ユーザーが削除ボタンをクリックした場合、システムは `freeModeLayout.cells` から該当 `cellId` のセルを削除しなければならない
- REQ-303: セルを削除後、カタログパネルの該当 `chartId` のインスタンス数表示がデクリメントされなければならない

### 制約要件

- REQ-401: ChartCatalogPanel のスタイルはインラインスタイルのみを使用し、Tailwind CSS クラスを使用してはならない
- REQ-402: AppShell のグリッド拡張は `gridTemplateColumns` の値を動的に変更することで実装し、既存の LeftPanel・ToolBar・BottomPanel のレイアウトに影響を与えてはならない
- REQ-403: 既存の FreeLayoutCanvas 内チャート再配置（タイルの掴み移動）は引き続き動作しなければならない
- REQ-404: `dataTransfer` ペイロードの型識別子は `'add-chart'`（カタログ追加）と `'move-chart'`（再配置）の2種で区別しなければならない

---

## 非機能要件

### パフォーマンス

- NFR-001: パネルの開閉アニメーションは CSS `transition: width 200ms ease` で実装し、JS アニメーションループを使用してはならない
- NFR-002: チャートカタログのレンダリングは14アイテム程度であるため仮想化は不要

### ユーザビリティ

- NFR-201: トグルボタンには `title` 属性でツールチップ（「チャートを追加」/「パネルを閉じる」）を表示しなければならない
- NFR-202: ドラッグ中のチャートアイテムは `opacity: 0.4` でゴーストを表示し、ドラッグ元のアイテムがどこから来たかをユーザーが認識できること
- NFR-203: ドロップゾーンへのドラッグオーバー時はゾーンのハイライト色を変更しなければならない（既存動作の維持）

---

## チャートカタログ定義

以下の14チャートをカタログに含める:

| ChartId | 表示名 |
|---------|--------|
| `pareto-front` | パレートフロント |
| `parallel-coords` | 平行座標 |
| `scatter-matrix` | 散布図行列 |
| `history` | 最適化履歴 |
| `hypervolume` | ハイパーボリューム |
| `objective-pair-matrix` | 目的関数ペア行列 |
| `pdp` | 部分依存プロット |
| `importance` | パラメータ重要度 |
| `sensitivity-heatmap` | 感度ヒートマップ |
| `cluster-view` | クラスタービュー |
| `umap` | UMAP |
| `slice` | スライスプロット |
| `edf` | EDF |
| `contour` | コンタープロット |

---

## Edgeケース

### エラー処理

- EDGE-001: `dataTransfer.getData('text/plain')` が空または不正 JSON の場合、システムはドロップ処理をサイレントに無視しなければならない
- EDGE-002: ドロップ先グリッドが全て埋まっている場合（16ゾーン中、配置可能スペースなし）のフォールバックは将来実装とし、現バージョンではサイレントに無視してよい

### 境界値

- EDGE-101: 同一 `chartId` を複数インスタンス配置した場合、各インスタンスは独立した `cellId` を持ち、個別に移動・削除できなければならない
- EDGE-102: パネル開閉トグルを素早く連打した場合も UI が壊れないこと（`useState` の同期的な更新で対応）
- EDGE-103: Mode D でないとき（A/B/C）にカタログから追加した場合、既存の freeModeLayout が `null` であれば `DEFAULT_FREE_LAYOUT` をベースに新チャートを追加しなければならない

---

## 受け入れ基準

### 機能テスト

- [ ] `data-testid="catalog-toggle-btn"` のボタンが DOM に存在する
- [ ] トグルボタンをクリックするとパネルが開閉する
- [ ] パネルが開いているとき14チャートアイテムが全て表示される
- [ ] `data-testid="catalog-item-{chartId}"` で各アイテムが取得できる
- [ ] カタログアイテムからキャンバスのドロップゾーンへドラッグして `freeModeLayout.cells` に追加される
- [ ] 同一 `chartId` を複数回ドロップすると複数インスタンスが配置される
- [ ] 配置済みインスタンス数がカタログアイテムに `（N個）` で表示される
- [ ] Mode D 以外でドロップすると `setLayoutMode('D')` が呼ばれる
- [ ] `data-testid="chart-close-btn-{cellId}"` ボタンで対応タイルが削除される
- [ ] タイル削除後、カタログの該当アイテムのインスタンス数がデクリメントされる
- [ ] 既存のチャートタイル移動（再配置ドラッグ）が引き続き動作する

### 非機能テスト

- [ ] Tailwind クラスが使用されていないこと（インラインスタイルのみ）
- [ ] パネル開閉が CSS transition で実装されていること
- [ ] AppShell の gridTemplateColumns がパネル状態に応じて変化すること

---

## コンポーネント構成（実装参考）

```
AppShell
├── ToolBar                   (gridColumn: 1 / -1)
├── LeftPanel                 (col 1)
├── FreeLayoutCanvas          (col 2, 既存)
│   └── ChartContent          (各タイル — 削除ボタン追加)
└── ChartCatalogPanel         (col 3, 新規)  ← NEW
    ├── ToggleButton          (data-testid="catalog-toggle-btn")
    └── CatalogList           (isOpen=true のときのみ表示)
        └── CatalogItem × 14  (data-testid="catalog-item-{chartId}")
```

### AppShell グリッド変更

```css
/* 現在 */
grid-template-columns: auto 1fr;

/* 変更後 */
grid-template-columns: auto 1fr [panel-width];
/* panel-width: isOpen ? '220px' : '28px' */
```

### ChartCatalogPanel の役割

| 責務 | 詳細 |
|------|------|
| 開閉状態管理 | `useState<boolean>(false)` — ローカル状態 |
| 配置済み判定 | `freeModeLayout.cells` を layoutStore から購読 |
| ドラッグ開始 | `dataTransfer.setData('text/plain', JSON.stringify({ type: 'add-chart', chartId }))` |

### FreeLayoutCanvas の変更点

| 変更 | 詳細 |
|------|------|
| タイル削除ボタン追加 | 各タイルタイトルバーに × ボタン |
| `removeCell(chartId)` アクション追加 | layoutStore に新アクション追加 |
| ドロップ処理分岐 | `dataTransfer.type` で `add-chart` / `move-chart` を判別 |
| 既存ドラッグに `type: 'move-chart'` 追加 | `onDragStart` の `dataTransfer.setData` を更新 |
