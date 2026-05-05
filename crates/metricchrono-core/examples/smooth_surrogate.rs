use metricchrono_core::{smooth_tick_distance, Tier};

fn main() -> metricchrono_core::Result<()> {
    let tier = Tier::new(1.0, 2.0, 0.5, 1.0)?;
    println!("{:.12}", smooth_tick_distance(0.95, tier, 10.0)?);
    Ok(())
}
