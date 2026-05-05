use std::hint::black_box;
use std::time::{Duration, Instant};

use metricchrono_core::{
    coherence_residuals, geometric_ladder, ladder_distance, simple_weight_update,
    smooth_tick_distance, tick_distance, weighted_consensus, EventLog, SmoothParams, Tier,
};

fn main() {
    bench_single_tick();
    bench_ladders();
    bench_smooth_tick();
    bench_event_log();
    bench_consensus();
}

fn bench_single_tick() {
    let tier = Tier::new(0.001, 0.002, 0.5, 1.0).unwrap();
    let iterations = 1_000_000;
    let elapsed = time_loop(iterations, |index| {
        let distance = (index as f64 % 10_000.0) / 100_000.0;
        black_box(tick_distance(black_box(distance), tier));
    });
    report("single tick throughput", iterations, elapsed);
}

fn bench_ladders() {
    for tiers in [4, 8, 16, 32] {
        let ladder = geometric_ladder(0.001, 0.002, 1.7, tiers, 0.5, 1.0).unwrap();
        let mut out = vec![0.0; ladder.len()];
        let iterations = 250_000;
        let elapsed = time_loop(iterations, |index| {
            let distance = (index as f64 % 10_000.0) / 100.0;
            ladder_distance(black_box(distance), &ladder, &mut out).unwrap();
            black_box(&out);
        });
        report(
            &format!("ladder throughput {tiers} tiers"),
            iterations,
            elapsed,
        );
    }
}

fn bench_smooth_tick() {
    let tier = Tier::new(0.001, 0.002, 0.5, 1.0).unwrap();
    let params = SmoothParams::new(10.0, 10.0, 4096).unwrap();
    let iterations = 250_000;
    let elapsed = time_loop(iterations, |index| {
        let distance = (index as f64 % 10_000.0) / 100_000.0;
        black_box(smooth_tick_distance(black_box(distance), tier, params).unwrap());
    });
    report("smooth tick throughput", iterations, elapsed);
}

fn bench_event_log() {
    let iterations = 50_000;
    let started = Instant::now();
    let mut log = EventLog::new(8).unwrap();
    for index in 0..iterations {
        let active = (index % 7 == 0) as u8 as f64;
        let ticks = vec![active, 0.0, active, 0.0, 0.0, active, 0.0, 0.0];
        let inserted = log.append(index as u64, ticks).unwrap();
        black_box(log.next_event(inserted.saturating_sub(1), 0));
    }
    report("event-log append/next_event", iterations, started.elapsed());
    black_box(log.len());
}

fn bench_consensus() {
    let sources = 16;
    let tiers = 8;
    let vectors: Vec<Vec<f64>> = (0..sources)
        .map(|source| {
            (0..tiers)
                .map(|tier| ((source + 1) * (tier + 1)) as f64)
                .collect()
        })
        .collect();
    let refs: Vec<&[f64]> = vectors.iter().map(Vec::as_slice).collect();
    let mut weights = vec![1.0 / sources as f64; sources];
    let mut consensus = vec![0.0; tiers];
    let mut residuals = vec![0.0; sources];
    let iterations = 100_000;
    let elapsed = time_loop(iterations, |_| {
        weighted_consensus(&refs, &weights, &mut consensus).unwrap();
        coherence_residuals(&refs, &consensus, &mut residuals).unwrap();
        simple_weight_update(&mut weights, &residuals, 0.01, 0.001).unwrap();
        black_box((&consensus, &weights));
    });
    report("consensus 16 sources x 8 tiers", iterations, elapsed);
}

fn time_loop(iterations: usize, mut run: impl FnMut(usize)) -> Duration {
    let started = Instant::now();
    for index in 0..iterations {
        run(index);
    }
    started.elapsed()
}

fn report(label: &str, iterations: usize, elapsed: Duration) {
    let ns_per_eval = elapsed.as_nanos() as f64 / iterations as f64;
    println!("{label}: {iterations} iterations in {elapsed:?} ({ns_per_eval:.1} ns/eval)");
}
