use tunny_core::pareto::hypervolume_nd;

fn main() {
    // Minimal counterexample with no dominated points and no duplicates: mix in a
    // dominated point (0.6,0.6) with the non-dominated front {(0.2,0.8),(0.8,0.2)}
    // (confirms duplicates are irrelevant here).
    let front_only = vec![vec![0.2, 0.8], vec![0.8, 0.2]];
    let with_dominated = vec![vec![0.2, 0.8], vec![0.8, 0.2], vec![0.6, 0.6]];
    let ref_pt = vec![1.0, 1.0];

    let hv_front = hypervolume_nd(&front_only, &ref_pt);
    let hv_with_dom = hypervolume_nd(&with_dominated, &ref_pt);

    println!("hv(front only)          = {hv_front}");
    println!("hv(front + dominated pt) = {hv_with_dom}");
    println!("expected: equal (dominated point contributes 0 additional HV)");
}
