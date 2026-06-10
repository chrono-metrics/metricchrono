use std::time::Instant;

use metricchrono_core::{CoverageMeter, Euclidean, Metric};

#[derive(Clone)]
struct Point([f64; 2]);

impl Metric<Point> for Euclidean {
    fn distance(&self, a: &Point, b: &Point) -> f64 {
        let dx = a.0[0] - b.0[0];
        let dy = a.0[1] - b.0[1];
        (dx * dx + dy * dy).sqrt()
    }
}

fn main() {
    // 8-tier geometric resolutions over a 2-D pseudo-random walk
    let epsilons: Vec<f64> = (0..8).map(|k| 0.05 * 2.0_f64.powi(k)).collect();
    let mut meter = CoverageMeter::from_epsilons(epsilons.clone()).unwrap();
    let metric = Euclidean;
    let iterations = 50_000_usize;
    let mut admitted = vec![false; epsilons.len()];

    let mut seed = 0x2545F4914F6CDD1D_u64;
    let mut uniform = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1_u64 << 53) as f64
    };

    let mut position = Point([0.0, 0.0]);
    let started = Instant::now();
    for _ in 0..iterations {
        position.0[0] += (uniform() - 0.5) * 0.2;
        position.0[1] += (uniform() - 0.5) * 0.2;
        meter
            .observe_into(&metric, &position, &mut admitted)
            .unwrap();
    }
    let elapsed = started.elapsed();
    let ns_per_observe = elapsed.as_nanos() as f64 / iterations as f64;
    println!("{iterations} coverage observations in {elapsed:?} ({ns_per_observe:.1} ns/observe)");
    println!(
        "final counts: {:?}, unique representatives: {}",
        meter.counts(),
        meter.unique_representatives()
    );
}
