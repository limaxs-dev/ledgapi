//! Observability facade traits. Concrete implementations wrap `tracing`
//! and `metrics` crates in `infra` and `app`.

/// Counter label set. Cardinality is bounded by the spec — never put
/// user-controlled values into label keys.
pub trait MetricsRecorder: Send + Sync {
    /// Increment a counter.
    fn increment_counter(&self, name: &str, labels: &[(&'static str, &str)]);
    /// Record a duration observation in seconds.
    fn record_histogram(&self, name: &str, labels: &[(&'static str, &str)], value: f64);
}

/// Request span facade so `core` and `domain` never depend on `tracing`
/// directly.
pub trait RequestSpan: Send + Sync {
    /// Record a scalar attribute on the active span.
    fn record_str(&self, key: &'static str, value: &str);
    /// Record a numeric attribute on the active span.
    fn record_u64(&self, key: &'static str, value: u64);
}

/// No-op recorder used in tests and when no exporter is configured.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMetrics;

impl MetricsRecorder for NoopMetrics {
    fn increment_counter(&self, _name: &str, _labels: &[(&'static str, &str)]) {}
    fn record_histogram(&self, _name: &str, _labels: &[(&'static str, &str)], _value: f64) {}
}

/// No-op span used in tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSpan;

impl RequestSpan for NoopSpan {
    fn record_str(&self, _key: &'static str, _value: &str) {}
    fn record_u64(&self, _key: &'static str, _value: u64) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_recorder_compiles() {
        let r = NoopMetrics;
        r.increment_counter("foo", &[("method", "GET")]);
        r.record_histogram("foo", &[], 0.1);
    }

    #[test]
    fn noop_span_compiles() {
        let s = NoopSpan;
        s.record_str("key", "value");
        s.record_u64("count", 1);
    }
}