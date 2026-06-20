# Chapter 3: Multi-Objective Optimization and the Role of MCDM

## What Multi-Objective Optimization Produces

Multi-objective optimization is a method for searching candidate individuals, trials, and design proposals while simultaneously handling multiple objective functions. Rather than converging to a single best value as in single-objective optimization, it typically yields multiple candidate solutions that have mutual trade-offs.

For example, when the goal is to maximize performance while minimizing cost and risk, individuals with high performance tend to have higher cost, while individuals with low cost may have insufficient performance. In such situations, there is generally no individual that is simultaneously best in all objectives.

The important output of multi-objective optimization is not a single answer but a set of selection candidates. MCDM is used to choose, from this candidate set, the individual that best matches the decision-maker's value judgments.

## The Difference Between Search and Selection

Multi-objective optimization and MCDM serve different roles.

| Item | Multi-objective optimization | MCDM |
| --- | --- | --- |
| Primary role | Search for good candidate solutions | Compare and select candidate solutions |
| Input | Search space, objective functions, constraints | Candidate set, evaluation criteria, weights, preferences |
| Output | Set of individuals, Pareto front, objective function values | Scores, rankings, selection rationale |
| Nature of judgment | Algorithmic search | Reflection of decision-maker's value judgments |

In the search phase, it is important to find as diverse and promising a set of individuals as possible. In the selection phase, it is important to choose, in an explainable manner, the individuals that are adoptable and aligned with the objectives.

## Relationship Between Objective Functions, Constraints, and Evaluation Criteria

The objective functions used in optimization can often serve as evaluation criteria in MCDM. However, objective functions and evaluation criteria are not always the same. Objective functions are indicators used to guide the search, and they do not necessarily cover all perspectives needed for the final adoption decision.

| Type of information | Treatment in MCDM | Example |
| --- | --- | --- |
| Objective functions | Used as evaluation criteria | Accuracy, cost, processing time, weight |
| Constraints | Treated as a pre-filter or as evaluation criteria | Maximum cost, minimum strength, allowable risk |
| Derived indicators | Added as additional criteria where needed | Stability, margin, ease of implementation |
| Meta-information | Used for explanation and traceability | Trial ID, generation, search conditions |

Individuals that violate constraints are generally excluded before MCDM. However, when comparing the degree of constraint violation, or when selecting individuals with large margins within an allowable range, the constraint margin or violation amount may be treated as an evaluation criterion.

## When to Apply MCDM

MCDM is typically applied after the optimization search. A representative workflow is as follows.

```text
1. Run multi-objective optimization
2. Obtain objective function values and constraint status for each individual
3. Exclude ineligible individuals
4. Construct the Pareto front or candidate set
5. Set evaluation criteria, evaluation directions, and weights
6. Compute scores and rankings using MCDM
7. Visualize the top candidates and make the final selection
```

MCDM scores may also be used during the search, but in that case they influence the search behavior of the optimization algorithm. This document primarily assumes the scenario of applying MCDM to individual selection after the search is complete.

## What MCDM Determines and What It Does Not

MCDM is a tool for making individual selection easier to explain. The top-ranked individual in MCDM is not always the one that should be adopted.

What MCDM can help organize includes the following.

- Comparing multiple objectives on a common basis
- Reflecting the relative importance of each objective through weights
- Explaining why top candidates scored highly, using scores and contributions
- Checking how rankings change when weights are varied
- Narrowing down the candidate set to individuals useful for decision-making

On the other hand, there are also things that MCDM alone cannot determine.

- Practical constraints not included in the objective functions
- Risks not captured in the criterion values
- The validity of data acquisition timing and search conditions
- Which trade-offs stakeholders are willing to accept

Therefore, MCDM results are treated not as the final decision itself but as an intermediate result that makes the final decision explainable.


---

[← Chapter 2: Basic Concepts of MCDM](02-basic-concepts.md) | [Table of Contents](TOC.md) | [Chapter 4: Candidate Selection from the Pareto Front →](04-pareto-front-selection.md)
