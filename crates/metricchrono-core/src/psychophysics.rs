use crate::{MetricChronoError, Result};

/// Compressed read-out: `mu^(1-s) * d^s`.
///
/// Maps a physical interval `d` through the power-law compression with slope
/// `s` and crossover `mu`. When `0 < s < 1`, short intervals are
/// overestimated and long intervals underestimated (Vierordt's law).
pub fn compressed_readout(d: f64, s: f64, mu: f64) -> f64 {
    mu.powf(1.0 - s) * d.powf(s)
}

/// Bisection PSE under the log decision convention: `sqrt(t_short * t_long)`.
///
/// Independent of every ladder parameter (`s`, `mu`, hence `m`, `alpha`, `p`,
/// `rho`, `beta` all cancel).
pub fn log_bisection_pse(t_short: f64, t_long: f64) -> Result<f64> {
    ensure_interval_pair(t_short, t_long)?;
    Ok((t_short * t_long).sqrt())
}

/// Bisection PSE under the magnitude decision convention: the order-`s` power
/// mean `((t_short^s + t_long^s) / 2)^(1/s)`.
///
/// `mu` cancels. The result is increasing in `s`, with limits: geometric mean
/// as `s -> 0+`, arithmetic mean at `s = 1`.
pub fn magnitude_bisection_pse(t_short: f64, t_long: f64, s: f64) -> Result<f64> {
    ensure_interval_pair(t_short, t_long)?;
    if !s.is_finite() || s <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "s must be finite and > 0",
        ));
    }
    // For tiny s, the power mean approaches the geometric mean; the
    // log-sum-exp subtraction loses precision below ~1e-7.
    if s < 1e-7 {
        return Ok((t_short * t_long).sqrt());
    }
    let a = s * t_short.ln();
    let b = s * t_long.ln();
    let c = a.max(b);
    let sum = (a - c).exp() + (b - c).exp();
    Ok(((c + sum.ln() - 2.0_f64.ln()) / s).exp())
}

/// Finite-depth aggregate slope `S(m)` for a geometric ladder.
///
/// Given base parameters `eps_0`, `delta_0`, growth ratio `alpha`, gain
/// exponent `p`, depth `m`, and reference scale `eps_ref`, computes:
///
/// ```text
/// pref = (1 - alpha^{-1}) * (eps_0/eps_ref)^p / ln(delta_0/eps_0)
/// S(m) = pref * (1 - alpha^{(p-1)m}) / (1 - alpha^{p-1})
/// ```
///
/// (with the `p = 1` case handled by the limit `S = pref * m`).
pub fn aggregate_slope(
    eps_0: f64,
    delta_0: f64,
    alpha: f64,
    p: f64,
    m: usize,
    eps_ref: f64,
) -> Result<f64> {
    if !eps_0.is_finite() || eps_0 <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "eps_0 must be finite and > 0",
        ));
    }
    if !delta_0.is_finite() || delta_0 <= eps_0 {
        return Err(MetricChronoError::InvalidArgument(
            "delta_0 must be finite and > eps_0",
        ));
    }
    if !alpha.is_finite() || alpha <= 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "alpha must be finite and > 1",
        ));
    }
    if !p.is_finite() {
        return Err(MetricChronoError::InvalidArgument("p must be finite"));
    }
    if m == 0 {
        return Err(MetricChronoError::InvalidArgument("m must be >= 1"));
    }
    if !eps_ref.is_finite() || eps_ref <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "eps_ref must be finite and > 0",
        ));
    }

    let rho = eps_0 / eps_ref;
    let beta = delta_0 / eps_0;
    let pref = (1.0 - alpha.powf(-1.0)) * rho.powf(p) / beta.ln();

    if (p - 1.0).abs() < 1e-7 {
        Ok(pref * m as f64)
    } else {
        let pm1 = p - 1.0;
        Ok(pref * (1.0 - alpha.powf(pm1 * m as f64)) / (1.0 - alpha.powf(pm1)))
    }
}

/// Vierordt crossover point for a geometric ladder: `delta_0 * alpha^((m-1)/2)`.
///
/// This is the geometric mean of the `delta_k` values and the unique point
/// where `compressed_readout(mu, s, mu) == mu` (zero Vierordt bias).
pub fn vierordt_crossover(delta_0: f64, alpha: f64, m: usize) -> Result<f64> {
    if !delta_0.is_finite() || delta_0 <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "delta_0 must be finite and > 0",
        ));
    }
    if !alpha.is_finite() || alpha <= 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "alpha must be finite and > 1",
        ));
    }
    if m == 0 {
        return Err(MetricChronoError::InvalidArgument("m must be >= 1"));
    }
    Ok(delta_0 * alpha.powf((m as f64 - 1.0) / 2.0))
}

fn ensure_interval_pair(t_short: f64, t_long: f64) -> Result<()> {
    if !t_short.is_finite() || t_short <= 0.0 || !t_long.is_finite() || t_long <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "intervals must be finite and > 0",
        ));
    }
    if t_short >= t_long {
        return Err(MetricChronoError::InvalidArgument(
            "t_short must be < t_long",
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
    fn compressed_readout_identity_at_s_one() {
        assert_close(compressed_readout(2.5, 1.0, 3.0), 2.5);
    }

    #[test]
    fn compressed_readout_fixed_point_at_mu() {
        assert_close(compressed_readout(3.0, 0.6, 3.0), 3.0);
    }

    #[test]
    fn compressed_readout_overestimates_below_mu() {
        assert!(compressed_readout(0.5, 0.6, 2.0) > 0.5);
    }

    #[test]
    fn compressed_readout_underestimates_above_mu() {
        assert!(compressed_readout(5.0, 0.6, 2.0) < 5.0);
    }

    #[test]
    fn log_bisection_pse_is_geometric_mean() {
        assert_close(
            log_bisection_pse(0.4, 1.6).expect("valid pair"),
            0.8,
        );
    }

    #[test]
    fn log_bisection_pse_rejects_bad_order() {
        assert!(log_bisection_pse(2.0, 1.0).is_err());
    }

    #[test]
    fn magnitude_bisection_pse_between_means() {
        let geo = (0.4_f64 * 1.6).sqrt();
        let arith = (0.4 + 1.6) / 2.0;
        let pse = magnitude_bisection_pse(0.4, 1.6, 0.5).expect("valid");
        assert!(pse >= geo - 1e-12 && pse <= arith + 1e-12);
    }

    #[test]
    fn magnitude_bisection_pse_at_s_one_is_arithmetic() {
        assert_close(
            magnitude_bisection_pse(0.4, 1.6, 1.0).expect("valid"),
            1.0,
        );
    }

    #[test]
    fn magnitude_bisection_pse_monotone_in_s() {
        let mut prev = magnitude_bisection_pse(0.4, 1.6, 0.01).expect("valid");
        for i in 2..=100 {
            let s = i as f64 * 0.01;
            let curr = magnitude_bisection_pse(0.4, 1.6, s).expect("valid");
            assert!(curr >= prev - 1e-12, "not monotone at s={s}");
            prev = curr;
        }
    }

    #[test]
    fn aggregate_slope_finite_depth() {
        let s = aggregate_slope(0.1, 0.3, 2.0, 0.4, 8, 0.1).expect("valid");
        assert!(s.is_finite() && s > 0.0);
    }

    #[test]
    fn aggregate_slope_matches_weighted_sum() {
        let eps_0 = 0.1;
        let delta_0 = 0.3;
        let alpha = 2.0_f64;
        let p = 0.4;
        let m = 8_usize;
        let eps_ref = 0.1;

        let s_formula = aggregate_slope(eps_0, delta_0, alpha, p, m, eps_ref).expect("valid");

        let mut s_direct = 0.0;
        for k in 0..m {
            let scale = alpha.powi(k as i32);
            let eps_k = eps_0 * scale;
            let delta_k = delta_0 * scale;
            let w_k = (1.0 - 1.0 / alpha) * alpha.powi(-(k as i32));
            let s_k = (eps_k / eps_ref).powf(p) / (delta_k / eps_k).ln();
            s_direct += w_k * s_k;
        }

        assert_close(s_formula, s_direct);
    }

    #[test]
    fn vierordt_crossover_geometric_mean() {
        let mu = vierordt_crossover(0.3, 2.0, 8).expect("valid");
        let expected = 0.3 * 2.0_f64.powf(3.5);
        assert_close(mu, expected);
    }

    #[test]
    fn scale_invariance_dissociation() {
        let eps_0 = 0.1;
        let delta_0 = 0.4;
        let alpha = 2.0;
        let p = 0.4;
        let m = 8;
        let eps_ref = 1.0;

        let s_base = aggregate_slope(eps_0, delta_0, alpha, p, m, eps_ref).expect("valid");
        let mu_base = vierordt_crossover(delta_0, alpha, m).expect("valid");

        for &lam in &[0.5, 2.0, 5.0] {
            let s_scaled = aggregate_slope(
                lam * eps_0, lam * delta_0, alpha, p, m, lam * eps_ref,
            ).expect("valid");
            let mu_scaled = vierordt_crossover(lam * delta_0, alpha, m).expect("valid");
            assert_close(s_scaled, s_base);
            assert_close(mu_scaled / mu_base, lam);
        }
    }

    #[test]
    fn vierordt_crossover_matches_direct_geometric_mean() {
        let delta_0 = 0.3;
        let alpha = 2.0_f64;
        let m = 8_usize;
        let mu = vierordt_crossover(delta_0, alpha, m).expect("valid");

        let log_sum: f64 = (0..m).map(|k| (delta_0 * alpha.powi(k as i32)).ln()).sum();
        let mu_direct = (log_sum / m as f64).exp();
        assert_close(mu, mu_direct);
    }
}
