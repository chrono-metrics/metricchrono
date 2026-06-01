# Basic Calibration

The open-source repository intentionally keeps calibration basic. A practical
starting workflow is:

1. Choose a domain metric that is meaningful for the states being compared.
2. Estimate a noise floor from quiet historical samples.
3. Set `epsilon` above the noise floor.
4. Set `delta` to the smallest active change size that should produce a new
   stair.
5. Choose `p = 0` for unweighted stair counts or a positive/negative `p` when
   scale-dependent gain is required.
6. Validate the ladder against golden or historical cases before using ticks in
   downstream decisions.

## From quiet samples to a starting ladder

This is a basic worked starting point, meant to be pasted into a small Rust
program and then validated against domain examples:

```rust
use metricchrono_core::geometric_ladder;

fn main() -> metricchrono_core::Result<()> {
    let quiet = [0.018, 0.021, 0.016, 0.024, 0.019, 0.022];
    let mean = quiet.iter().copied().sum::<f64>() / quiet.len() as f64;
    // Population variance over the quiet calibration window.
    let variance = quiet.iter().map(|d| (*d - mean).powi(2)).sum::<f64>() / quiet.len() as f64;
    let noise_floor = mean + 3.0 * variance.sqrt();
    let epsilon0 = noise_floor;
    let delta0 = 2.0 * epsilon0;
    let ladder = geometric_ladder(epsilon0, delta0, 2.0, 4, 0.0, epsilon0)?;
    println!("epsilon0={epsilon0:.4} delta0={delta0:.4} tiers={}", ladder.len());
    Ok(())
}
```

Advanced auto-calibration, task-loss calibration, drift-aware recalibration,
and calibration reports are enterprise features.
