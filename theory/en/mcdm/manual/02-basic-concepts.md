# Chapter 2: Basic Concepts of MCDM

## What Is Multi-Criteria Decision Making?

Multi-Criteria Decision Making (MCDM) is a decision-making methodology for comparing and evaluating multiple alternatives while simultaneously considering multiple evaluation criteria.

In real-world decision-making, it is common to be unable to judge a subject by a single measure. For example, product selection requires simultaneously examining price, performance, reliability, delivery time, and maintainability. When selecting optimization results, multiple objective functions exist, and trade-offs arise where one solution is superior in cost while another is superior in performance.

MCDM addresses such situations by explicitly defining the evaluation criteria, organizing the criterion values for each alternative, and, where necessary, computing an overall evaluation that reflects the relative importance of each criterion as weights. This makes the basis for decisions easier to explain and facilitates sharing evaluation assumptions among stakeholders.

The basic inputs to MCDM are the following three.

| Input | Description |
| --- | --- |
| Alternatives | The candidates under comparison. In multi-objective optimization, these include individuals, trials, and candidate solutions. |
| Evaluation criteria | The perspectives used to assess alternatives. Examples: price, performance, risk, quality, objective functions. |
| Criterion values | The value each alternative holds for each evaluation criterion. |

In addition to these, the relative importance of each criterion is specified as weights, along with an evaluation direction indicating whether larger or smaller values are more desirable.

## Difference from Single-Criterion Evaluation

In single-criterion evaluation, alternatives are compared using a single criterion value. For example: choosing the lowest-priced product, selecting the method with the shortest processing time, or choosing the plan with the highest profit. In this case, the ranking is relatively straightforward because criterion values can be compared directly.

MCDM, on the other hand, handles multiple evaluation criteria simultaneously. With multiple criteria, it is not guaranteed that any single alternative will be best on all of them. One alternative may be superior in price, another in quality, and yet another in risk—advantages and disadvantages are distributed across criteria.

The differences between single-criterion evaluation and MCDM are as follows.

| Aspect | Single-criterion evaluation | MCDM |
| --- | --- | --- |
| Evaluation criteria | 1 | Multiple |
| Comparison method | Compare criterion values directly | Integrate or compare across multiple criteria |
| Difficulty of judgment | Relatively straightforward | Requires handling trade-offs |
| Weighting | Usually unnecessary | Often reflects the importance of each criterion |
| What to explain | Why that value is good | Which criteria were weighted how much |

In MCDM, handling trade-offs between criteria is central. For example, prioritizing cost may place the cheapest alternative higher, while prioritizing performance may favor the highest-performing one. For this reason, MCDM results depend heavily not only on the calculation method but also on how criteria are chosen and how weights are set.

## Relationship Between Alternatives, Criteria, and Weights

In MCDM, the relationship between alternatives, evaluation criteria, and weights is organized as a decision matrix. The decision matrix is a table with alternatives in rows, criteria in columns, and criterion values in cells.

For example, evaluating three alternatives across four criteria produces the following decision matrix.

| Alternative | Cost | Performance | Risk | Delivery |
| --- | ---: | ---: | ---: | ---: |
| A | 80 | 70 | 30 | 20 |
| B | 60 | 85 | 45 | 30 |
| C | 75 | 90 | 25 | 25 |

This table alone may not allow a unique judgment of which alternative is most desirable. Cost and risk are better when smaller, while performance is better when larger—so the evaluation direction differs by criterion. Furthermore, the conclusion changes depending on how much weight is placed on delivery time.

Weights are therefore assigned to each criterion.

| Criterion | Direction | Example weight |
| --- | --- | ---: |
| Cost | Smaller is better | 0.30 |
| Performance | Larger is better | 0.40 |
| Risk | Smaller is better | 0.20 |
| Delivery | Smaller is better | 0.10 |

Weights represent the relative importance of each criterion. In the example above, performance is weighted most heavily, followed by cost, risk, and delivery. Most MCDM methods normalize criterion values and then apply weights to compute an overall score or ranking for each alternative.

Organizing this relationship, the basic structure of MCDM is as follows.

```text
Alternatives × Criteria = Decision matrix
Decision matrix + Evaluation directions + Weights = Overall evaluation
Overall evaluation = Score or ranking
```

An important point is that weights are not merely computational parameters—they represent the value judgments of the decision-maker. Even with the same decision matrix, the ranking may change if the weights change. For this reason, MCDM places importance not only on the results but also on the rationale behind weights and on sensitivity analysis.

## When MCDM Is Effective

MCDM is effective in situations where multiple evaluation criteria exist and there are trade-offs among them. It is particularly suitable when the rationale for a decision needs to be explained or when evaluation criteria need to be shared among stakeholders.

Representative application scenarios are as follows.

| Scenario | Example |
| --- | --- |
| Product/service selection | Comparing multiple products by price, performance, maintainability, and support quality. |
| Supplier evaluation | Comprehensively evaluating price, delivery time, quality, supply stability, and risk. |
| Project prioritization | Determining execution order based on expected impact, implementation cost, risk, and urgency. |
| Investment decision | Comparing profitability, risk, initial cost, and payback period. |
| Selection from multi-objective optimization results | Choosing the most comprehensively desirable solution from trials with multiple objective functions. |

This document focuses primarily on the last scenario: selecting from multi-objective optimization results. That is, it assumes an input of individuals obtained through the optimization search, and the goal is to narrow down adoption candidates using objective function values, constraint status, and additional evaluation criteria.

MCDM is particularly well suited when the following conditions are met.

- Multiple alternatives are subject to evaluation
- Multiple criteria exist and a judgment cannot be based on a single criterion alone
- The relative importance of each criterion needs to be made explicit
- Evaluation results need to be explained to stakeholders
- The effect of changing weights or criterion values on results needs to be examined

On the other hand, using MCDM does not always yield an objectively correct answer. The selection of criteria, the quality of criterion values, and the setting of weights all involve subjectivity and assumptions. MCDM is not so much a tool for automating decisions as it is a support technology for organizing the assumptions behind decisions and clarifying the basis for comparison.

Therefore, when applying MCDM, it is important not to simply adopt the computed results but to verify the validity of the evaluation criteria, the rationale for the weights, and the stability of the ranking.


---

[← Chapter 1: Introduction](01-introduction.md) | [Table of Contents](TOC.md) | [Chapter 3: Multi-Objective Optimization and the Role of MCDM →](03-multi-objective-context.md)
