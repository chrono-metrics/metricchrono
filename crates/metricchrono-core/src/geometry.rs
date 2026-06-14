use crate::{MetricChronoError, Result, Tier};

const TRIANGLE_TOLERANCE: f64 = 1e-10;

/// Comparison angle at the present:
/// `arccos((r1^2 + r2^2 - c^2) / (2 * r1 * r2))`.
///
/// Well-defined for any metric triple where `|r1 - r2| <= c <= r1 + r2`.
/// Returns an error if `r1` or `r2` are non-positive, or `c` violates the
/// triangle inequality.
pub fn comparison_angle(r1: f64, r2: f64, c: f64) -> Result<f64> {
    ensure_metric_triple(r1, r2, c)?;
    Ok(comparison_angle_argument(r1, r2, c).clamp(-1.0, 1.0).acos())
}

/// Unchecked comparison angle for hot paths. Caller guarantees a valid metric
/// triple; no validation is performed.
pub fn comparison_angle_unchecked(r1: f64, r2: f64, c: f64) -> f64 {
    comparison_angle_argument(r1, r2, c).clamp(-1.0, 1.0).acos()
}

/// Shell membership at a single tier. Returns `None` if `r < epsilon`
/// (now-shell), `Some(j)` where `j = ceil(r / delta)` if `r >= epsilon`.
pub fn shell_index(r: f64, epsilon: f64, delta: f64) -> Option<usize> {
    if r < epsilon {
        None
    } else {
        Some((r / delta).ceil() as usize)
    }
}

/// Multi-tier shell membership across a ladder. Returns one shell index per
/// tier.
pub fn shell_indices(r: f64, ladder: &[Tier]) -> Vec<Option<usize>> {
    ladder
        .iter()
        .map(|tier| shell_index(r, tier.epsilon, tier.delta))
        .collect()
}

/// Greedy maximal epsilon-separated packing from a symmetric distance matrix.
/// Returns indices of the selected points.
pub fn greedy_packing(distances: &[Vec<f64>], epsilon: f64) -> Vec<usize> {
    let n = distances.len();
    let mut marked = vec![false; n];
    let mut packing = Vec::new();

    while let Some(picked) = marked.iter().position(|is_marked| !*is_marked) {
        packing.push(picked);
        marked[picked] = true;

        for (candidate, is_marked) in marked.iter_mut().enumerate() {
            if distance_at(distances, picked, candidate) < epsilon {
                *is_marked = true;
            }
        }
    }

    packing
}

/// Branching number: greedy surrogate for the epsilon-packing number of points
/// within horizon `delta`.
///
/// A greedy maximal packing is computable in one pass and is sandwiched between
/// the true packing numbers. Given radii and pairwise distances, selects points
/// with `r <= delta`, then counts the maximal epsilon-separated subset.
pub fn branching_number(radii: &[f64], distances: &[Vec<f64>], epsilon: f64, delta: f64) -> usize {
    let eligible = radii
        .iter()
        .enumerate()
        .filter_map(|(index, radius)| (*radius <= delta).then_some(index))
        .collect::<Vec<_>>();
    let mut marked = vec![false; eligible.len()];
    let mut count = 0;

    while let Some(packed_index) = marked.iter().position(|is_marked| !*is_marked) {
        let picked = eligible[packed_index];
        count += 1;
        marked[packed_index] = true;

        for (candidate_index, is_marked) in marked.iter_mut().enumerate() {
            let candidate = eligible[candidate_index];
            if distance_at(distances, picked, candidate) < epsilon {
                *is_marked = true;
            }
        }
    }

    count
}

/// Sort futures by radius (ascending). Returns permutation indices.
pub fn radial_sort(radii: &[f64]) -> Vec<usize> {
    let mut indices = (0..radii.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        radii[*left]
            .total_cmp(&radii[*right])
            .then_with(|| left.cmp(right))
    });
    indices
}

fn comparison_angle_argument(r1: f64, r2: f64, c: f64) -> f64 {
    (r1 * r1 + r2 * r2 - c * c) / (2.0 * r1 * r2)
}

fn ensure_metric_triple(r1: f64, r2: f64, c: f64) -> Result<()> {
    if !r1.is_finite() || !r2.is_finite() || r1 <= 0.0 || r2 <= 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "radii must be finite and > 0",
        ));
    }
    if !c.is_finite() || c < 0.0 {
        return Err(MetricChronoError::InvalidArgument(
            "distance must be finite and >= 0",
        ));
    }
    if c + TRIANGLE_TOLERANCE < (r1 - r2).abs() || c > r1 + r2 + TRIANGLE_TOLERANCE {
        return Err(MetricChronoError::InvalidArgument(
            "distances must satisfy the triangle inequality",
        ));
    }
    Ok(())
}

fn distance_at(distances: &[Vec<f64>], left: usize, right: usize) -> f64 {
    distances
        .get(left)
        .and_then(|row| row.get(right))
        .or_else(|| distances.get(right).and_then(|row| row.get(left)))
        .copied()
        .unwrap_or(f64::INFINITY)
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
    fn comparison_angle_matches_right_triangle() {
        assert_close(
            comparison_angle(3.0, 4.0, 5.0).expect("valid triangle"),
            std::f64::consts::FRAC_PI_2,
        );
    }

    #[test]
    fn comparison_angle_allows_zero_separation_degenerate_triangle() {
        assert_close(
            comparison_angle(1.0, 1.0, 0.0).expect("valid degenerate triangle"),
            0.0,
        );
    }

    #[test]
    fn comparison_angle_allows_antipodal_degenerate_triangle() {
        assert_close(
            comparison_angle(1.0, 1.0, 2.0).expect("valid degenerate triangle"),
            std::f64::consts::PI,
        );
    }

    #[test]
    fn comparison_angle_rejects_triangle_inequality_violation() {
        assert!(comparison_angle(1.0, 1.0, 3.0).is_err());
    }

    #[test]
    fn comparison_angle_rejects_zero_radius() {
        assert!(comparison_angle(0.0, 1.0, 1.0).is_err());
    }

    #[test]
    fn shell_index_returns_none_inside_now_shell() {
        assert_eq!(shell_index(0.05, 0.1, 0.2), None);
    }

    #[test]
    fn shell_index_returns_ceiling_shell_number() {
        assert_eq!(shell_index(0.3, 0.1, 0.2), Some(2));
    }

    #[test]
    fn greedy_packing_selects_one_point_per_cluster() {
        let distances = vec![
            vec![0.0, 0.05, 0.5, 0.5],
            vec![0.05, 0.0, 0.5, 0.5],
            vec![0.5, 0.5, 0.0, 0.05],
            vec![0.5, 0.5, 0.05, 0.0],
        ];

        assert_eq!(greedy_packing(&distances, 0.1).len(), 2);
    }

    #[test]
    fn radial_sort_returns_ascending_permutation() {
        assert_eq!(radial_sort(&[3.0, 1.0, 2.0]), vec![1, 2, 0]);
    }

    #[test]
    fn branching_number_packs_points_inside_horizon() {
        let radii = [0.1, 0.2, 0.3, 0.8, 0.9];
        let distances = vec![
            vec![0.0, 0.05, 0.5, 0.8, 0.9],
            vec![0.05, 0.0, 0.5, 0.8, 0.9],
            vec![0.5, 0.5, 0.0, 0.8, 0.9],
            vec![0.8, 0.8, 0.8, 0.0, 0.05],
            vec![0.9, 0.9, 0.9, 0.05, 0.0],
        ];

        assert_eq!(branching_number(&radii, &distances, 0.1, 0.5), 2);
    }
}
