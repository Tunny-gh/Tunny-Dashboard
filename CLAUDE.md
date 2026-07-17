# Tunny Dashboard

Optuna 最適化結果分析のための Rust egui デスクトップアプリ。

## 言語ポリシー

- **Git コミットメッセージ、PR のタイトル・本文は英語で書くこと。**
- **ROADMAP.md などのプロジェクトドキュメントは英語で書くこと。**
- **ソースコード内のコメント・doc コメントは英語で書くこと。**
- **UI 文言（ラベル・進捗・エラーメッセージ等のユーザー向け文字列）は英語で書くこと。**
- 日本語レポート出力（`ReportLang::Ja`）とバイリンガルのヘルプ/理論ドキュメントは
  日本語コンテンツが機能仕様なので日本語のまま。
- チャットでの応答はユーザーの使用言語（日本語）に合わせる。

## 実装分担

適切にAgentを使った作業分担を行いトークン管理を行うこと。

- 設計、実装やレビューなどの困難な作業
  - FableやOpusが担当
- 機械的な作業
  - Sonnetが担当

## 開発コマンド

### ビルド

```bash
# ワークスペース全体ビルド
cargo build --workspace

# リリースビルド
cargo build --workspace --release
```

### テスト実行

```bash
# 全テスト実行
cargo test --workspace

# rust_core のみ
cargo test -p tunny-core

# egui-app のみ
cargo test -p tunny-desktop
```

### 静的解析

コミット前には必ず確認すること。CI と同条件で実行する（`--all-targets` がないとテストコードが clippy の対象外になる）。

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings

cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo fmt -p tunny-mcp -- --check
```

### アプリケーション実行

```bash
cargo run -p tunny-desktop
```

### ベンチマーク

```bash
cargo bench -p tunny-core
```
