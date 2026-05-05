use std::time::Instant;

use metricchrono_core::{geometric_ladder, ladder_distance};

fn main() {
    let ladder = geometric_ladder(0.001, 0.002, 1.7, 32, 0.5, 1.0).unwrap();
    let mut out = vec![0.0; ladder.len()];
    let iterations = 250_000;

    let started = Instant::now();
    for i in 0..iterations {
        let distance = (i as f64 % 10_000.0) / 100.0;
        ladder_distance(distance, &ladder, &mut out).unwrap();
    }
    let elapsed = started.elapsed();
    let ns_per_ladder = elapsed.as_nanos() as f64 / iterations as f64;
    println!("{iterations} ladder evaluations in {elapsed:?} ({ns_per_ladder:.1} ns/eval)");
}
