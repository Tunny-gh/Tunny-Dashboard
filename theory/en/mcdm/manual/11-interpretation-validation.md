# Chapter 11: Interpreting and Validating Individual Selection Results

## How to Read Ranking Results

An MCDM ranking is a relative ordering obtained based on evaluation criteria, weights, normalization method, and computation method. The ranking is information that supports decision-making; it does not by itself guarantee a final conclusion.

When reviewing a ranking, first check the following points.

| Check Item | Content |
| --- | --- |
| Rank | Which alternatives are at the top and bottom? |
| Score gap | Is the difference between top-ranked alternatives large enough? |
| Contributing criteria | Which evaluation criteria are influencing the rank? |
| Preconditions | What weights, normalization method, and evaluation directions were used? |
| Constraints | Do the top-ranked alternatives meet practical mandatory requirements? |

For example, if the score difference between first and second place is very small, it is not appropriate to emphasize the rank order alone. In such a case, treat them as equivalent candidates, conduct additional evaluation, or confirm ranking stability through a sensitivity analysis.

It is also important to break down and review the reasons behind high scores by criterion. Looking only at the overall score makes it impossible to see where a particular alternative excels and where it is weak.

## Cases Where First Place Is Not Directly Adopted

In individual selection from multi-objective optimization results, it may not be appropriate to mechanically adopt the top-ranked individual from the MCDM ranking. MCDM produces a ranking that reflects value judgments, but for final adoption, constraint margins, implementability, reproducibility, and stakeholder acceptability must also be verified.

Representative situations where first place is not directly adopted are as follows.

| Situation | Response |
| --- | --- |
| Small score gap between first and second | Treat as equivalent candidates and conduct additional comparison |
| First place is at the edge of a constraint | Also review top candidates with larger constraint margins |
| First place is an extreme solution | Keep balanced solutions and knee-point candidates too |
| Rank changes substantially with weight changes | Identify individuals that remain in the top across multiple scenarios |
| Unevaluated practical risks exist | Conduct additional evaluation before the final decision |

In such cases, the MCDM ranking is treated not as "a result that determines a single individual to adopt" but as "a result that narrows down candidates for detailed review."

## Sensitivity Analysis

Sensitivity analysis is an analysis that checks how much the ranking results change when weights or evaluation values are varied. Because weights and normalization methods affect results in MCDM, sensitivity analysis is an important step for verifying the reliability of results.

Representative sensitivity analyses are as follows.

| Method | Explanation |
| --- | --- |
| Weight variation | Increase or decrease the weight on a specific criterion and observe rank changes. |
| Equal-weight comparison | Compare with equal-weight results to see the effect of the weight settings. |
| Method comparison | Compare ranks across multiple methods such as TOPSIS, VIKOR, and WSM. |
| Normalization method comparison | Compare Min-Max normalization with vector normalization, etc. |
| Criterion exclusion | Observe rank changes when a specific criterion is excluded. |

The main focus of sensitivity analysis is the stability of the top candidates. If the top rank changes frequently with only a small adjustment to the weights, the ranking is unstable. Conversely, if the same candidates remain in the top even when weights and methods are varied, those candidates can be considered relatively stable choices.

## Checking the Impact of Weight Changes

Because weights represent the value judgment of decision makers, they have a large influence on MCDM results. Checking the impact of weight changes involves deliberately varying weights and examining how the ranking changes.

Example checks are as follows.

- Increase or decrease the weight on the most important criterion by 10%.
- Compare three patterns: cost-focused, performance-focused, and risk-focused.
- Compare expert-judgment weights with entropy weights.
- Determine the weight range over which the top two candidates swap places.
- Set the weight on a specific criterion to 0 and check the impact.

Weight change results can be explained more easily when organized as follows.

| Scenario | Weight Characteristics | 1st Place | Notes |
| --- | --- | --- | --- |
| Baseline | Standard weights | A | Good overall balance |
| Cost-focused | Increased cost weight | C | Low-cost alternative rises to top |
| Performance-focused | Increased performance weight | B | High-performance alternative rises to top |
| Risk-focused | Increased risk weight | A | Low-risk alternative maintained |

By showing rank changes due to weight variation in this way, it becomes possible to explain which value judgment the final decision depends on.

## Checking for Outliers and Bias

MCDM results depend on the quality of the input data. If outliers, missing values, bias in evaluation criteria, or variability in subjective assessments are present, ranking results may be distorted.

Representative problems to check are as follows.

| Problem | Impact | Response |
| --- | --- | --- |
| Outliers | Strongly affects normalization results and distance calculations | Confirm the reason for the outlier; consider exclusion, correction, or separate analysis |
| Missing values | Makes comparisons incomplete | Treat as imputed, excluded, or unevaluable |
| Overlapping criteria | Evaluates a particular aspect twice | Integrate criteria or adjust weights |
| Variability in subjective assessment | Results vary by evaluator | Define the evaluation scale and verify with multiple evaluators |
| Differences in data acquisition timing | Cannot compare under the same conditions | Align the acquisition period and prerequisite conditions |

In particular, Min-Max normalization is sensitive to outliers. When extremely large or small values are present, the differences among other alternatives may appear small. It is necessary to determine whether the outlier is a valid measured value or a value resulting from an input error or special condition.

## Recording Validation Results

When using MCDM results in practice, record not only the rankings but also the validation results, so that decisions can be reviewed later.

Information to record is as follows.

- The decision matrix used
- Evaluation criteria and evaluation directions
- Weights and the method used to set them
- Normalization method
- The MCDM method used
- Ranking results
- Results of sensitivity analyses
- Handling of outliers and missing values
- Final decision and its rationale

Preserving this information improves the reproducibility and explainability of ranking results. It is important to operate MCDM not merely as a tool for producing computation results, but as a technique for making the premises of decisions explicit.


---

[← Chapter 10: Individual Ranking Calculation Flow](10-calculation-flow.md) | [Table of Contents](TOC.md) | [Chapter 12: Application Examples for Optimization Results →](12-use-cases.md)
