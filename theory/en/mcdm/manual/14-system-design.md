# Chapter 14: Design Points in System Implementation

## Overview of Implementation Design

When implementing MCDM in a system, it is necessary to consistently manage not only the evaluation computation logic but also input data, evaluation criteria, weights, results, logs, and recomputation conditions.

The basic flow in the system is as follows.

```text
1. Receive input data
2. Manage feasibility and the candidate set
3. Manage evaluation criteria and evaluation directions
4. Set or compute weights
5. Perform normalization and MCDM computation
6. Return scores and rankings
7. Record computation conditions and results
```

In implementation, it is important to be able to reproduce the same result from the same input conditions, to be able to identify the cause when errors occur, and to be able to explain the premises of results to users.

## Input Data Structure

MCDM input data consists of alternatives, evaluation criteria, evaluation values, evaluation directions, and weights.

A representative input structure is as follows.

| Element | Content |
| --- | --- |
| alternatives | Individual ID, trial number, name, metadata. |
| criteria | Criterion ID, name, unit, evaluation direction. |
| values | Evaluation values for alternatives × criteria. |
| weights | Weight for each criterion. |
| method | The MCDM method to use. |
| options | Additional settings such as normalization method and VIKOR's v parameter. |
| feasibility | Constraint satisfaction, feasibility, reason for exclusion. |
| candidate_set | Candidate scope such as all individuals, non-dominated set, or user-selected set. |

In implementation, evaluation values are sometimes handled as a 2D array and sometimes as row-oriented records.

```text
values[i][j] = evaluation value of alternative i for criterion j
```

During input validation, confirm that the number of alternatives, number of criteria, and size of the value array are consistent. Missing values, NaN, infinity, and weights summing to zero must also be handled explicitly.

## Managing the Candidate Set and Selection State

In systems handling multi-objective optimization results, explicitly manage the candidate set included in the MCDM computation. Whether all individuals are included, only individuals on the Pareto front, or only user-selected individuals changes the meaning of the ranking.

Information required for the candidate set is as follows.

| Attribute | Explanation |
| --- | --- |
| set_id | ID that identifies the candidate set. |
| source | Options such as all, pareto_front, filtered, or manual_selection. |
| included_ids | List of target individual IDs. |
| excluded_ids | Excluded individual IDs and the reason for exclusion. |
| constraints | Constraint conditions applied. |
| created_at | Time at which the candidate set was created. |

It is also convenient if the selection state in the MCDM ranking and the selection state in the optimization view can be synchronized in the result display. For example, selecting a top individual in the ranking table would cause the same individual to be highlighted in the Pareto scatter plot and parallel coordinates.

## Managing Evaluation Criteria and Weights

Evaluation criteria and weights are important settings that determine the meaning of MCDM results. In the system, manage evaluation criteria and weights not as mere numeric arrays but as settings with meaning.

Required attributes for evaluation criteria are as follows.

| Attribute | Explanation |
| --- | --- |
| id | ID that identifies the criterion. |
| name | Display name. |
| direction | benefit or cost. |
| unit | Unit. |
| description | Explanation of the criterion. |
| value_source | Source of evaluation values. |

Required attributes for weights are as follows.

| Attribute | Explanation |
| --- | --- |
| criterion_id | The corresponding criterion ID. |
| value | The weight value. |
| method | Manual setting, AHP, entropy method, etc. |
| reason | Reason for the weight setting. |

Weights are often normalized to sum to 1 before internal computation. However, also retaining the original values entered by the user makes it easier to confirm the intent behind the settings.

## Computation Logic

The computation logic is easier to maintain when divided into input validation, normalization, weight application, method-specific computation, and ranking generation.

```text
validate_input()
normalize_values()
apply_weights()
compute_method()
rank_results()
build_result_metadata()
```

Making method-specific computation share a common interface makes the system easier to extend.

| Process | Perspective for Sharing |
| --- | --- |
| Input validation | Size, missing values, weights, and evaluation direction checks. |
| Normalization | Make Min-Max, vector normalization, etc. switchable. |
| Method computation | Implement TOPSIS, VIKOR, PROMETHEE, etc. as independent implementations. |
| Ranking | Define ascending, descending, and tie-handling per method. |
| Metadata | Record conditions used, computation time, and warnings. |

In the computation logic, make edge cases explicit. Examples include criteria where all alternatives have the same value, criteria where the column norm becomes 0, and alternatives that contain NaN.

## Result Display

In MCDM result display, showing not only rankings but also the basis for scores is important.

Information that should be displayed includes the following.

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

An example ranking table is as follows.

| Rank | Individual | Score | Main Strengths | Main Weaknesses |
| ---: | --- | ---: | --- | --- |
| 1 | A | 0.82 | Risk, Maintainability | Cost |
| 2 | B | 0.78 | Performance | Delivery time |
| 3 | C | 0.61 | Cost | Performance |

It is desirable to display the reason for the rank as a breakdown by criterion so that users can interpret the results.

In individual selection, integration with visualization is also important beyond the ranking table. By linking with Pareto scatter plots, parallel coordinates, objective function distributions, and weight sensitivity charts, it becomes possible to confirm what tradeoffs the top individuals possess.

## Logging and Auditability

Because MCDM results are used for decision-making, computation conditions and results must be recorded. Ensuring auditability makes it possible to verify afterwards "why this rank was obtained."

Log information to record is as follows.

| Information | Explanation |
| --- | --- |
| input_hash | Identifier for the input data. |
| criteria_version | Version of the criterion definition. |
| weights | Weights used. |
| normalization | Normalization method. |
| method | MCDM method used. |
| options | Method-specific parameters. |
| candidate_set | Candidate set construction conditions and target individual IDs. |
| feasibility_filter | Exclusion conditions for constraint-violating or infeasible individuals. |
| warnings | Warnings such as missing values, outliers, and division by zero. |
| result | Scores, ranking, and execution time. |

Include in the log all settings that affected the computation. Saving only the ranking does not ensure reproducibility.

## Recomputation and Version Management

MCDM results require recomputation when input data or settings change. In the system, make it explicit which changes necessitate recomputation.

Changes that require recomputation are as follows.

- Evaluation values changed
- Alternatives were added or removed
- The candidate set or constraint filter was changed
- Evaluation criteria were added, removed, or changed
- Evaluation direction was changed
- Weights were changed
- Normalization method was changed
- MCDM method or method parameters were changed

Conversely, changes that do not affect the computation results themselves — such as display order, color, or filter display — may not require recomputation.

In version management, save evaluation criterion definitions, weight settings, and computation results in association with one another. This makes it possible to reproduce past decisions and to compare the effects of condition changes afterwards.

In system implementation of MCDM, it is important to include not only computation accuracy but also reproducibility, explainability, and change management in the design.


---

[← Chapter 13: Notes on Adopting MCDM](13-adoption-notes.md) | [Table of Contents](TOC.md) | [Chapter 15: Appendix →](15-appendix.md)
