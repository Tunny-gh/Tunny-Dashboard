# Slice Chart

## Overview

The Slice Chart visualizes how the objective function changes along one parameter while all other parameters are held fixed at their current best trial values. It uses L-BFGS local optimization to find the optimal value along the sliced parameter's range.

Use this chart to understand the individual effect of each parameter, identify the optimal operating point, and explore the sensitivity of the objective to small parameter changes.

## Operations

- **Parameter selector**: Choose which parameter to slice (X axis). The remaining parameters are fixed at their best-trial values.
- **Objective selector**: Choose which objective to optimize along the slice.
- **Run**: Click Run to execute the slice optimization. The chart updates with the computed slice curve.
- **Hover**: Hover over the curve to read objective values at specific parameter values.
- **Optimal marker**: a vertical marker indicates the locally optimal parameter value found by L-BFGS.

## How to Read

- **Smooth curve with clear minimum/maximum**: the objective varies predictably with this parameter. The optimal point is reliable.
- **Flat region**: the objective is insensitive to this parameter in that range — other parameters may matter more.
- **Multiple local optima**: multiple dips/peaks indicate a complex, multimodal response. The L-BFGS optimum may be local — consider running from different starting points.
- **Steep slope**: the objective is very sensitive to small changes in this parameter. Precise control of this parameter is important.
- **Compare with Importance Chart**: if a parameter shows high importance but a flat slice, the interaction with other parameters may be driving the importance rather than the parameter's individual effect.

## L-BFGS Optimization

The Slice Chart uses L-BFGS (Limited-memory BFGS) to find the locally optimal value of the selected parameter. L-BFGS is a gradient-based quasi-Newton method that converges in 30–100 iterations for smooth objectives. See the L-BFGS theory tab for details.
