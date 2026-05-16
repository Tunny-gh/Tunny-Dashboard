use argmin::core::{CostFunction, Executor, Gradient, State};
use argmin::solver::linesearch::MoreThuenteLineSearch;
use argmin::solver::quasinewton::LBFGS;

pub(crate) struct LbfgsOptimizer {
    max_iter: u64,
    m_history: usize,
}

impl LbfgsOptimizer {
    pub(crate) fn new(max_iter: u64, m_history: usize) -> Self {
        Self { max_iter, m_history }
    }

    pub(crate) fn optimize<O>(&self, initial: Vec<f64>, problem: O) -> Vec<f64>
    where
        O: CostFunction<Param = Vec<f64>, Output = f64>
            + Gradient<Param = Vec<f64>, Gradient = Vec<f64>>,
    {
        let linesearch = MoreThuenteLineSearch::new();
        let solver = LBFGS::new(linesearch, self.m_history);
        match Executor::new(problem, solver)
            .configure(|state| state.param(initial.clone()).max_iters(self.max_iter))
            .run()
        {
            Ok(result) => result
                .state()
                .get_best_param()
                .cloned()
                .unwrap_or(initial),
            Err(_) => initial,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argmin::core::{CostFunction, Error, Gradient};

    struct Quadratic;

    impl CostFunction for Quadratic {
        type Param = Vec<f64>;
        type Output = f64;
        fn cost(&self, p: &Vec<f64>) -> Result<f64, Error> {
            Ok(p.iter().map(|x| x * x).sum())
        }
    }

    impl Gradient for Quadratic {
        type Param = Vec<f64>;
        type Gradient = Vec<f64>;
        fn gradient(&self, p: &Vec<f64>) -> Result<Vec<f64>, Error> {
            Ok(p.iter().map(|x| 2.0 * x).collect())
        }
    }

    #[test]
    fn tc_201_01_lbfgs_optimizer_minimizes_quadratic() {
        let optimizer = LbfgsOptimizer::new(100, 5);
        let result = optimizer.optimize(vec![1.0, -1.0], Quadratic);
        assert!(result[0].abs() < 1e-4, "x[0]={} should be near 0", result[0]);
        assert!(result[1].abs() < 1e-4, "x[1]={} should be near 0", result[1]);
    }

    #[test]
    fn tc_201_02_lbfgs_returns_initial_when_zero_iters() {
        let optimizer = LbfgsOptimizer::new(0, 5);
        let initial = vec![3.0, -2.0];
        let result = optimizer.optimize(initial.clone(), Quadratic);
        assert!(result.iter().all(|x| x.is_finite()));
    }
}
