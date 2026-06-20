# Chapter 4: Candidate Selection from the Pareto Front

## Pareto Dominance and Non-Dominated Solutions

In multi-objective optimization, when one individual is at least as good as another in all objectives and strictly better in at least one objective, that individual is said to Pareto-dominate the other.

For a minimization problem, the condition for individual $a$ to dominate individual $b$ can be stated as follows.

$$
f_j(a) \le f_j(b) \quad \forall j
$$

and, for at least one objective $k$:

$$
f_k(a) < f_k(b)
$$

When maximization objectives are included, the dominance check is performed by aligning each objective's preferred direction. An individual that is not dominated by any other individual is called a non-dominated solution, and the set of non-dominated solutions is called the Pareto front or non-dominated set.

## Why Use the Pareto Front as the Candidate Set

When selecting individuals with MCDM, it is natural to start by using the individuals on the Pareto front as the candidate set. Dominated individuals are at least as bad as some other individual in all objectives, so they are generally weak candidates for adoption.

In practice, however, it is not always sufficient to mechanically use only the Pareto front. In the following situations, the method of constructing the candidate set must be made explicit.

| Situation | Response |
| --- | --- |
| Too many individuals on the Pareto front | Narrow down by filtering, clustering, or extracting the top N |
| Many extreme solutions on the front | Separately examine knee points and balanced solutions |
| Front includes individuals at the constraint boundary | Add constraint margin as an evaluation criterion |
| Noise present from approximate search | Verify not only dominance relations but also score differences and re-evaluations |

## When to Include Dominated Solutions

In principle, dominated individuals can be excluded from MCDM candidates. However, there are cases where dominated solutions are included in the comparison for the following reasons.

- There are measurement or simulation errors, and the differences in objective function values are small
- Individuals on the Pareto front may not satisfy practical constraints
- Some individual is superior on a qualitative criterion not included in the objective functions
- A wider comparison of search-in-progress candidates is desired

Even in such cases, the reason for including dominated individuals must be recorded. If an individual that ranks high in MCDM is dominated by other individuals in terms of the objective functions, that must be explainable.

## Knee Points, Balanced Solutions, and Extreme Solutions

Individuals on the Pareto front can have different characteristics.

| Type | Description | Significance for selection |
| --- | --- | --- |
| Knee point | The point of maximum curvature on the Pareto front. Often defined as the point farthest from the line connecting the two extreme solutions; moving away from this point causes exchange efficiency to deteriorate rapidly. | A candidate that is easy to adopt with an overall balance |
| Balanced solution | A point with few extreme weaknesses across multiple objectives | A candidate that is easy to explain and adopt |
| Extreme solution | A point that is exceptionally good in a particular objective | A candidate suited to a clear policy such as performance-focused or cost-focused |

With MCDM, which of these—knee points, balanced solutions, or extreme solutions—tends to rank highly depends on the weights and the method used. For example, WSM strongly reflects objectives with large weights; TOPSIS gives high scores to individuals that are comprehensively close to the ideal solution; VIKOR considers the balance between overall utility and maximum regret.

## Relationship Between Pareto Dominance and MCDM Rankings

Pareto dominance is a superiority relation based solely on objective functions. MCDM rankings, on the other hand, are overall evaluations that include normalization, weights, the chosen method, and additional criteria.

For this reason, even among individuals on the Pareto front—where no clear dominance relation exists—MCDM can assign rankings. This is an advantage of MCDM, but it is also a point of caution. The ranking reflects a value judgment and is not a universal measure of superiority.

For individual selection, the following sequence of checks is recommended.

```text
1. Exclude ineligible individuals
2. Examine Pareto dominance relations
3. Use the non-dominated set as the candidate set
4. Rank candidates using MCDM
5. Visualize top candidates and confirm trade-offs
6. Record the rationale for the final selection
```

Following this flow allows the structure of the search results to be respected while explicitly reflecting the decision-maker's preferences.


---

[← Chapter 3: Multi-Objective Optimization and the Role of MCDM](03-multi-objective-context.md) | [Table of Contents](TOC.md) | [Chapter 5: The MCDM Process for Multi-Objective Optimization Results →](05-process.md)
