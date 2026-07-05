# Clustering Methods — Quick Reference Guide

An overview of the methods used in Tunny Dashboard's clustering widget (`ClusterScatter`).
For detailed theory and implementation, refer to the individual method documents.

---

## Clustering Methods

| Method                          | Role                              | Strengths                                        | Limitations                                     | Details                  |
| ------------------------------- | --------------------------------- | ------------------------------------------------ | ----------------------------------------------- | ------------------------ |
| k-means (Lloyd's algorithm)     | Partition data into $k$ clusters  | Fast, intuitive, quality measurable via WCSS     | Assumes spherical clusters; risk of local optima | [kmeans.md](./kmeans.md) |

### k-means Initialization Strategies (Init)

The initial centroid selection method for k-means. Both are **internal settings of k-means, not separate clustering methods** — the Lloyd's algorithm itself (assign → update → converge) is shared.

| Init strategy | Description                                                                           | Best for                            |
| ------------- | ------------------------------------------------------------------------------------- | ----------------------------------- |
| k-means++     | D²-proportional probability sampling (Xoshiro256Plus, fixed seed derived from n · k) | Avoid local optima; quality-first   |
| Deterministic | Same k-means++ D²-proportional sampling (delegated to linfa), but with a fixed seed (42) | Fully reproducible results required |

## Choosing the Number of Clusters (k)

The Elbow method is an auxiliary tool for k-means, not a clustering method itself. It is used to decide "how many clusters to split into" before running k-means.

| Method      | Role                                                                    | Strengths                      | Limitations                                    | Details                |
| ----------- | ----------------------------------------------------------------------- | ------------------------------ | ---------------------------------------------- | ---------------------- |
| Elbow method | Auto-estimate optimal $k$ from the rate of change in WCSS (within-cluster sum of squares) | No need for user to specify $k$ | Estimation accuracy decreases when WCSS is smooth | [elbow.md](./elbow.md) |

---

## Workflow

```
Run clustering
  │
  ├── k selection: Elbow (Auto)
  │    └── Elbow method exhaustively tries k=2..max_k and auto-estimates optimal k
  │         └── Run k-means with estimated k (applying Init strategy)
  │
  └── k selection: Manual
       └── Run k-means directly with user-specified k (applying Init strategy)

Init strategy (differs only in k-means initialization)
  ├── k-means++    → D²-proportional probability sampling (seed derived from n, k)
  └── Deterministic → same D²-proportional sampling, seed fixed to 42
```

---

## Choosing the Input Space

| Setting         | Features used              | Best analysis                              |
| --------------- | -------------------------- | ------------------------------------------ |
| Objective Space | Objective values only      | Find trials with similar performance       |
| Variable Space  | Parameter values only      | Identify patterns in design variables      |
| Combined        | Objectives + parameters    | See joint structure across both spaces     |
