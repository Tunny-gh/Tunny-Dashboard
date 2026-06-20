# Chapter 7: Weighting Methods for Individual Selection

## The Role of Weighting

Weighting is the step of expressing the relative importance of each evaluation criterion as a numerical value. In MCDM, even with the same decision matrix, the overall score or ranking can change when weights change. For this reason, weights are not merely computational parameters—they are an important assumption representing the value judgments of the decision-maker.

For example, when selecting an individual from multi-objective optimization results, prioritizing "performance" versus prioritizing "cost" or "risk" may lead to different conclusions even when evaluating the same Pareto front. MCDM makes this difference in priorities explicit through weights.

The basic form of weights is a vector as follows.

| Criterion | Weight |
| --- | ---: |
| Cost | 0.30 |
| Performance | 0.40 |
| Risk | 0.20 |
| Delivery | 0.10 |

Most MCDM methods normalize weights so that they sum to 1.

$$
\sum_{j=1}^{n} w_j = 1
$$

Here $w_j$ is the weight for criterion $j$ and $n$ is the number of criteria. When the weights sum to 1, each weight can be interpreted as the proportion of contribution to the overall evaluation.

## Scenario-Based Weights

In individual selection, comparing multiple weight scenarios is more effective than drawing conclusions from a single set of weights. Because multi-objective optimization results involve trade-offs, the top-ranked individual can change depending on which value judgment is adopted.

Representative weight scenarios are as follows.

| Scenario | Weighting approach | Suitable for |
| --- | --- | --- |
| Equal weights | Treat all objectives equally | Ranking as a baseline |
| Performance-focused | Increase weights for performance and quality | Identifying high-performance individuals |
| Cost-focused | Increase weights for cost, time, resource consumption | Identifying easy-to-implement individuals |
| Risk-focused | Prioritize risk, constraint margin, stability | Identifying safe, easy-to-adopt candidates |
| Balance-focused | Adjust weights to avoid the worst criteria | Identifying individuals with few extreme weaknesses |

When the same individual ranks highly across multiple scenarios, that individual can be considered a comparatively stable candidate. Conversely, when first place changes with only a small shift in weights, it is necessary to examine not just the ranking but also score differences and trade-off content.

## Equal Weights

Equal weights treat all evaluation criteria as equally important. When there are $n$ criteria, the weight for each criterion is as follows.

$$
w_j = \frac{1}{n}
$$

For example, with four criteria, each has a weight of 0.25.

| Criterion | Weight |
| --- | ---: |
| Cost | 0.25 |
| Performance | 0.25 |
| Risk | 0.25 |
| Delivery | 0.25 |

Equal weights are useful when the basis for weighting has not yet been determined, or when a neutral evaluation is desired as an initial analysis. They can also serve as a baseline for comparison with other weighting methods.

However, equal weights place the explicit assumption that "all evaluation criteria are equally important." While this appears neutral, it is not necessarily appropriate in practice. For example, whether it is appropriate to treat cost and safety as equally important depends on the decision objective.

When using equal weights, verify the following.

- Is there no clear difference in priority among criteria?
- Is this for initial analysis or for a final decision?
- Is the equal-weight result substantially different from results under other weight settings?
- Can equal importance for all criteria be explained?

## Expert Judgment Weighting

Expert judgment weighting is a method in which decision-makers, business staff, engineers, or domain experts set the relative importance of criteria directly. It is easy to reflect the practical objectives and constraints of the decision, and is the most intuitively usable method.

For example, importance is set directly as follows.

| Criterion | Rationale for importance | Weight |
| --- | --- | ---: |
| Cost | Budget constraints are strong, so weighted highly | 0.35 |
| Performance | Directly tied to final quality, so weighted most | 0.40 |
| Risk | Accounts for impact if problems occur | 0.15 |
| Delivery | Important but lower priority than other criteria | 0.10 |

Expert judgment easily reflects practical value judgments, but it also tends to be subjective. When strongly dependent on the experience or interests of a particular person, weights may become skewed.

When using expert judgment, the following approaches are effective.

- Have multiple stakeholders review the weights
- Record the rationale for weights in writing
- Confirm that no weights are extremely large or small
- Check how rankings change when weights are varied
- Compare with equal weights or entropy-based weights

Expert judgment is effective as a method for reflecting the decision-maker's values. However, if the rationale for the weights cannot be explained, the explainability of the ranking results also diminishes.

## AHP Weighting

AHP (Analytic Hierarchy Process) is a method that derives weights by conducting pairwise comparisons among criteria. Rather than directly entering weights, comparisons such as "How much more important is cost than performance?" or "How much more important is risk than delivery?" are used to determine relative importance between criteria.

In AHP, it is common to use Saaty's 1–9 scale to express importance.

| Value | Meaning |
| ---: | --- |
| 1 | Equally important |
| 3 | Slightly more important |
| 5 | Considerably more important |
| 7 | Very strongly more important |
| 9 | Extremely more important |
| 2, 4, 6, 8 | Intermediate values |

Pairwise comparison results are expressed as a matrix such as the following.

|  | Cost | Performance | Risk |
| --- | ---: | ---: | ---: |
| Cost | 1 | 1/3 | 2 |
| Performance | 3 | 1 | 4 |
| Risk | 1/2 | 1/4 | 1 |

From this matrix, the priority vector—that is, the weights—for each criterion is computed. The advantage of AHP is that judging relative importance between criteria is often easier than assigning weights directly. In addition, whether the comparisons are consistent can be verified using the Consistency Ratio (CR).

Generally, a CR of 0.10 or below indicates acceptable consistency in the pairwise comparisons. When the CR is high, there may be contradictions in the comparison judgments, so the input should be revisited.

AHP is suitable in the following situations.

- The number of criteria is small to moderate
- It is difficult to determine importance directly as a numerical value
- The rationale for weights needs to be explained to stakeholders
- Consistency of judgments needs to be verified

On the other hand, as the number of criteria increases, so does the number of pairwise comparisons. For $n$ criteria, the number of comparisons is:

$$
\frac{n(n-1)}{2}
$$

The greater the number of criteria, the greater the input burden and the harder it becomes to maintain consistency. For this reason, when using AHP it is important to reduce the criteria to a necessary and sufficient number.

## Entropy Weight Method

The Entropy Weight Method derives weights objectively based on the variance in the evaluation data. Rather than having the decision-maker set weights directly, it uses the information content or discriminative power of each criterion to compute weights.

The basic idea is that a criterion with larger value differences across alternatives holds more information for distinguishing among alternatives. Conversely, a criterion that has nearly the same value for all alternatives is less able to explain differences between alternatives, and therefore receives a smaller weight.

The Entropy Weight Method proceeds as follows.

```text
1. Convert criterion values to non-negative values
2. Proportionally normalize the values for each criterion
3. Compute entropy for each criterion
4. Compute diversity
5. Normalize diversity to obtain weights
```

Letting $e_j$ be the entropy for criterion $j$, the diversity $d_j$ is expressed as follows.

First, compute the proportionally normalized value $p_{ij}$ for alternative $i$ on criterion $j$.

$$
p_{ij} = \frac{x_{ij}}{\sum_{i=1}^{m} x_{ij}}
$$

When $p_{ij} = 0$, define $p_{ij} \ln p_{ij} = 0$. The entropy $e_j$ for criterion $j$ is computed as:

$$
e_j = -\frac{1}{\ln m} \sum_{i=1}^{m} p_{ij} \ln p_{ij}
$$

$e_j$ falls within the range $[0, 1]$. The diversity $d_j$ is expressed as:

$$
d_j = 1 - e_j
$$

Normalizing diversity gives the weight $w_j$.

$$
w_j = \frac{d_j}{\sum_{k=1}^{n} d_k}
$$

Criteria with larger diversity are considered to have greater discriminative power among alternatives, and receive larger weights.

The advantage of the Entropy Weight Method is that it avoids subjective weight setting. It is effective when there is a large amount of evaluation data and weights need to be determined mechanically from data variance.

On the other hand, the Entropy Weight Method carries the assumption that "criteria with greater variance are more important." This is not always correct. For example, even a safety indicator that is critically important in practice will receive a small weight under the Entropy Weight Method if all alternatives have nearly the same value.

The Entropy Weight Method is suitable in the following situations.

- Subjective weight setting is to be avoided
- Sufficient evaluation data is available
- The discriminative power of each criterion should be reflected in the weights
- Weights are needed for initial analysis or comparison

Even when using the Entropy Weight Method, it is necessary to confirm that the derived weights are consistent with the decision objective.

## Validating Weight Appropriateness

After setting weights, verify their appropriateness. Since MCDM ranking results depend strongly on weights, it is important not to stop at the point of deciding weights but to examine their effect on results.

When verifying weight appropriateness, confirm the following.

| Aspect | What to confirm |
| --- | --- |
| Sum | Do the weights sum to 1? |
| Non-negativity | Are there any negative weights? |
| Rationale | Can the reason for each weight be explained? |
| Extremity | Is any particular criterion's weight excessively large? |
| Stability | Would a small change in weights cause large disruption to the ranking? |
| Consensus | Do stakeholders understand and agree with the weighting approach? |

Sensitivity analysis is particularly important. In sensitivity analysis, one checks how much the ranking changes when weights are varied slightly. If the ranking changes significantly with a small weight change, the conclusion is unstable.

For example, the following checks are useful.

- Increase and decrease the weight of the most important criterion
- Compare with equal weights
- Compare expert judgment weights with entropy weights
- Determine the range of weights in which the top candidates change order
- Check the ranking when a particular criterion is excluded

When explaining weight appropriateness, present not only the final weights but also the method used to set them, the rationale, and the results of sensitivity analysis. This makes it easier to explain what assumptions the ranking results are based on.

## Guidelines for Choosing a Weighting Method

The weighting method should be selected based on the decision objective and the available information.

| Method | Suitable situations | Key cautions |
| --- | --- | --- |
| Equal weights | Initial analysis, baseline, no clear priority difference | Verify whether the equal-importance assumption is appropriate |
| Expert judgment | When practical value judgments should be reflected directly | Be attentive to subjectivity and accountability |
| AHP | When pairwise comparison is used to organize importance | Input burden grows with more criteria |
| Entropy Weight Method | When weights are to be derived objectively from data | Note the assumption that variance equals importance |

In practice, it is effective to compare results under multiple weight settings rather than relying on a single weighting method. For example, compare rankings under equal weights, expert judgment, and the Entropy Weight Method, and confirm whether the conclusions are stable.

The purpose of weighting is not to produce the numerically most precise values in the computation. It is to make value judgments explicit and to make ranking results explainable. Therefore, regardless of which weighting method is used, it is important to record the meaning and rationale of the weights.


---

[← Chapter 6: Evaluation Criteria Design for Individual Selection](06-criteria-design.md) | [Table of Contents](TOC.md) | [Chapter 8: Representative MCDM Methods →](08-methods.md)
