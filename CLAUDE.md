# Tunny Dashboard

Optuna 最適化結果分析のための Rust egui デスクトップアプリ。

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

コミット前には必ず確認すること。

```bash
cargo clippy --workspace

cargo fmt --check --workspace
```

### アプリケーション実行
```bash
cargo run -p tunny-desktop
```

### ベンチマーク
```bash
cargo bench -p tunny-core
```
