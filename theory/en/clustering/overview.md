# Clustering — Overview

The Cluster Scatter widget groups optimization trials into clusters to reveal structure in objective or parameter space.

## Method Summary

| Method  | Role                              | Strength                           | Limitation                         |
| ------- | --------------------------------- | ---------------------------------- | ---------------------------------- |
| k-means | Partition data into k clusters    | Fast, intuitive, WCSS measurable   | Assumes spherical clusters         |
| Elbow   | Auto-estimate optimal k           | No manual k selection needed        | Less reliable on smooth WCSS curves |

## Initialization Strategies for k-means

Both use Lloyd's algorithm (assign → update → converge). Only the initial centroid selection differs.

| Strategy      | Method                                      | Best for                        |
| ------------- | ------------------------------------------- | ------------------------------- |
| k-means++     | D²-proportional probabilistic sampling      | Default — avoids local optima   |
| Deterministic | Cumulative-distance threshold spread        | Fully reproducible results      |

## Workflow

```
Run clustering
  │
  ├─ k selection: Elbow (Auto)
  │    └─ try k=2..max_k with Elbow method → auto-pick best k
  │         └─ run k-means with chosen k
  │
  └─ k selection: Manual
       └─ run k-means with user-specified k

Init strategy (applies within k-means):
  ├─ k-means++    → probabilistic D² sampling
  └─ Deterministic → deterministic spread
```

## Input Space

| Setting         | Features used              | Best analysis                             |
| --------------- | -------------------------- | ----------------------------------------- |
| Objective Space | Objective values only      | Find trials with similar performance      |
| Variable Space  | Parameter values only      | Identify patterns in design variables     |
| Combined        | Objectives + parameters    | See joint structure across both spaces    |
