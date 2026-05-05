# Limitations

MetricChrono is a measurement and change-coding layer.

- MetricChrono is non-additive.
- MetricChrono does not encode causality.
- MetricChrono does not preserve ordering by itself.
- MetricChrono is not a replacement for a physical clock.
- MetricChrono does not choose the right metric, threshold, or policy action by
  itself.
- MetricChrono does not prove safety, medical efficacy, financial performance,
  or robotics reliability.

Use clocks for elapsed physical time. Use causal models for causality. Use
planners and controllers for action. Use MetricChrono when you need compact,
deterministic evidence that a measured system changed in a metric space.
