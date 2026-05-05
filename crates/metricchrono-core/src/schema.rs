use serde::{Deserialize, Serialize};

use crate::{custom_ladder, Ladder, Result, Tier};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TierDocument {
    pub metricchrono_schema: String,
    pub epsilon: f64,
    pub delta: f64,
    pub p: f64,
    pub epsilon_ref: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LadderDocument {
    pub metricchrono_schema: String,
    pub tiers: Vec<Tier>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TickVectorDocument {
    pub metricchrono_schema: String,
    pub ticks: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsensusResultDocument {
    pub metricchrono_schema: String,
    pub consensus: Vec<f64>,
    pub residuals: Vec<f64>,
    pub weights: Vec<f64>,
}

impl TierDocument {
    pub fn new(tier: Tier) -> Self {
        Self {
            metricchrono_schema: "tier.v1".to_owned(),
            epsilon: tier.epsilon,
            delta: tier.delta,
            p: tier.p,
            epsilon_ref: tier.epsilon_ref,
        }
    }

    pub fn into_tier(self) -> Result<Tier> {
        ensure_schema(&self.metricchrono_schema, "tier.v1")?;
        Tier::new(self.epsilon, self.delta, self.p, self.epsilon_ref)
    }
}

impl LadderDocument {
    pub fn new(tiers: Vec<Tier>) -> Result<Self> {
        custom_ladder(tiers.clone())?;
        Ok(Self {
            metricchrono_schema: "ladder.v1".to_owned(),
            tiers,
        })
    }

    pub fn into_ladder(self) -> Result<Ladder> {
        ensure_schema(&self.metricchrono_schema, "ladder.v1")?;
        Ladder::new(self.tiers)
    }
}

impl TickVectorDocument {
    pub fn new(ticks: Vec<f64>) -> Self {
        Self {
            metricchrono_schema: "tick_vector.v1".to_owned(),
            ticks,
        }
    }
}

impl ConsensusResultDocument {
    pub fn new(consensus: Vec<f64>, residuals: Vec<f64>, weights: Vec<f64>) -> Self {
        Self {
            metricchrono_schema: "consensus_result.v1".to_owned(),
            consensus,
            residuals,
            weights,
        }
    }
}

fn ensure_schema(actual: &str, expected: &'static str) -> Result<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(crate::MetricChronoError::InvalidArgument(
            "unsupported schema version",
        ))
    }
}
