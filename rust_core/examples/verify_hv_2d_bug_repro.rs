use tunny_core::pareto::hypervolume_nd;

fn main() {
    // 支配点なし・重複なしの最小反例: 非支配フロント {(0.2,0.8),(0.8,0.2)} に
    // 支配される点 (0.6,0.6) を1つ混ぜるだけ（重複は無関係であることの確認）。
    let front_only = vec![vec![0.2, 0.8], vec![0.8, 0.2]];
    let with_dominated = vec![vec![0.2, 0.8], vec![0.8, 0.2], vec![0.6, 0.6]];
    let ref_pt = vec![1.0, 1.0];

    let hv_front = hypervolume_nd(&front_only, &ref_pt);
    let hv_with_dom = hypervolume_nd(&with_dominated, &ref_pt);

    println!("hv(front only)          = {hv_front}");
    println!("hv(front + dominated pt) = {hv_with_dom}");
    println!("expected: equal (dominated point contributes 0 additional HV)");
}
