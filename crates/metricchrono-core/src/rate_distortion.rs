use crate::{MetricChronoError, Result};

/// Geometric quantizer decision boundaries: t_k = a * (b/a)^(k/n) for k = 0..=n.
/// Returns n+1 boundary values. Errors if a <= 0, b <= a, or n == 0.
pub fn geometric_boundaries(a: f64, b: f64, n: usize) -> Result<Vec<f64>> {
    validate_range(a, b, n)?;

    let len = n
        .checked_add(1)
        .ok_or(MetricChronoError::InvalidArgument("n is too large"))?;
    let scale = b / a;
    let mut boundaries = Vec::with_capacity(len);
    if scale.is_finite() {
        let ratio = scale.powf(1.0 / n as f64);
        let mut boundary = a;
        boundaries.push(boundary);
        for _ in 1..n {
            boundary *= ratio;
            boundaries.push(boundary);
        }
        boundaries.push(b);
    } else {
        let log_a = a.ln();
        let log_range = b.ln() - log_a;
        for k in 0..=n {
            let boundary = if k == 0 {
                a
            } else if k == n {
                b
            } else {
                (log_a + log_range * k as f64 / n as f64).exp()
            };
            boundaries.push(boundary);
        }
    }
    Ok(boundaries)
}

/// Harmonic-mean representative for cell [t_k, t_{k+1}]: r = 2*t_k*t_{k+1} / (t_k + t_{k+1}).
/// This is the Lloyd-optimal representative under relative distortion.
pub fn harmonic_mean_representative(t_lo: f64, t_hi: f64) -> f64 {
    2.0 * t_lo * t_hi / (t_lo + t_hi)
}

/// Verify Lloyd fixed-point condition: for geometric boundaries with harmonic
/// representatives, the nearest-neighbour boundaries match the geometric boundaries.
/// Returns the maximum relative error across all boundaries.
pub(crate) fn lloyd_fixed_point_error(a: f64, b: f64, n: usize) -> Result<f64> {
    let boundaries = geometric_boundaries(a, b, n)?;
    let representatives: Vec<f64> = boundaries
        .windows(2)
        .map(|cell| harmonic_mean_representative(cell[0], cell[1]))
        .collect();

    let mut max_error = 0.0_f64;
    for (index, reps) in representatives.windows(2).enumerate() {
        let expected = boundaries[index + 1];
        let observed = 0.5 * (reps[0] + reps[1]);
        max_error = max_error.max(((observed - expected) / expected).abs());
    }
    Ok(max_error)
}

/// Staircase penalty: ratio D_staircase / D_geo for a tier with M uniform sub-levels
/// and arithmetic-midpoint representatives vs geometric sub-levels with harmonic reps.
/// For moderate alpha (1.5..3) and m_sub >= 4 the ratio stays below ~1.25.
pub fn staircase_penalty_ratio(alpha: f64, m_sub: usize) -> Result<f64> {
    if !alpha.is_finite() || alpha <= 1.0 {
        return Err(MetricChronoError::InvalidArgument(
            "alpha must be finite and > 1",
        ));
    }
    if m_sub == 0 {
        return Err(MetricChronoError::InvalidArgument("m_sub must be > 0"));
    }

    let step = (alpha - 1.0) / m_sub as f64;
    let mut staircase = 0.0;
    for k in 0..m_sub {
        let t_lo = 1.0 + step * k as f64;
        let t_hi = if k + 1 == m_sub { alpha } else { t_lo + step };
        let representative = 0.5 * (t_lo + t_hi);
        staircase += cell_distortion_integral(t_lo, t_hi, representative);
    }

    let boundaries = geometric_boundaries(1.0, alpha, m_sub)?;
    let geometric = boundaries
        .windows(2)
        .map(|cell| {
            let representative = harmonic_mean_representative(cell[0], cell[1]);
            cell_distortion_integral(cell[0], cell[1], representative)
        })
        .sum::<f64>();

    Ok(staircase / geometric)
}

fn validate_range(a: f64, b: f64, n: usize) -> Result<()> {
    if !a.is_finite() || a <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "a must be finite and > 0",
        ));
    }
    if !b.is_finite() || b <= a {
        return Err(MetricChronoError::InvalidArgument(
            "b must be finite and > a",
        ));
    }
    if n == 0 {
        return Err(MetricChronoError::InvalidArgument("n must be > 0"));
    }
    Ok(())
}

fn cell_distortion_integral(t_lo: f64, t_hi: f64, representative: f64) -> f64 {
    let ratio = t_hi / t_lo;
    let scaled = representative / t_lo;
    ratio.ln()
        + 2.0 * scaled * (1.0 / ratio - 1.0)
        + 0.5 * scaled * scaled * (1.0 - 1.0 / (ratio * ratio))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometric_boundaries_double_on_power_two_range() {
        let boundaries = geometric_boundaries(1.0, 256.0, 8).unwrap();
        assert_eq!(
            boundaries,
            vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0]
        );
    }

    #[test]
    fn harmonic_mean_matches_closed_form() {
        assert_eq!(harmonic_mean_representative(1.0, 2.0), 4.0 / 3.0);
    }

    #[test]
    fn lloyd_error_is_machine_small_on_power_two_range() {
        assert!(lloyd_fixed_point_error(1.0, 256.0, 8).unwrap() < 1e-14);
    }

    #[test]
    fn staircase_penalty_stays_bounded_for_typical_alpha() {
        let ratio = staircase_penalty_ratio(2.0, 8).unwrap();
        assert!((1.0..1.25).contains(&ratio));
    }

    #[test]
    fn geometric_boundaries_reject_invalid_arguments() {
        assert!(geometric_boundaries(0.0, 2.0, 1).is_err());
        assert!(geometric_boundaries(-1.0, 2.0, 1).is_err());
        assert!(geometric_boundaries(1.0, 1.0, 1).is_err());
        assert!(geometric_boundaries(1.0, 2.0, 0).is_err());
    }
}
