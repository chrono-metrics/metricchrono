# Event Log

Run:

```sh
cargo run -p metricchrono-core --example event_log
```

Expected output:

```text
2: s2 -> 1
3: s3 -> 1
[EventSummary { index: 2, state_id: "s2", tick: 1.0 }, EventSummary { index: 3, state_id: "s3", tick: 1.0 }]
```
