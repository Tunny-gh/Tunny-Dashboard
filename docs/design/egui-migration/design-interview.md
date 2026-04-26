# egui Migration 設計ヒアリング記録

**作成日**: 2026-04-11
**ヒアリング実施**: 2026-04-11 step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

TypeScript/React UIからRust eguiデスクトップアプリへの移行について、既存要件定義・アーキテクチャ設計・コードベース分析を基に、不明点・設計判断事項を明確化した。

## 質問と回答

### Q1: 設計の作業規模

**質問日時**: 2026-04-11
**カテゴリ**: 全体方針
**背景**: 移行規模が大きいため（TypeScript 68ファイル・33チャートコンポーネント）、設計深度を確認

**回答**: フル設計（推奨）を選択

**信頼性への影響**:
- この回答により、全設計文書（アーキテクチャ・データフロー・型定義）を完全に作成

---

### Q2: eguiデスクトップ版のターゲット環境

**質問日時**: 2026-04-11
**カテゴリ**: アーキテクチャ
**背景**: egui は Web (WASM + Canvas) とデスクトップ (eframe native) の両方に対応可能。どちらをターゲットとするかで設計が大きく変わる。

**回答**: デスクトップアプリ（推奨）を選択
- `eframe::run_native()` を使用したネイティブバイナリ
- WASM不要、OS標準ウィンドウ管理
- 既存 `rust_core` の計算関数を直接呼び出し

**信頼性への影響**:
- WASM ブリッジオーバーヘッド除去 → 全フィルター処理の信頼性 🔵 に向上
- `std::thread::spawn` + `mpsc::channel` パターン確定 → 非同期処理設計が🔵 に

---

### Q3: 既存コードの詳細分析の必要性

**質問日時**: 2026-04-11
**カテゴリ**: 調査深度
**背景**: 68ファイルのTypeScriptコードに対して詳細分析を行うかの判断

**回答**: 必要 を選択

**信頼性への影響**:
- 詳細分析により、lib.rs の全48関数のシグネチャが判明 → egui版への直接マッピング可能
- AppShell のレイアウト詳細 (Grid Columns/Rows) が判明 → LayoutState設計が🔵 に
- Zustand store の型構造が判明 → AppState struct設計の精度向上

---

### Q4: eguiデスクトップ版のアーキテクチャ方針

**質問日時**: 2026-04-11
**カテゴリ**: アーキテクチャ
**背景**: rust_core を拡張するか、新規クレートを作成するかの判断

**回答**: 新規クレート作成（推奨）を選択
- `egui-app/` ディレクトリに新規クレート作成
- `rust_core` は計算ライブラリとしてそのまま保持
- `Cargo.toml` をワークスペースルートに変更

**信頼性への影響**:
- クレート構成が確定 → ディレクトリ構造・依存関係の信頼性 🔵 に

---

### Q5: 3Dチャートの描画方式

**質問日時**: 2026-04-11
**カテゴリ**: 技術選択
**背景**: Pareto3D・Surface3Dの描画に wgpu/egui Painter/egui_plot の3択があった

**回答**: wgpu直接（推奨）を選択
- `egui-wgpu` 統合で wgpu レンダーパスを egui フレーム内に埋め込み
- GPU instancing で5万点の3D散布図を高速描画
- カスタム WGSL シェーダーで点群描画

**信頼性への影響**:
- 3D描画方式が確定 → `render/` モジュール設計が🟡 から 🔵 に向上

---

### Q6: WASMビルドの維持

**質問日時**: 2026-04-11
**カテゴリ**: アーキテクチャ
**背景**: 後方互換性のためWASMを残すか、完全廃止するかの判断

**回答**: WASMを廃止する を選択
- featura/egui ブランチでは後方互換性不要
- `rust_core/Cargo.toml` の `default = ["wasm"]` を `default = []` に変更
- `#[wasm_bindgen]` 属性は `#[cfg(feature = "wasm")]` でラップ（機能フラグとして保持）

**信頼性への影響**:
- `rust_core` の変更範囲が確定 → 移行コスト見積もりが🔵 に

---

### Q7: 新規クレートのディレクトリ名

**質問日時**: 2026-04-11
**カテゴリ**: 実装詳細
**背景**: `egui-app/` / `tunny-desktop/` / ルートクレートの3択

**回答**: `egui-app/`（推奨）を選択

**信頼性への影響**:
- ディレクトリ構造が確定 → 設計文書内のパス記載が🔵 に

---

### Q8: 2Dチャートライブラリの方針

**質問日時**: 2026-04-11
**カテゴリ**: 技術選択
**背景**: `egui_plot` 主体か、`egui Painter API` フルカスタムかの判断

**回答**: `egui_plot` 主体（推奨）を選択
- 折れ線グラフ（最適化履歴・HV推移・PDP）: `egui_plot::Line`
- バーチャート（感度分析重要度）: `egui_plot::Bar`
- 散布図（Pareto2D基本）: `egui_plot::Points`（小規模向け）または wgpu
- 平行座標図・Scatter Matrix: `egui::Painter API` でカスタム実装（egui_plot未対応）

**信頼性への影響**:
- チャートごとのライブラリ選択が確定 → widgets/ モジュール設計が🔵 に

---

## ヒアリング結果サマリー

### 確認できた事項
- デスクトップネイティブアプリ（WASM不要）
- 新規 `egui-app/` クレート作成
- 3D: wgpu直接、2D: egui_plot主体
- WASMビルドは `#[cfg(feature = "wasm")]` でオプション化
- `frontend/` ディレクトリ全削除
- 後方互換性不要（featura/eguiブランチ）

### 設計方針の決定事項
1. `Cargo.toml` をワークスペースルートに変更（members: rust_core, egui-app）
2. `rust_core` の default features から "wasm" を除外
3. `egui-app` の依存: eframe 0.30 + egui 0.30 + egui_plot + egui-wgpu + wgpu 24
4. 非同期計算: `std::thread::spawn` + `std::sync::mpsc::channel`
5. 状態管理: `TunnyApp` の `AppState` フィールドで Rust 所有権モデル
6. `rfd` クレートでネイティブファイルダイアログ

### 残課題
- `wgpu` と `egui-wgpu` のバージョン互換性の確認が必要（🟡）
- 平行座標図のブラッシングUXの詳細設計（egui Painter でどう実装するか）
- Scatter Matrix の egui Painter での最適描画方法
- WGSL シェーダーの詳細設計（散布図点群描画）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 6件
- 🟡 黄信号: 10件
- 🔴 赤信号: 4件

**ヒアリング後**:
- 🔵 青信号: 29件 (+23)
- 🟡 黄信号: 9件 (-1)
- 🔴 赤信号: 0件 (-4)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [tunny-dashboard-requirements.md](../../spec/tunny-dashboard-requirements.md)
