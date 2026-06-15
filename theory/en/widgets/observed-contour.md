# Observed Contour

Observed Contour interpolates a contour (colormap) from the **observed trials only**.
It uses no surrogate model, so — unlike PDP or surrogate response surfaces — it never shows
a model's extrapolation into regions with no data. **Regions without data are left blank.**

## Features

- X / Y / Value (color) can each be **a parameter or an objective**. Putting objectives on the
  axes draws an empirically-grounded trade-off surface (e.g. obj1 × obj2 → obj3).
- Values come from **Delaunay triangulation + linear interpolation** of the observed points,
  and everything **outside the convex hull is masked**.
- The **Coverage** slider drops large triangles that would bridge far-apart points
  (smaller = mask more aggressively / less extrapolation, larger = fill more).
- Observed trial points are overlaid in color (Show points).
- Toggle **Contours** (iso-lines) and a **Log color** scale (positive ranges only).
- The **3D** view draws a surface with the value as height. Masked regions stay as holes — no
  extrapolation here either. Turn on **Density shade** to darken cells with few observed points,
  countering the false confidence a glossy 3D surface can invite.
- In the 2D view, click an observed point to inspect that trial's details.

## How to read it

- Blank regions mean "no data" — they are not model estimates.
- For constrained studies, **Feasible only** interpolates from feasible trials only.

## Difference from other tools

- **PDP**: fits a surrogate and marginalizes other variables — an averaged effect with model
  assumptions and extrapolation.
- **Observed Contour**: no model, interpolation of observed points only, no extrapolation —
  for seeing what was actually observed.
