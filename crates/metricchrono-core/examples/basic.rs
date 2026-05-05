use metricchrono_core::{
    adaptive_ladder_distance, coherence_residuals, geometric_ladder, ladder_pair,
    simple_weight_update, smooth_ladder_values, weighted_consensus, Euclidean, EventLog,
    PromotionCounter,
};

fn main() -> metricchrono_core::Result<()> {
    let ladder = geometric_ladder(0.5, 0.5, 2.0, 4, 0.5, 1.0)?;
    let metric = Euclidean;
    let ticks = ladder_pair(&[0.0, 0.0][..], &[3.0, 4.0][..], &metric, &ladder)?;
    println!("ticks: {ticks:?}");

    let smooth = smooth_ladder_values(3.0, &ladder, 10.0)?;
    println!("smooth: {smooth:?}");

    let mut early = vec![0.0; ladder.len()];
    let decision = adaptive_ladder_distance(0.75, &ladder, &mut early)?;
    println!("adaptive: {early:?} ({decision:?})");

    let mut log = EventLog::new(ladder.len())?;
    log.append(0_u64, vec![0.0, 0.0, 0.0, 0.0])?;
    log.append(1_u64, ticks.clone())?;
    println!("tier-0 summary: {:?}", log.compact_summary(0));

    let mut carry = PromotionCounter::new(vec![2, 3, 4, 5])?;
    let mut promotions = vec![false; carry.len()];
    carry.step(Some(&[false; 4]), &mut promotions)?;
    println!("promotions: {promotions:?}");

    let peer_a = ticks.as_slice();
    let peer_b = smooth.as_slice();
    let mut consensus = vec![0.0; ladder.len()];
    weighted_consensus(&[peer_a, peer_b], &[0.7, 0.3], &mut consensus)?;
    let mut residuals = vec![0.0; 2];
    coherence_residuals(&[peer_a, peer_b], &consensus, &mut residuals)?;
    let mut weights = vec![0.7, 0.3];
    simple_weight_update(&mut weights, &residuals, 0.1, 0.01)?;
    println!("consensus: {consensus:?}, residuals: {residuals:?}, weights: {weights:?}");

    Ok(())
}
