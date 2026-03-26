# TASK-001 設定作業実行記録

## 作業概要

- **タスクID**: TASK-001
- **作業内容**: プロジェクト初期化・ビルド環境構築
- **実行日時**: 2026-03-26
- **関連要件**: NFR-010, NFR-011

## 設計文書参照

- `docs/design/tunny-dashboard/architecture.md`
- `docs/tasks/tunny-dashboard-tasks.md` TASK-001

## 実行した作業

### 1. ツール・ランタイムの確認・インストール

```bash
node --version    # v24.5.0
npm --version     # 11.5.2
rustc --version   # 1.93.1
cargo --version   # 1.93.1
cargo install wasm-pack        # 0.14.0 をインストール
rustup target add wasm32-unknown-unknown
```

### 2. Rust クレートの初期化

```bash
cargo init rust_core --lib
```

**設定内容** (`rust_core/Cargo.toml`):
- `crate-type = ["cdylib", "rlib"]`（WASM + Native両対応）
- `features.wasm = ["wasm-bindgen", "js-sys", "web-sys", "serde-wasm-bindgen"]`
- `profile.release.opt-level = "s"` (WASMサイズ最適化)

**モジュールスタブ** (`rust_core/src/`):
- `lib.rs`: メインエントリ（WASMパニックハンドラ設定）
- `journal_parser.rs`, `dataframe.rs`, `filter.rs`, `pareto.rs`
- `clustering.rs`, `sensitivity.rs`, `pdp.rs`, `sampling.rs`
- `export.rs`, `live_update.rs`

### 3. Vite + React + TypeScript プロジェクト作成

```bash
npm create vite@latest frontend -- --template react-ts
cd frontend && npm install
```

追加インストール: `zustand@5`, `vitest@3`

### 4. Vite設定（COOP/COEP ヘッダー）

`frontend/vite.config.ts` に以下を追加:
```ts
server.headers: {
  'Cross-Origin-Opener-Policy': 'same-origin',
  'Cross-Origin-Embedder-Policy': 'require-corp',
}
```
SharedArrayBuffer使用のため必須。

### 5. TypeScript設定

- `target: "ESNext"` に変更（WASM top-level await対応）

### 6. ディレクトリ構造の作成

```
frontend/src/
  types/        ← interfaces.ts をコピー
  wasm/         ← WASMバインディング配置先
  stores/       ← Zustand Store
  components/
    layout/     ← AppShell, ToolBar
    charts/     ← 各チャートコンポーネント
    panels/     ← LeftPanel, BottomPanel等
    export/     ← ReportBuilder等
```

### 7. WASMビルド確認

```bash
cd rust_core && wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg
```

### 8. フロントエンドビルド確認

```bash
cd frontend && npm run build
# → dist/index.html が生成されることを確認
```

### 9. npm スクリプト追加

```json
"build:wasm": "cd ../rust_core && wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg",
"build:all": "npm run build:wasm && npm run build"
```

## 作業結果

- [x] wasm-pack インストール完了 (v0.14.0)
- [x] wasm32-unknown-unknown ターゲット追加完了
- [x] Rust クレート初期化完了（cdylib + rlib）
- [x] `cargo build` 成功
- [x] `wasm-pack build` 成功
- [x] Vite + React + TypeScript プロジェクト作成完了
- [x] COOP/COEP ヘッダー設定完了
- [x] `npm run build` で `dist/index.html` 生成確認
- [x] ディレクトリ構造作成完了
- [x] TypeScript型定義ファイル配置完了

## 成果物一覧

| ファイル/ディレクトリ | 内容 |
|---|---|
| `rust_core/Cargo.toml` | Rustクレート設定（cdylib+rlib・wasm feature） |
| `rust_core/src/lib.rs` | WASMエントリポイント・モジュール宣言 |
| `rust_core/src/*.rs` | 各機能モジュールのスタブ（10ファイル） |
| `frontend/vite.config.ts` | COOP/COEPヘッダー・WASMアセット設定 |
| `frontend/package.json` | zustand・vitest追加・build:wasmスクリプト |
| `frontend/tsconfig.app.json` | ESNext target設定 |
| `frontend/src/types/index.ts` | 全TypeScript型定義 |
| `frontend/.prettierrc` | Prettier設定 |

## 遭遇した問題と解決方法

### 問題1: `npm create vite . --yes` がキャンセルされる

- **状況**: 既存ディレクトリへのViteプロジェクト作成が`Operation cancelled`
- **解決**: `frontend/` サブディレクトリに作成してから必要ファイルを参照

## 次のステップ

- `direct-verify` で環境の動作確認
- TASK-002: TypeScript型定義ファイルの整備へ進む
