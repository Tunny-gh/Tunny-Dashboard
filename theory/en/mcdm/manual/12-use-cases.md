# Chapter 12: Application Examples for Optimization Results

## How to Think About Practical Application

MCDM is effective in situations that require simultaneous consideration of multiple evaluation criteria. In practice, it is important not just to produce a ranking, but to be able to explain the evaluation criteria, weights, and rationale for results to stakeholders.

When applying MCDM to multi-objective optimization results, organizing the work in the following flow makes it easier to structure.

```text
1. Define which optimization results to select individuals from
2. Exclude infeasible individuals and constraint-violating individuals
3. Organize the Pareto front or candidate set
4. Determine objective functions and additional evaluation criteria
5. Set weight scenarios
6. Rank with an MCDM method
7. Visualize top candidates and conduct sensitivity analysis as needed
```

This chapter primarily covers application examples for selecting individuals from multi-objective optimization results. The latter part also presents general MCDM application examples for reference.

## Selecting a Balanced Solution from the Pareto Front

Consider optimization results that maximize accuracy while minimizing cost and processing time. The Pareto front contains individuals with high accuracy but high cost, individuals with low cost but low accuracy, and balanced individuals in between.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Accuracy | Larger is better | Performance of the model or design. |
| Cost | Smaller is better | Expenses required for implementation and operation. |
| Processing time | Smaller is better | Time required for execution. |
| Constraint margin | Larger is better | Safety margin after adoption. |

In this case, using TOPSIS or VIKOR makes it easier to find candidates that are close to the ideal solution or have low maximum regret. Ultimately, rather than looking only at the top-ranked individual, compare the top several individuals via scatter plots or parallel coordinates and choose a balanced solution that is straightforward to adopt.

## Selecting Based on Performance-Focused, Cost-Focused, or Risk-Focused Scenarios

Even on the same Pareto front, the selected individual changes depending on the weight scenario.

| Scenario | Weight Characteristics | Expected Top Candidates |
| --- | --- | --- |
| Performance-focused | Higher weights on accuracy and quality | High-performance but high-cost individuals |
| Cost-focused | Higher weights on cost and processing time | Low-cost individuals that are easy to implement |
| Risk-focused | Higher weights on constraint margin and stability | Individuals with low adoption risk |
| Balance-focused | Relatively equal treatment of each objective | Individuals with few extreme weaknesses |

When the same individual remains at the top across multiple scenarios, that individual is a stable candidate with respect to changes in value judgment. Conversely, when the top individuals change substantially from scenario to scenario, confirm with stakeholders which value judgment to adopt.

## Selecting Adoption Candidates from Constrained Optimization Results

In constrained optimization, individuals with good objective function values may be infeasible. For example, an individual with high performance but exceeding the maximum weight limit, or one that slightly exceeds the cost ceiling.

In this case, first exclude individuals that violate hard constraints. Then apply MCDM to the feasible individuals, including objective function values and constraint margins as evaluation criteria.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Objective performance | Larger is better | The performance to be maximized in optimization. |
| Cost | Smaller is better | Must remain below the upper limit. |
| Constraint margin | Larger is better | Margin from upper and lower limits. |
| Stability | Larger is better | Robustness under varying conditions. |

By including constraint margin as an evaluation criterion, it becomes easier to select individuals with lower adoption risk over those that are right at the constraint boundary.

## Visualizing and Comparing Top Candidates

MCDM rankings are easier to interpret when combined with visualization rather than reviewed in a table alone.

| Visualization | What Can Be Confirmed |
| --- | --- |
| Pareto scatter plot | Where on the front the top individuals are located. |
| Parallel coordinates | Strengths and weaknesses across each objective and criterion. |
| Ranking bar chart | MCDM score differences and proximity of top candidates. |
| Weight sensitivity chart | Rank changes due to weight variation. |

In particular, if the top individuals are concentrated at the ends of the Pareto front, extreme solutions may have been selected. Also reviewing balanced solutions and knee points makes it easier to explain the decision.

## General MCDM Application Examples

### Product Selection

In product selection, multiple product candidates are compared across price, performance, maintainability, support quality, and ease of adoption.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Price | Smaller is better | Upfront and licensing costs. |
| Performance | Larger is better | Processing capacity, accuracy, response time, etc. |
| Maintainability | Larger is better | Ease of operation, modification, and extension. |
| Support quality | Larger is better | Vendor responsiveness, documentation, inquiry channels. |
| Adoption timeline | Smaller is better | Time to complete adoption. |

In product selection, choosing the cheapest option on price alone can lead to problems with performance and maintainability. Conversely, selecting on performance alone may result in high adoption costs and operational burden. MCDM makes it possible to organize multiple perspectives and explain the rationale for the selection.

### Supplier Evaluation

Supplier evaluation involves comprehensively assessing price, quality, delivery performance, supply stability, and risk.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Price | Smaller is better | Procurement cost. |
| Quality | Larger is better | Defect rate, quality audit results, etc. |
| On-time delivery rate | Larger is better | Proportion of deliveries made on time. |
| Supply stability | Larger is better | Sustained supply capacity, inventory arrangements, alternative supply capability. |
| Transaction risk | Smaller is better | Financial risk, geopolitical risk, dependency risk. |

Supplier evaluation requires consideration not only of short-term price but also of quality and supply stability. MCDM enables comparison of the balance between cost reduction and stable procurement.

### Investment Decisions

Investment decisions involve comparing profitability, risk, upfront cost, payback period, and strategic alignment.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Expected return | Larger is better | Anticipated future profit. |
| Upfront cost | Smaller is better | Expenses required at the start of the investment. |
| Payback period | Smaller is better | Time until the investment is recovered. |
| Risk | Smaller is better | Market, technical, and operational uncertainty. |
| Strategic alignment | Larger is better | Degree of alignment with business policy and long-term strategy. |

In investment decisions, qualitative strategic alignment is as important as the more easily quantified profitability. When using qualitative criteria, define scoring standards clearly and minimize variation across evaluators.

### Project Prioritization

When there are multiple project candidates, priority is determined using expected impact, implementation cost, risk, urgency, and feasibility.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Expected impact | Larger is better | Effects such as revenue increase, efficiency gains, and quality improvement. |
| Implementation cost | Smaller is better | Personnel, timeline, and budget. |
| Implementation risk | Smaller is better | Technical uncertainty and dependencies. |
| Urgency | Larger is better | Need for early response. |
| Feasibility | Larger is better | Whether execution is possible with the current organization and technology. |

In project prioritization, rather than immediately adopting the top-scored project, also check dependencies and resource constraints. MCDM is used as material for organizing candidates, and the final decision is made in conjunction with the implementation plan.

### Risk Assessment

In risk assessment, the priority of risks requiring a response is determined based on probability of occurrence, impact, detectability, countermeasure cost, and urgency.

| Criterion | Evaluation Direction | Explanation |
| --- | --- | --- |
| Probability of occurrence | Larger is more critical | Likelihood that the risk will materialize. |
| Impact | Larger is more critical | Damage or effect if the risk materializes. |
| Difficulty of detection | Larger is more critical | Harder to detect before occurrence means higher priority. |
| Countermeasure cost | Smaller is better | Cost and effort required to respond. |
| Urgency | Larger is more critical | Need for early response. |

Risk assessment differs from the usual "select a good candidate" problem in that danger level and response priority are evaluated more highly. It is therefore necessary to define evaluation directions clearly and explain to stakeholders what it means for a score to be high.

## Common Points to Note for Any Application

Regardless of the application, verify the following when using MCDM.

- Do the evaluation criteria align with the purpose of the decision?
- Can evaluation values be obtained for all alternatives?
- Is the scoring standard for qualitative evaluation clear?
- Can the rationale for the weights be explained?
- Do the top-ranked candidates satisfy the constraint conditions?
- Are the results stable under sensitivity analysis?

MCDM does not replace practical judgment. It is important to use it as a support technology for organizing multiple sources of judgment and making the basis for comparison explicit.


---

[← Chapter 11: Interpreting and Validating Individual Selection Results](11-interpretation-validation.md) | [Table of Contents](TOC.md) | [Chapter 13: Notes on Adopting MCDM →](13-adoption-notes.md)
