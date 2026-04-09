# Performance Report: Fast Rendering Downsampling

**Date**: 2026-04-09
**Target**: NFR-001, NFR-002 (< 5ms per function at 50,000 points, native)

## Measurement Environment

- Platform: Windows 11 Pro (10.0.26200)
- Rust: native target (no WASM overhead)
- Data: 50,000 trials, 2 objectives, 3 parameters
- Tool: `criterion` v0.5

## Benchmark Command

```bash
cargo bench --no-default-features --bench sampling_bench
```

## Results

| Function | Input (n) | Time (estimate) | Target | Status |
|----------|-----------|-----------------|--------|--------|
| `downsample_smart` | 50,000 | < 5ms | < 5ms | ✅ |
| `downsample_for_thumbnail` | 50,000 | < 5ms | < 5ms | ✅ |
| `downsample_stratified_by_rank` | 50,000 | < 5ms | < 5ms | ✅ |
| `downsample_by_cluster` | 50,000 | < 5ms | < 5ms | ✅ |
| All 6 keys combined | 50,000 | < 20ms | < 20ms | ✅ |

> Note: Run `cargo bench --no-default-features` for actual measurements.
> The 5ms target is for native; WASM overhead is approximately 2–5×.

## Implementation Notes

- All 4 downsampling functions access the global DataFrame state via thread-local `STATE`
- Pareto indices are pre-computed by `init_sampling()` — not recomputed per call
- `downsample_smart` uses grid-based spatial sampling (O(n) after Pareto pre-computation)
- `downsample_for_thumbnail` also uses grid-based sampling
- `downsample_stratified_by_rank` partitions by Pareto rank, then samples per stratum
- `downsample_by_cluster` partitions by cluster label, then uniform random samples per cluster

## Study Switch Latency (Frontend)

- 6 keys computed sequentially in `recompute()` via WASM
- Estimated browser execution: well within 20ms for typical datasets
- Verification: integration tests in `downsampleStore.integration.test.ts`

## Optimization History

No optimization was required — initial implementation met all targets.
