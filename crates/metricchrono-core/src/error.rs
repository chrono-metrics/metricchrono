use std::fmt;

/// Result alias used by the public MetricChrono APIs.
pub type Result<T> = std::result::Result<T, MetricChronoError>;

/// Errors returned by safe MetricChrono APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetricChronoError {
    EmptyLadder,
    InvalidTier {
        index: usize,
        reason: &'static str,
    },
    OutputTooSmall {
        needed: usize,
        actual: usize,
    },
    ShapeMismatch {
        expected: usize,
        actual: usize,
        context: &'static str,
    },
    InvalidArgument(&'static str),
}

impl fmt::Display for MetricChronoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLadder => write!(f, "ladder must contain at least one tier"),
            Self::InvalidTier { index, reason } => {
                write!(f, "invalid tier at index {index}: {reason}")
            }
            Self::OutputTooSmall { needed, actual } => {
                write!(f, "output buffer too small: need {needed}, got {actual}")
            }
            Self::ShapeMismatch {
                expected,
                actual,
                context,
            } => write!(
                f,
                "shape mismatch for {context}: expected {expected}, got {actual}"
            ),
            Self::InvalidArgument(reason) => write!(f, "invalid argument: {reason}"),
        }
    }
}

impl std::error::Error for MetricChronoError {}
