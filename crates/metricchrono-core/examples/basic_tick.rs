use metricchrono_core::{tick_distance, Tier};

fn main() -> metricchrono_core::Result<()> {
    let tier = Tier::new(0.1, 0.3, 0.5, 1.0)?;
    println!("{:.17}", tick_distance(0.2, tier));
    Ok(())
}
