# Chapter 9: Normalizing Optimization Result Data

## Why Normalization Is Needed

MCDM deals with multiple evaluation criteria simultaneously. Because each criterion has its own units and value range, simply summing or computing distances across raw values causes criteria with larger scales to dominate the result disproportionately.

For example, if price is expressed in currency units and runs into the hundreds of thousands while a quality score ranges from 1 to 5, naively adding the two numbers will let price dominate — not because price is genuinely more important, but simply because its units and scale are larger.

Normalization is the process of converting evaluation values with different units and ranges into a comparable scale. After normalization, every criterion can be handled on the same footing.

The main reasons normalization is required are as follows.

| Reason | Explanation |
| --- | --- |
| Different units | Comparing different units such as currency, seconds, percentages, and scores. |
| Different scales | Preventing criteria with a large value range from having excessive influence. |
| Different evaluation directions | Aligning criteria where higher is better with those where lower is better. |
| Method prerequisites | Satisfying the assumption of normalized values required by methods such as TOPSIS and WSM. |
| Explainability | Making it easier to explain how each criterion contributed to the overall evaluation. |

Normalization is a useful process, but the choice of normalization method can affect the results. Therefore, the normalization method should be chosen not solely for computational convenience, but in accordance with the nature of the evaluation criteria and the assumptions of the MCDM method being used.

## Min-Max Normalization

Min-Max normalization converts evaluation values into a specified range, typically 0 to 1. In MCDM, separate formulas are used for benefit-type and cost-type criteria.

For benefit-type criteria — criteria where larger values are more desirable — the following formula is used.

$$
r_{ij} = \frac{x_{ij} - x_j^{\min}}{x_j^{\max} - x_j^{\min}}
$$

For cost-type criteria — criteria where smaller values are more desirable — the following formula is used.

$$
r_{ij} = \frac{x_j^{\max} - x_{ij}}{x_j^{\max} - x_j^{\min}}
$$

Here, $x_{ij}$ is the evaluation value of alternative $i$ for criterion $j$, $x_j^{\min}$ is the minimum value of criterion $j$, and $x_j^{\max}$ is the maximum. The normalized value $r_{ij}$ falls in the range [0, 1].

As an example, consider processing speed where higher is better.

| Alternative | Processing Speed | After Min-Max Normalization |
| --- | ---: | ---: |
| A | 100 | 0.00 |
| B | 150 | 0.50 |
| C | 200 | 1.00 |

Conversely, for processing time where lower is better, the direction of values is inverted.

| Alternative | Processing Time | After Min-Max Normalization |
| --- | ---: | ---: |
| A | 10 | 1.00 |
| B | 20 | 0.50 |
| C | 30 | 0.00 |

The strength of Min-Max normalization is that the results fall within 0 to 1 and are intuitively easy to interpret. In WSM and AHP weighted-sum scores, the normalized values can be directly multiplied by weights and summed.

However, Min-Max normalization is sensitive to outliers. If even one value is extremely large or extremely small, the other values are compressed into a narrow range, making their differences appear smaller.

## Vector Normalization

Vector normalization divides each criterion column by its Euclidean norm. It is commonly used in TOPSIS.

For criterion $j$, the normalized value $r_{ij}$ is computed as follows.

$$
r_{ij} = \frac{x_{ij}}{\sqrt{\sum_{i=1}^{m} x_{ij}^2}}
$$

Here, $m$ is the number of alternatives. Each column is treated as a vector and scaled so that its length becomes 1.

As an example, suppose the values for a given criterion are as follows.

| Alternative | Value |
| --- | ---: |
| A | 3 |
| B | 4 |

The Euclidean norm of this column is:

$$
\sqrt{3^2 + 4^2} = 5
$$

Therefore, the normalized values are:

| Alternative | Value | After Vector Normalization |
| --- | ---: | ---: |
| A | 3 | 0.60 |
| B | 4 | 0.80 |

Vector normalization is well suited to distance calculations. In TOPSIS, weights are applied after vector normalization, and distances to the positive ideal and negative ideal solutions are computed.

However, vector-normalized values are not necessarily confined to [0, 1] in a way that gives an intuitive "achievement rate." When values include negatives or the criterion is cost-type, the ideal and anti-ideal solution assignments must be defined clearly according to the method's definition.

## Handling Cost-Type Criteria

In MCDM, the desirable direction of values varies by criterion. Some criteria — such as performance or profit — are more desirable when larger, while others — such as price, time, or risk — are more desirable when smaller.

If cost-type criteria are not handled appropriately, a value that should be smaller to be better is treated computationally as though larger is better, fundamentally distorting the ranking results.

Representative processing approaches are as follows.

| Approach | Explanation | Primary Use |
| --- | --- | --- |
| Inverted Min-Max | Converts to [0, 1] using the distance from the maximum | WSM, AHP weighted sum |
| Handled via ideal solution | For cost-type criteria, the minimum is the positive ideal and the maximum is the negative ideal | TOPSIS |
| Handled via gap | Uses the gap between best and worst values | VIKOR |
| Reciprocal transformation | Converts small values to large via $1 / x$ or similar | Ratio-based evaluation — beware of zeros |

For instance, in Min-Max normalization, using separate formulas for benefit-type and cost-type criteria ensures that after normalization both types can be treated as "larger is better."

| Alternative | Cost | After Normalization |
| --- | ---: | ---: |
| A | 100 | 1.00 |
| B | 150 | 0.50 |
| C | 200 | 0.00 |

With this transformation, alternatives with lower cost receive higher normalized values.

In TOPSIS, it is not strictly necessary to pre-convert cost-type criteria to "larger is better." Instead, for cost-type criteria, the minimum is treated as the positive ideal and the maximum as the negative ideal according to the evaluation direction. What matters is being explicit about at which stage the evaluation direction is incorporated.

## The Effect of Outliers and Extreme Solutions

Multi-objective optimization results often include extreme solutions that are outstanding on only one particular objective. Extreme solutions are important for understanding the ends of the Pareto front, but they require care in normalization.

For example, in Min-Max normalization the maximum and minimum values serve as the reference. If even one individual has an extremely low cost or extremely high performance, the normalized values of all other individuals are compressed into a narrow range, making their differences harder to see.

Points to check are as follows.

| Check Item | Explanation |
| --- | --- |
| Validity of extreme solutions | Determine whether the value is a real measurement, a simulation result, or an input error. |
| Constraint status | Verify that the extreme solution is not at the edge of or beyond its constraints. |
| Impact on normalization | Check whether the presence of the extreme solution substantially changes the top-ranked individuals. |
| Candidate set scope | Decide whether to normalize across all individuals, the non-dominated set, or only the top candidates. |

In practice, rather than excluding extreme solutions outright, visualize them first to understand their meaning. If they prove unsuitable as adoption candidates, remove them from the candidate set, or compare results with and without them in a sensitivity analysis.

## How the Normalization Method Affects Results

The normalization method affects MCDM results. Even with the same decision matrix, the same weights, and the same MCDM method, different normalization methods can yield different scores and rankings.

The main effects are as follows.

| Effect | Explanation |
| --- | --- |
| Outlier sensitivity | Min-Max normalization depends strongly on the maximum and minimum values. |
| Distance interpretation | Vector normalization is well matched to distance-based calculations such as TOPSIS. |
| Ratio handling | WPM is well suited to ratio-based differences but requires care with zeros and negative values. |
| Score range | Methods that produce values in [0, 1] are easier to explain, but the meaning differs between methods. |
| Ranking stability | Changing the normalization method can cause rankings to change. |

For example, Min-Max normalization examines relative position within the range of evaluation values. Vector normalization treats each column as a vector and expresses each value as a ratio relative to the total length of the column. Both equalize scale, but the numerical meaning differs.

When choosing a normalization method, verify the following:

- What normalization method is standard for the MCDM method being used?
- Does the evaluation data contain outliers?
- Are there criteria with zero or negative values?
- How will the normalized values be explained?
- Do the top candidates remain stable when the normalization method is changed?

In practice, comparing results across several normalization methods before settling on one is effective. If the top candidates change substantially depending on the normalization method, revisit the criteria design, outliers, weights, and method assumptions.

## Implementation Notes

When implementing normalization in a system, it is necessary to handle not only the formulas but also edge cases explicitly.

Typical points to watch are as follows.

| Point | Example Handling |
| --- | --- |
| Maximum equals minimum | The criterion does not distinguish alternatives; set the normalized value to 0 or a constant. |
| Column norm is 0 | Avoid division by zero in vector normalization; treat the normalized value as 0. |
| Missing values present | Clearly specify whether to impute, exclude, or raise an error. |
| Negative values present | Confirm whether the method allows negative values; apply Min-Max conversion if needed. |
| Evaluation direction not set | Do not run the computation; request that benefit-type or cost-type be specified. |

Because normalization forms the foundation of ranking results, it is desirable to record the normalization method, evaluation directions, and exception handling in logs or result metadata. This makes it possible to reproduce ranking results afterwards and explain why a particular result was obtained.


---

[← Chapter 8: Representative MCDM Methods](08-methods.md) | [Table of Contents](TOC.md) | [Chapter 10: Individual Ranking Calculation Flow →](10-calculation-flow.md)
