use metricchrono_core::{ladder_distance, Tier};

fn main() {
    let (clock_accuracy, metricchrono_accuracy) = benchmark_accuracy();
    println!("clock-only accuracy: {clock_accuracy:.3}");
    println!("metricchrono accuracy: {metricchrono_accuracy:.3}");
    println!(
        "absolute lift: {:.3}",
        metricchrono_accuracy - clock_accuracy
    );
    assert!(
        metricchrono_accuracy > clock_accuracy + 0.45,
        "MetricChrono should beat the clock-only baseline on the deterministic regime-shift task"
    );
}

fn benchmark_accuracy() -> (f64, f64) {
    let ladder = [
        Tier::new(0.4, 0.4, 0.0, 1.0).unwrap(),
        Tier::new(0.8, 0.8, 0.0, 1.0).unwrap(),
        Tier::new(1.6, 1.6, 0.0, 1.0).unwrap(),
    ];

    let mut clock_correct = 0_usize;
    let mut metricchrono_correct = 0_usize;
    let samples = 240_usize;
    let mut ticks = [0.0; 3];

    for index in 0..samples {
        let label = index % 2 == 1;
        let clock_dt = 1.0_f64;
        let state_distance = if label { 1.8 } else { 0.1 };

        let clock_prediction = clock_dt > 1.0;
        if clock_prediction == label {
            clock_correct += 1;
        }

        ladder_distance(state_distance, &ladder, &mut ticks).unwrap();
        let metricchrono_prediction = ticks.iter().sum::<f64>() >= 2.0;
        if metricchrono_prediction == label {
            metricchrono_correct += 1;
        }
    }

    (
        clock_correct as f64 / samples as f64,
        metricchrono_correct as f64 / samples as f64,
    )
}
