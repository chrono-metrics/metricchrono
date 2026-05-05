# Basic Calibration

The open-source repository intentionally keeps calibration basic. A practical
starting workflow is:

1. Choose a domain metric that is meaningful for the states being compared.
2. Estimate a noise floor from quiet historical samples.
3. Set `epsilon` above the noise floor.
4. Set `delta` to the smallest active change size that should produce a new
   stair.
5. Choose `p = 0` for unweighted stair counts or a positive/negative `p` when
   scale-dependent gain is required.
6. Validate the ladder against golden or historical cases before using ticks in
   downstream decisions.

Advanced auto-calibration, task-loss calibration, drift-aware recalibration,
and calibration reports are enterprise features.
