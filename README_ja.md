# Tunny Dashboard

[Optuna](https://optuna.org/) の最適化結果をブラウザ内で分析するダッシュボードです。Rust/WebAssembly と React で構築されています。

> **英語ドキュメント**: [README.md](README.md)

---

## 概要

Tunny Dashboard は Optuna の Journal ログファイルをブラウザ内で完結してパースします。サーバーも Python のインストールも不要です。出力物は単一の自己完結型 `index.html` ファイルで、モダンブラウザで直接開くだけで動作します。

### 主な機能

- **依存関係ゼロの配布** — 全アセットをインライン化した単一の `index.html` として配布
- **高性能データ処理** — Journal パースと DataFrame 操作が Rust/WASM + Web Worker で動作
- **インタラクティブな可視化** — Pareto Front（3D/2D）、Parallel Coordinates、Scatter Matrix、Hypervolume 推移、感度分析など
- **Brushing & Linking** — クロスチャート選択がリアルタイムで全ビューに同期
- **フリーレイアウト（Mode D）** — 4×4 グリッドへのドラッグ&ドロップによる自由なチャート配置
- **複数 Study 比較** — 最大 4 件の Study をオーバーレイまたは並列比較
- **CSV / HTML レポートエクスポート** — カラム選択付きでトライアルをエクスポート
- **セッション保存** — UI の状態を丸ごと JSON ファイルとして保存・復元

### 技術スタック

| レイヤー | 技術 |
|---|---|
| コア処理 | Rust + [wasm-pack](https://rustwasm.github.io/wasm-pack/)（WebAssembly） |
| フロントエンドフレームワーク | React 19 + TypeScript |
| 状態管理 | [Zustand](https://zustand-demo.pmnd.rs/) |
| 3D/2D チャート | [deck.gl](https://deck.gl/) |
| 統計チャート | [Apache ECharts](https://echarts.apache.org/) |
| ビルドツール | [Vite](https://vite.dev/) + [vite-plugin-singlefile](https://github.com/richardtallent/vite-plugin-singlefile) |
| テスト | [Vitest](https://vitest.dev/) + [Playwright](https://playwright.dev/) |

### リポジトリ構成

```
tunny-dashboard/
├── rust_core/          # Rust/WASM コア（Journal パーサ、DataFrame、分析処理）
│   ├── src/
│   └── Cargo.toml
└── frontend/           # React/TypeScript フロントエンド
    ├── src/
    │   ├── components/ # UI コンポーネント（チャート、パネル、レイアウト）
    │   ├── stores/     # Zustand ストア
    │   ├── wasm/       # WASM ローダー、GPU バッファ、Web Worker
    │   └── types/      # 共通 TypeScript 型定義
    ├── e2e/            # Playwright E2E テスト
    └── package.json
```

---

## クイックスタート（Windows）

プロジェクトルートにあるバッチファイルをダブルクリックするだけで実行できます：

| ファイル | 内容 |
|---|---|
| `build.bat` | 本番ビルドを一括実行 → `frontend/dist/index.html` を生成 |
| `dev.bat` | 開発サーバーを起動（`http://localhost:5173`） |
| `format.bat` | Rust と TypeScript/TSX のソースを一括フォーマット |

---

## 必要なツール

| ツール | バージョン | 用途 |
|---|---|---|
| [Node.js](https://nodejs.org/) | ≥ 20 | フロントエンドのビルド |
| [Rust](https://www.rust-lang.org/tools/install) | stable (1.70+) | WASM コアのビルド |
| [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) | ≥ 0.12 | Rust → WASM コンパイル |

`wasm-pack` が未インストールの場合は以下でインストールします：

```bash
cargo install wasm-pack
```

---

## ビルド

フロントエンドのコマンドはすべて `frontend/` ディレクトリ内で実行します。

```bash
cd frontend
npm install
```

### 開発サーバー

```bash
npm run dev
```

`http://localhost:5173` でサーバーが起動します。`SharedArrayBuffer` に必要な `Cross-Origin-Opener-Policy` および `Cross-Origin-Embedder-Policy` ヘッダーが自動的に設定されます。

### WASM コアのビルド

```bash
npm run build:wasm
```

`rust_core/` を `wasm-pack` でコンパイルし、WASM パッケージを `frontend/src/wasm/pkg/` に出力します。

### 本番ビルド（フル）

```bash
npm run build:all
```

`build:wasm` に続いて Vite の本番ビルドを実行します。JavaScript、CSS、WASM をすべて base64 でインライン化した単一ファイル `frontend/dist/index.html` が生成されます。

本番ビルドをローカルでプレビューするには：

```bash
npm run preview
```

### ビルドを個別に実行する場合

```bash
# 1. Rust コアを WASM にコンパイル
npm run build:wasm

# 2. 型チェックとフロントエンドのバンドル
npm run build
```

---

## テスト

### ユニット・統合テスト（Vitest）

```bash
cd frontend
npm test
```

全 Vitest テスト（ユニット・統合・性能ベンチマーク）を実行します。ブラウザや WASM ランタイムなしで `jsdom` 環境で動作します。

### E2E テスト（Playwright）

```bash
cd frontend

# 初回のみ: ブラウザバイナリをインストール（約 200 MB）
npx playwright install chromium

# E2E テストを実行（dev サーバーは自動起動）
npm run test:e2e

# インタラクティブ UI モード
npm run test:e2e:ui
```

---

## フォーマット

### Rust（rust_core）

```bash
cd rust_core
cargo fmt
```

標準の `rustfmt` フォーマッターを使用します。必要に応じて `rustfmt.toml` で設定をカスタマイズできます。

変更を加えずにフォーマットのチェックだけ行う場合：

```bash
cargo fmt -- --check
```

### TypeScript / TSX（frontend）

[ESLint](https://eslint.org/) をリンター、[Prettier](https://prettier.io/) をコードフォーマッターとして使用します。

**リント（ESLint）:**

```bash
cd frontend
npm run lint
```

**フォーマット（Prettier）:**

Prettier は `frontend/.prettierrc` で設定されています。インストールして実行するには：

```bash
cd frontend
npm install --save-dev prettier
npx prettier --write "src/**/*.{ts,tsx}"
```

Prettier の設定（`.prettierrc`）:

| オプション | 値 |
|---|---|
| `semi` | `false`（セミコロンなし） |
| `singleQuote` | `true`（シングルクォート） |
| `tabWidth` | `2`（インデント 2 スペース） |
| `trailingComma` | `"all"`（末尾カンマあり） |
| `printWidth` | `100`（1 行の最大文字数） |

VS Code や JetBrains などのエディターでは、[Prettier 拡張機能](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode) をインストールし `.prettierrc` が存在していれば、保存時に自動でフォーマットが適用されます。

---

## ライセンス

[MIT](LICENSE)
