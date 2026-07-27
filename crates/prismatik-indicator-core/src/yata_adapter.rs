//! YATA adapter contracts.

use crate::{Bar, Indicator, IndicatorDescriptor, IndicatorError};

/// Contract for third-party indicators that can be adapted to canonical `Indicator`.
pub trait YataCompatibleIndicator: Send + Sync {
    /// Indicator descriptor.
    fn descriptor(&self) -> IndicatorDescriptor;
    /// Required warmup bars.
    fn warmup_len(&self) -> usize;
    /// Evaluate next value from current bar.
    fn next_value(&mut self, bar: &Bar) -> Result<f64, IndicatorError>;
}

/// Adapter that wraps a YATA-compatible implementation.
#[derive(Debug)]
pub struct YataAdapter<T: YataCompatibleIndicator> {
    inner: T,
    descriptor: IndicatorDescriptor,
}

impl<T: YataCompatibleIndicator> YataAdapter<T> {
    /// Create a new adapter.
    pub fn new(inner: T) -> Self {
        let descriptor = inner.descriptor();
        Self { inner, descriptor }
    }
}

impl<T: YataCompatibleIndicator> Indicator for YataAdapter<T> {
    fn descriptor(&self) -> &IndicatorDescriptor {
        &self.descriptor
    }

    fn warmup_len(&self) -> usize {
        self.inner.warmup_len()
    }

    fn evaluate(&self, _window: &[Bar]) -> Result<f64, IndicatorError> {
        Err(IndicatorError::InvalidInput(
            "evaluate requires mutable streaming state for YATA adapter".to_owned(),
        ))
    }

    fn next(&mut self, bar: &Bar) -> Result<f64, IndicatorError> {
        self.inner.next_value(bar)
    }
}
