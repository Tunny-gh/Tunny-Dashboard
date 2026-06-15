# Response Surface

The response surface visualizes the predicted objective over two selected parameters as a 3D surface, with the objective on the vertical axis. The surface is produced by the same surrogate engine as the Optimizer, and the trained model is shared between them.

## Features

- Displays a 3D response surface with parameters on the X / Y axes and the predicted objective on the vertical axis.
- Choose the surrogate model (Ridge / GP (FITC, VFE, MoE) / LightGBM, or Auto via cross-validation). Ridge yields a flat plane; GP and LightGBM yield curved surfaces.
- For Gaussian Process models, the predictive uncertainty can be overlaid as a 95% confidence band (±1.96σ).
- Other parameters are held at the best-observed point to form the surface slice.
- Shows the cross-validated R² (CV R²) from model validation.

## Integration with the Optimizer

The surrogate model fitted here is cached and reused directly by the Optimizer (surrogate optimization), and vice versa. No re-fitting is needed for the same objective and model.
