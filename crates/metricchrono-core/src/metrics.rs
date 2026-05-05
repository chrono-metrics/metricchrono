use crate::{ladder_distance, try_tick_distance, Result, Tier};

/// Distance metric abstraction for pair APIs.
pub trait Metric<T: ?Sized> {
    fn distance(&self, a: &T, b: &T) -> f64;
}

/// Function-backed metric adapter.
#[derive(Clone, Copy)]
pub struct MetricFn<F>(pub F);

impl<T: ?Sized, F> Metric<T> for MetricFn<F>
where
    F: Fn(&T, &T) -> f64,
{
    fn distance(&self, a: &T, b: &T) -> f64 {
        (self.0)(a, b)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Absolute;

#[derive(Clone, Copy, Debug, Default)]
pub struct Euclidean;

#[derive(Clone, Copy, Debug, Default)]
pub struct SquaredEuclidean;

#[derive(Clone, Copy, Debug, Default)]
pub struct Manhattan;

#[derive(Clone, Copy, Debug, Default)]
pub struct Cosine;

#[derive(Clone, Copy, Debug)]
pub struct KullbackLeibler {
    pub epsilon: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct JensenShannon {
    pub epsilon: f64,
}

#[derive(Clone, Debug)]
pub struct DiagonalMahalanobis {
    inverse_variance: Vec<f64>,
}

impl Default for KullbackLeibler {
    fn default() -> Self {
        Self { epsilon: 1e-12 }
    }
}

impl Default for JensenShannon {
    fn default() -> Self {
        Self { epsilon: 1e-12 }
    }
}

impl DiagonalMahalanobis {
    pub fn from_variance(variance: impl Into<Vec<f64>>) -> Self {
        let inverse_variance = variance
            .into()
            .into_iter()
            .map(|value| {
                if value.is_finite() && value > 0.0 {
                    1.0 / value
                } else {
                    0.0
                }
            })
            .collect();
        Self { inverse_variance }
    }

    pub fn from_inverse_variance(inverse_variance: impl Into<Vec<f64>>) -> Self {
        Self {
            inverse_variance: inverse_variance.into(),
        }
    }

    pub fn inverse_variance(&self) -> &[f64] {
        &self.inverse_variance
    }
}

impl Metric<f64> for Absolute {
    fn distance(&self, a: &f64, b: &f64) -> f64 {
        (a - b).abs()
    }
}

impl Metric<[f64]> for Euclidean {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return f64::NAN;
        }
        a.iter()
            .zip(b)
            .map(|(left, right)| {
                let diff = left - right;
                diff * diff
            })
            .sum::<f64>()
            .sqrt()
    }
}

impl Metric<[f64]> for SquaredEuclidean {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return f64::NAN;
        }
        a.iter()
            .zip(b)
            .map(|(left, right)| {
                let diff = left - right;
                diff * diff
            })
            .sum()
    }
}

impl Metric<[f64]> for Manhattan {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return f64::NAN;
        }
        a.iter()
            .zip(b)
            .map(|(left, right)| (left - right).abs())
            .sum()
    }
}

impl Metric<[f64]> for Cosine {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return f64::NAN;
        }
        let dot = a
            .iter()
            .zip(b)
            .map(|(left, right)| left * right)
            .sum::<f64>();
        let norm_a = a.iter().map(|value| value * value).sum::<f64>().sqrt();
        let norm_b = b.iter().map(|value| value * value).sum::<f64>().sqrt();
        if norm_a <= 0.0 || norm_b <= 0.0 {
            1.0
        } else {
            (1.0 - dot / (norm_a * norm_b)).clamp(0.0, 2.0)
        }
    }
}

impl Metric<[f64]> for KullbackLeibler {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        divergence_inputs(a, b, self.epsilon)
            .map(|(p, q)| {
                p.iter()
                    .zip(q)
                    .map(|(left, right)| left * (left.ln() - right.ln()))
                    .sum()
            })
            .unwrap_or(f64::NAN)
    }
}

impl Metric<[f64]> for JensenShannon {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        divergence_inputs(a, b, self.epsilon)
            .map(|(p, q)| {
                p.iter()
                    .zip(q.iter())
                    .map(|(left, right)| {
                        let midpoint = 0.5 * (left + right);
                        0.5 * left * (left.ln() - midpoint.ln())
                            + 0.5 * right * (right.ln() - midpoint.ln())
                    })
                    .sum()
            })
            .unwrap_or(f64::NAN)
    }
}

impl Metric<[f64]> for DiagonalMahalanobis {
    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.len() != self.inverse_variance.len() {
            return f64::NAN;
        }
        a.iter()
            .zip(b)
            .zip(&self.inverse_variance)
            .map(|((left, right), inv_var)| {
                let diff = left - right;
                diff * diff * (*inv_var).max(0.0)
            })
            .sum::<f64>()
            .sqrt()
    }
}

/// Compute a tick by first measuring a metric distance between two values.
pub fn tick_pair<T: ?Sized, M: Metric<T>>(a: &T, b: &T, metric: &M, tier: Tier) -> Result<f64> {
    try_tick_distance(metric.distance(a, b), tier)
}

/// Compute a ladder vector from a metric distance between two values.
pub fn ladder_pair<T: ?Sized, M: Metric<T>>(
    a: &T,
    b: &T,
    metric: &M,
    ladder: &[Tier],
) -> Result<Vec<f64>> {
    let mut out = vec![0.0; ladder.len()];
    ladder_distance(metric.distance(a, b), ladder, &mut out)?;
    Ok(out)
}

fn divergence_inputs(a: &[f64], b: &[f64], epsilon: f64) -> Option<(Vec<f64>, Vec<f64>)> {
    if a.len() != b.len() || a.is_empty() || !epsilon.is_finite() || epsilon <= 0.0 {
        return None;
    }
    Some((
        normalize_probabilities(a, epsilon),
        normalize_probabilities(b, epsilon),
    ))
}

fn normalize_probabilities(values: &[f64], epsilon: f64) -> Vec<f64> {
    let mut out: Vec<f64> = values
        .iter()
        .map(|value| {
            if value.is_finite() {
                (*value).max(0.0) + epsilon
            } else {
                epsilon
            }
        })
        .collect();
    let total = out.iter().sum::<f64>().max(epsilon);
    for value in &mut out {
        *value /= total;
    }
    out
}
