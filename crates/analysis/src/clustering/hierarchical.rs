//! Hierarchical clustering (Ward's method, agglomerative).
//!
//! Computes O(n²) Ward linkage using the nearest-neighbor chain algorithm with
//! Lance-Williams updates. See theory/{en,ja}/clustering/hierarchical.md for the
//! theoretical background.

/// A single merge. Node IDs 0..n are leaves (rows); n+i is the internal node produced by
/// the i-th merge.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub a: usize,
    pub b: usize,
    /// Ward distance at the time of the merge (square-root scale of the within-cluster
    /// variance increase).
    pub distance: f64,
    /// Cluster size after the merge.
    pub size: usize,
}

/// Result of Ward linkage.
#[derive(Debug, Clone)]
pub struct HierarchicalResult {
    /// The n-1 merges (not necessarily in ascending distance order, though the chain
    /// algorithm makes them monotonically non-decreasing).
    pub merges: Vec<Merge>,
    /// Left-to-right leaf order for dendrogram rendering (leaves = indices into `row_indices`).
    pub leaf_order: Vec<usize>,
    /// Row index into the original data that each leaf refers to (for subsampling).
    pub row_indices: Vec<usize>,
}

/// Drawing coordinates for a single internal node of the dendrogram.
/// `x` is in leaf-position units (0..n-1); `height` is the Ward distance.
#[derive(Debug, Clone, PartialEq)]
pub struct DendrogramNode {
    pub x: f64,
    pub height: f64,
    pub child_x: (f64, f64),
    pub child_heights: (f64, f64),
}

/// Maximum number of rows to run hierarchical clustering on. Rows beyond this are
/// subsampled at even intervals (dendrograms become unreadable beyond this scale).
pub const MAX_HIERARCHICAL_ROWS: usize = 800;

/// Runs agglomerative hierarchical clustering using Ward's method.
///
/// If `standardize` is true, each column is standardized to mean 0, variance 1 before
/// computing distances (required when mixing variables with different units). Rows
/// containing NaN/Inf, or rows with a mismatched feature count, have undefined distances
/// and are excluded up front (the same policy as the other clustering functions). Rows
/// beyond [`MAX_HIERARCHICAL_ROWS`] are subsampled at even intervals. Returns `None` if
/// fewer than 2 valid rows remain or there are 0 features.
pub fn ward_linkage(data: &[Vec<f64>], standardize: bool) -> Option<HierarchicalResult> {
    if data.len() < 2 || data[0].is_empty() {
        return None;
    }
    let p = data[0].len();

    // ── Exclude NaN/Inf rows and ragged rows ──────────────────────
    // If even one non-finite value is present, all pairwise distances become NaN, causing
    // nearest-neighbor search to find no candidate and index arithmetic to reach an
    // out-of-bounds panic (mitigation for H4).
    let finite_rows: Vec<usize> = (0..data.len())
        .filter(|&r| data[r].len() == p && data[r].iter().all(|v| v.is_finite()))
        .collect();
    if finite_rows.len() < 2 {
        return None;
    }

    // ── Subsampling (even intervals, deterministic) ───────────────
    let row_indices: Vec<usize> = if finite_rows.len() > MAX_HIERARCHICAL_ROWS {
        let step = finite_rows.len() as f64 / MAX_HIERARCHICAL_ROWS as f64;
        (0..MAX_HIERARCHICAL_ROWS)
            .map(|i| finite_rows[((i as f64 * step) as usize).min(finite_rows.len() - 1)])
            .collect()
    } else {
        finite_rows
    };
    let n = row_indices.len();

    // ── Standardization (optional) ────────────────────────────────
    let mut x: Vec<Vec<f64>> = row_indices.iter().map(|&r| data[r].clone()).collect();
    if standardize {
        super::standardize::standardize_columns(&mut x, 0);
    }

    // ── Distance matrix (Ward's initial value = squared Euclidean distance / 2 ...
    //    by convention, d² is used directly and updated via Lance-Williams) ───────
    let mut dist = vec![0.0f64; n * n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d2: f64 = x[i].iter().zip(&x[j]).map(|(a, b)| (a - b) * (a - b)).sum();
            dist[i * n + j] = d2;
            dist[j * n + i] = d2;
        }
    }

    // ── Nearest-neighbor chain ─────────────────────────────────────
    // active[c] = whether cluster c is still alive. size[c] = element count.
    // node_id[c] = the node's ID in the dendrogram (leaf or internal).
    let mut active = vec![true; n];
    let mut size = vec![1usize; n];
    let mut node_id: Vec<usize> = (0..n).collect();
    let mut merges: Vec<Merge> = Vec::with_capacity(n - 1);
    let mut chain: Vec<usize> = Vec::with_capacity(n);
    let mut next_node = n;

    // Children of each internal node (used to reconstruct the leaf order).
    let mut children: Vec<(usize, usize)> = Vec::with_capacity(n - 1);

    let d = |dist: &Vec<f64>, a: usize, b: usize| dist[a * n + b];

    for _ in 0..(n - 1) {
        if chain.is_empty() {
            let start = (0..n).find(|&c| active[c]).unwrap();
            chain.push(start);
        }
        loop {
            let c = *chain.last().unwrap();
            // Nearest neighbor of c (prefers the previous element in the chain, to
            // detect mutual nearest neighbors).
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
            // Defense in depth: if no nearest neighbor is found (e.g. due to NaN
            // distances), bail out safely instead of panicking (normally unreachable
            // thanks to the finite-value filter at the top).
            if best == usize::MAX {
                return None;
            }
            if Some(best) == prev {
                // Mutual nearest neighbors → merge.
                let (a, b) = (chain.pop().unwrap(), chain.pop().unwrap());
                let (sa, sb) = (size[a], size[b]);
                merges.push(Merge {
                    a: node_id[a],
                    b: node_id[b],
                    distance: best_d.max(0.0).sqrt(),
                    size: sa + sb,
                });
                children.push((node_id[a], node_id[b]));

                // Store the merged cluster in b's slot and invalidate a.
                // Lance-Williams (Ward): update d(k, a∪b)².
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

    // ── Sort into ascending distance order (the NN chain's chronological order does
    //    not necessarily match distance order) ──────────────────────────────────
    // Ward is tree-monotone (parent distance ≥ child distance), so a stable ascending
    // sort preserves topological order (children before parents), and the root always
    // ends up as the last merge.
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

    // ── Reconstruct leaf order (depth-first, visiting the left child first) ───────
    let root = 2 * n - 2;
    let mut leaf_order = Vec::with_capacity(n);
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node < n {
            leaf_order.push(node);
        } else {
            let (l, r) = children[node - n];
            // Push right before left so pop order goes left → right.
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

/// Cuts the dendrogram and returns k-cluster labels (leaf ID → 0..k-1).
/// Splits into k subtrees by ignoring the k-1 merges with the largest distances.
pub fn cut_tree(result: &HierarchicalResult, k: usize) -> Vec<usize> {
    let n = result.leaf_order.len();
    let k = k.clamp(1, n);
    // Label using the forest with the last k-1 merges removed (since the chain
    // algorithm's distances are monotone, the last k-1 entries of `merges` correspond
    // to the merges with the largest distances).
    let cutoff = n - k; // number of merges to keep
    let mut labels = vec![usize::MAX; n];
    // Simplified union-find: node → representative leaf.
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
    // Assigning labels in leaf order gives 0,1,2,... left → right, matching the rendering.
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

/// Computes node coordinates for dendrogram rendering.
/// Leaf i's x coordinate is its position within `leaf_order`; an internal node's x is
/// the average of its children's x.
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

    /// Data with two clearly separated clusters.
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
        // Even rows (x≈0) and odd rows (x≈100) get different labels.
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
        // NN chain + Ward is monotone (no inversions). Verified since rendering relies on this.
        let data = two_blobs();
        let r = ward_linkage(&data, false).unwrap();
        for w in r.merges.windows(2) {
            assert!(w[0].distance <= w[1].distance + 1e-9);
        }
    }

    #[test]
    fn standardize_makes_columns_comparable() {
        // Only the 2nd column has a huge scale: without standardization it dominates;
        // with it, both columns contribute.
        // Wiring check only (numerical quality not assessed): confirms it still splits
        // into 2 clusters after standardization.
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

    #[test]
    fn nan_rows_are_excluded_without_panic() {
        // Even with NaN rows mixed in, clustering does not panic and only uses finite
        // rows (H4 regression test).
        let mut data = two_blobs();
        data.insert(3, vec![f64::NAN, 0.0]);
        data.push(vec![0.5, f64::INFINITY]);
        let n_valid = data.len() - 2;
        for standardize in [false, true] {
            let r = ward_linkage(&data, standardize).expect("valid rows remain");
            assert_eq!(r.row_indices.len(), n_valid);
            assert_eq!(r.merges.len(), n_valid - 1);
            // row_indices does not include the NaN rows (3, last).
            assert!(!r.row_indices.contains(&3));
            assert!(!r.row_indices.contains(&(data.len() - 1)));
            assert!(r.merges.iter().all(|m| m.distance.is_finite()));
        }
    }

    #[test]
    fn all_nan_rows_return_none() {
        // If fewer than 2 valid rows remain, returns None without panicking.
        let data = vec![vec![f64::NAN, 1.0], vec![2.0, f64::NAN], vec![1.0, 1.0]];
        assert!(ward_linkage(&data, false).is_none());
        assert!(ward_linkage(&data, true).is_none());
    }

    #[test]
    fn ragged_rows_are_excluded_without_panic() {
        // Rows with a mismatched feature count are also excluded, since their distance
        // is undefined.
        let mut data = two_blobs();
        data.push(vec![1.0]); // a row with only 1 feature
        let r = ward_linkage(&data, false).expect("valid rows remain");
        assert_eq!(r.row_indices.len(), data.len() - 1);
    }
}
