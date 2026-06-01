use metricchrono_core::{
    geometric_ladder, ladder_values, Euclidean, EventLog, Metric, PromotionCounter,
};

fn main() -> metricchrono_core::Result<()> {
    // A small geometric ladder: below tier-0 epsilon the whole vector is quiet;
    // above that, each tier gates independently against its own epsilon.
    let ladder = geometric_ladder(0.10, 0.50, 2.0, 4, 0.0, 0.10)?;
    let metric = Euclidean;

    // EventLog stores only salient states. PromotionCounter advances every
    // stream step: tiers accrue while quiet and promote after their quota of
    // quiet steps; events reset the tier.
    let mut log = EventLog::new(ladder.len())?;
    let mut carry = PromotionCounter::new(vec![3, 5, 7, 9])?;
    let mut promotions = vec![false; ladder.len()];

    // Quiet drift for several samples, one sharp jump, then quiet drift again.
    let stream = [
        [0.00, 0.00],
        [0.02, 0.01],
        [0.04, 0.02],
        [0.05, 0.03],
        [0.07, 0.04],
        [3.00, 4.00],
        [3.02, 4.01],
        [3.04, 4.02],
        [3.06, 4.03],
        [3.08, 4.04],
        [3.10, 4.05],
        [3.12, 4.06],
        [3.13, 4.07],
        [3.16, 4.08],
        [3.18, 4.09],
    ];

    // The driver owns the previous observation and advances it after each
    // comparison. This is the core streaming pattern.
    let mut previous = stream[0];

    println!("step distance ticks event_flags marker counters promotions");
    for (step, current) in stream.into_iter().enumerate().skip(1) {
        let distance = metric.distance(&previous[..], &current[..]);
        let ticks = ladder_values(distance, &ladder)?;
        let event_flags: Vec<bool> = ticks.iter().map(|tick| *tick > 0.0).collect();
        carry.step(Some(&event_flags), &mut promotions)?;

        // Salience is intentionally simple here: any positive tier tick marks
        // the current observation as an event candidate.
        let salient = event_flags.iter().any(|event| *event);
        let marker = if salient {
            log.append(step as u64, ticks.clone())?;
            "EVENT"
        } else {
            "quiet"
        };

        println!(
            "{step:>4} {distance:>8.3} {ticks:?} {event_flags:?} {marker:>5} {:?} {promotions:?}",
            carry.counters()
        );

        previous = current;
    }

    println!("events logged: {}", log.len());
    println!("tier-0 summary: {:?}", log.compact_summary(0));
    Ok(())
}
