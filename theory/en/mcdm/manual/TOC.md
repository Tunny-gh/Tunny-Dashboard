# MCDM Technical Manual: Individual Selection from Multi-Objective Optimization Results — Table of Contents

## 1. [Introduction](./01-introduction.md)

- Purpose of this document
- Overview of MCDM
- Target audience
- Term definitions
- How to read this document

## 2. [Basic Concepts of MCDM](./02-basic-concepts.md)

- What is multi-criteria decision making?
- Difference from single-criterion evaluation
- Relationship between alternatives, evaluation criteria, and weights
- Situations where MCDM is effective

## 3. [Positioning of Multi-Objective Optimization and MCDM](./03-multi-objective-context.md)

- What multi-objective optimization yields
- The difference between exploration and selection
- Relationship between objective functions, constraints, and evaluation criteria
- When to apply MCDM
- What MCDM decides and what it does not decide

## 4. [Selecting Candidates from the Pareto Front](./04-pareto-front-selection.md)

- Pareto dominance and non-dominated solutions
- Why the Pareto front is used as the candidate set
- When to include dominated solutions
- Knee points, balanced solutions, and extreme solutions
- Relationship between Pareto dominance and MCDM rankings

## 5. [MCDM Process for Multi-Objective Optimization Results](./05-process.md)

- Problem formulation
- Definition of individuals, trials, and candidate solutions
- Setting evaluation criteria
- Preparing optimization result data
- Weighting
- Evaluation and ranking
- Interpreting top candidates

## 6. [Evaluation Criteria Design for Individual Selection](./06-criteria-design.md)

- Correspondence between objective functions and evaluation criteria
- Quantitative and qualitative criteria
- Benefit-type and cost-type criteria
- Constraints and feasibility
- Independence of evaluation criteria
- Notes on criteria design

## 7. [Weighting Methods for Individual Selection](./07-weighting-methods.md)

- Equal weights
- Scenario-based weights
- Expert-judgment weighting
- AHP-based weighting
- Entropy method
- Validating weights

## 8. [Representative MCDM Methods](./08-methods.md)

- AHP
- TOPSIS
- VIKOR
- ELECTRE
- PROMETHEE
- WSM / WPM
- Comparison of methods

## 9. [Normalizing Optimization Result Data](./09-normalization.md)

- Why normalization is needed
- Min-Max normalization
- Vector normalization
- Handling cost-type criteria
- The effect of outliers and extreme solutions
- How the normalization method affects results

## 10. [Individual Ranking Calculation Flow](./10-calculation-flow.md)

- Building the decision matrix
- Filtering the candidate set
- Weighted decision matrix
- Score computation
- Building the ranking
- Comparing the top-N individuals
- Worked example

## 11. [Interpreting and Validating Individual Selection Results](./11-interpretation-validation.md)

- How to read ranking results
- Cases where first place is not directly adopted
- Sensitivity analysis
- Checking the impact of weight changes
- Checking for outliers and bias
- Recording selection rationale

## 12. [Application Examples for Optimization Results](./12-use-cases.md)

- Selecting a balanced solution from the Pareto front
- Selecting based on performance-focused, cost-focused, or risk-focused scenarios
- Selecting adoption candidates from constrained optimization results
- Visualizing and comparing top candidates
- General MCDM application examples

## 13. [Notes on Adopting MCDM](./13-adoption-notes.md)

- Bias in evaluation criteria
- Handling subjective judgment
- The impact of data quality
- Ensuring explainability
- Building consensus among stakeholders

## 14. [Design Points in System Implementation](./14-system-design.md)

- Input data structure
- Managing evaluation criteria and weights
- Managing the candidate set and selection state
- Computation logic
- Result display and visualization integration
- Logging and auditability
- Recomputation and version management

## 15. [Appendix](./15-appendix.md)

- Formula reference
- Glossary
- Sample data
- References
- Method selection guide
