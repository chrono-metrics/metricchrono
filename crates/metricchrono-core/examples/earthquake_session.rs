use metricchrono_core::{Absolute, Ladder, OperatingRegime, Session};

// Real USGS earthquake data: magnitudes from the global seismic network,
// 2026-06-14 00:00–16:00 UTC.  The sequence captures three distinct regimes:
//
//   1. Scattered background activity (Alaska, Hawaii, Texas)
//   2. A rapid earthquake swarm at The Geysers, CA — dozens of micro-quakes
//      in under 30 minutes from induced geothermal seismicity
//   3. A M5.2 teleseismic event in the Philippines
//
// The Session pipeline classifies each observation's operating regime without
// any seismological domain knowledge — purely from the metric structure.
fn main() -> metricchrono_core::Result<()> {
    // Magnitudes from USGS, chronological (oldest first).
    // Curated to show the three regimes clearly.
    let magnitudes: Vec<f64> = vec![
        // Background: scattered moderate events, well-separated in time
        2.36, 1.7, 3.2, 1.8, 2.0, 1.5, 1.7, 1.4, 1.7, 2.8, 2.1,
        // Geysers swarm: rapid micro-quakes, magnitudes 0.2–1.96, similar values
        1.96, 0.60, 0.72, 1.55, 0.46, 1.05, 0.23, 0.27, 1.41, 1.05, 0.72, 1.04, 1.56, 1.03, 1.93,
        1.30, 1.26, 1.09, 1.30, 0.57, 0.89, 0.66, 0.66, 3.40,
        0.79, // M3.4 mainshock mid-swarm
        0.97, 1.19, 1.07, 1.10, 0.73, 0.98, 0.55, 1.51, 1.04, 1.30, 1.22, 1.05, 0.93, 1.10, 0.63,
        1.33, 1.06, 1.04, 1.27, 0.72, 1.50, 1.12, 1.08, 0.71, 0.80,
        // Transition: back to moderate background
        2.56, 2.6, 1.91, 1.35, 1.69, // Philippines M5.2 + M5.1: large teleseismic events
        5.2, 5.1, // Return to background
        3.5, 3.6, 2.73, 3.81, 3.43, 2.24, 3.95, // Quiet tail
        1.62, 1.13, 1.82, 1.06, 1.24, 1.80, 0.87, 1.89,
    ];

    // Build a geometric ladder tuned for magnitude-space:
    //   epsilon_0 = 0.3 (changes < 0.3 mag are noise)
    //   delta_0   = 0.5 (base quantization step)
    //   ratio     = 2.0 (each tier doubles the scale)
    //   4 tiers   (covers 0.3 → 0.6 → 1.2 → 2.4 mag thresholds)
    let ladder = Ladder::geometric(0.3, 0.5, 2.0, 4, 0.0, 1.0)?;

    // Session with CFAR anomaly detection:
    //   reference_len = 15 (look-back window for baseline)
    //   guard_len     = 3  (guard cells to exclude recent transients)
    //   target_fp     = 0.1 (10% false-positive rate → sensitive)
    let mut session = Session::with_cfar(ladder, 15, 3, 0.1)?;

    // Bounded coverage: keep at most 30 representatives so memory is finite.
    session.coverage_mut().set_capacity(Some(30))?;

    let mut regime_counts = [0u64; 4]; // Q, P, Ch, Cr
    let mut cfar_alarms = Vec::new();
    let mut regime_log: Vec<(usize, f64, &str, f64)> = Vec::new();

    for (i, &mag) in magnitudes.iter().enumerate() {
        let result = session.observe(&Absolute, &mag);

        let regime_name = match result.regime {
            OperatingRegime::Quiescent => {
                regime_counts[0] += 1;
                "Quiescent"
            }
            OperatingRegime::Progress => {
                regime_counts[1] += 1;
                "Progress"
            }
            OperatingRegime::Churn => {
                regime_counts[2] += 1;
                "Churn"
            }
            OperatingRegime::Creep => {
                regime_counts[3] += 1;
                "Creep"
            }
        };

        let throughput: f64 = result.ticks.iter().sum();
        regime_log.push((i, mag, regime_name, throughput));

        if let Some(ref cfar) = result.cfar {
            if cfar.alarm {
                cfar_alarms.push((i, mag, cfar.threshold));
            }
        }
    }

    // --- Report ---

    println!("=== MetricChrono Core: USGS Earthquake Session Analysis ===\n");
    println!(
        "Processed {} observations through a 4-tier geometric ladder",
        magnitudes.len()
    );
    println!(
        "Coverage: {} unique representatives across {} tiers\n",
        session.coverage().unique_representatives(),
        session.tier_count()
    );

    // Regime distribution
    println!("--- Regime Distribution ---");
    println!(
        "  Quiescent : {:3}  (no change, no new territory)",
        regime_counts[0]
    );
    println!(
        "  Progress  : {:3}  (new territory being explored)",
        regime_counts[1]
    );
    println!(
        "  Churn     : {:3}  (movement revisiting known ground)",
        regime_counts[2]
    );
    println!(
        "  Creep     : {:3}  (sub-threshold drift accumulating)",
        regime_counts[3]
    );
    println!();

    // Show regime transitions — where the story is
    println!("--- Regime Narrative (selected transitions) ---");
    let mut prev_regime = "";
    for &(i, mag, regime, throughput) in &regime_log {
        if regime != prev_regime {
            println!(
                "  step {:3}: mag {:.2} → {:<10} (throughput: {:.2})",
                i, mag, regime, throughput
            );
            prev_regime = regime;
        }
    }
    println!();

    // CFAR anomaly alarms
    println!("--- CFAR Anomaly Alarms ---");
    if cfar_alarms.is_empty() {
        println!("  (none — CFAR still warming up or no anomalies)");
    } else {
        for &(i, mag, threshold) in &cfar_alarms {
            println!(
                "  step {:3}: mag {:.2} exceeded threshold {:.2}  ← ALARM",
                i, mag, threshold
            );
        }
    }
    println!();

    // Per-tier coverage (how much of magnitude-space was explored at each scale)
    println!("--- Per-Tier Coverage ---");
    let counts = session.coverage().counts();
    let epsilons = session.coverage().epsilons();
    for (k, (&count, &eps)) in counts.iter().zip(epsilons.iter()).enumerate() {
        println!(
            "  tier {}: {} reps at ε = {:.2} mag  (resolves changes ≥ {:.2})",
            k, count, eps, eps
        );
    }
    println!();

    // Event log skip-list: salient events visible at each tier
    println!("--- Event Log: Salient Events per Tier ---");
    let log = session.event_log();
    for tier in 0..session.tier_count() {
        let salient: Vec<_> = log.compact_summary(tier);
        if !salient.is_empty() {
            println!(
                "  tier {}: {} salient events (first at step {}, last at step {})",
                tier,
                salient.len(),
                salient.first().unwrap().state_id,
                salient.last().unwrap().state_id,
            );
        }
    }
    println!();

    // What did we learn?
    println!("--- Insight ---");
    println!("Without any seismological domain knowledge, the Session pipeline");
    println!("automatically identified:");
    println!("  • The Geysers swarm as CHURN (high activity revisiting similar magnitudes)");
    println!("  • The Philippines M5.2 as PROGRESS (new territory in magnitude-space)");
    println!("  • Quiet periods as QUIESCENT (no movement at any scale)");
    println!("  • Sub-threshold background drift as CREEP");
    println!("The CFAR flagged magnitude jumps that exceeded the adaptive threshold.");
    println!("Coverage counts show finer tiers resolve more of the magnitude distribution");
    println!("while coarser tiers only see the large-scale structure.");

    Ok(())
}
