use crate::{MetricChronoError, Result};

/// Crossover constant `kappa_{p,q}` for axis-factorised vs unified comparators.
///
/// ```text
/// kappa = (1/p) * (q/p)^(q/p) * |1 - p/q|^(1 - p/q)
/// ```
///
/// Defined for `q > p`; returns `f64::INFINITY` when `q <= p` (unified
/// dominance regime).
pub fn kappa_pq(p: f64, q: f64) -> f64 {
    if !p.is_finite() || p <= 0.0 || !q.is_finite() || q <= 0.0 {
        return f64::NAN;
    }
    if q <= p {
        return f64::INFINITY;
    }
    let qp = q / p;
    let ratio = 1.0 - p / q;
    (1.0 / p) * qp.powf(qp) * ratio.powf(ratio)
}

/// Asymptotic threshold for the axis-factorised comparator under generalised
/// Gaussian noise.
///
/// `eps_0 = sigma * (ln(n / alpha))^(1/q)`
pub fn factored_threshold(sigma: f64, n: usize, alpha: f64, q: f64) -> Result<f64> {
    ensure_crossover_params(sigma, n, alpha, q)?;
    Ok(sigma * (n as f64 / alpha).ln().powf(1.0 / q))
}

/// Asymptotic threshold for the unified (L_p) comparator under generalised
/// Gaussian noise.
///
/// `eps = sigma * kappa^{-1/p} * n^{1/p} * (ln(1/alpha))^{1/(pq)}`
pub fn unified_threshold(sigma: f64, n: usize, alpha: f64, p: f64, q: f64) -> Result<f64> {
    ensure_crossover_params(sigma, n, alpha, q)?;
    if !p.is_finite() || p < 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "p must be finite and >= 1",
        ));
    }
    if q <= p {
        return Err(MetricChronoError::InvalidArgument(
            "q must be > p for the unified threshold formula",
        ));
    }
    let k = kappa_pq(p, q);
    Ok(
        sigma
            * k.powf(-1.0 / p)
            * (n as f64).powf(1.0 / p)
            * alpha.recip().ln().powf(1.0 / (p * q)),
    )
}

/// Asymptotic threshold ratio `eps_0 / eps`.
///
/// `ratio = kappa^{1/p} * n^{-1/p} * (ln n)^{1/q} * (ln(1/alpha))^{-1/(pq)}`
pub fn threshold_ratio(n: usize, alpha: f64, p: f64, q: f64) -> Result<f64> {
    if n < 2 {
        return Err(MetricChronoError::InvalidArgument("n must be >= 2"));
    }
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "alpha must be in (0, 1)",
        ));
    }
    if !p.is_finite() || p < 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "p must be finite and >= 1",
        ));
    }
    if !q.is_finite() || q <= p {
        return Err(MetricChronoError::InvalidArgument(
            "q must be finite and > p",
        ));
    }
    let k = kappa_pq(p, q);
    let nf = n as f64;
    Ok(k.powf(1.0 / p)
        * nf.powf(-1.0 / p)
        * nf.ln().powf(1.0 / q)
        * alpha.recip().ln().powf(-1.0 / (p * q)))
}

/// Sparse dominance boundary: `k = n * (ln n)^{-p/q}`.
///
/// Below this sparsity, the axis-factorised comparator has higher power.
pub fn sparse_dominance_boundary(n: usize, p: f64, q: f64) -> Result<f64> {
    if n < 2 {
        return Err(MetricChronoError::InvalidArgument("n must be >= 2"));
    }
    if !p.is_finite() || p < 1.0 || !q.is_finite() || q <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "p must be >= 1, q must be > 0",
        ));
    }
    let nf = n as f64;
    Ok(nf * nf.ln().powf(-p / q))
}

/// Miss-probability upper bound for the axis-factorised comparator.
///
/// `P_miss <= (eps_0 * sqrt(n) / eps)^n`
///
/// Valid when a deviation of L2 norm `eps` is uniformly distributed on the
/// sphere.
pub fn miss_probability_bound(eps: f64, eps_0: f64, n: usize) -> Result<f64> {
    if !eps.is_finite() || eps <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "eps must be finite and > 0",
        ));
    }
    if !eps_0.is_finite() || eps_0 <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "eps_0 must be finite and > 0",
        ));
    }
    if n == 0 {
        return Err(MetricChronoError::InvalidArgument("n must be >= 1"));
    }
    let base = eps_0 * (n as f64).sqrt() / eps;
    Ok(base.powf(n as f64))
}

fn ensure_crossover_params(sigma: f64, n: usize, alpha: f64, q: f64) -> Result<()> {
    if !sigma.is_finite() || sigma <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "sigma must be finite and > 0",
        ));
    }
    if n < 2 {
        return Err(MetricChronoError::InvalidArgument("n must be >= 2"));
    }
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "alpha must be in (0, 1)",
        ));
    }
    if !q.is_finite() || q <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "q must be finite and > 0",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn kappa_pq_p1_q2() {
        let k = kappa_pq(1.0, 2.0);
        let expected = 1.0 * 2.0_f64.powf(2.0) * 0.5_f64.powf(0.5);
        assert_close(k, expected);
    }

    #[test]
    fn kappa_pq_returns_inf_when_q_le_p() {
        assert!(kappa_pq(2.0, 2.0).is_infinite());
        assert!(kappa_pq(2.0, 1.5).is_infinite());
    }

    #[test]
    fn factored_threshold_basic() {
        let t = factored_threshold(1.0, 100, 0.05, 1.0).expect("valid");
        let expected = (100.0 / 0.05_f64).ln();
        assert_close(t, expected);
    }

    #[test]
    fn unified_threshold_basic() {
        let t = unified_threshold(1.0, 100, 0.05, 1.0, 2.0).expect("valid");
        assert!(t.is_finite() && t > 0.0);
    }

    #[test]
    fn threshold_ratio_decreases_with_n() {
        let r1 = threshold_ratio(10, 0.05, 1.0, 2.0).expect("valid");
        let r2 = threshold_ratio(100, 0.05, 1.0, 2.0).expect("valid");
        let r3 = threshold_ratio(1000, 0.05, 1.0, 2.0).expect("valid");
        assert!(r1 > r2 && r2 > r3);
    }

    #[test]
    fn sparse_dominance_boundary_basic() {
        let k = sparse_dominance_boundary(100, 1.0, 2.0).expect("valid");
        let expected = 100.0 * 100.0_f64.ln().powf(-0.5);
        assert_close(k, expected);
    }

    #[test]
    fn miss_probability_bound_at_inscribed() {
        let n = 10;
        let eps = 1.0;
        let eps_0 = eps / (n as f64).sqrt();
        let bound = miss_probability_bound(eps, eps_0, n).expect("valid");
        assert_close(bound, 1.0);
    }

    #[test]
    fn miss_probability_bound_decreases_with_smaller_eps_0() {
        let n = 10;
        let eps = 1.0;
        let b1 = miss_probability_bound(eps, 0.5, n).expect("valid");
        let b2 = miss_probability_bound(eps, 0.3, n).expect("valid");
        assert!(b2 < b1);
    }

    #[test]
    fn kappa_pq_nan_for_invalid_inputs() {
        assert!(kappa_pq(0.0, 2.0).is_nan());
        assert!(kappa_pq(-1.0, 2.0).is_nan());
        assert!(kappa_pq(f64::NAN, 2.0).is_nan());
        assert!(kappa_pq(1.0, f64::NAN).is_nan());
        assert!(kappa_pq(1.0, 0.0).is_nan());
    }

    #[test]
    fn kappa_pq_p2_q4() {
        let k = kappa_pq(2.0, 4.0);
        let expected = (1.0 / 2.0) * 2.0_f64.powf(2.0) * 0.5_f64.powf(0.5);
        assert_close(k, expected);
    }

    #[test]
    fn unified_threshold_increases_with_dimension() {
        let t1 = unified_threshold(1.0, 10, 0.05, 1.0, 2.0).expect("valid");
        let t2 = unified_threshold(1.0, 100, 0.05, 1.0, 2.0).expect("valid");
        assert!(t2 > t1);
    }
}
