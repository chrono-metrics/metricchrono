use metricchrono_core::{coherence_residuals, simple_weight_update, weighted_consensus};

fn main() -> metricchrono_core::Result<()> {
    let a = [1.0, 2.0];
    let b = [3.0, 0.0];
    let mut consensus = [0.0; 2];
    weighted_consensus(&[&a, &b], &[0.25, 0.75], &mut consensus)?;
    let mut residuals = [0.0; 2];
    coherence_residuals(&[&a, &b], &consensus, &mut residuals)?;
    let mut weights = [0.5, 0.5];
    simple_weight_update(&mut weights, &residuals, 0.2, 0.01)?;
    println!("consensus={consensus:?} weights={weights:?}");
    Ok(())
}
