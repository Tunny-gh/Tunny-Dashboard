# Chapter 6: Evaluation Criteria Design for Individual Selection

## The Role of Criteria Design

Evaluation criteria are the perspectives used to compare alternatives in MCDM. The meaning of the ranking results changes greatly depending on which criteria are used. No matter how sophisticated the MCDM method, results that are useful for decision-making cannot be obtained if the criteria are inappropriate.

Criteria design involves not merely listing measurable items, but selecting the perspectives that are necessary for the decision objective and organizing them in a comparable form. It is especially important to clarify the meaning of each criterion, how it is measured, its evaluation direction, whether it overlaps with other criteria, and the quality of its data.

The basic perspectives for designing evaluation criteria are as follows.

| Perspective | What to confirm |
| --- | --- |
| Relevance | Is it directly related to the decision objective? |
| Measurability | Can it be expressed as a numerical value or a consistent rating scale? |
| Comparability | Can it be compared across alternatives under the same conditions and meaning? |
| Independence | Does it overlap in meaning too heavily with other criteria? |
| Explainability | Can the reason for using this criterion be explained? |

In MCDM, evaluation criteria become the very assumptions of the decision. For this reason, before beginning the computation, it is necessary to explicitly state what each criterion represents, how it is measured, and whether larger or smaller values are more desirable.

## Correspondence Between Objective Functions and Evaluation Criteria

When handling multi-objective optimization results, using the objective functions from the optimization as MCDM evaluation criteria is the standard approach. However, using the objective functions as-is is not always sufficient. Objective functions are indicators used to guide the search, and they do not necessarily cover all the perspectives needed for the final adoption decision.

| Information from optimization | Treatment in MCDM | Example |
| --- | --- | --- |
| Objective functions | Used as evaluation criteria | Accuracy, cost, processing time |
| Constraints | Pre-filter or evaluation criterion | Maximum cost, minimum strength, allowable risk |
| Constraint margin | Added as an evaluation criterion | Margin to the limit, amount of violation |
| Search meta-information | Used for explanation and traceability | Trial ID, generation, parameters |
| Practical perspectives | Added as additional criteria where needed | Stability, reproducibility, ease of implementation |

When converting objective functions to evaluation criteria, explicitly indicate whether each objective is a benefit type or a cost type. For example, accuracy is a criterion where larger is better, while processing time and cost are criteria where smaller is better.

When adding perspectives not included in the optimization as additional criteria after the fact, verify that those criteria can be obtained fairly for all individuals. If only some individuals have the additional assessment, rankings may be skewed.

## Quantitative and Qualitative Criteria

Evaluation criteria are broadly divided into quantitative and qualitative types.

Quantitative criteria are criteria that can be directly measured as numerical values. Examples include price, processing time, accuracy, profit, energy consumption, and failure rate. Quantitative criteria are easy to enter directly into the decision matrix and tend to produce reproducible computation results.

Qualitative criteria are criteria that are difficult to express directly as numbers. Examples include usability, maintainability, ease of adoption, support quality, and brand reliability. When handling qualitative criteria in MCDM, it is necessary to convert them to numerical values through rating scales or scoring.

| Type | Example | Characteristic | Note |
| --- | --- | --- | --- |
| Quantitative criterion | Price, processing time, accuracy, profit | Measured values can be used directly | Units and measurement conditions must be aligned |
| Qualitative criterion | Usability, maintainability, ease of adoption | Easy to reflect practically important perspectives | Scoring standards can become ambiguous |

When converting qualitative criteria to numerical values, define the rating scale in advance. For example, when rating maintainability on a 5-point scale, clearly define what each score means.

| Score | Example rating for maintainability |
| ---: | --- |
| 5 | Documentation is sufficient; anyone can maintain it, not just the person in charge |
| 4 | Basic documentation exists; minor modifications are straightforward |
| 3 | Maintainable, but depends on the knowledge of the assigned person |
| 2 | Complex structure; impact scope of changes is hard to predict |
| 1 | Maintenance carries significant risk |

When using qualitative criteria, concretizing the meaning of each score is important in order to reduce variability among evaluators.

## Benefit and Cost Criteria

Evaluation criteria include those where larger values are more desirable and those where smaller values are more desirable. In MCDM, failing to explicitly state this evaluation direction can result in normalization and score computation that evaluate in the opposite direction.

Criteria where larger values are more desirable are called benefit criteria. Examples include performance, quality, profit, accuracy, and availability.

Criteria where smaller values are more desirable are called cost criteria. Examples include price, processing time, risk, loss, energy consumption, and error rate.

| Direction | Meaning | Example |
| --- | --- | --- |
| Benefit criterion | Larger values are more desirable | Performance, profit, accuracy, quality, availability |
| Cost criterion | Smaller values are more desirable | Price, time, risk, loss, energy consumption |

For example, suppose the following evaluation criteria are defined.

| Criterion | Meaning of value | Direction |
| --- | --- | --- |
| Price | Cost required for adoption | Smaller is better |
| Processing speed | Number of operations per second | Larger is better |
| Error rate | Failure rate relative to total operations | Smaller is better |
| Support quality | Evaluation score for the support structure | Larger is better |

The handling of evaluation direction differs by MCDM method. In methods such as TOPSIS that use positive ideal and negative ideal solutions, the maximum value becomes the positive ideal solution for benefit criteria, while the minimum value becomes the positive ideal solution for cost criteria. When using a simple weighted sum, cost criteria must be inverted or transformed before aggregation.

Incorrect evaluation direction will fundamentally distort results—for example, making low-cost or low-risk alternatives appear disadvantaged. Therefore, during criteria definition, always record the evaluation direction.

## Constraints and Feasibility

In individual selection, the handling of constraints must be considered separately from evaluation criteria. Constraints include hard constraints that render an alternative ineligible if violated, and soft constraints where greater margin is more desirable.

| Type | Description | Recommended handling |
| --- | --- | --- |
| Hard constraint | A condition whose violation makes adoption impossible | Exclude before MCDM |
| Soft constraint | A condition where greater margin is more desirable | Treat as an evaluation criterion |
| Constraint violation amount | A value indicating the degree of violation | Exclude or treat as a penalty criterion |
| Constraint margin | Margin from the upper or lower bound | Treat as a safety-side evaluation criterion |

For example, if an individual that exceeds the maximum weight is infeasible, that individual is excluded before MCDM. On the other hand, if a larger margin to the weight constraint allows for safer design among the feasible individuals, the constraint margin can be added as an evaluation criterion.

If constraint-violating individuals are included in MCDM, they may appear at the top of the ranking despite being inadoptable. Therefore, feasibility must be clearly indicated in result displays to prevent infeasible individuals from being mistakenly treated as adoption candidates.

## Independence of Criteria

Design evaluation criteria to be as independent in meaning as possible. Including multiple criteria with similar meanings results in the same perspective being evaluated redundantly, which can skew the ranking.

For example, the following criteria may overlap.

| Criterion 1 | Criterion 2 | Potential overlap |
| --- | --- | --- |
| Processing time | Processing speed | May represent the same performance characteristic in opposite directions |
| Initial cost | Adoption cost | May represent nearly the same cost item |
| Error rate | Reliability | If reliability is assessed from error rate, they overlap |
| Maintainability | Operational burden | If operational burden is a subset of maintainability, they overlap |

Including overlapping criteria increases the effective weight of that perspective. For example, entering "processing time" and "processing speed" as separate criteria and assigning weights to both results in performance being evaluated twice.

The following questions are useful for checking the independence of criteria.

- Are two criteria measuring the same phenomenon in different expressions?
- Can one criterion be almost entirely derived from the other?
- Does including both result in a particular perspective being overweighted?
- Can the difference between the two criteria be explained to the decision-maker?
- Even if the two are highly correlated, do they have different meanings for the decision?

Selecting criteria that are completely independent may be difficult in practice. What matters is to check for overlap, and when overlap exists, to integrate the criteria, remove one, or adjust through weights.

## Considerations When Setting Criteria

When setting evaluation criteria, verify that they are aligned with the decision objective, that they can be measured, and that they can be explained. In particular, having too many criteria or criteria that are too abstract makes the results difficult to interpret.

Common problems and countermeasures are as follows.

| Problem | Example | Countermeasure |
| --- | --- | --- |
| Ambiguous criterion | Broad terms like "usability" or "goodness" | Specify the rating scale or evaluation perspective |
| Overlapping criteria | Treating cost and expense as separate criteria | Consolidate or remove one |
| Inconsistent measurement conditions | Comparing data from different periods | Align measurement conditions and aggregation periods |
| Unclear direction | Unclear whether larger or smaller is better | Explicitly state benefit or cost type |
| Too many criteria | Including many similar indicators | Narrow down to criteria that drive the decision |
| Unstable subjective assessments | Scores vary by evaluator | Define scoring standards |

Before finalizing evaluation criteria, check the following list.

- Is this criterion necessary for the decision objective?
- Can criterion values be obtained for all alternatives?
- Are the units and measurement conditions for criterion values clearly defined?
- Is it defined as a benefit or cost criterion?
- For qualitative criteria, is the meaning of each score defined?
- Does it overlap excessively in meaning with other criteria?
- When explaining results, can the reason for using this criterion be explained?

Evaluation criteria are the central element of accountability for MCDM computation results. When ranking results lack intuitive plausibility, the cause is often in the design of the evaluation criteria rather than in the method. Therefore, evaluation criteria should not be treated as fixed once decided, but as something to revisit as needed while examining results.

However, changing criteria in ways that happen to be convenient after seeing the ranking results will compromise the objectivity of the decision. When criteria are changed, the reason must be recorded and results before and after the change must be kept available for comparison.


---

[← Chapter 5: The MCDM Process for Multi-Objective Optimization Results](05-process.md) | [Table of Contents](TOC.md) | [Chapter 7: Weighting Methods for Individual Selection →](07-weighting-methods.md)
