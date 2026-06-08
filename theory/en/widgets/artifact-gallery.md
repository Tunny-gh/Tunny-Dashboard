# Artifact Gallery

The Artifact Gallery displays the files stored under the study's `artifacts/`
folder (loaded with the **Artifacts** toolbar button), associating each file
with its trial and with analysis results.

## View modes

- **All** — every trial that has artifacts, shown as a paginated thumbnail grid
  (sorted by trial ID).
- **By Cluster** — runs k-means clustering with the settings you choose
  (k / target space / selection mode / init) and groups the artifacts into a
  collapsible section per cluster.
- **By MCDM Rank** — runs the selected MCDM method and lists the artifacts in
  ranking order (top-N), with the rank and score shown on each card.

When a trial has multiple artifacts, the **Artifact #** selector chooses which
one (0-based) is shown per trial, so each trial contributes a single card.
Trials that don't have an artifact at the selected index are skipped.

Image files (PNG / JPEG / GIF / WEBP) are shown as thumbnails; clicking a
thumbnail opens a large preview in a modal (close with the button, Esc, or by
clicking outside). Other files
(CSV, etc.) show an icon and an **Open** button that opens the file with the
system default application. Clicking a card highlights the corresponding trial
across the other widgets. Each card also shows the trial's objective values so
you can judge the result without opening the file.

## Comparing settings (A vs B)

Clustering and MCDM results are cached per setting. To compare, for example,
`k = 3` against `k = 5`, place **two** Artifact Gallery widgets on the canvas
and give each its own setting — they read the result for their own setting from
the shared cache, so both configurations can be viewed side by side. A setting
already computed by another Clustering / MCDM widget is reused instantly; an
uncomputed setting can be run from the gallery itself with **Run**.
