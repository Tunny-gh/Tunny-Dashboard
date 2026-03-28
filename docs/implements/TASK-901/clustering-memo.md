# TDD開発メモ: clustering-wasm

## 概要

- 機能名: クラスタリングWASM（PCA + k-means）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 実装ファイル:
  - `rust_core/src/clustering.rs`: PCA / k-means / Elbow / ClusterStats
- テストファイル:
  - `rust_core/src/clustering.rs` (inline tests)

## テストケース（11件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-901-01 | 正常系 | PCA: 既知の2D直交データで主成分方向が正しい |
| TC-901-02 | 正常系 | PCA: 投影後の形状が(n, n_components)になる |
| TC-901-03 | 境界値 | PCA: 空データでパニックしない |
| TC-901-04 | 正常系 | k-means: n=100・k=3で収束し labels.len()==n |
| TC-901-05 | 正常系 | k-meansのWCSSはk増加で単調減少 |
| TC-901-06 | 正常系 | Elbow法: recommended_k が 2〜max_k 範囲内 |
| TC-901-07 | 正常系 | ClusterStats: centroidが各クラスタの平均に一致 |
| TC-901-08 | 正常系 | ClusterStats: 有意差マーク(|t|>3)が付与される |
| TC-901-P01 | 性能 | PCA(n=2000,p=4): 50ms以内 |
| TC-901-P02 | 性能 | k-means(n=2000,p=4,k=5): 200ms以内 |
| TC-901-P03 | 性能 | ClusterStats(n=2000,p=4,k=5): 150ms以内 |

## 主要な設計決定

1. **PCA: 共分散行列 + Jacobi固有値分解**
   - データに対してO(np²)で共分散行列を構築、p×pのJacobi分解はO(p³)
   - p≤30前提でNIPALS不要; スイープ収束条件: max_off < 1e-12

2. **k-means++初期化（決定的）**
   - D²比例サンプリングをしきい値ベースで実装（ランダム要素なし）
   - テストの再現性を確保

3. **Lloyd法の収束判定**
   - ラベル変化なし or max_iter=300 で停止
   - 最大300イテレーションは実用上充分

4. **Elbow推定: 二階差分**
   - `second_diffs[i] = wcss[i] - 2*wcss[i+1] + wcss[i+2]`
   - argmax + offset=3（k=2スタート）で推薦k

5. **性能テストのコンパイル条件分岐**
   - `#[cfg(debug_assertions)]`: n=2000, p=4
   - release: n=50000, p=10

## 最終テスト結果

```
running 11 tests
test clustering::tests::tc_901_01_pca_dominant_axis ... ok
test clustering::tests::tc_901_02_pca_projection_shape ... ok
test clustering::tests::tc_901_03_pca_empty_data ... ok
test clustering::tests::tc_901_04_kmeans_convergence ... ok
test clustering::tests::tc_901_05_kmeans_wcss_decreases_with_k ... ok
test clustering::tests::tc_901_06_elbow_recommended_k_valid ... ok
test clustering::tests::tc_901_07_cluster_stats_centroid ... ok
test clustering::tests::tc_901_08_cluster_stats_significant ... ok
test clustering::tests::tc_901_p01_pca_performance ... ok
test clustering::tests::tc_901_p02_kmeans_performance ... ok
test clustering::tests::tc_901_p03_cluster_stats_performance ... ok

test result: ok. 11 passed; 0 failed
```

## 品質評価

✅ **高品質**
- テスト: 11/11 通過
- セキュリティ: 重大な脆弱性なし
- パフォーマンス: debug buildで400ms以内達成
- REQ-080〜REQ-087 準拠
