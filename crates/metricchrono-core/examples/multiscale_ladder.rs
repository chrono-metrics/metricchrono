use metricchrono_core::{custom_ladder, ladder_values, Tier};

fn main() -> metricchrono_core::Result<()> {
    let ladder = custom_ladder(vec![
        Tier::new(0.03, 0.10, 0.0, 1.0)?,
        Tier::new(0.10, 0.30, 0.0, 1.0)?,
        Tier::new(0.30, 0.90, 0.0, 1.0)?,
    ])?;
    println!("{:?}", ladder_values(1.0, &ladder)?);
    Ok(())
}
