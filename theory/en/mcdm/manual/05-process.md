# Chapter 5: The MCDM Process for Multi-Objective Optimization Results

## Process Overview

MCDM is not simply a process of feeding in criterion values and computing scores. To obtain appropriate results, it is necessary to treat problem definition, organization of the evaluation targets, criteria design, data collection, weighting, evaluation computation, and result interpretation as a continuous process.

The workflow for applying MCDM to multi-objective optimization results is as follows.

```text
1. Problem formulation
2. Definition of individuals / trials / candidate solutions
3. Organization of constraints and the candidate set
4. Setting evaluation criteria
5. Preparation of optimization result data
6. Weighting
7. Evaluation and ranking
8. Interpretation of top candidates
```

This sequence is not merely a checklist of steps—it is a structure for progressively clarifying the assumptions behind the decision. If a problem is discovered in a later step, return to the earlier step and revise. For example, if the ranking result differs greatly from intuition, it is necessary to examine not only the calculation method but also the evaluation criteria, weights, and quality of the input data.

## Problem Formulation

The first thing to do is clarify what decision needs to be made. In MCDM, leaving the problem formulation vague when defining evaluation criteria and weights makes the computation results difficult to interpret.

Problem formulation involves organizing at least the following points.

| Item | What to confirm |
| --- | --- |
| Objective of the decision | What is being selected, ranked, or compared among the individuals? |
| Use case | In which business process, analysis, or judgment will the results be used? |
| Constraints | Are there conditions that must be satisfied or grounds for exclusion? |
| Decision-maker | Whose judgment or preferences should be reflected? |
| Use of results | Will the top individual be adopted, the top N retained as candidates, or used in explanatory materials? |

Even for the same problem—"select the best trial from optimization results"—the approach differs depending on the objective. Whether the goal is to maximize performance, strike a balance between cost and performance, or minimize risk will change the evaluation criteria and weights.

In problem formulation, it is important to articulate—before any computation—what counts as "good."

## Definition of Individuals / Trials / Candidate Solutions

Alternatives are the subjects compared and evaluated in MCDM. When handling multi-objective optimization results, alternatives correspond to individuals, trials, candidate solutions, and design proposals.

When defining alternatives, it is necessary to ensure that the candidates are aligned within a scope that is appropriate for comparison. Placing candidates with very different characteristics into the same decision matrix makes the ranking difficult to interpret.

Key points to confirm when defining alternatives are as follows.

- The candidates are all alternatives for the same objective
- The data needed for evaluation can be obtained
- Individuals that clearly violate constraints are excluded
- Duplicate candidates or candidates under identical conditions are consolidated
- The assumptions at the time of evaluation are aligned across candidates

Alternatives that violate constraints are generally excluded before the MCDM computation. For example, proposals that exceed the budget limit or products that do not meet mandatory requirements cannot be adopted even if they rank highly in the overall evaluation. In such cases, treating these conditions as pre-filter criteria rather than evaluation criteria is often clearer.

In multi-objective optimization, using individuals on the Pareto front as the candidate set is the standard approach. However, to account for measurement error, constraint margins, and unevaluated qualitative criteria, individuals near the Pareto front or dominated solutions may be included. In such cases, the reason for including them in the candidate set must be recorded.

## Setting Evaluation Criteria

Evaluation criteria are the perspectives used to compare individuals. In multi-objective optimization, the objective functions often directly become evaluation criteria, but derived indicators such as constraint margin, stability, ease of implementation, and reproducibility may also be added.

Evaluation criteria include both quantitative and qualitative types.

| Type | Example | Handling |
| --- | --- | --- |
| Quantitative criterion | Price, processing time, accuracy, profit, risk value | Easily handled directly as a numerical value. |
| Qualitative criterion | Usability, maintainability, ease of adoption, support quality | Requires conversion to a numerical score or rating scale. |

Evaluation criteria also have an evaluation direction.

| Direction | Description | Example |
| --- | --- | --- |
| Benefit criterion | Larger values are more desirable | Performance, quality, profit, accuracy |
| Cost criterion | Smaller values are more desirable | Price, time, risk, loss |

When setting evaluation criteria, keep the following in mind.

- Select criteria that are directly relevant to the decision objective
- Avoid including criteria with overlapping meaning
- Clarify the measurement method or scoring method
- Explicitly indicate whether each criterion is a benefit or cost type
- Choose a level of granularity that can be explained later

Too many criteria make weighting and interpretation of results difficult. Conversely, missing important criteria means the ranking will fail to reflect the reality of the decision. Selecting a necessary and sufficient set of criteria is critical.

## Preparation of Optimization Result Data

After setting the evaluation criteria, organize the criterion values for each individual. Criterion values are arranged as a decision matrix, which serves as the computation input for MCDM.

The basic form of the decision matrix is as follows.

| Individual | Criterion 1 | Criterion 2 | Criterion 3 |
| --- | ---: | ---: | ---: |
| A | 10 | 0.80 | 5 |
| B | 12 | 0.75 | 3 |
| C | 8 | 0.70 | 6 |

In data collection, verify not only the accuracy of values but also their comparability. If units, measurement conditions, aggregation periods, or evaluation methods differ, comparison as the same criterion may not be possible.

Points to verify include the following.

- No missing or anomalous values
- Units are consistent
- Measurement conditions are aligned
- The minimize / maximize direction is recorded for each objective
- The handling of constraint violations and infeasible individuals has been decided
- The basis for converting qualitative assessments to numerical values is clear
- Data is current and evaluation timestamps are aligned

If missing values are present, consider options such as imputation, exclusion of the subject, or substitution with an alternative criterion. Whichever approach is chosen must be recorded so that the results can be explained later.

## Weighting

Weighting is the step of determining the relative importance of each evaluation criterion. Because weights have a large influence on MCDM results, the setting method and rationale must be made clear.

An example of weights is as follows.

| Criterion | Weight |
| --- | ---: |
| Cost | 0.30 |
| Performance | 0.40 |
| Risk | 0.20 |
| Delivery | 0.10 |

In most cases, weights are normalized so that they sum to 1. However, some methods use only ratios in their computation. Even in such cases, expressing weights as summing to 1 in explanatory materials makes them easier to interpret.

Weighting methods include the following.

| Method | Description |
| --- | --- |
| Equal weights | Treat all evaluation criteria as equally important. |
| Expert judgment | Experts or decision-makers set relative importance directly. |
| AHP | Derive weights through pairwise comparisons. |
| Entropy Weight Method | Derive weights objectively based on data variance. |

Since weights represent the decision-maker's value judgments, there is not necessarily a single correct answer. For this reason, sensitivity analysis—confirming how much the ranking changes when weights are varied—is important.

## Evaluation and Ranking

In the evaluation and ranking step, the decision matrix, evaluation directions, and weights are used to compute an overall score or ranking for each alternative.

The typical computation flow is as follows.

```text
1. Construct the decision matrix
2. Normalize criterion values
3. Apply weights
4. Compute scores according to the MCDM method
5. Produce a ranking based on scores or preference relations
```

How scores are computed depends on the MCDM method used. For example, TOPSIS computes scores based on distance from the ideal solution. VIKOR ranks by the compromise solution concept. PROMETHEE ranks using preference relations between alternatives.

In the evaluation and ranking step, verify the following in addition to the raw computation results.

- Score gap between top and bottom candidates
- Criteria whose weights have the greatest effect
- Alternatives that are extremely advantaged on a single criterion
- Influence of missing values or outliers
- Stability of rankings when the method is changed

Rankings are information that supports decision-making, not the final decision itself. In particular, when score differences are small, it is necessary to examine the breakdown by criterion rather than drawing conclusions from the ranking alone.

## Interpretation of Results

The final step of MCDM is to interpret the computation results and organize them into a form usable for decision-making. Even if a ranking is obtained, if the rationale cannot be explained, building consensus among stakeholders and adoption in practice will be difficult.

In interpreting results, verify the following.

| Item | Description |
| --- | --- |
| Rationale for top candidates | Which evaluation criteria contributed to the score? |
| Weaknesses of lower-ranked candidates | Which evaluation criteria worked against them? |
| Influence of weights | To what extent did the importance settings affect the ranking? |
| Ranking stability | Is the ranking stable even with small changes to weights or data? |
| Practical constraints | Are there constraints that make top-ranked alternatives unadoptable? |

When explaining results, simply stating "Alternative A ranked first" is insufficient. It is necessary to show why Alternative A ranked highly, in which criteria it excels, and which assumptions it depends on.

Furthermore, MCDM results depend on the criteria, weights, data, and method. If these assumptions change, so do the results. Therefore, results should be treated not as a fixed correct answer but as material for organizing the basis for decision-making.

In practice, recording the following information alongside the final conclusion improves reproducibility and explainability.

- Evaluation criteria used
- Evaluation direction for each criterion
- Source and timestamp of criterion values
- Method used for weight setting
- MCDM method used
- Ranking results
- Results of sensitivity analysis

This makes it easier to verify the validity of decisions after the fact and to reuse the process for similar decisions in the future.


---

[← Chapter 4: Candidate Selection from the Pareto Front](04-pareto-front-selection.md) | [Table of Contents](TOC.md) | [Chapter 6: Evaluation Criteria Design for Individual Selection →](06-criteria-design.md)
