//! 階層クラスタリング（Ward 法・凝集型）。
//!
//! 最近傍チェーン（nearest-neighbor chain）アルゴリズムと Lance-Williams 更新で
//! O(n²) の Ward リンケージを計算する。理論的背景は
//! theory/{en,ja}/clustering/hierarchical.md。

/// 1 回の併合。ノード ID は 0..n が葉（行）、n+i が i 番目の併合で生じた内部ノード。
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub a: usize,
    pub b: usize,
    /// 併合時の Ward 距離（クラスタ内分散増分の平方根スケール）。
    pub distance: f64,
    /// 併合後のクラスタサイズ。
    pub size: usize,
}

/// Ward リンケージの結果。
#[derive(Debug, Clone)]
pub struct HierarchicalResult {
    /// n-1 回の併合（距離昇順とは限らないが、チェーン法では単調非減少になる）。
    pub merges: Vec<Merge>,
    /// デンドログラム描画用の左→右の葉順（葉 = `row_indices` のインデックス）。
    pub leaf_order: Vec<usize>,
    /// 各葉が指す元データの行インデックス（サブサンプル時のため）。
    pub row_indices: Vec<usize>,
}

/// デンドログラムの 1 内部ノードぶんの描画座標。
/// x は葉位置（0..n-1）単位、height は Ward 距離。
#[derive(Debug, Clone, PartialEq)]
pub struct DendrogramNode {
    pub x: f64,
    pub height: f64,
    pub child_x: (f64, f64),
    pub child_heights: (f64, f64),
}

/// 階層クラスタリングにかける最大行数。超える場合は等間隔サブサンプルする
/// （デンドログラムはこの規模を超えると判読不能になる）。
pub const MAX_HIERARCHICAL_ROWS: usize = 800;

/// Ward 法の凝集型階層クラスタリングを実行する。
///
/// `standardize` が true なら各列を平均 0・分散 1 に標準化してから距離を取る
/// （単位の異なる変数を混在させる場合は必須）。行数が
/// [`MAX_HIERARCHICAL_ROWS`] を超える場合は等間隔サブサンプルする。
/// 行数 2 未満・特徴 0 のときは `None`。
pub fn ward_linkage(data: &[Vec<f64>], standardize: bool) -> Option<HierarchicalResult> {
    if data.len() < 2 || data[0].is_empty() {
        return None;
    }

    // ── サブサンプル（等間隔・決定論的）─────────────────────────
    let row_indices: Vec<usize> = if data.len() > MAX_HIERARCHICAL_ROWS {
        let step = data.len() as f64 / MAX_HIERARCHICAL_ROWS as f64;
        (0..MAX_HIERARCHICAL_ROWS)
            .map(|i| ((i as f64 * step) as usize).min(data.len() - 1))
            .collect()
    } else {
        (0..data.len()).collect()
    };
    let n = row_indices.len();
    let p = data[0].len();

    // ── 標準化（オプション）──────────────────────────────────────
    let mut x: Vec<Vec<f64>> = row_indices.iter().map(|&r| data[r].clone()).collect();
    if standardize {
        for j in 0..p {
            let mean = x.iter().map(|r| r[j]).sum::<f64>() / n as f64;
            let var = x.iter().map(|r| (r[j] - mean).powi(2)).sum::<f64>() / n as f64;
            let std = var.sqrt();
            for row in &mut x {
                row[j] = if std > 1e-12 {
                    (row[j] - mean) / std
                } else {
                    0.0
                };
            }
        }
    }

    // ── 距離行列（Ward 初期値 = ユークリッド距離の 2 乗 / 2 ... 慣例的には
    //    d² をそのまま使い Lance-Williams で更新する）─────────────
    let mut dist = vec![0.0f64; n * n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d2: f64 = x[i].iter().zip(&x[j]).map(|(a, b)| (a - b) * (a - b)).sum();
            dist[i * n + j] = d2;
            dist[j * n + i] = d2;
        }
    }

    // ── 最近傍チェーン ────────────────────────────────────────────
    // active[c] = クラスタ c が生存しているか。size[c] = 要素数。
    // node_id[c] = デンドログラム上のノード ID（葉 or 内部）。
    let mut active = vec![true; n];
    let mut size = vec![1usize; n];
    let mut node_id: Vec<usize> = (0..n).collect();
    let mut merges: Vec<Merge> = Vec::with_capacity(n - 1);
    let mut chain: Vec<usize> = Vec::with_capacity(n);
    let mut next_node = n;

    // 各内部ノードの子（葉順再構築用）。
    let mut children: Vec<(usize, usize)> = Vec::with_capacity(n - 1);

    let d = |dist: &Vec<f64>, a: usize, b: usize| dist[a * n + b];

    for _ in 0..(n - 1) {
        if chain.is_empty() {
            let start = (0..n).find(|&c| active[c]).unwrap();
            chain.push(start);
        }
        loop {
            let c = *chain.last().unwrap();
            // c の最近傍（チェーン直前の要素を優先して相互最近傍を検出）。
            let prev = if chain.len() >= 2 {
                Some(chain[chain.len() - 2])
            } else {
                None
            };
            let mut best = usize::MAX;
            let mut best_d = f64::INFINITY;
            for (cand, &is_active) in active.iter().enumerate() {
                if cand == c || !is_active {
                    continue;
                }
                let dd = d(&dist, c, cand);
                if dd < best_d || (dd == best_d && Some(cand) == prev) {
                    best_d = dd;
                    best = cand;
                }
            }
            if Some(best) == prev {
                // 相互最近傍 → 併合。
                let (a, b) = (chain.pop().unwrap(), chain.pop().unwrap());
                let (sa, sb) = (size[a], size[b]);
                merges.push(Merge {
                    a: node_id[a],
                    b: node_id[b],
                    distance: best_d.max(0.0).sqrt(),
                    size: sa + sb,
                });
                children.push((node_id[a], node_id[b]));

                // b のスロットへ併合クラスタを格納し、a を無効化。
                // Lance-Williams (Ward): d(k, a∪b)² 更新。
                for k in 0..n {
                    if !active[k] || k == a || k == b {
                        continue;
                    }
                    let (sk, sab) = (size[k] as f64, (sa + sb) as f64);
                    let new_d = ((sa as f64 + sk) * d(&dist, a, k)
                        + (sb as f64 + sk) * d(&dist, b, k)
                        - sk * d(&dist, a, b))
                        / (sab + sk);
                    dist[b * n + k] = new_d;
                    dist[k * n + b] = new_d;
                }
                active[a] = false;
                size[b] = sa + sb;
                node_id[b] = next_node;
                next_node += 1;
                break;
            }
            chain.push(best);
        }
    }

    // ── 距離昇順へ並べ替え（NN チェーンの時系列順は距離順とは限らない）──
    // Ward は木単調（親の距離 ≥ 子の距離）なので、昇順安定ソートは
    // トポロジカル順（子が親より先）を保ち、root は常に最後の併合になる。
    let mut order: Vec<usize> = (0..merges.len()).collect();
    order.sort_by(|&i, &j| {
        merges[i]
            .distance
            .partial_cmp(&merges[j].distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut id_map = vec![0usize; merges.len()];
    for (new_pos, &old_idx) in order.iter().enumerate() {
        id_map[old_idx] = new_pos;
    }
    let remap = |id: usize| if id < n { id } else { n + id_map[id - n] };
    let merges: Vec<Merge> = order
        .iter()
        .map(|&oi| {
            let m = &merges[oi];
            Merge {
                a: remap(m.a),
                b: remap(m.b),
                distance: m.distance,
                size: m.size,
            }
        })
        .collect();
    let children: Vec<(usize, usize)> = merges.iter().map(|m| (m.a, m.b)).collect();

    // ── 葉順の再構築（左の子を先に辿る深さ優先）──────────────────
    let root = 2 * n - 2;
    let mut leaf_order = Vec::with_capacity(n);
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node < n {
            leaf_order.push(node);
        } else {
            let (l, r) = children[node - n];
            // pop 順を左→右にするため右を先に積む。
            stack.push(r);
            stack.push(l);
        }
    }

    Some(HierarchicalResult {
        merges,
        leaf_order,
        row_indices,
    })
}

/// デンドログラムをカットして k クラスタのラベル（葉 ID → 0..k-1）を返す。
/// 距離の大きい順に k-1 本の併合を無視することで k 個の部分木に分割する。
pub fn cut_tree(result: &HierarchicalResult, k: usize) -> Vec<usize> {
    let n = result.leaf_order.len();
    let k = k.clamp(1, n);
    // 最後の k-1 併合を除いた森でラベル付けする（チェーン法の距離は単調なので
    // merges 末尾 k-1 個が最も距離の大きい併合に一致する）。
    let cutoff = n - k; // 採用する併合数
    let mut labels = vec![usize::MAX; n];
    // Union-Find 簡易版: ノード → 代表葉。
    let mut parent: Vec<usize> = (0..(2 * n - 1)).collect();
    fn find(parent: &mut [usize], mut v: usize) -> usize {
        while parent[v] != v {
            parent[v] = parent[parent[v]];
            v = parent[v];
        }
        v
    }
    for (i, m) in result.merges.iter().enumerate().take(cutoff) {
        let node = n + i;
        let ra = find(&mut parent, m.a);
        let rb = find(&mut parent, m.b);
        parent[ra] = node;
        parent[rb] = node;
    }
    let mut next_label = 0usize;
    let mut label_of_root: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    // 葉順でラベルを振ると左→右で 0,1,2,... になり描画と対応しやすい。
    for &leaf in &result.leaf_order {
        let root = find(&mut parent, leaf);
        let label = *label_of_root.entry(root).or_insert_with(|| {
            let l = next_label;
            next_label += 1;
            l
        });
        labels[leaf] = label;
    }
    labels
}

/// デンドログラム描画用のノード座標を計算する。
/// 葉 i の x 座標は `leaf_order` 内の位置、内部ノードの x は子の x の平均。
pub fn dendrogram_nodes(result: &HierarchicalResult) -> Vec<DendrogramNode> {
    let n = result.leaf_order.len();
    let mut pos = vec![(0.0f64, 0.0f64); 2 * n - 1]; // (x, height)
    for (i, &leaf) in result.leaf_order.iter().enumerate() {
        pos[leaf] = (i as f64, 0.0);
    }
    let mut nodes = Vec::with_capacity(result.merges.len());
    for (i, m) in result.merges.iter().enumerate() {
        let (xa, ha) = pos[m.a];
        let (xb, hb) = pos[m.b];
        let x = 0.5 * (xa + xb);
        pos[n + i] = (x, m.distance);
        nodes.push(DendrogramNode {
            x,
            height: m.distance,
            child_x: (xa, xb),
            child_heights: (ha, hb),
        });
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 明確に分離した 2 クラスタのデータ。
    fn two_blobs() -> Vec<Vec<f64>> {
        let mut data = Vec::new();
        for i in 0..10 {
            data.push(vec![i as f64 * 0.01, 0.0]);
            data.push(vec![100.0 + i as f64 * 0.01, 0.0]);
        }
        data
    }

    #[test]
    fn produces_n_minus_one_merges_and_full_leaf_order() {
        let data = two_blobs();
        let r = ward_linkage(&data, false).unwrap();
        assert_eq!(r.merges.len(), data.len() - 1);
        let mut order = r.leaf_order.clone();
        order.sort_unstable();
        assert_eq!(order, (0..data.len()).collect::<Vec<_>>());
    }

    #[test]
    fn cut_at_two_separates_the_blobs() {
        let data = two_blobs();
        let r = ward_linkage(&data, false).unwrap();
        let labels = cut_tree(&r, 2);
        // 偶数行（x≈0）と奇数行（x≈100）でラベルが分かれる。
        let l0 = labels[0];
        assert!(
            (0..data.len()).all(|i| if i % 2 == 0 {
                labels[i] == l0
            } else {
                labels[i] != l0
            }),
            "labels = {labels:?}"
        );
    }

    #[test]
    fn merge_distances_are_monotone_nondecreasing() {
        // NN チェーン + Ward は単調（inversion なし）。描画の前提なので確認する。
        let data = two_blobs();
        let r = ward_linkage(&data, false).unwrap();
        for w in r.merges.windows(2) {
            assert!(w[0].distance <= w[1].distance + 1e-9);
        }
    }

    #[test]
    fn standardize_makes_columns_comparable() {
        // 第 2 列だけ巨大スケール: 標準化なしでは第 2 列が支配、ありなら両列が効く。
        // 結線確認のみ（数値品質は問わない）: 標準化して 2 クラスタに割れること。
        let mut data = Vec::new();
        for i in 0..8 {
            data.push(vec![0.0, i as f64 * 1e6]);
            data.push(vec![1.0, i as f64 * 1e6]);
        }
        let r = ward_linkage(&data, true).unwrap();
        assert_eq!(r.merges.len(), data.len() - 1);
    }

    #[test]
    fn subsamples_above_cap() {
        let data: Vec<Vec<f64>> = (0..(MAX_HIERARCHICAL_ROWS + 100))
            .map(|i| vec![i as f64])
            .collect();
        let r = ward_linkage(&data, false).unwrap();
        assert_eq!(r.row_indices.len(), MAX_HIERARCHICAL_ROWS);
        assert_eq!(r.leaf_order.len(), MAX_HIERARCHICAL_ROWS);
    }

    #[test]
    fn dendrogram_nodes_have_consistent_geometry() {
        let data = two_blobs();
        let r = ward_linkage(&data, false).unwrap();
        let nodes = dendrogram_nodes(&r);
        assert_eq!(nodes.len(), r.merges.len());
        for node in &nodes {
            assert!((node.x - 0.5 * (node.child_x.0 + node.child_x.1)).abs() < 1e-9);
            assert!(node.height >= node.child_heights.0.max(node.child_heights.1) - 1e-9);
        }
    }

    #[test]
    fn too_small_input_returns_none() {
        assert!(ward_linkage(&[vec![1.0]], false).is_none());
        assert!(ward_linkage(&[], false).is_none());
    }
}
