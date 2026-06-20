# Chapter 8: Representative MCDM Methods

## How to Choose a Method

MCDM encompasses many methods. Which one to use depends on the decision objective, the nature of the evaluation criteria, how weights are handled, what needs to be explained, and what form of ranking is required.

The concept used to evaluate alternatives differs from method to method. Some methods use a simple weighted sum, others use distance from an ideal solution, others seek a compromise solution, and still others compare preference relations between alternatives.

Representative differences are as follows.

| Aspect | Description |
| --- | --- |
| Evaluation concept | Weighted sum, distance from ideal, compromise solution, pairwise comparison, etc. |
| Input complexity | Whether only a simple decision matrix and weights are needed, or additional parameters are required. |
| Output format | Overall score, complete ranking, partial ranking, preference flows, etc. |
| Explainability | Whether the reason for the computed results can be easily explained to stakeholders. |
| Computational cost | Whether it is practical as the number of alternatives or criteria grows. |

This chapter covers AHP, TOPSIS, VIKOR, ELECTRE, PROMETHEE, and WSM / WPM, explaining the characteristics and appropriate use of each.

In the following, the number of alternatives is $m$, the number of criteria is $n$, the criterion value for alternative $i$ on criterion $j$ is $x_{ij}$, and the weight for criterion $j$ is $w_j$. Weights are normally normalized to satisfy:

$$
w_j \ge 0,\qquad \sum_{j=1}^{n} w_j = 1
$$

Normalized criterion values are denoted $r_{ij}$, and weighted normalized values are denoted $v_{ij}$.

## AHP

AHP (Analytic Hierarchy Process) is a method that organizes criteria and alternatives in a hierarchical structure and derives importance through pairwise comparisons. In MCDM, it is primarily used as a method for determining criterion weights.

The basic workflow of AHP is as follows.

```text
1. Define the decision objective
2. Organize evaluation criteria in a hierarchical structure
3. Conduct pairwise comparisons among criteria
4. Compute weights from the pairwise comparison matrix
5. Check the Consistency Ratio
6. Evaluate alternatives using the weights
```

The distinguishing feature of AHP is that rather than entering weights directly, weights are derived through comparisons—"How much more important is criterion A than criterion B?" For example, comparisons are made between cost and performance, performance and risk, and risk and delivery.

| Feature | Content |
| --- | --- |
| Primary use | Criterion weighting, hierarchical decision-making |
| Input | Pairwise comparison matrix, decision matrix |
| Output | Weights, Consistency Ratio, scores, ranking |
| Strength | Rationale for weights is easy to explain |
| Caution | Input burden grows with more criteria |

AHP allows the Consistency Ratio (CR) to verify contradictions in pairwise comparisons. For example, if one judges "A is more important than B" and "B is more important than C" but then strongly judges "C is more important than A," there may be a contradiction in the comparisons.

### AHP Computation

Let the pairwise comparison matrix for criteria be $A = (a_{jk})$. $a_{jk}$ represents "how much more important is criterion $j$ than criterion $k$," and satisfies the reciprocal relationship:

$$
a_{jj} = 1,\qquad a_{kj} = \frac{1}{a_{jk}}
$$

The weight vector $w$ is theoretically obtained as the eigenvector corresponding to the largest eigenvalue $\lambda_{\max}$:

$$
A w = \lambda_{\max} w,\qquad \sum_{j=1}^{n} w_j = 1
$$

In implementations or hand calculations, an approximation using column normalization and row averaging can also be used.

$$
b_{jk} = \frac{a_{jk}}{\sum_{\ell=1}^{n} a_{\ell k}}
$$

$$
w_j = \frac{1}{n}\sum_{k=1}^{n} b_{jk}
$$

The consistency of the pairwise comparisons is verified using the Consistency Index $CI$ and the Consistency Ratio $CR$.

$$
CI = \frac{\lambda_{\max} - n}{n - 1}
$$

$$
CR = \frac{CI}{RI}
$$

Here $RI$ is the Random Index corresponding to matrix size $n$. Generally, $CR \le 0.10$ indicates that the pairwise comparisons are consistent to an acceptable degree.

The overall score for each alternative is computed as the weighted sum of normalized criterion values $r_{ij}$ and weights $w_j$.

$$
S_i = \sum_{j=1}^{n} w_j r_{ij}
$$

Rankings are produced in descending order of $S_i$.

AHP is effective when criterion importance needs to be determined in agreement with stakeholders. When many criteria are involved, the input burden increases, so criteria should be organized before applying AHP.

## TOPSIS

TOPSIS (Technique for Order Preference by Similarity to Ideal Solution) gives high scores to alternatives that are close to the ideal solution and far from the anti-ideal solution.

In TOPSIS, for each criterion, the positive ideal solution (the collection of most desirable values) and the negative ideal solution (the collection of least desirable values) are defined. Each alternative's score is then computed based on how close it is to the positive ideal and how far it is from the negative ideal.

The basic workflow is as follows.

```text
1. Normalize the decision matrix
2. Construct the weighted normalized matrix
3. Determine the positive ideal and negative ideal solutions
4. Compute distances from each alternative to the ideal and anti-ideal solutions
5. Compute relative closeness as the score
6. Rank in descending order of score
```

| Feature | Content |
| --- | --- |
| Primary use | Selecting alternatives that are comprehensively closest to the ideal |
| Input | Decision matrix, evaluation directions, weights |
| Output | TOPSIS scores, ranking, ideal solution, anti-ideal solution |
| Strength | Scores are intuitive and easy to explain |
| Caution | Results are sensitive to the normalization method and weights |

TOPSIS scores are typically in the range [0, 1], where higher scores indicate more desirable alternatives. A high-scoring alternative is considered close to the positive ideal solution and far from the negative ideal solution.

### TOPSIS Computation

First, normalize the decision matrix $X = (x_{ij})$ by column using vector normalization.

$$
r_{ij} = \frac{x_{ij}}{\sqrt{\sum_{i=1}^{m} x_{ij}^2}}
$$

Next, construct the weighted normalized matrix.

$$
v_{ij} = w_j r_{ij}
$$

For benefit criteria (larger is better), the positive ideal $A_j^+$ and negative ideal $A_j^-$ are:

$$
A_j^+ = \max_i v_{ij},\qquad A_j^- = \min_i v_{ij}
$$

For cost criteria (smaller is better), the direction is reversed:

$$
A_j^+ = \min_i v_{ij},\qquad A_j^- = \max_i v_{ij}
$$

Compute distances from each alternative to the positive ideal and negative ideal solutions.

$$
D_i^+ = \sqrt{\sum_{j=1}^{n} (v_{ij} - A_j^+)^2}
$$

$$
D_i^- = \sqrt{\sum_{j=1}^{n} (v_{ij} - A_j^-)^2}
$$

Finally, compute the relative closeness as the TOPSIS score.

$$
C_i = \frac{D_i^-}{D_i^+ + D_i^-}
$$

$C_i$ close to 1 means close to the positive ideal solution; close to 0 means close to the negative ideal solution. Rankings are produced in descending order of $C_i$.

TOPSIS is suitable when combining multiple criteria such as performance, cost, and risk into a single score to select the overall best candidate.

## VIKOR

VIKOR seeks the compromise solution closest to the ideal in situations where multiple criteria are in conflict. When no alternative is best across all criteria, it ranks alternatives by balancing overall utility and maximum regret for the worst criterion.

VIKOR primarily uses the following three values.

| Indicator | Meaning |
| --- | --- |
| S | Sum of weighted gaps across all criteria relative to the ideal. Represents overall utility. |
| R | Weighted gap for the worst criterion. Represents maximum regret. |
| Q | Compromise index integrating S and R. Smaller is more desirable. |

The basic workflow is as follows.

```text
1. Find the best and worst values for each criterion
2. Compute S values for each alternative
3. Compute R values for each alternative
4. Normalize S and R
5. Compute Q values using the strategy parameter v
6. Rank in ascending order of Q
```

| Feature | Content |
| --- | --- |
| Primary use | Selecting a compromise solution from trade-offs |
| Input | Decision matrix, evaluation directions, weights, strategy parameter v |
| Output | S, R, Q, ranking |
| Strength | Overall utility and maximum regret can be considered separately |
| Caution | The meaning of parameter v must be understood when setting it |

The strategy parameter $v$ adjusts whether to emphasize overall utility or maximum regret for the worst criterion. With $v = 0.5$, both are treated equally.

### VIKOR Computation

First, find the best value $f_j^*$ and worst value $f_j^-$ for each criterion $j$. For benefit criteria, the maximum is the best and the minimum is the worst.

$$
f_j^* = \max_i x_{ij},\qquad f_j^- = \min_i x_{ij}
$$

For cost criteria, the minimum is the best and the maximum is the worst.

$$
f_j^* = \min_i x_{ij},\qquad f_j^- = \max_i x_{ij}
$$

For each alternative $i$, compute the normalized gap from the ideal for criterion $j$.

$$
g_{ij} = \frac{f_j^* - x_{ij}}{f_j^* - f_j^-}
$$

This formula, when $f_j^*$ and $f_j^-$ are defined to match the evaluation direction for both benefit and cost criteria, yields 0 at the best value and 1 at the worst value. When $f_j^* = f_j^-$, that criterion does not distinguish among alternatives, so $g_{ij} = 0$.

Compute $S_i$ representing overall utility and $R_i$ representing maximum regret as follows.

$$
S_i = \sum_{j=1}^{n} w_j g_{ij}
$$

$$
R_i = \max_{j} \left(w_j g_{ij}\right)
$$

Next, find the best and worst values of $S_i$ and $R_i$.

$$
S^* = \min_i S_i,\qquad S^- = \max_i S_i
$$

$$
R^* = \min_i R_i,\qquad R^- = \max_i R_i
$$

Compute the compromise index $Q_i$ using the strategy parameter $v \in [0,1]$.

$$
Q_i
= v \frac{S_i - S^*}{S^- - S^*}
+ (1-v)\frac{R_i - R^*}{R^- - R^*}
$$

Larger $v$ places more weight on overall utility $S$; smaller $v$ places more weight on maximum regret $R$. Rankings are produced in ascending order of $Q_i$.

### VIKOR Acceptance Conditions for the Compromise Solution

To accept the alternative with the minimum Q value as the compromise solution, the following two conditions should be verified.

**Condition 1 (Acceptable advantage)**: The difference between the Q value of rank 1 ($Q_{(1)}$) and rank 2 ($Q_{(2)}$) must be at least $\frac{1}{m-1}$.

$$
Q_{(2)} - Q_{(1)} \ge \frac{1}{m - 1}
$$

**Condition 2 (Acceptable stability)**: The rank-1 alternative by Q value must also be rank 1 by S or R value.

If only Condition 1 is not satisfied, the alternatives up to the highest rank satisfying $Q_{(2)} - Q_{(1)} < \frac{1}{m-1}$ are presented together as a compromise set. If only Condition 2 is not satisfied, both the Q-value rank-1 alternative and the rank-1 alternative by S or R are presented as compromise solutions. If both conditions are satisfied, the Q-value rank-1 alternative can be adopted as the compromise solution.

VIKOR is suitable when every alternative has both strengths and weaknesses, and a well-balanced compromise point is preferred over extreme optimization.

## ELECTRE

ELECTRE (ELimination Et Choix Traduisant la REalite) is an MCDM method based on outranking. Outranking refers to the concept of comparing alternatives and judging whether "one alternative is at least as desirable as another."

Rather than constructing a simple overall score, ELECTRE examines dominance relations between alternatives. It uses the degree of support and opposition from each criterion to determine whether one alternative can be considered superior to another.

The basic concept is as follows.

```text
1. Compare alternatives in pairs
2. Check the proportion of criteria that support one alternative over another
3. Check whether there is any strongly opposing criterion
4. Determine the outranking relation from the support and opposition conditions
5. Select candidates or rank based on the dominance relation
```

| Feature | Content |
| --- | --- |
| Primary use | Candidate screening, analysis of dominance relations |
| Input | Decision matrix, weights, thresholds |
| Output | Outranking relations, partial ranking, candidate set |
| Strength | More amenable to problems where simple scoring is difficult |
| Caution | Threshold setting and result interpretation can become complex |

ELECTRE is better suited for candidate screening or "excluding clearly inferior alternatives" than for producing a simple ranking from first to last. Explanations must indicate which criteria support the dominance relation and which criteria oppose it.

### ELECTRE Computation

In ELECTRE, whether alternative $a$ outranks alternative $b$ is determined using concordance and discordance measures. Here the basic ELECTRE I approach is presented.

First, define the concordance set $C(a,b)$ as the set of criteria for which $a$ is at least as desirable as $b$.

$$
C(a,b) = \{j \mid r_{aj} \ge r_{bj}\}
$$

Here $r_{ij}$ is the normalized value with direction aligned so that larger is more desirable. The concordance index is the sum of weights for criteria in the concordance set.

$$
c(a,b) = \sum_{j \in C(a,b)} w_j
$$

For criteria where $a$ is inferior to $b$, the discordance measure quantifies how strongly those criteria oppose outranking.

$$
D(a,b) = \{j \mid r_{aj} < r_{bj}\}
$$

$$
d(a,b)
= \max_{j \in D(a,b)} \frac{r_{bj} - r_{aj}}{r_j^{\max} - r_j^{\min}}
$$

Here $r_j^{\max}$ and $r_j^{\min}$ are the maximum and minimum values across all alternatives for criterion $j$. When criterion values have been pre-normalized to $[0, 1]$, the denominator is 1, simplifying to $d(a,b) = \max_{j \in D(a,b)} (r_{bj} - r_{aj})$. If $D(a,b)$ is empty, $d(a,b) = 0$.

Let $c^*$ be the concordance threshold and $d^*$ the discordance threshold. The following conditions must be satisfied for $a$ to outrank $b$:

$$
a S b \iff c(a,b) \ge c^* \quad \text{and} \quad d(a,b) \le d^*
$$

This judgment is performed for all pairs of alternatives to construct an outranking relation graph. Alternatives that outrank many others while resisting strong opposition become leading candidates.

## PROMETHEE

PROMETHEE (Preference Ranking Organisation METHod for Enrichment Evaluations) is an outranking method that computes preference degrees based on pairwise comparisons between alternatives.

In PROMETHEE, how much one alternative is preferred over another is computed for each criterion using a preference function, and the results are aggregated with weights. From these results, positive flow, negative flow, and net flow are derived.

| Indicator | Meaning |
| --- | --- |
| Positive flow | How much this alternative outperforms others. |
| Negative flow | How much this alternative is outperformed by others. |
| Net flow | Positive flow minus negative flow. |

The basic workflow is as follows.

```text
1. Compare alternatives in pairs
2. Apply the preference function for each criterion
3. Compute the aggregated preference index with weights
4. Compute positive and negative flows
5. Rank based on net flow or partial relations
```

| Feature | Content |
| --- | --- |
| Primary use | Detailed analysis of preference relations between alternatives |
| Input | Decision matrix, evaluation directions, weights, preference function, thresholds |
| Output | Positive flow, negative flow, net flow, ranking |
| Strength | Preference relations can be expressed in fine detail |
| Caution | Computational cost increases with more alternatives |

PROMETHEE I handles partial rankings, allowing incomparable relations to remain. PROMETHEE II produces a complete ranking based on net flow.

### PROMETHEE Computation

In PROMETHEE, the degree of preference for alternative $a$ over alternative $b$ is expressed by a preference function for each criterion. First, define the difference $d_j(a,b)$ for criterion $j$ aligned with the evaluation direction.

For benefit criteria:

$$
d_j(a,b) = x_{aj} - x_{bj}
$$

For cost criteria, reverse the direction of the difference:

$$
d_j(a,b) = x_{bj} - x_{aj}
$$

A representative linear preference function using preference threshold $p_j$ is defined as follows.

$$
P_j(d) =
\begin{cases}
0 & d \le 0 \\
\dfrac{d}{p_j} & 0 < d < p_j \\
1 & d \ge p_j
\end{cases}
$$

$P_j(d)$ expresses on a scale of 0 to 1 how much $a$ is preferred over $b$ on criterion $j$. Aggregating criterion-by-criterion preferences with weights yields the aggregated preference index $\pi(a,b)$.

$$
\pi(a,b) = \sum_{j=1}^{n} w_j P_j(d_j(a,b))
$$

For each alternative $a$, compute positive flow, negative flow, and net flow.

$$
\Phi^+(a) = \frac{1}{m-1}\sum_{b \ne a} \pi(a,b)
$$

$$
\Phi^-(a) = \frac{1}{m-1}\sum_{b \ne a} \pi(b,a)
$$

$$
\Phi(a) = \Phi^+(a) - \Phi^-(a)
$$

PROMETHEE I constructs a partial ranking from the relation of $\Phi^+$ and $\Phi^-$; PROMETHEE II produces a complete ranking in descending order of net flow $\Phi(a)$.

PROMETHEE is effective when not just a single overall score but also the wins and losses between alternatives and the strength of preferences need to be explained.

## WSM / WPM

WSM (Weighted Sum Model) is the most basic MCDM method: it multiplies normalized criterion values by weights and sums them.

Using direction-aligned normalized values $r_{ij}$, the WSM score is:

$$
S_i = \sum_{j=1}^{n} w_j r_{ij}
$$

Here $S_i$ is the overall score for alternative $i$. If $r_{ij}$ is normalized so that larger values are more desirable, the ranking can be produced in descending order of $S_i$.

WPM (Weighted Product Model) raises criterion values to the power of their weights and multiplies them together.

$$
S_i = \prod_{j=1}^{n} r_{ij}^{w_j}
$$

Using logarithms, this can be computed as:

$$
\log S_i = \sum_{j=1}^{n} w_j \log r_{ij}
$$

WPM requires $r_{ij} > 0$. If normalized values include zeros, the preprocessing policy must be clarified—e.g., adding a small positive value $\epsilon$, choosing an alternative formulation that permits zeros, or switching to WSM.

A comparison of WSM and WPM is as follows.

| Method | Concept | Characteristic |
| --- | --- | --- |
| WSM | Weighted additive aggregation | Simple and easy to explain |
| WPM | Weighted multiplicative aggregation | Better reflects proportional differences |

WSM is easy to implement and explain, making it effective as a baseline or for initial analysis. However, when criteria have very different units or scales, it is heavily influenced by normalization.

WPM is suitable when the ratio of criterion values is important. However, when criterion values include zero or negative values, handling becomes difficult and preprocessing is required.

## Comparison of Methods

Comparing representative MCDM methods gives the following.

| Method | Core concept | Output | Suitable for | Caution |
| --- | --- | --- | --- | --- |
| AHP | Derive weights from pairwise comparisons | Weights, scores, ranking | When rationale for weights needs to be explained | High input burden with many criteria |
| TOPSIS | Select the alternative closest to ideal and farthest from anti-ideal | Scores, ranking | When selecting the overall best alternative close to ideal | Sensitive to normalization and weights |
| VIKOR | Find compromise from overall utility and maximum regret | S, R, Q, ranking | When seeking a well-balanced compromise solution | Care needed in setting v |
| ELECTRE | Examine dominance relation from support/opposition conditions | Outranking relations | Candidate screening and exclusion | Threshold setting is complex |
| PROMETHEE | Compute flows from pairwise preferences | Flows, ranking | When preference relations need detailed explanation | Computational cost increases with more alternatives |
| WSM | Sum normalized values with weights | Scores, ranking | Simple baseline or initial analysis | Strong influence of normalization |
| WPM | Multiply normalized values with weights | Scores, ranking | When proportional evaluation is desired | Care needed with zeros and negative values |

In method selection, it is important to think not only about computational sophistication but also about how the results will be explained. If simple and explainable results are needed, WSM or TOPSIS are convenient; if compromise solutions are the priority, VIKOR is a candidate; if detailed preference relations between alternatives need to be examined, PROMETHEE or ELECTRE are options.

It is also effective to compare rankings from multiple methods rather than fixing on a single method. When the same alternative ranks highly across multiple methods, that conclusion can be considered comparatively stable. On the other hand, when rankings differ greatly by method, the evaluation criteria, weights, normalization method, and assumptions of each method need to be verified.


---

[← Chapter 7: Weighting Methods for Individual Selection](07-weighting-methods.md) | [Table of Contents](TOC.md) | [Chapter 9: Normalization of Optimization Result Data →](09-normalization.md)
