# pdp-maintainability タスク概要

**作成日**: 2026-05-04
**プロジェクト期間**: Phase 1 - Phase 4（約2週間）
**推定工数**: 20時間
**総タスク数**: 14件

## 関連文書

- **要件定義書**: [📋 requirements.md](../../spec/pdp-maintainability/requirements.md)
- **設計文書**: [📐 architecture.md](../../design/pdp-maintainability/architecture.md)
- **データフロー図**: [🔄 dataflow.md](../../design/pdp-maintainability/dataflow.md)
- **インターフェース定義**: [📝 interfaces.rs](../../design/pdp-maintainability/interfaces.rs)
- **ユーザーストーリー**: [📖 user-stories.md](../../spec/pdp-maintainability/user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](../../spec/pdp-maintainability/acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](../../spec/pdp-maintainability/note.md)

## フェーズ構成

| フェーズ | 期間 | 目標 | タスク数 | 工数 | 主なファイル |
|---------|------|------|----------|------|----------|
| Phase 1 | 1日 | utils.rs ヘルパー追加 | 4件 | 5.5h | [TASK-2161~2164](#phase-1-ベースライン確認--utilsrsヘルパー追加) |
| Phase 2 | 1日 | kriging_core.rs リファクタリング | 3件 | 5.5h | [TASK-2165~2167](#phase-2-kriging_coreのリファクタリング) |
| Phase 3 | 1日 | api.rs / ridge_core.rs 修正 | 3件 | 2.5h | [TASK-2168~2170](#phase-3-apirsridge_coreの修正) |
| Phase 4 | 1日 | rayon 並列化 | 4件 | 6.5h | [TASK-2171~2174](#phase-4-rayon並列化) |

## タスク番号管理

**使用済みタスク番号**: TASK-2161 ~ TASK-2174
**次回開始番号**: TASK-2175

## 全体進捗

- [ ] Phase 1: ベースライン確認 + utils.rs ヘルパー追加
- [ ] Phase 2: kriging_core.rs リファクタリング
- [ ] Phase 3: api.rs / ridge_core.rs 修正
- [ ] Phase 4: rayon 並列化

## マイルストーン

- **M1: 基礎ヘルパー完成** (TASK-2161~2164): utils.rs に共通ヘルパー4つを実装
- **M2: kriging_core.rs リファクタリング完了** (TASK-2165~2167): 全4関数をヘルパー呼び出しに変更
- **M3: API層修正完了** (TASK-2168~2170): api.rs・ridge_core.rs 修正と全テスト確認
- **M4: 並列化完成** (TASK-2171~2174): rayon 導入と性能検証完了

---

## Phase 1: ベースライン確認 + utils.rs ヘルパー追加

**期間**: 1日（約5.5時間）
**目標**: `rust_core/src/pdp/utils.rs` に共通ヘルパー関数を追加し、コード重複を解消する基盤を構築
**成果物**: 
- normalize_x_minmax（2関数で重複するmin-max正規化を統合）
- normalize_y（y正規化の共通化）
- r_squared（R²計算の共通化）
- extract_xy（DataFrame抽出の共通化）

### タスク一覧

- [ ] [TASK-2161: cargo test ベースライン確認](TASK-2161.md) - 0.5h (DIRECT) 🔵
- [ ] [TASK-2162: normalize_x_minmax / normalize_y を utils.rs に追加](TASK-2162.md) - 2h (TDD) 🔵
- [ ] [TASK-2163: r_squared を utils.rs に追加](TASK-2163.md) - 1.5h (TDD) 🔵
- [ ] [TASK-2164: extract_xy を utils.rs に追加](TASK-2164.md) - 1.5h (TDD) 🔵

**小計**: 5.5時間

### 依存関係

```
TASK-2161 (ベースライン)
  ↓
  ├→ TASK-2162 (normalize_x_minmax/normalize_y)
  │   ├→ TASK-2165, TASK-2166 (Phase 2 で使用)
  │
  ├→ TASK-2163 (r_squared)
  │   ├→ TASK-2165, TASK-2166, TASK-2167 (Phase 2 で使用)
  │
  └→ TASK-2164 (extract_xy)
      └→ TASK-2168 (Phase 3 で使用)
```

### 実装の特徴

- **normalize_x_minmax**: 各列の (min, range) と正規化済み行列を1回のループで同時計算（効率的）
- **normalize_y**: y の (mean, std, normalized_y) を返す。ゼロ除算ガード付き
- **r_squared**: R² 計算時の ss_tot < EPSILON のゼロ除算ガード実装
- **extract_xy**: DataFrame から param と objective を抽出する共通処理を関数化

---

## Phase 2: kriging_core.rs のリファクタリング

**期間**: 1日（約5.5時間）
**目標**: `rust_core/src/pdp/kriging_core.rs` の4関数で重複する正規化・R²計算をPhase 1のヘルパーに置き換える
**成果物**: 
- compute_pdp_1d_kriging_raw がヘルパーを使用
- compute_pdp_1d_sparse_kriging_raw がヘルパーを使用
- compute_pdp_2d_kriging_raw が r_squared ヘルパーを使用
- compute_pdp_2d_sparse_kriging_raw が r_squared ヘルパーを使用

### タスク一覧

- [ ] [TASK-2165: compute_pdp_1d_kriging_raw の正規化/R² をヘルパー呼び出しに変更](TASK-2165.md) - 2h (TDD) 🔵
- [ ] [TASK-2166: compute_pdp_1d_sparse_kriging_raw の正規化/R² をヘルパー呼び出しに変更](TASK-2166.md) - 2h (TDD) 🔵
- [ ] [TASK-2167: compute_pdp_2d_kriging_raw / compute_pdp_2d_sparse_kriging_raw の R² をヘルパーに変更](TASK-2167.md) - 1.5h (TDD) 🔵

**小計**: 5.5時間

### 依存関係

```
Phase 1 完了
  ↓
  ├→ TASK-2165 (1d kriging)
  │   └→ TASK-2170 (Phase 3 確認)
  │
  ├→ TASK-2166 (1d sparse kriging)
  │   └→ TASK-2170 (Phase 3 確認)
  │
  └→ TASK-2167 (2d kriging)
      └→ TASK-2170 (Phase 3 確認)
```

### リファクタリングのポイント

- **REQ-101/102**: normalize_x_minmax / normalize_y ヘルパーを呼び出しに置き換え
- **REQ-201/202**: r_squared ヘルパーを呼び出しに置き換え
- **回帰テスト**: 既存テスト tc_1645_*, tc_1652_*, tc_1653_* で動作確認

---

## Phase 3: api.rs / ridge_core.rs の修正

**期間**: 1日（約2.5時間）
**目標**: api.rs・ridge_core.rs 内の重複を解消し、全テストが通ることを確認
**成果物**: 
- api.rs の compute_pdp / compute_pdp_2d / compute_pdp_from_data が extract_xy を使用
- ridge_core.rs の fold スタイルを統一（f64::min / f64::max）
- 全テスト PASS 確認

### タスク一覧

- [ ] [TASK-2168: api.rs の extract_xy 適用](TASK-2168.md) - 1.5h (TDD) 🔵
- [ ] [TASK-2169: ridge_core.rs の fold スタイル統一](TASK-2169.md) - 0.5h (DIRECT) 🔵
- [ ] [TASK-2170: Phase 1-3 完了後の全テスト確認](TASK-2170.md) - 0.5h (DIRECT) 🔵

**小計**: 2.5時間

### 依存関係

```
Phase 2 完了
  ↓
  ├→ TASK-2168 (extract_xy 適用)
  │   └→ TASK-2170 (全テスト確認)
  │
  ├→ TASK-2169 (fold スタイル統一)
  │   └→ TASK-2170 (全テスト確認)
  │
  └→ TASK-2170 (全テスト確認)
      ├→ TASK-2171 (Phase 4 rayon 追加)
```

### 修正の焦点

- **REQ-301/302**: DataFrame 抽出を extract_xy に統一
- **REQ-601**: ridge_core.rs で f64::min / f64::max スタイルに統一
- **NFR-002**: 全テスト PASS 確認

---

## Phase 4: rayon 並列化

**期間**: 1日（約6.5時間）
**目標**: `rayon` クレートを導入し、PDP計算ループを並列化してパフォーマンスを向上させる
**成果物**: 
- Cargo.toml に rayon 依存追加
- compute_pdp_1d_kriging_raw の mean グリッドループが par_iter() で並列化
- compute_pdp_1d_sparse_kriging_raw のグリッドループが par_iter() で並列化
- 全テスト PASS + パフォーマンス測定完了

### タスク一覧

- [ ] [TASK-2171: Cargo.toml に rayon を追加](TASK-2171.md) - 0.5h (DIRECT) 🔵
- [ ] [TASK-2172: compute_pdp_1d_kriging_raw の mean ループ並列化](TASK-2172.md) - 2h (TDD) 🔵
- [ ] [TASK-2173: compute_pdp_1d_sparse_kriging_raw のグリッドループ並列化](TASK-2173.md) - 3h (TDD) 🔵
- [ ] [TASK-2174: rayon 導入後の全テスト確認 + パフォーマンス測定](TASK-2174.md) - 1h (DIRECT) 🔵

**小計**: 6.5時間

### 依存関係

```
Phase 3 完了
  ↓
  ├→ TASK-2171 (rayon 追加)
  │   ├→ TASK-2172 (1d kriging 並列化)
  │   │   └→ TASK-2174 (全テスト確認)
  │   │
  │   └→ TASK-2173 (1d sparse kriging 並列化)
  │       └→ TASK-2174 (全テスト確認)
```

### 並列化戦略

- **REQ-501**: Cargo.toml に rayon 追加（制限なし）
- **REQ-502**: compute_pdp_1d_sparse_kriging_raw の grid.par_iter() 並列化
- **REQ-503**: compute_pdp_1d_kriging_raw の mean ループを par_iter() 化
- **NFR-001**: パフォーマンステスト（tc_803_p01_pdp_1d_performance: 20ms以内）通過
- **NFR-002**: 全テスト PASS 確認

---

## 信頼性レベルサマリー

### 全タスク統計

- **総タスク数**: 14件
- 🔵 **青信号**: 14件 (100%)
- 🟡 **黄信号**: 0件 (0%)
- 🔴 **赤信号**: 0件 (0%)

### フェーズ別信頼性

| フェーズ | 🔵 青 | 🟡 黄 | 🔴 赤 | 合計 | 工数 |
|---------|-------|-------|-------|------|------|
| Phase 1 | 4 | 0 | 0 | 4 | 5.5h |
| Phase 2 | 3 | 0 | 0 | 3 | 5.5h |
| Phase 3 | 3 | 0 | 0 | 3 | 2.5h |
| Phase 4 | 4 | 0 | 0 | 4 | 6.5h |
| **合計** | **14** | **0** | **0** | **14** | **20h** |

### タスクタイプ別統計

| タイプ | 件数 | 工数 | 説明 |
|--------|------|------|------|
| **TDD** | 10件 | 16.5h | 機能実装・コード変更 |
| **DIRECT** | 4件 | 3.5h | 環境構築・設定・確認 |

**品質評価**: ✅ 高品質
- すべてのタスクが設計文書・要件定義書・ユーザヒアリングに基づいている（🔵青信号100%）
- 依存関係が明確で実装順序が決定可能
- テストカバレッジが包括的（既存テスト + 新規テストケース）

---

## クリティカルパス分析

### クリティカルパス

```
TASK-2161
  → TASK-2164
    → TASK-2168
      → TASK-2170
        → TASK-2171
          → TASK-2173
            → TASK-2174
```

**クリティカルパス工数**: 7.5時間（最短達成時間）
**総工数**: 20時間
**並行作業可能工数**: 12.5時間（TASK-2162/2163/2165/2166/2167/2172 の一部）

### 推奨実装順序

1. **必須順序**（依存関係のため）:
   - Phase 1: 4タスクを順序通り実装
   - Phase 2: Phase 1完了後に3タスクを並行実装可能
   - Phase 3: Phase 2完了後に3タスクを並行実装可能
   - Phase 4: Phase 3完了後に4タスクを並行実装可能

2. **並行実装可能な組み合わせ**:
   - Phase 1: TASK-2162/2163/2164 は TASK-2161 後に並行可能
   - Phase 2: TASK-2165/2166/2167 は全て並行可能
   - Phase 3: TASK-2168/2169 は TASK-2170 前に並行可能
   - Phase 4: TASK-2172/2173 は TASK-2171 後に並行可能

---

## コード変更サマリー

### 変更ファイル

| ファイル | 変更内容 | タスク | 影響度 |
|---------|---------|--------|--------|
| `rust_core/src/pdp/utils.rs` | 4関数追加（normalize_x_minmax, normalize_y, r_squared, extract_xy） | TASK-2162~2164 | 高 |
| `rust_core/src/pdp/kriging_core.rs` | 4関数をヘルパー呼び出しに置き換え + rayon 並列化 | TASK-2165~2167, TASK-2172~2173 | 高 |
| `rust_core/src/pdp/api.rs` | extract_xy 適用 | TASK-2168 | 中 |
| `rust_core/src/pdp/ridge_core.rs` | fold スタイル統一 | TASK-2169 | 低 |
| `Cargo.toml` | rayon 依存追加 | TASK-2171 | 低 |

### 変更ファイル数

- **変更**: 5ファイル
- **変更なし**: 3ファイル（mod.rs, types.rs, tests.rs）
- **新規**: 0ファイル

---

## 品質保証

### テストカバレッジ

- **既存テスト**: tc_803_*, tc_1645_*, tc_1652_*, tc_1653_* （継続的に実行）
- **新規テスト**: 各ヘルパー関数に単体テスト + 呼び出し関数で統合テスト
- **パフォーマンステスト**: tc_803_p01, tc_803_p02 で改善を測定

### チェックリスト

- [ ] `cargo test` が全件 PASS （全フェーズ）
- [ ] `cargo clippy -- -D warnings` が警告なし （全フェーズ）
- [ ] パフォーマンステスト合格（tc_803_p01: 20ms以内, tc_803_p02: 100ms以内）（Phase 4）

---

## 実装推奨ワークフロー

### TDDタスクの実装例（TASK-2162）

```bash
# 1. 詳細要件定義
/tsumiki:tdd-requirements TASK-2162

# 2. テストケース作成
/tsumiki:tdd-testcases

# 3. テスト実装（RED）
/tsumiki:tdd-red

# 4. 最小実装（GREEN）
/tsumiki:tdd-green

# 5. リファクタリング
/tsumiki:tdd-refactor

# 6. 品質確認
/tsumiki:tdd-verify-complete
```

### DIRECTタスクの実装例（TASK-2161）

```bash
# 1. 直接実装・設定
/tsumiki:direct-setup TASK-2161

# 2. 動作確認
/tsumiki:direct-verify
```

---

## 次のステップ

タスクを実装するには:

- **全タスク順番に実装**: `/tsumiki:kairo-implement`
- **特定タスクを実装**: `/tsumiki:kairo-implement TASK-2161`
- **特定フェーズを実装**: `/tsumiki:kairo-implement TASK-2161 TASK-2162`

### 推奨実装順序

1. Phase 1 完成（5.5時間） → M1: 基礎ヘルパー完成
2. Phase 2 完成（5.5時間） → M2: kriging_core.rs リファクタリング完了
3. Phase 3 完成（2.5時間） → M3: API層修正完了
4. Phase 4 完成（6.5時間） → M4: 並列化完成

---

## トラブルシューティング

### 一般的な問題と対応

| 問題 | 原因 | 対応 |
|------|------|------|
| ヘルパー関数のテスト失敗 | エッジケース未処理（空配列など） | EPSILON クランプを確認 |
| 並列化後にテスト失敗 | データ競合（非スレッドセーフな型） | f64 が Send+Sync であることを確認 |
| パフォーマンス改善なし | スレッド生成オーバーヘッド | グリッド数が十分か確認（rayon threshold） |
| Clippy 警告 | 非効率なイテレータ | `collect()` を避け、`into_iter()` を使用 |

---

## 関連リソース

- **Rust官式**: [rayon クレート](https://docs.rs/rayon/)
- **PDP理論**: [設計文書](../../design/pdp-maintainability/architecture.md#設計詳細)
- **テスト結果**: [acceptance-criteria.md](../../spec/pdp-maintainability/acceptance-criteria.md)
