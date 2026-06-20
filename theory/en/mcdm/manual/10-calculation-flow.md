# Chapter 10: Individual Ranking Calculation Flow

## Overview of the Evaluation Computation

MCDM evaluation computation follows a flow in which a decision matrix is built, evaluation values are normalized, weights are applied, and scores or rankings are derived in accordance with the chosen MCDM method.

The general computation procedure is as follows.

```text
1. Build the candidate set from the optimization results
2. Exclude infeasible individuals
3. Build the decision matrix
4. Define the evaluation directions
5. Normalize the evaluation values
6. Apply weights
7. Compute scores according to the MCDM method
8. Build the ranking based on scores or preference relations
9. Compare the top-N individuals and review the results
```

This chapter uses a representative weighted-sum computation as an example to explain the flow from the decision matrix to the ranking. In methods such as TOPSIS, VIKOR, and PROMETHEE the score computation differs, but the preprocessing steps — decision matrix, evaluation direction, normalization, and weighting — share the same underlying rationale.

## Building the Decision Matrix

The decision matrix is a table with individuals in rows and evaluation criteria in columns. Each cell contains the evaluation value of an individual with respect to a criterion.

For example, when evaluating three alternatives on three criteria — cost, performance, and risk — the decision matrix is as follows.

| Individual | Cost | Performance | Risk |
| --- | ---: | ---: | ---: |
| A | 100 | 70 | 20 |
| B | 120 | 90 | 40 |
| C | 80 | 60 | 30 |

At this stage, overall superiority must not be judged from the raw numbers alone. Cost and risk are more desirable when smaller, while performance is more desirable when larger, so the evaluation directions differ. The units and value ranges also differ, so normalization is required.

When building the decision matrix, also define the following information.

| Item | Example |
| --- | --- |
| Criterion name | Cost, Performance, Risk |
| Evaluation direction | Cost-type, Benefit-type, Cost-type |
| Unit | Ten-thousand yen, Points, Risk score |
| Handling of missing values | Exclude, impute, or error |
| Data source | Measured values, estimated values, expert evaluation, etc. |

The decision matrix is the input data for MCDM itself. Errors or inconsistencies here will also make subsequent computation results inaccurate.

## Filtering the Candidate Set

Before building the decision matrix, decide which individuals to include in the MCDM evaluation. In multi-objective optimization results, rather than ranking all individuals directly, using constraint status and Pareto dominance relations to organize the candidate set makes the results easier to interpret.

Representative filtering steps are as follows.

| Operation | Purpose |
| --- | --- |
| Exclude infeasible individuals | Prevent individuals that cannot be adopted from appearing at the top of the ranking |
| Exclude constraint-violating individuals | Compare only candidates that satisfy hard constraints |
| Extract the non-dominated set | Narrow candidates to those on the Pareto front |
| Consolidate duplicate individuals | Group candidates with the same objective function values or identical conditions |
| Check outliers | Review input errors and anomalous exploration results |

When the candidate set is narrowed, record which conditions were used for exclusion. Looking only at the ranking results later makes it impossible to understand why certain individuals are absent.

## Weighted Decision Matrix

Rather than using the decision matrix as-is, first normalize the evaluation values into a comparable form and then apply weights.

In this example, the following evaluation directions and weights are used.

| Criterion | Evaluation Direction | Weight |
| --- | --- | ---: |
| Cost | Smaller is better | 0.30 |
| Performance | Larger is better | 0.50 |
| Risk | Smaller is better | 0.20 |

The weights sum to 1.

$$
0.30 + 0.50 + 0.20 = 1.00
$$

For simplicity of explanation here, Min-Max normalization is used. After normalization, all criteria are treated as "larger is better."

### Step 1: Normalize the Evaluation Values

Cost is more desirable when smaller, so inverted Min-Max normalization is applied.

| Alternative | Cost | After Normalization |
| --- | ---: | ---: |
| A | 100 | 0.50 |
| B | 120 | 0.00 |
| C | 80 | 1.00 |

Performance is more desirable when larger, so standard Min-Max normalization is applied.

| Alternative | Performance | After Normalization |
| --- | ---: | ---: |
| A | 70 | 0.33 |
| B | 90 | 1.00 |
| C | 60 | 0.00 |

Risk is more desirable when smaller, so inverted Min-Max normalization is applied.

| Alternative | Risk | After Normalization |
| --- | ---: | ---: |
| A | 20 | 1.00 |
| B | 40 | 0.00 |
| C | 30 | 0.50 |

The normalized decision matrix is as follows.

| Alternative | Cost | Performance | Risk |
| --- | ---: | ---: | ---: |
| A | 0.50 | 0.33 | 1.00 |
| B | 0.00 | 1.00 | 0.00 |
| C | 1.00 | 0.00 | 0.50 |

### Step 2: Apply Weights

Multiply each normalized evaluation value by the weight for its criterion.

$$
v_{ij} = w_j r_{ij}
$$

Here, $r_{ij}$ is the normalized evaluation value, $w_j$ is the weight for criterion $j$, and $v_{ij}$ is the weighted evaluation value.

| Alternative | Cost 0.30 | Performance 0.50 | Risk 0.20 |
| --- | ---: | ---: | ---: |
| A | 0.15 | 0.17 | 0.20 |
| B | 0.00 | 0.50 | 0.00 |
| C | 0.30 | 0.00 | 0.10 |

This table is the weighted decision matrix. It allows you to see how much each criterion contributes to the overall evaluation of each alternative.

## Score Computation

In the weighted sum method, the overall score for each alternative is obtained by summing its weighted evaluation values.

$$
S_i = \sum_{j=1}^{n} w_j r_{ij}
$$

In the above example, the scores for each alternative are as follows.

| Alternative | Computation | Overall Score |
| --- | --- | ---: |
| A | 0.15 + 0.17 + 0.20 | 0.52 |
| B | 0.00 + 0.50 + 0.00 | 0.50 |
| C | 0.30 + 0.00 + 0.10 | 0.40 |

In this result, A has the highest overall score. B performs best on performance but is disadvantaged on cost and risk. C performs best on cost but scores lower overall due to its low performance.

Score computation differs depending on the MCDM method.

| Method | Score Computation Approach |
| --- | --- |
| WSM | Multiply normalized values by weights and sum. |
| TOPSIS | Compute the distance ratio from the positive ideal and negative ideal solutions. |
| VIKOR | Compute the compromise index from overall utility and maximum regret. |
| PROMETHEE | Compute flows from pairwise preferences between alternatives. |
| AHP | Integrate normalized values using weights derived from pairwise comparisons. |

Therefore, even with the same decision matrix and the same weights, the meaning of scores differs depending on which MCDM method is selected. When comparing scores, it is necessary to clearly state which method produced the values.

## Building the Ranking

After computing scores, arrange alternatives according to the rules for each method.

In the weighted sum method and TOPSIS, a higher score is more desirable, so alternatives are ranked in descending order of score.

| Rank | Alternative | Overall Score |
| ---: | --- | ---: |
| 1 | A | 0.52 |
| 2 | B | 0.50 |
| 3 | C | 0.40 |

In VIKOR, a lower Q value is more desirable, so ranking is in ascending order of Q. In PROMETHEE II, a higher net flow is more desirable, so ranking is in descending order of net flow.

The ranking direction for each method is as follows.

| Method | Ranking Direction |
| --- | --- |
| WSM | Score descending |
| TOPSIS | Score descending |
| VIKOR | Q ascending |
| PROMETHEE II | Net flow descending |
| AHP | Score descending |

When building rankings, also decide how to handle ties and near-ties. If the score difference is very small, overemphasizing the rank order can be misleading. In such cases, treating alternatives as tied, showing the score difference, or confirming with a sensitivity analysis are effective approaches.

## Comparing the Top-N Individuals

In individual selection from multi-objective optimization results, the top-ranked individual is not necessarily adopted as-is. The MCDM ranking is material for narrowing down candidates, and in practice it is realistic to compare the top-N individuals before making the final decision.

When comparing top candidates, present the following information side by side.

| Information | What to Review |
| --- | --- |
| MCDM score | How large is the gap in overall evaluation? |
| Objective function values | What tradeoffs exist among the original optimization objectives? |
| Constraint status | Is the individual adoptable? Is the constraint margin sufficient? |
| Contributing criteria | Which criteria pushed the rank up? |
| Rank by weight scenario | Does the individual remain in the top under different value judgments? |

If the score difference between first and second place is small, it is more appropriate to treat them as equivalent candidates rather than emphasizing the rank. Particularly when measurement error or simulation error is present, avoid over-interpreting narrow rank differences.

## Interpreting the Computation Example

In the computation example above, A ranked first, B second, and C third. However, this ranking depends on the following assumptions.

- The three evaluation criteria are cost, performance, and risk.
- Cost and risk are more desirable when smaller; performance is more desirable when larger.
- Weights are cost 0.30, performance 0.50, risk 0.20.
- The normalization method is Min-Max normalization.
- Score computation uses the weighted sum method.

If the assumptions change, the ranking may also change. For example, if the weight on performance is reduced and the weight on cost is increased, C's rank may rise. If risk is weighted more heavily, A's advantage may increase further.

In this way, MCDM computation results are determined by the combination of decision matrix, evaluation directions, normalization method, weights, and method. When presenting results, it is necessary to show not only the ranks but also these underlying assumptions.

## Implementation Checkpoints

When implementing evaluation computation as a system, it is necessary to ensure not only the correctness of the computation procedure but also input validation and reproducibility of results.

Points to verify are as follows.

| Check Item | Content |
| --- | --- |
| Input size | Do the number of alternatives, number of criteria, and length of the value arrays match? |
| Candidate set | Is the handling of infeasible or constraint-violating individuals explicitly specified? |
| Evaluation direction | Is each criterion assigned benefit-type or cost-type? |
| Weights | Are weights non-negative and is their sum non-zero? Are they normalized if required? |
| Missing values | Will missing values and NaN be imputed, excluded, or treated as errors? |
| Division by zero | Is the case where max equals min or column norm is 0 handled? |
| Ranking direction | Is ascending or descending order applied correctly for each method? |
| Metadata | Are the normalization method, weights, method, and execution time recorded? |

In particular, to validate computation results afterwards, it is important to save both the input data and the computation conditions. Saving only the ranking makes it impossible to reproduce why a particular rank was obtained.

In MCDM evaluation computation, it is important not only that the formulas are correct, but also that preconditions are stated explicitly and that the same output can be reproduced from the same input.


---

[← Chapter 9: Normalizing Optimization Result Data](09-normalization.md) | [Table of Contents](TOC.md) | [Chapter 11: Interpreting and Validating Individual Selection Results →](11-interpretation-validation.md)
