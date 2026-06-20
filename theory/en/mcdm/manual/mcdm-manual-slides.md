---
marp: true
theme: default
paginate: true
math: mathjax
title: MCDM for Individual Selection from Multi-Objective Optimization Results
description: Manual slides based on the MCDM technical document
footer: "MCDM Manual @Copyright 2026 hrntsm"
---

<style>
:root {
  --tunny-blue: #4f7fe8;
  --tunny-blue-soft: #9ec3f2;
  --tunny-blue-pale: #eaf3ff;
  --tunny-navy: #07096f;
  --tunny-ink: #101216;
  --tunny-muted: #5d6068;
  --tunny-line: #d8e4f4;
  --tunny-green: #c8f7d8;
  --tunny-yellow: #fff09a;
}

section {
  background: #fff;
  color: var(--tunny-ink);
  font-family: "Inter", "Helvetica Neue", Arial, sans-serif;
  font-size: 26px;
  letter-spacing: 0;
  line-break: strict;
  overflow-wrap: anywhere;
  padding: 42px 76px 52px;
}

section::before {
  content: "";
  position: absolute;
  inset: 0 0 auto 0;
  height: 28px;
  background: var(--tunny-blue-soft);
  border-bottom: 2px solid rgba(79, 127, 232, 0.35);
}

section::after {
  color: rgba(16, 18, 22, 0.42);
  font-size: 16px;
  font-weight: 700;
  right: 38px;
  bottom: 28px;
}

h1 {
  color: var(--tunny-ink);
  font-size: 42px;
  font-weight: 900;
  line-height: 1.14;
  margin: 0 0 18px;
  max-width: 100%;
  overflow-wrap: anywhere;
  word-break: normal;
}

h1 strong,
h2 strong,
strong {
  font-weight: 900;
}

h1::first-letter {
  color: var(--tunny-blue);
}

h2 {
  color: var(--tunny-ink);
  font-size: 28px;
  font-weight: 850;
  line-height: 1.18;
  margin: 22px 0 12px;
}

h3 {
  color: var(--tunny-muted);
  font-size: 24px;
  font-weight: 800;
  margin: 22px 0 10px;
}

p {
  color: var(--tunny-muted);
  line-height: 1.42;
  margin: 0 0 14px;
  max-width: 100%;
  overflow-wrap: anywhere;
}

ul,
ol {
  color: var(--tunny-muted);
  line-height: 1.34;
  margin: 10px 0 0 1.05em;
  padding: 0;
}

li {
  margin: 6px 0;
  overflow-wrap: anywhere;
}

li::marker {
  color: var(--tunny-blue);
  font-weight: 900;
}

section table {
  border-collapse: separate;
  border-spacing: 0;
  display: inline-table;
  width: auto !important;
  max-width: 100%;
  table-layout: auto;
  margin: 14px 0 8px;
  font-size: 20px;
  line-height: 1.2;
  overflow: hidden;
  border: 1px solid var(--tunny-line);
  border-radius: 8px;
}

th {
  background: var(--tunny-blue-pale);
  color: var(--tunny-ink);
  font-weight: 850;
}

td {
  color: var(--tunny-muted);
}

th,
td {
  border: 0;
  border-bottom: 1px solid var(--tunny-line);
  padding: 7px 11px;
  vertical-align: top;
  overflow-wrap: anywhere;
}

tr:last-child td {
  border-bottom: 0;
}

blockquote,
pre {
  border: 1px solid var(--tunny-line);
  border-radius: 8px;
  background: #f7fbff;
}

pre {
  padding: 14px 18px;
  font-size: 20px;
  line-height: 1.24;
}

code {
  color: var(--tunny-navy);
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
}

section > h1:first-child + h2 {
  color: var(--tunny-blue);
  font-size: 24px;
  font-weight: 900;
  line-height: 1.25;
  margin-top: -8px;
  margin-bottom: 18px;
}

section:first-of-type {
  padding: 72px 76px 64px;
}

section:first-of-type::before {
  height: 54px;
  background: var(--tunny-blue-soft);
}

section:first-of-type h1 {
  font-size: 62px;
  max-width: 1080px;
}

section:first-of-type h1::first-letter {
  color: var(--tunny-ink);
}

section:first-of-type h1::after {
  content: "";
  display: block;
  width: 126px;
  height: 10px;
  margin-top: 24px;
  border-radius: 999px;
  background: var(--tunny-blue);
}

section:first-of-type p {
  max-width: 1120px;
  font-size: 26px;
  line-break: strict;
  overflow-wrap: normal;
  text-wrap: pretty;
  word-break: keep-all;
}

section:first-of-type::after {
  content: "MCDM Manual";
  color: var(--tunny-navy);
  font-size: 20px;
  font-weight: 900;
  right: 54px;
  bottom: 42px;
}

section.lead {
  text-align: left;
}

mark {
  background: linear-gradient(transparent 62%, rgba(79, 127, 232, 0.32) 62%);
  color: inherit;
  padding: 0 0.04em;
}

.columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 42px;
}

section.section-title {
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 78px 86px 70px;
  background: var(--tunny-navy);
}

section.section-title::before {
  height: 54px;
  background: var(--tunny-blue);
  border-bottom: 2px solid rgba(255, 255, 255, 0.18);
}

section.section-title h1 {
  color: #fff;
  font-size: 64px;
  line-height: 1.08;
  max-width: 1040px;
  margin-bottom: 24px;
}

section.section-title h1::first-letter {
  color: #fff;
}

section.section-title h1::after {
  content: "";
  display: block;
  width: 140px;
  height: 10px;
  margin-top: 28px;
  border-radius: 999px;
  background: var(--tunny-blue-soft);
}

section.section-title h2 {
  color: var(--tunny-blue-soft);
  font-size: 30px;
  font-weight: 900;
  margin: 0 0 16px;
}

section.section-title p {
  color: rgba(255, 255, 255, 0.72);
  max-width: 920px;
  font-size: 28px;
}
</style>

# MCDM for Individual Selection from Multi-Objective Optimization Results

## Technical Manual Slides

An approach and implementation guide for selecting explainable adoption candidates from multi-objective exploration results

---

# Purpose of These Slides

## Connecting Exploration Results to Selection Decisions

- Understand the flow for comparing and ranking individuals, trials, and candidate solutions obtained through multi-objective optimization using MCDM
- Organize the effects of evaluation criteria, weights, normalization, and method selection on ranking results
- Treat rankings not as the final decision themselves, but as explainable decision-making material
- Confirm the input validation, result display, logging, and recomputation conditions required during implementation

---

<!-- _class: section-title -->

# 1. Overview of MCDM

## What is compared and what is explained?

First, organize the role of MCDM and how it differs from single-criterion evaluation.

---

# What Is MCDM?

## A framework for comparing candidates on multiple criteria

MCDM (Multi-Criteria Decision Making) is a methodology for comparing, evaluating, and ranking alternatives by simultaneously considering multiple evaluation criteria.

| Element | Meaning in multi-objective optimization results |
| ---------- | ---------------------------------- |
| Alternative | Individual, trial, candidate solution, design candidate |
| Evaluation criterion | Objective function, constraint margin, risk, stability |
| Evaluation value | The value an individual holds for each criterion |
| Weight | Relative importance of each evaluation criterion |
| Ranking | Candidate order based on overall evaluation |

---

# Is MCDM Automatic?

## Settings are required

- Which criteria were used
- Which criteria were prioritized
- Which normalization method was used
- Which method was used to produce the ranking
- Which premises the top candidates depend on

These must be made explicit — MCDM is a support technology for making decisions explainable.

---

# Difference from Single-Criterion Evaluation

## Handling tradeoffs is the essential difference

| Aspect | Single-criterion evaluation | MCDM |
| ------------------- | ------------------ | ---------------------- |
| Evaluation criteria (objective functions) | 1 | Multiple |
| Comparison method | Directly compare values | Integrate and compare multiple criteria |
| Main challenge | Judging magnitude of values | Handling tradeoffs |
| Weighting | Usually not required | Reflects importance of criteria |
| What is explained | Why that value is good | What was prioritized and by how much |

In MCDM, how evaluation criteria are chosen and how weights are set determine the meaning of the results.

---

# Basic Structure

## Combining candidates, criteria, weights, and method

In MCDM, candidates are arranged in a table and placed in a form where each candidate can be compared on the same evaluation criteria.

| Element | Role | Example |
| -------- | ------------ | ------------------------------- |
| Alternative | Subject of comparison | Individual A, Individual B, Individual C |
| Evaluation criterion | Aspect of comparison | Cost, Performance, Risk |
| Evaluation direction | Desirability of value | Smaller is better / Larger is better |
| Weight | Importance of criterion | Performance 0.40, Cost 0.30 |
| Method | Approach to integration | WSM, TOPSIS, VIKOR |

Even for the same optimization results, rankings change when weights, normalization, or method change.

---

# Roles of Multi-Objective Optimization and MCDM

## Think of exploration and selection as separate processes

| Item | Multi-objective optimization | MCDM |
| ---------- | -------------------------------------- | ------------------------------ |
| Main role | Explore good candidate solutions | Compare and select candidate solutions |
| Input | Search space, objective functions, constraints | Candidate set, evaluation criteria, weights, preferences |
| Output | Individual set, Pareto front, objective function values | Scores, rankings, selection rationale |
| Nature of judgment | Algorithm-driven exploration | Reflection of decision maker's value judgment |

Optimization is the process of creating candidates; MCDM is the process of selecting from those candidates.

---

# Application Timing

## Flow

This manual primarily covers the use of MCDM for individual selection after exploration.

```text
1. Run multi-objective optimization
2. Obtain objective function values and constraint status for each individual
3. Build the Pareto front or candidate set
4. Set evaluation criteria, evaluation directions, and weights
5. Compute scores and rankings with MCDM
6. Visualize top candidates and make the final selection
```

---

# What MCDM Decides and Does Not Decide

## What it decides

- Compare multiple objectives on the same basis
- Reflect the importance of each objective in weights
- Explain the reason for top candidates through scores and contributions
- Confirm rank changes when weights are varied

## What it cannot decide

- Practical constraints not included in the objective functions
- Risks that do not appear in evaluation values
- Which tradeoffs stakeholders will accept

---

<!-- _class: section-title -->

# 2. Building the Candidate Set

## Define the comparison targets from the Pareto front

Before ranking, clearly specify which individuals will be subject to MCDM evaluation.

---

# Pareto Dominance and Non-Dominated Solutions

## First, build the foundation for the candidate set

Condition under which individual $a$ dominates individual $b$ in a minimization problem:

$$
f_j(a) \le f_j(b) \quad \forall j
$$

And for at least one objective $k$:

$$
f_k(a) < f_k(b)
$$

Individuals not dominated by any other individual are called non-dominated solutions; their set is called the Pareto front.

---

# Using the Pareto Front as the Candidate Set

## Start comparisons from undominated candidates

A dominated individual is at least as poor as some other individual on every objective. Therefore, the basic approach in MCDM is to use individuals on the Pareto front as the candidate set.

| Situation | Response |
| ------------------------ | ----------------------------------- |
| Many individuals on the front | Filtering, clustering, extracting top N |
| Many extreme solutions | Also check knee points and balanced solutions separately |
| Individuals at constraint boundary | Add constraint margin as an evaluation criterion |
| Noise from approximate exploration | Also check score gaps and re-evaluation |

---

# Including Dominated Solutions

## Exclude as a rule, but keep them as candidates when there is a reason

Dominated individuals can in principle be excluded. However, they may be kept as candidates in the following situations.

- There is measurement error or simulation error and the difference in objective function values is small
- Individuals on the Pareto front may not satisfy practical constraints
- An individual excels on a qualitative criterion not included in the objective functions
- A wider comparison of intermediate candidates is desired during exploration

Record the reason for inclusion, and be prepared to explain it if such an individual ranks at the top.

---

# Knee Points, Balanced Solutions, and Extreme Solutions

## Identifying the character of top candidates

| Type | Explanation | Meaning for Selection |
| ---------- | -------------------------------------------- | -------------------------- |
| Knee point | The point on the Pareto front where exchange efficiency begins to worsen | Balanced and easy to adopt |
| Balanced solution | A point with few extreme weaknesses across multiple objectives | Easy to explain |
| Extreme solution | A point that excels greatly on a specific objective | Suits performance-focused or cost-focused scenarios |

The MCDM method and weights determine which type of candidate tends to rank at the top.

---

# Safe Order for Individual Selection

## Proceed in the order: exclude, candidate, ranking, interpretation

```text
1. Exclude individuals that cannot be adopted
2. Confirm Pareto dominance relations
3. Use the non-dominated set as the candidate set
4. Rank candidates with MCDM
5. Visualize top candidates and confirm tradeoffs
6. Record the reason for the final selection
```

Pareto dominance is superiority/inferiority by objective functions only; the MCDM ranking is a comprehensive evaluation including weights, normalization, and additional criteria.

---

<!-- _class: section-title -->

# 3. Process and Criteria Design

## Build the premises that determine the meaning of results

Organize the problem formulation, candidate solutions, evaluation criteria, and constraints in order.

---

# Overview of the MCDM Process

## Organizing premises before computation determines the quality of results

```text
1. Problem formulation
2. Definition of individuals, trials, and candidate solutions
3. Organizing constraints and the candidate set
4. Setting evaluation criteria
5. Preparing optimization result data
6. Weighting
7. Evaluation and ranking
8. Interpreting top candidates
```

This order is a structure for making the premises of the decision explicit step by step.

---

# What the Problem Formulation Decides

## Articulate in advance what counts as a good candidate

| Item | Content to Verify |
| -------------- | -------------------------------------- |
| Decision purpose | Select adoption candidates, rank, or compare |
| Usage context | Which operations, analyses, or decisions will use this |
| Constraints | Conditions that must be satisfied or exclusion conditions |
| Decision maker | Whose judgment or preferences are being reflected |
| How results will be used | Adopt the top-ranked, or narrow down to top-N candidates |

Articulate "what counts as good" before computation.

---

# The Role of Evaluation Criteria Design

## Criteria are the meaning of the ranking itself

Evaluation criteria are the very premises of MCDM. Without appropriate criteria, even a sophisticated method will not yield results useful for decision-making.

| Aspect | Content to Verify |
| ---------- | ---------------------------------- |
| Relevance | Is it directly related to the purpose of the decision? |
| Measurability | Can it be expressed as a number or a consistent scale? |
| Comparability | Can it be compared under the same conditions and with the same meaning? |
| Independence | Does it not overlap excessively with other criteria? |
| Explainability | Can the reason for using this criterion be explained? |

---

# Objective Functions and Evaluation Criteria

## The optimization objective functions alone may not be sufficient

| Optimization-side information | Handling in MCDM | Example |
| -------------- | -------------------------- | -------------------------- |
| Objective function | Use as evaluation criterion | Accuracy, cost, processing time |
| Constraint | Pre-filter or evaluation criterion | Maximum cost, minimum strength |
| Constraint margin | Additional evaluation criterion | Margin from limit, violation amount |
| Exploration metadata | Explanatory/tracking information | Trial ID, generation, parameters |
| Practical perspectives | Add as needed | Stability, reproducibility, ease of implementation |

The optimization objective functions do not necessarily cover every perspective needed for the final adoption decision.

---

# Quantitative and Qualitative Criteria

## Separate ease of quantification from practical importance

| Type | Example | Note |
| -------- | -------------------------- | -------------------------- |
| Quantitative criterion | Price, processing time, accuracy, profit | Align units and measurement conditions |
| Qualitative criterion | Usability, maintainability, ease of adoption | Clarify the scoring standard |

When quantifying qualitative criteria, make the meaning of scores concrete to reduce variation across evaluators.

Example: For a 5-point maintainability scale, define in advance what "5," "4," and "3" each mean.

---

# Benefit-Type and Cost-Type

## Larger is not always better

| Evaluation direction | Meaning | Example |
| -------------- | ---------------------- | ------------------------ |
| Benefit-type | Larger values are more desirable | Performance, profit, accuracy, quality |
| Cost-type | Smaller values are more desirable | Price, time, risk, loss |

If the evaluation direction is wrong, alternatives with low cost or low risk will be evaluated unfavorably — fundamentally distorting the results.

Record the evaluation direction for each criterion when defining it.

---

# Constraints and Feasibility

## Decide how to handle infeasible candidates before ranking

| Type | Explanation | Recommended Handling |
| ---------- | -------------------- | ------------------------ |
| Hard constraint | Violation makes adoption impossible | Exclude before MCDM |
| Soft constraint | More desirable when satisfied | Treat as evaluation criterion |
| Constraint violation amount | Magnitude of violation | Exclude or penalty criterion |
| Constraint margin | Margin from limits | Safety-side evaluation criterion |

When including constraint-violating individuals, display feasibility explicitly in the result view.

---

# Independence of Evaluation Criteria

## Do not evaluate the same aspect twice

Including multiple criteria with similar meanings effectively increases the weight of that aspect.

| Criterion 1 | Criterion 2 | Possibility of Overlap |
| -------- | ---------- | ------------------------------ |
| Processing time | Processing speed | Express the same performance characteristic in opposite directions |
| Upfront cost | Adoption cost | Nearly the same cost item |
| Error rate | Reliability | Reliability is being evaluated through error rate |
| Maintainability | Operational burden | Operational burden is part of maintainability |

When overlap is found, consider integration, deletion, or weight adjustment.

---

<!-- _class: section-title -->

# 4. Weighting and Method Selection

## Quantify value judgments and choose the integration approach

Confirm how to determine weights and the differences between representative MCDM methods.

---

# The Role of Weighting

## Weights represent the decision maker's value judgment

Weights represent the relative importance of each evaluation criterion.

$$
\sum_{j=1}^{n} w_j = 1
$$

| Evaluation criterion | Weight |
| -------- | ---: |
| Cost | 0.30 |
| Performance | 0.40 |
| Risk | 0.20 |
| Delivery time | 0.10 |

Weights are not merely computational parameters — they are the value judgment of the decision maker.

---

# Scenario-Based Weights

## Compare value judgments across multiple patterns

| Scenario | Weight Approach | Suitable for Checking |
| ------------ | ------------------------------ | ---------------------- |
| Equal weights | Treat all objectives similarly | Rank as baseline |
| Performance-focused | Prioritize performance and quality | High-performance individuals |
| Cost-focused | Prioritize cost, time, resource consumption | Easy-to-implement individuals |
| Risk-focused | Prioritize risk, constraint margin, stability | Safe-side candidates |
| Balance-focused | Avoid the worst criterion | Individuals with few extreme weaknesses |

Candidates that remain at the top across multiple scenarios are comparatively stable with respect to changes in value judgment.

---

# Weighting Methods

## Choose based on whether subjective judgment, consensus, or data takes priority

| Method | Well Suited For | Main Caution |
| -------------- | ------------------------------ | -------------------------------- |
| Equal weights | Initial analysis, baseline | Assumes all criteria are equally important |
| Expert judgment | Reflecting operational value judgment | Subjectivity and accountability |
| AHP | Organizing importance through pairwise comparisons | High input burden with many criteria |
| Entropy method | Deriving weights objectively from data | Assumes variance equals importance |

Whatever method is used, record the meaning and rationale of the weights.

---

# AHP-Based Weighting

## Build the rationale for weights through pairwise comparisons

AHP derives weights by comparing evaluation criteria pairwise and computing weights from those comparison results.

```text
1. Define the purpose of the decision
2. Organize evaluation criteria in a hierarchical structure
3. Compare evaluation criteria pairwise
4. Compute weights from the pairwise comparison matrix
5. Verify the consistency ratio
6. Evaluate alternatives using the weights
```

Generally, a consistency ratio $CR \le 0.10$ indicates that comparisons are acceptably consistent.

---

# Entropy Method

## Derive weights from data variance

The entropy method derives weights based on the variance in evaluation data.

```text
1. Convert evaluation values to non-negative values
2. Normalize each criterion's values proportionally
3. Compute entropy for each criterion
4. Compute diversity
5. Normalize diversity to obtain weights
```

Criteria with greater variance are considered to carry more information for distinguishing alternatives. However, this does not necessarily align with practical importance.

---

# Representative MCDM Methods

## Each method has a different conception of "a good candidate"

| Method | Pronunciation | Main Approach | Suited For |
| --------- | ---------- | ------------------------------------ | ---------------------- |
| AHP | — | Derive weights from pairwise comparisons | Explaining the rationale for weights |
| TOPSIS | — | Select the alternative closest to the ideal and farthest from the anti-ideal | Overall closest to ideal |
| VIKOR | — | Select compromise solution from overall utility and maximum regret | Well-balanced compromise solution |
| ELECTRE | — | Examine dominance relations from concordance and discordance | Selecting or excluding candidates |
| PROMETHEE | — | Compute flows from pairwise preferences | Explaining preference relations |
| WSM/WPM | — | Weighted sum / weighted product | Simple baseline or ratio evaluation |

---

# TOPSIS

## Select candidates closest to the ideal and farthest from the anti-ideal

TOPSIS ranks alternatives that are close to the ideal solution and far from the anti-ideal solution highly.

```text
1. Normalize the decision matrix
2. Build the weighted normalized matrix
3. Determine the positive ideal and negative ideal solutions
4. Compute distances to the ideal and anti-ideal solutions
5. Compute relative closeness as the score
6. Rank by score descending
```

The TOPSIS score is typically in the range [0, 1]; higher is a more desirable candidate.

---

# VIKOR

## Select a compromise solution balancing overall utility and maximum regret

VIKOR is a method for finding the compromise solution closest to the ideal.

| Index | Meaning |
| ---- | ---------------------------------------- |
| S | Sum of weighted gaps across all evaluation criteria |
| R | Weighted gap at the worst evaluation criterion |
| Q | Compromise index integrating S and R |

The strategy parameter $v$ adjusts whether overall utility or maximum regret is emphasized. At $v = 0.5$ both are weighted equally.

---

# PROMETHEE / ELECTRE

## PROMETHEE

- Outranking method based on pairwise comparisons between alternatives
- Expresses preference relations through positive flow, negative flow, and net flow
- PROMETHEE II produces a complete ranking in descending order of net flow

## ELECTRE

- Determines whether one alternative outperforms another based on concordance and discordance
- Better suited to selecting or excluding candidates than a simple overall score
- Threshold setting and result interpretation tend to become complex

---

# WSM

## Weighted Sum Method

$$
S_i = \sum_{j=1}^{n} w_j r_{ij}
$$

- Easy to implement and explain
- Effective as an initial analysis or baseline
- Affected by the normalization method

---

# WPM

## Weighted Product Method

$$
S_i = \prod_{j=1}^{n} r_{ij}^{w_j}
$$

- Well suited to reflecting ratio-based differences
- Requires $r_{ij} > 0$

---

# How to Choose a Method

## Choose based on what you want to explain, not computational sophistication

| Purpose | Recommended Method | Reason |
| ------------------------------ | --------- | ------------------------ |
| Simply produce an overall score first | WSM | Easy to implement and explain |
| Select candidates close to the ideal | TOPSIS | Easy to explain in terms of distance |
| Select a compromise solution | VIKOR | Balances overall utility and maximum regret |
| Organize the rationale for weights | AHP | Pairwise comparisons and consistency check |
| Examine preference relations in detail | PROMETHEE | Analyze wins and losses between alternatives |
| Select or exclude candidates | ELECTRE | Narrowing based on dominance relations |

---

<!-- _class: section-title -->

# 5. Normalization and Ranking Computation

## Align different scales to produce scores

Convert evaluation values into a comparable form and produce weighted scores and rankings.

---

# Why Normalization Is Needed

## Align differences in units and scale

Because each evaluation criterion has its own units and value range, summing or computing distances across raw values causes criteria with larger scales to have excessive influence.

| Reason | Explanation |
| -------------- | ---------------------------------------- |
| Different units | Comparing currency, seconds, percentages, and scores |
| Different scales | Preventing criteria with large value ranges from dominating |
| Different evaluation directions | Aligning larger-is-better with smaller-is-better |
| Method prerequisites | Satisfying requirements of TOPSIS, WSM, etc. |
| Explainability | Making the contribution of each criterion easier to explain |

---

# Min-Max Normalization

## Convert to [0, 1] for intuitive comparison

Benefit-type:

$$
r_{ij} = \frac{x_{ij} - x_j^{\min}}{x_j^{\max} - x_j^{\min}}
$$

Cost-type:

$$
r_{ij} = \frac{x_j^{\max} - x_{ij}}{x_j^{\max} - x_j^{\min}}
$$

Results fall within [0, 1] and are intuitively interpretable, but sensitive to outliers.

---

# Vector Normalization

## Normalization well suited to distance calculation

$$
r_{ij} = \frac{x_{ij}}{\sqrt{\sum_{i=1}^{m} x_{ij}^2}}
$$

Each criterion column is divided by its Euclidean norm.

- Well matched to distance calculations such as TOPSIS
- Normalized values are not necessarily intuitive achievement rates
- For negative values and cost-type criteria, the handling of ideal and anti-ideal solutions must be made explicit according to the method's definition

---

# The Effect of Outliers and Normalization Method

## Normalization is preprocessing that changes rankings

| Effect | Explanation |
| ---------------- | ------------------------------------------ |
| Outlier sensitivity | Min-Max depends strongly on the maximum and minimum values |
| Distance interpretation | Vector normalization is well matched to distance calculations |
| Ratio handling | WPM handles ratio differences well but requires care with zeros and negative values |
| Score range | [0, 1] is easy to explain but the meaning differs by method |
| Ranking stability | Changing normalization method can cause ranks to change |

Confirm that the top candidates remain stable when the normalization method is changed.

---

# Individual Ranking Calculation Flow

## Make the process from decision matrix to ranking reproducible

```text
1. Build the candidate set from optimization results
2. Exclude infeasible individuals
3. Build the decision matrix
4. Define the evaluation directions
5. Normalize the evaluation values
6. Apply weights
7. Compute scores according to the MCDM method
8. Build the ranking based on scores or preference relations
9. Compare the top-N individuals and review the results
```

---

# Example Decision Matrix

## An input table where overall evaluation is not yet possible

| Individual | Cost | Performance | Risk |
| ---- | -----: | ---: | -----: |
| A | 100 | 70 | 20 |
| B | 120 | 90 | 40 |
| C | 80 | 60 | 30 |

Overall superiority must not be judged at this stage.

- Cost and risk are more desirable when smaller
- Performance is more desirable when larger
- Units and value ranges differ
- Normalization and weighting are required

---

# Normalized Decision Matrix

## Align all criteria to the same direction

After Min-Max normalization, all criteria are treated as "larger is better."

| Alternative | Cost | Performance | Risk |
| ------ | -----: | ---: | -----: |
| A | 0.50 | 0.33 | 1.00 |
| B | 0.00 | 1.00 | 0.00 |
| C | 1.00 | 0.00 | 0.50 |

| Criterion | Evaluation Direction | Weight |
| -------- | -------------- | ---: |
| Cost | Smaller is better | 0.30 |
| Performance | Larger is better | 0.50 |
| Risk | Smaller is better | 0.20 |

---

# Weighted Decision Matrix and Scores

## Sum the contributions of each criterion

$$
v_{ij} = w_j r_{ij}
$$

| Alternative | Cost 0.30 | Performance 0.50 | Risk 0.20 | Overall Score |
| ------ | ----------: | --------: | ----------: | ---------: |
| A | 0.15 | 0.17 | 0.20 | 0.52 |
| B | 0.00 | 0.50 | 0.00 | 0.50 |
| C | 0.30 | 0.00 | 0.10 | 0.40 |

A excels in overall balance, B in performance, C in cost.

---

<!-- _class: section-title -->

# 6. Interpretation, Validation, and Application Examples

## Do not adopt first place directly — verify the premises

Organize how to read rankings, conduct sensitivity analyses, and apply results in practice.

---

# How to Read Rankings

## Look not only at the rank but also at the gap and the rationale

An MCDM ranking is a relative order based on evaluation criteria, weights, normalization method, and computation method.

| Check Item | Content |
| -------- | -------------------------------- |
| Rank | Which alternatives are at the top and bottom? |
| Score gap | Is the gap between top-ranked alternatives large enough? |
| Contributing criteria | Which evaluation criteria influenced the rank? |
| Preconditions | Weights, normalization method, evaluation directions |
| Constraints | Do the top-ranked alternatives satisfy mandatory requirements? |

---

# Cases Where First Place Is Not Directly Adopted

## Treat top candidates as subjects for final verification

| Situation | Response |
| -------------------------- | -------------------------------- |
| Small score gap between first and second | Treat as equivalent; conduct additional comparison |
| First place is at constraint boundary | Also review candidates with larger constraint margins |
| First place is an extreme solution | Keep balanced solutions and knee points as candidates too |
| Ranks change with weight variation | Identify individuals stable across multiple scenarios |
| Unevaluated practical risks exist | Conduct additional evaluation before the final decision |

Treat rankings as "results that narrow candidates for detailed review."

---

# Sensitivity Analysis

## Change the premises to verify the stability of conclusions

Sensitivity analysis checks how much ranking results change when weights or evaluation values are varied.

| Method | Explanation |
| -------------- | --------------------------------- |
| Weight variation | Increase or decrease the weight on a specific criterion |
| Equal-weight comparison | Observe the effect of the weight settings |
| Method comparison | Compare TOPSIS, VIKOR, WSM, etc. |
| Normalization method comparison | Compare Min-Max with vector normalization, etc. |
| Criterion exclusion | Observe rank changes when a specific criterion is excluded |

The main goal is to confirm the stability of top candidates.

---

# Example: Explaining Weight Changes

## Compare the top-ranked alternative across scenarios

| Scenario | Weight Characteristics | 1st Place | Notes |
| ---------- | ---------------- | --- | ------------------ |
| Baseline | Standard weights | A | Good overall balance |
| Cost-focused | Increased cost weight | C | Low-cost alternative rises to top |
| Performance-focused | Increased performance weight | B | High-performance alternative rises |
| Risk-focused | Increased risk weight | A | Low-risk alternative maintained |

Showing rank changes due to weight variation makes it possible to explain which value judgment the final decision depends on.

---

# Application Example: Selecting a Balanced Solution

## Find candidates that are easy to adopt rather than extreme solutions

Assume optimization results that maximize accuracy while minimizing cost and processing time.

| Criterion | Evaluation Direction | Explanation |
| -------- | -------------- | ---------------------- |
| Accuracy | Larger is better | Performance of the model or design |
| Cost | Smaller is better | Expenses required for implementation and operation |
| Processing time | Smaller is better | Time required for execution |
| Constraint margin | Larger is better | Safety margin after adoption |

Using TOPSIS or VIKOR makes it easier to find individuals close to the ideal solution or with low maximum regret.

---

# Application Example: Weight Scenarios

## Different priorities change the top candidates

| Scenario | Weight Characteristics | Expected Top Candidates |
| ------------ | ---------------------------------- | ------------------------ |
| Performance-focused | Higher weights on accuracy and quality | High-performance but high-cost individuals |
| Cost-focused | Higher weights on cost and processing time | Low-cost individuals easy to implement |
| Risk-focused | Higher weights on constraint margin and stability | Individuals with low adoption risk |
| Balance-focused | Relatively equal treatment of each objective | Individuals with few extreme weaknesses |

When top individuals change substantially from scenario to scenario, reach consensus with stakeholders on which value judgment to adopt.

---

# Visualizing and Comparing Top Candidates

## See tradeoffs in position and shape, not just in tables

| Visualization | What Can Be Confirmed |
| ---------------- | ---------------------------------- |
| Pareto scatter plot | Where the top individuals are on the front |
| Parallel coordinates | Strengths and weaknesses across each objective and criterion |
| Ranking bar chart | MCDM score differences and proximity of top candidates |
| Weight sensitivity chart | Rank changes due to weight variation |

If top individuals are concentrated at the ends of the Pareto front, extreme solutions may have been selected.

---

<!-- _class: section-title -->

# 7. Adoption and System Implementation

## Build explainability and reproducibility into operations

Covers consensus building for sustained MCDM use, input structure, logging, and recomputation conditions.

---

# Notes on Adoption

## Verify operational premises before choosing a method

| Item | Content to Verify |
| -------- | ------------------------------------ |
| Purpose | What to select, compare, or rank |
| Alternatives | Same exploration results or same-condition candidates? |
| Candidate set | All individuals, non-dominated set, top candidates, etc. |
| Evaluation criteria | Are the perspectives necessary for the decision included? |
| Data | Is source, unit, and quality clear? |
| Weights | Whose value judgment is being reflected? |
| Usage | Final decision, or narrowing down candidates? |

---

# Ensuring Explainability

## Be able to explain afterwards why that candidate was chosen

Content to explain to stakeholders:

- Why were those evaluation criteria used?
- Where were the evaluation values obtained from?
- How were the weights determined?
- Which MCDM method was used?
- Why did that individual rank highly?
- Where on the Pareto front is that individual located?
- Does the individual satisfy the constraint conditions and is it adoptable?
- Is the conclusion stable even when weights or methods are changed?

---

# Building Consensus

## Reach consensus on purpose, candidates, criteria, and weights in order

```text
1. Reach consensus on the purpose of the decision
2. Reach consensus on the scope of alternatives
3. Reach consensus on how to construct the Pareto front or candidate set
4. Reach consensus on evaluation criteria
5. Reach consensus on how evaluation values are obtained
6. Reach consensus on how to approach weights
7. Reach consensus on how results will be used
```

When opinions on weights are divided, rather than forcing a single set, compare results across multiple weight scenarios.

---

# Overview of System Implementation

## Design not only computation but also condition management

```text
1. Receive input data
2. Manage feasibility and the candidate set
3. Manage evaluation criteria and evaluation directions
4. Set or compute weights
5. Perform normalization and MCDM computation
6. Return scores and rankings
7. Record computation conditions and results
```

It is important to be able to reproduce the same result from the same input conditions, to identify the cause when errors occur, and to be able to explain the premises of results.

---

# Input Data Structure

## Hold candidates, criteria, values, and settings consistently

| Element | Content |
| ------------- | ---------------------------------------- |
| alternatives | Individual ID, trial number, name, metadata |
| criteria | Criterion ID, name, unit, evaluation direction |
| values | Evaluation values for alternatives × criteria |
| weights | Weight for each criterion |
| method | The MCDM method to use |
| options | Normalization method, VIKOR's v, etc. |
| feasibility | Constraint satisfaction, feasibility, reason for exclusion |
| candidate_set | All individuals, non-dominated set, user-selected set |

---

# Dividing Computation Logic

## Separate validation, normalization, method computation, and result construction

```text
validate_input()
normalize_values()
apply_weights()
compute_method()
rank_results()
build_result_metadata()
```

| Process | Perspective for Sharing |
| ---------- | -------------------------------------- |
| Input validation | Size, missing values, weights, evaluation directions |
| Normalization | Make Min-Max, vector normalization, etc. switchable |
| Method computation | Implement TOPSIS, VIKOR, PROMETHEE, etc. independently |
| Ranking | Ascending, descending, and tie handling |
| Metadata | Conditions used, computation time, warnings |

---

# Result Display

## Make the reason for rankings visible

Information to display:

- Overall ranking
- Overall score or method-specific index
- Values per evaluation criterion
- Normalized values
- Weights
- Score contributions
- Method used and normalization method
- Warnings and cautions
- Constraint status and candidate set construction conditions
- Position on the Pareto front

Integration with visualization is as important as the ranking table.

---

# Logging and Auditability

## Enable reproduction of the same result afterwards

| Information | Explanation |
| ------------------ | ---------------------------- |
| input_hash | Identifier for the input data |
| criteria_version | Version of the criterion definition |
| weights | Weights used |
| normalization | Normalization method |
| method | MCDM method used |
| options | Method-specific parameters |
| candidate_set | Candidate set construction conditions |
| feasibility_filter | Exclusion conditions |
| warnings | Missing values, outliers, division by zero, etc. |
| result | Scores, ranking, and execution time |

Saving only the ranking does not ensure reproducibility.

---

# Changes That Require Recomputation

## Detect configuration changes that affect rankings

- Evaluation values changed
- Alternatives were added or removed
- The candidate set or constraint filter was changed
- Evaluation criteria were added, removed, or changed
- Evaluation direction was changed
- Weights were changed
- Normalization method was changed
- MCDM method or method parameters were changed

Changes that do not affect computation results — such as display order and color — may not require recomputation.

---

# Input Validation During Implementation

## Stop conditions that would break computation before they run

| Check Item | Content |
| -------------- | ------------------------------------ |
| Input size | Number of alternatives, criteria, and value arrays match |
| Candidate set | Handling of infeasible and constraint-violating individuals |
| Evaluation direction | benefit / cost is set |
| Weights | Non-negative and sum is not zero |
| Missing values | Policy for imputation, exclusion, or error |
| Division by zero | Cases where max equals min, etc. |
| Ranking direction | Ascending or descending per method |
| Metadata | Normalization method, weights, method, execution time |

---

# Summary

## MCDM supports explainable selection

- MCDM connects multi-objective optimization exploration results to explainable individual selection
- Build the candidate set by verifying constraint conditions and Pareto dominance relations
- Evaluation criteria, evaluation directions, weights, and normalization method determine the meaning of rankings
- Choose the method based on "what you want to explain"
- Do not mechanically adopt first place — verify with top candidates, sensitivity analysis, and visualization
- In implementation, include reproducibility, logging, auditability, and recomputation conditions in the design

---

# Appendix: Representative Formulas

## Key formulas for reference during implementation and explanation

| Content | Formula |
| --------------------- | ------------------------------------------------ |
| Weight normalization | $\sum_{j=1}^{n} w_j = 1$ |
| Weighted sum score | $S_i = \sum_{j=1}^{n} w_j r_{ij}$ |
| TOPSIS score | $\mathrm{score}_i = \frac{D_i^-}{D_i^+ + D_i^-}$ |
| PROMETHEE net flow | $\Phi^{net}(i) = \Phi^{+}(i) - \Phi^{-}(i)$ |

VIKOR:

$$
Q_i = v \cdot \frac{S_i - S^*}{S^- - S^*} + (1 - v) \cdot \frac{R_i - R^*}{R^- - R^*}
$$
