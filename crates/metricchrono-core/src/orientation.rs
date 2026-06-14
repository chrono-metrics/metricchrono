use crate::{MetricChronoError, Result};

/// Discrete derivatives of a signal: velocity, acceleration, jerk.
///
/// Given a sequence `xs` of length `n`, returns three vectors:
/// - velocity:     length `n-1`, `v[i] = xs[i+1] - xs[i]`
/// - acceleration: length `n-2`, `a[i] = v[i+1] - v[i]`
/// - jerk:         length `n-3`, `j[i] = a[i+1] - a[i]`
///
/// These are the first three backward differences. The `k`-th derivative
/// carries reversal parity `(-1)^k` (Prop reversal_parity).
pub fn discrete_derivatives(xs: &[f64]) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    if xs.len() < 4 {
        return Err(MetricChronoError::InvalidArgument(
            "need at least 4 values for velocity, acceleration, and jerk",
        ));
    }
    ensure_finite_slice(xs)?;
    let velocity: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    let acceleration: Vec<f64> = velocity.windows(2).map(|w| w[1] - w[0]).collect();
    let jerk: Vec<f64> = acceleration.windows(2).map(|w| w[1] - w[0]).collect();
    Ok((velocity, acceleration, jerk))
}

/// `k`-th order discrete derivative of a signal.
///
/// Returns a vector of length `n - k`. The `k`-th derivative carries reversal
/// parity `(-1)^k`.
pub fn discrete_derivative(xs: &[f64], order: usize) -> Result<Vec<f64>> {
    if order == 0 {
        return Ok(xs.to_vec());
    }
    if xs.len() <= order {
        return Err(MetricChronoError::InvalidArgument(
            "signal too short for the requested derivative order",
        ));
    }
    ensure_finite_slice(xs)?;
    let mut current = xs.to_vec();
    for _ in 0..order {
        current = current.windows(2).map(|w| w[1] - w[0]).collect();
    }
    Ok(current)
}

/// Shannon entropy of a probability distribution (openness scalar).
///
/// Weights are normalised internally. Zero or negative weights are ignored.
/// Returns `H = -sum_i p_i ln(p_i)` where `p_i = w_i / sum(w)`.
pub fn entropy_openness(weights: &[f64]) -> Result<f64> {
    if weights.is_empty() {
        return Err(MetricChronoError::InvalidArgument("weights must be non-empty"));
    }
    let total: f64 = weights.iter().copied().filter(|w| *w > 0.0).sum();
    if total <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "weights must contain at least one positive value",
        ));
    }
    let h = weights
        .iter()
        .copied()
        .filter(|w| *w > 0.0)
        .map(|w| {
            let p = w / total;
            -p * p.ln()
        })
        .sum::<f64>();
    Ok(h)
}

/// One-dimensional earth mover (Wasserstein-1) distance between two
/// distributions.
///
/// Both `p` and `q` must have the same length and sum to the same total
/// (typically 1.0). Computed as `sum |cumsum(p - q)|`.
pub fn earth_mover_1d(p: &[f64], q: &[f64]) -> Result<f64> {
    if p.len() != q.len() {
        return Err(MetricChronoError::ShapeMismatch {
            expected: p.len(),
            actual: q.len(),
            context: "earth_mover_1d distributions",
        });
    }
    if p.is_empty() {
        return Err(MetricChronoError::InvalidArgument(
            "distributions must be non-empty",
        ));
    }
    ensure_finite_slice(p)?;
    ensure_finite_slice(q)?;
    let mut cumulative = 0.0;
    let mut distance = 0.0;
    for (pi, qi) in p.iter().zip(q.iter()) {
        cumulative += pi - qi;
        distance += cumulative.abs();
    }
    Ok(distance)
}

/// Check reversal parity of the `k`-th discrete derivative.
///
/// Given a signal `xs`, computes the `k`-th derivative of `xs` and of its
/// reversal, and checks that `D^k(rev)[i] = (-1)^k * D^k(xs)[n-k-1-i]`.
/// Returns the maximum absolute violation.
pub fn reversal_parity_error(xs: &[f64], order: usize) -> Result<f64> {
    let forward = discrete_derivative(xs, order)?;
    let reversed: Vec<f64> = xs.iter().copied().rev().collect();
    let rev_deriv = discrete_derivative(&reversed, order)?;

    let sign = if order % 2 == 0 { 1.0 } else { -1.0 };
    let n = forward.len();
    let mut max_err = 0.0_f64;
    for i in 0..n {
        let expected = sign * forward[n - 1 - i];
        let actual = rev_deriv[i];
        max_err = max_err.max((actual - expected).abs());
    }
    Ok(max_err)
}

fn ensure_finite_slice(xs: &[f64]) -> Result<()> {
    if xs.iter().any(|x| !x.is_finite()) {
        return Err(MetricChronoError::InvalidArgument(
            "input must contain only finite values (no NaN or infinity)",
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
    fn discrete_derivatives_linear() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let (v, a, j) = discrete_derivatives(&xs).expect("valid");
        assert!(v.iter().all(|x| (*x - 1.0).abs() < 1e-12));
        assert!(a.iter().all(|x| x.abs() < 1e-12));
        assert!(j.iter().all(|x| x.abs() < 1e-12));
    }

    #[test]
    fn discrete_derivatives_quadratic() {
        let xs: Vec<f64> = (0..6).map(|i| (i * i) as f64).collect();
        let (v, a, j) = discrete_derivatives(&xs).expect("valid");
        assert_eq!(v.len(), 5);
        assert_eq!(a.len(), 4);
        assert!(a.iter().all(|x| (*x - 2.0).abs() < 1e-12));
        assert!(j.iter().all(|x| x.abs() < 1e-12));
        let _ = v;
    }

    #[test]
    fn discrete_derivatives_rejects_short_input() {
        assert!(discrete_derivatives(&[1.0, 2.0, 3.0]).is_err());
    }

    #[test]
    fn reversal_parity_velocity_flips() {
        let xs = [1.0, 3.0, 2.0, 7.0, 4.0, 6.0];
        let err = reversal_parity_error(&xs, 1).expect("valid");
        assert!(err < 1e-12, "velocity parity error: {err}");
    }

    #[test]
    fn reversal_parity_acceleration_invariant() {
        let xs = [1.0, 3.0, 2.0, 7.0, 4.0, 6.0];
        let err = reversal_parity_error(&xs, 2).expect("valid");
        assert!(err < 1e-12, "acceleration parity error: {err}");
    }

    #[test]
    fn reversal_parity_jerk_flips() {
        let xs = [1.0, 3.0, 2.0, 7.0, 4.0, 6.0];
        let err = reversal_parity_error(&xs, 3).expect("valid");
        assert!(err < 1e-12, "jerk parity error: {err}");
    }

    #[test]
    fn entropy_openness_uniform() {
        let w = [1.0, 1.0, 1.0, 1.0];
        let h = entropy_openness(&w).expect("valid");
        assert_close(h, 4.0_f64.ln());
    }

    #[test]
    fn entropy_openness_degenerate() {
        let w = [1.0, 0.0, 0.0];
        let h = entropy_openness(&w).expect("valid");
        assert_close(h, 0.0);
    }

    #[test]
    fn earth_mover_identical_is_zero() {
        let p = [0.25, 0.25, 0.25, 0.25];
        assert_close(earth_mover_1d(&p, &p).expect("valid"), 0.0);
    }

    #[test]
    fn earth_mover_dirac_shift() {
        let p = [1.0, 0.0, 0.0];
        let q = [0.0, 0.0, 1.0];
        assert_close(earth_mover_1d(&p, &q).expect("valid"), 2.0);
    }

    #[test]
    fn earth_mover_rejects_length_mismatch() {
        assert!(earth_mover_1d(&[0.5, 0.5], &[1.0]).is_err());
    }

    #[test]
    fn earth_mover_is_symmetric() {
        let p = [0.5, 0.3, 0.2];
        let q = [0.1, 0.6, 0.3];
        let d1 = earth_mover_1d(&p, &q).expect("valid");
        let d2 = earth_mover_1d(&q, &p).expect("valid");
        assert_close(d1, d2);
    }

    #[test]
    fn discrete_derivative_second_order_quadratic() {
        let xs: Vec<f64> = (0..6).map(|i| (i * i) as f64).collect();
        let d2 = discrete_derivative(&xs, 2).expect("valid");
        assert_eq!(d2.len(), 4);
        assert!(d2.iter().all(|x| (*x - 2.0).abs() < 1e-12));
    }

    #[test]
    fn reversal_parity_order_zero_is_trivial() {
        let xs = [1.0, 3.0, 2.0, 7.0];
        let err = reversal_parity_error(&xs, 0).expect("valid");
        assert!(err < 1e-12, "order-0 parity error: {err}");
    }

    #[test]
    fn nan_input_rejected_by_derivatives() {
        assert!(discrete_derivatives(&[1.0, f64::NAN, 3.0, 4.0]).is_err());
        assert!(discrete_derivative(&[1.0, f64::INFINITY, 3.0], 1).is_err());
    }

    #[test]
    fn nan_input_rejected_by_earth_mover() {
        assert!(earth_mover_1d(&[0.5, f64::NAN], &[0.5, 0.5]).is_err());
        assert!(earth_mover_1d(&[0.5, 0.5], &[f64::INFINITY, 0.5]).is_err());
    }
}
