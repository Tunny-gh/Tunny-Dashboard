use argmin::core::{CostFunction, Error, Gradient};

use super::likelihood::log_ml_with_gradient;
use crate::optimization::LbfgsOptimizer;

struct GpNegLml {
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
}

impl CostFunction for GpNegLml {
    type Param = Vec<f64>;
    type Output = f64;
    fn cost(&self, params: &Vec<f64>) -> Result<f64, Error> {
        let (lml, _) = log_ml_with_gradient(&self.x, &self.y, params);
        Ok(if lml.is_finite() { -lml } else { f64::MAX })
    }
}

impl Gradient for GpNegLml {
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;
    fn gradient(&self, params: &Vec<f64>) -> Result<Vec<f64>, Error> {
        let (_, grad) = log_ml_with_gradient(&self.x, &self.y, params);
        Ok(grad.iter().map(|g| -g).collect())
    }
}

/// Optimize GP hyperparameters via argmin L-BFGS.
pub(super) fn optimize_hyperparams(
    x: &[Vec<f64>],
    y: &[f64],
    n_iter: usize,
    m_history: usize,
) -> (Vec<f64>, usize) {
    if x.is_empty() {
        return (vec![], 0);
    }
    let ndim = x[0].len();
    let mut initial = vec![0.0; ndim + 2];
    initial[ndim + 1] = -2.0;

    let problem = GpNegLml {
        x: x.to_vec(),
        y: y.to_vec(),
    };
    let optimizer = LbfgsOptimizer::new(n_iter as u64, m_history);
    let params = optimizer.optimize(initial, problem);
    (params, n_iter)
}
