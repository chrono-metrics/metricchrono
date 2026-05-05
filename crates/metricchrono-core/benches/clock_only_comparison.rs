use metricchrono_core::{ladder_distance, Tier};

#[derive(Clone, Copy)]
struct Sample {
    clock_dt: f64,
    state_distance: f64,
    salient: bool,
}

fn main() {
    let samples = synthetic_regime_samples();
    let clock_accuracy =
        best_threshold_accuracy(samples.iter().map(|sample| sample.clock_dt), &samples);
    let metricchrono_accuracy = metricchrono_accuracy(&samples);

    println!("samples: {}", samples.len());
    println!("clock-only best-threshold balanced accuracy: {clock_accuracy:.3}");
    println!("metricchrono balanced accuracy: {metricchrono_accuracy:.3}");
    println!(
        "balanced-accuracy lift: {:.3}",
        metricchrono_accuracy - clock_accuracy
    );
    assert!(
        metricchrono_accuracy > clock_accuracy + 0.35,
        "MetricChrono should beat the clock-only baseline on the synthetic regime-shift guardrail"
    );
    assert!(metricchrono_accuracy >= 0.90);
}

fn metricchrono_accuracy(samples: &[Sample]) -> f64 {
    let ladder = [
        Tier::new(0.35, 0.50, 0.0, 1.0).unwrap(),
        Tier::new(0.75, 1.00, 0.0, 1.0).unwrap(),
        Tier::new(1.35, 1.80, 0.0, 1.0).unwrap(),
    ];
    let mut ticks = [0.0; 3];
    let scores = samples.iter().map(|sample| {
        ladder_distance(sample.state_distance, &ladder, &mut ticks).unwrap();
        ticks.iter().sum::<f64>()
    });
    best_threshold_accuracy(scores, samples)
}

fn synthetic_regime_samples() -> Vec<Sample> {
    let mut rng = Lcg::new(0x5eed_1234_5678_9abc);
    (0..512)
        .map(|index| {
            let phase = index % 128;
            let burst = (30..38).contains(&phase) || (92..98).contains(&phase);
            let drift = (64..92).contains(&phase);
            let salient = burst || (drift && index % 5 == 0);
            let baseline = if salient {
                if burst {
                    1.55
                } else {
                    0.95
                }
            } else if drift {
                0.42
            } else {
                0.18
            };
            let state_noise = rng.centered(0.16);
            let clock_jitter = rng.centered(0.08);
            Sample {
                clock_dt: 1.0 + clock_jitter,
                state_distance: (baseline + state_noise).max(0.0),
                salient,
            }
        })
        .collect()
}

fn best_threshold_accuracy(scores: impl Iterator<Item = f64>, samples: &[Sample]) -> f64 {
    let scores: Vec<f64> = scores.collect();
    let mut thresholds = scores.clone();
    thresholds.sort_by(f64::total_cmp);
    thresholds.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);

    thresholds
        .into_iter()
        .map(|threshold| balanced_accuracy(&scores, samples, threshold))
        .fold(0.0, f64::max)
}

fn balanced_accuracy(scores: &[f64], samples: &[Sample], threshold: f64) -> f64 {
    let mut tp = 0_usize;
    let mut tn = 0_usize;
    let mut pos = 0_usize;
    let mut neg = 0_usize;

    for (score, sample) in scores.iter().copied().zip(samples) {
        if sample.salient {
            pos += 1;
            if score >= threshold {
                tp += 1;
            }
        } else {
            neg += 1;
            if score < threshold {
                tn += 1;
            }
        }
    }

    let tpr = tp as f64 / pos.max(1) as f64;
    let tnr = tn as f64 / neg.max(1) as f64;
    0.5 * (tpr + tnr)
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn centered(&mut self, width: f64) -> f64 {
        width * (2.0 * self.next_unit() - 1.0)
    }

    fn next_unit(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.state >> 11) as f64) / ((1_u64 << 53) as f64)
    }
}
