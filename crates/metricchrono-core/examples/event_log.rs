use metricchrono_core::EventLog;

fn main() -> metricchrono_core::Result<()> {
    let mut log = EventLog::new(2)?;

    // Append-per-timestamp contract: call append once per observation,
    // including quiet all-zero records.
    // Events are the positive-tick subset; first_event(tier) is the chain head.
    log.append("s0", vec![0.0, 0.0])?;
    log.append("s1", vec![0.0, 1.0])?;
    log.append("s2", vec![1.0, 0.0])?;
    log.append("s3", vec![1.0, 1.0])?;

    let tier = 0;
    let mut next = log.first_event(tier);
    while let Some(index) = next {
        let record = log
            .record(index)
            .expect("event chain contains only valid record ids");
        println!("{index}: {} -> {}", record.state_id, record.ticks[tier]);
        next = log.next_event(index, tier);
    }

    println!("{:?}", log.compact_summary(tier));
    Ok(())
}
