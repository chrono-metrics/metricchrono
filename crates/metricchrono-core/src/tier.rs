use crate::{MetricChronoError, Result};

/// One epsilon-delta-p comparator tier.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tier {
    pub epsilon: f64,
    pub delta: f64,
    pub p: f64,
    pub epsilon_ref: f64,
}

impl Tier {
    /// Construct and validate a tier.
    pub fn new(epsilon: f64, delta: f64, p: f64, epsilon_ref: f64) -> Result<Self> {
        let tier = Self {
            epsilon,
            delta,
            p,
            epsilon_ref,
        };
        tier.validate_at(0)?;
        Ok(tier)
    }

    /// Start a builder for APIs that prefer named setters.
    pub fn builder() -> TierBuilder {
        TierBuilder::default()
    }

    /// Gain term `(epsilon / epsilon_ref)^p`.
    pub fn gain(self) -> f64 {
        (self.epsilon / self.epsilon_ref).powf(self.p)
    }

    pub(crate) fn validate_at(self, index: usize) -> Result<()> {
        if !self.epsilon.is_finite() || self.epsilon <= 0.0 {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "epsilon must be finite and > 0",
            });
        }
        if !self.delta.is_finite() || self.delta <= 0.0 {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "delta must be finite and > 0",
            });
        }
        if self.epsilon >= self.delta {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "epsilon must be < delta",
            });
        }
        if !self.p.is_finite() {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "p must be finite",
            });
        }
        if !self.epsilon_ref.is_finite() || self.epsilon_ref <= 0.0 {
            return Err(MetricChronoError::InvalidTier {
                index,
                reason: "epsilon_ref must be finite and > 0",
            });
        }
        Ok(())
    }
}

/// Builder for [`Tier`].
#[derive(Clone, Copy, Debug)]
pub struct TierBuilder {
    epsilon: f64,
    delta: f64,
    p: f64,
    epsilon_ref: f64,
}

impl Default for TierBuilder {
    fn default() -> Self {
        Self {
            epsilon: 1.0,
            delta: 1.0,
            p: 0.5,
            epsilon_ref: 1.0,
        }
    }
}

impl TierBuilder {
    pub fn epsilon(mut self, value: f64) -> Self {
        self.epsilon = value;
        self
    }

    pub fn delta(mut self, value: f64) -> Self {
        self.delta = value;
        self
    }

    pub fn p(mut self, value: f64) -> Self {
        self.p = value;
        self
    }

    pub fn epsilon_ref(mut self, value: f64) -> Self {
        self.epsilon_ref = value;
        self
    }

    pub fn build(self) -> Result<Tier> {
        Tier::new(self.epsilon, self.delta, self.p, self.epsilon_ref)
    }
}
