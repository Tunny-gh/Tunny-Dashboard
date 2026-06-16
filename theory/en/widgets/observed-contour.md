# Observed Contour

Observed Contour draws how a value is distributed over two axes using **only the trials you
actually ran** (the observed points), shown as a contour / colormap. It is for inspecting the
real measured values smoothly joined together — not a model prediction — and it **leaves regions
with no data blank** instead of filling them in.

## What it tells you

- At a glance, see **where a value (color) is high or low** across two axes (X and Y).
- Both the axes and the color can be **either a parameter or an objective**, so you can draw,
  for example, how a third objective behaves over the combination of two other objectives — an
  **empirically grounded trade-off**.
- Blank areas mean "**not yet explored**." They are not model-filled estimates, so you can read
  them honestly as "no data here."

## How to read it

- **Color**: encodes the value, per the colorbar on the right. The colorbar shows max / mid / min
  numeric ticks plus the **name of the value** it represents (the legend title).
- **Blank**: no observed points there (or trimmed away by Coverage, below).
- **Dots**: the overlaid circles are the actual trials, colored with the same colormap.
- The subtitle "Interpolated from observed trials; blank = no data" is a reminder that the figure
  interpolates observed points and does not extrapolate.

## Controls

- **X / Y / Value**: choose the columns for the horizontal axis, vertical axis, and color
  (pick from numeric parameters and objectives).
- **Coverage**: how far apart points may be before the surface between them is dropped. Smaller
  masks sparse regions more strictly (less extrapolation); larger fills wider.
- **Show points**: toggle the overlay of observed trial points.
- **Contours** (2D): overlay iso-lines to make value levels easier to see.
- **Log color** (2D): use a logarithmic color scale (only when all values are positive).
- **3D**: show the value as a height surface. Masked (blank) regions stay as holes — no
  extrapolation here either.
- **Density shade** (3D): fade out cells with few nearby observations, countering the false
  confidence a glossy 3D surface can invite.
- **Feasible only**: in a constrained study, use only feasible (constraint-satisfying) trials.

## Trial details

In the 2D view, **click** an overlaid observed point to inspect that trial's details (its
parameters, objective values, and so on).

## How to read it carefully

- Blank regions mean "no data" — **they are not model predictions.**
- The fewer nearby observations, the less reliable the interpolation (visualized by Density shade
  in 3D).
- Selecting the same column for both X and Y is degenerate, so a warning is shown.

## Difference from other charts (vs PDP)

- **PDP**: fits a surrogate model and shows an averaged effect with the other variables
  marginalized out. It assumes a model and **extrapolates** into regions with no data.
- **Observed Contour**: uses no model, interpolates observed points only, and **does not
  extrapolate** — for checking what was actually observed.
