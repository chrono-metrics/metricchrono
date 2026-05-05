use metricchrono_core::EventLog;

fn main() -> metricchrono_core::Result<()> {
    let mut log = EventLog::new(2)?;
    log.append("s0", vec![0.0, 0.0])?;
    log.append("s1", vec![1.0, 0.0])?;
    log.append("s2", vec![1.0, 1.0])?;
    println!("{:?}", log.next_event(1, 0));
    Ok(())
}
