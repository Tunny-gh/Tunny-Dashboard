# Chapter 13: Notes on Adopting MCDM

## Prerequisites to Verify at Adoption

When adopting MCDM, it is necessary to clarify the evaluation target, evaluation criteria, data, stakeholders, and how results will be used before selecting a computation method. If these points are left vague and computation is carried out anyway, a ranking may be produced but it may not serve as actionable decision-making material in practice.

Items to verify before adoption are as follows.

| Item | Content to Verify |
| --- | --- |
| Purpose | What is to be selected, compared, or ranked from which optimization results? |
| Alternatives | Are the comparison targets individuals or trials from the same exploration results or the same-condition candidates? |
| Candidate set | What scope is to be covered — all individuals, the non-dominated set, top candidates, etc.? |
| Evaluation criteria | Are the perspectives necessary for the decision included? |
| Data | Is the source, unit, and quality of evaluation values clear? |
| Weights | Whose value judgment is being reflected? |
| Usage | Will results be used for the final decision or for narrowing down candidates? |

## Bias in Evaluation Criteria

Bias in evaluation criteria causes bias in MCDM results. For example, including multiple performance-related criteria while including few cost- or risk-related criteria causes performance to be effectively overweighted.

Common biases are as follows.

| Bias | Example | Impact |
| --- | --- | --- |
| Overlapping criteria | Using processing speed and processing time as separate criteria | Evaluates the same aspect twice |
| Missing criteria | Not evaluating risk or maintainability | Practically important factors are not reflected |
| Bias toward measurability | Using only criteria that are easy to quantify | Qualitatively important factors are omitted |
| Bias toward a particular department's perspective | Leaning toward the viewpoint of the cost department, technical department, etc. | Consensus across all stakeholders is difficult to achieve |

To prevent bias in evaluation criteria, conduct a stakeholder review during criteria design and confirm that the criteria are necessary and sufficient for the purpose.

## Handling Subjective Judgment

In MCDM, subjective judgment may enter into weights and qualitative evaluations. The inclusion of subjectivity is not inherently a problem. What matters is explicitly indicating which judgments are based on subjectivity and being able to explain their rationale.

Points to note when handling subjective judgment are as follows.

- Clearly identify the evaluator.
- Define the evaluation scale.
- Verify with multiple evaluators.
- Record the reasoning behind judgments.
- Use sensitivity analysis to confirm the impact of subjective judgments.

For example, when assessing maintainability on a 5-point scale, simply recording "maintainability = 4" is not sufficient. It is necessary to define why the score is 4, what condition corresponds to 5, and what condition corresponds to 3.

## The Impact of Data Quality

MCDM computation results depend on the input data. If evaluation values contain errors, missing entries, outliers, or discrepancies in measurement conditions, ranking results will also be distorted.

Data quality check items are as follows.

| Check Item | Content |
| --- | --- |
| Completeness | Do all alternatives have the required evaluation values? |
| Accuracy | Are there any input or calculation errors? |
| Consistency | Are units, measurement conditions, and aggregation periods aligned? |
| Timeliness | Are the evaluation values valid at the time of the decision? |
| Outliers | Are extreme values valid measured values? |
| Constraint status | Are infeasible individuals or constraint-violating individuals included? |
| Exploration conditions | Were the compared individuals obtained under the same exploration and evaluation conditions? |

When data quality problems are identified, consider correction, exclusion, re-measurement, or revision of evaluation criteria. Document the response policy so that it can be presented when explaining the results.

## Ensuring Explainability

When adopting MCDM, it is important to be able to explain the ranking results to stakeholders. Simply stating "the result of this method is that A ranks first" will not produce acceptance.

Content that should be explained includes the following.

- Why were those evaluation criteria used?
- Where were the evaluation values obtained from?
- How were the weights determined?
- Which MCDM method was used?
- Why did that individual rank highly?
- Where on the Pareto front is that individual located?
- Does the individual satisfy the constraint conditions and is it adoptable?
- Is the conclusion stable even when weights or methods are changed?

To improve explainability, present the score breakdown by criterion, the contribution of each evaluation criterion, and sensitivity analysis results together. Making not only the overall score but also the premises and reasons for the decision visible is important.

## Building Consensus Among Stakeholders

MCDM is particularly effective for decisions that involve multiple stakeholders. However, if there is no consensus among stakeholders on evaluation criteria and weights, acceptance of the ranking results will be low.

For consensus building, proceeding in the following order makes the process easier.

```text
1. Reach consensus on the purpose of the decision
2. Reach consensus on the scope of alternatives
3. Reach consensus on how to construct the Pareto front or candidate set
4. Reach consensus on evaluation criteria
5. Reach consensus on how evaluation values are obtained
6. Reach consensus on how to approach weights
7. Reach consensus on how results will be used
```

Weights in particular reflect the value judgments of stakeholders. When opinions are divided, rather than forcing a single set of weights, an effective approach is to compare results across multiple weight scenarios.

## Post-Adoption Operations

MCDM does not end with a single computation. If evaluation data, candidates, weights, or the business environment change, the ranking will also change.

After adoption, establish the following as operational rules.

- At what point will recomputation be performed?
- Under what conditions will evaluation criteria be changed?
- Who is authorized to update weights?
- How will past results be preserved?
- What is the approval process for results?

To use MCDM continuously in practice, it is necessary to manage not only the computation logic but also the evaluation conditions and a record of changes.


---

[← Chapter 12: Application Examples for Optimization Results](12-use-cases.md) | [Table of Contents](TOC.md) | [Chapter 14: Design Points in System Implementation →](14-system-design.md)
