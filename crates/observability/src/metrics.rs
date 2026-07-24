//! Cardinality-limited Prometheus-style metrics.
//!
//! Tenant, project, resource and job IDs must be attached to traces/logs, never
//! to metric labels.  The registry rejects reserved high-cardinality label names
//! and caps the total number of time series.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const RESERVED_LABELS: &[&str] = &[
    "tenant_id",
    "project_id",
    "user_id",
    "resource_id",
    "dataset_id",
    "dataset_version_id",
    "model_id",
    "model_version_id",
    "experiment_id",
    "training_job_id",
    "attempt_id",
    "operation_id",
    "replica_id",
    "deployment_id",
    "command_id",
    "generation",
];

const DEFAULT_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Errors returned by metric label/name validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricError {
    InvalidName(String),
    InvalidLabelName(String),
    InvalidLabelValue(String),
    ReservedLabel(String),
    CardinalityExceeded,
    InvalidValue(String),
}

impl std::fmt::Display for MetricError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(n) => write!(f, "invalid metric name: {n}"),
            Self::InvalidLabelName(n) => write!(f, "invalid label name: {n}"),
            Self::InvalidLabelValue(v) => write!(f, "invalid label value: {v}"),
            Self::ReservedLabel(n) => write!(f, "reserved label {n} cannot be used"),
            Self::CardinalityExceeded => write!(f, "metric cardinality limit exceeded"),
            Self::InvalidValue(v) => write!(f, "invalid metric value: {v}"),
        }
    }
}

impl std::error::Error for MetricError {}

#[derive(Debug)]
struct Inner {
    counters: HashMap<MetricKey, u64>,
    gauges: HashMap<MetricKey, f64>,
    histograms: HashMap<MetricKey, Histogram>,
    rejected: u64,
}

/// In-memory registry with Prometheus text exposition.
#[derive(Clone, Debug)]
pub struct MetricsRegistry {
    inner: Arc<Mutex<Inner>>,
    max_series: usize,
    max_label_value_len: usize,
    max_label_name_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MetricKey {
    name: String,
    labels: LabelSet,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LabelSet(Vec<(String, String)>);

impl LabelSet {
    fn new(pairs: &[(&str, &str)]) -> Self {
        let mut v: Vec<_> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        Self(v)
    }

    fn to_pairs(&self) -> Vec<(&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new(10_000, 256, 128)
    }
}

impl MetricsRegistry {
    pub fn new(max_series: usize, max_label_value_len: usize, max_label_name_len: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                counters: HashMap::new(),
                gauges: HashMap::new(),
                histograms: HashMap::new(),
                rejected: 0,
            })),
            max_series,
            max_label_value_len,
            max_label_name_len,
        }
    }

    fn validate(&self, name: &str, labels: &[(&str, &str)]) -> Result<(), MetricError> {
        validate_metric_name(name)?;
        let mut seen = std::collections::HashSet::new();
        for (k, v) in labels {
            validate_label_name(k, self.max_label_name_len)?;
            if !seen.insert(k.to_string()) {
                return Err(MetricError::InvalidLabelName(format!(
                    "duplicate label {k}"
                )));
            }
            if v.len() > self.max_label_value_len {
                return Err(MetricError::InvalidLabelValue(format!(
                    "label {k} value exceeds {max} bytes",
                    max = self.max_label_value_len
                )));
            }
            if !v.chars().all(|c| c.is_ascii() && c != '\n') {
                return Err(MetricError::InvalidLabelValue(format!(
                    "label {k} value contains non-ascii or newline"
                )));
            }
        }
        Ok(())
    }

    fn maybe_bump_rejected(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.rejected = inner.rejected.saturating_add(1);
    }

    /// Increment a counter by `n`.
    pub fn counter_inc(&self, name: &str, labels: &[(&str, &str)], n: u64) {
        if let Err(e) = self.record_counter(name, labels, n) {
            tracing::debug!(error = %e, "metric counter rejected");
            self.maybe_bump_rejected();
        }
    }

    fn record_counter(
        &self,
        name: &str,
        labels: &[(&str, &str)],
        n: u64,
    ) -> Result<(), MetricError> {
        self.validate(name, labels)?;
        let key = MetricKey {
            name: name.to_string(),
            labels: LabelSet::new(labels),
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.counters.contains_key(&key) {
            let series = inner
                .counters
                .len()
                .saturating_add(inner.gauges.len())
                .saturating_add(inner.histograms.len());
            if series >= self.max_series {
                return Err(MetricError::CardinalityExceeded);
            }
        }
        let entry = inner.counters.entry(key).or_insert(0);
        *entry = entry.saturating_add(n);
        Ok(())
    }

    /// Set a gauge to `value`.
    pub fn gauge_set(&self, name: &str, labels: &[(&str, &str)], value: f64) {
        if let Err(e) = self.record_gauge(name, labels, value) {
            tracing::debug!(error = %e, "metric gauge rejected");
            self.maybe_bump_rejected();
        }
    }

    fn record_gauge(
        &self,
        name: &str,
        labels: &[(&str, &str)],
        value: f64,
    ) -> Result<(), MetricError> {
        if !value.is_finite() {
            return Err(MetricError::InvalidValue(format!("{value}")));
        }
        self.validate(name, labels)?;
        let key = MetricKey {
            name: name.to_string(),
            labels: LabelSet::new(labels),
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.gauges.contains_key(&key) {
            let series = inner
                .counters
                .len()
                .saturating_add(inner.gauges.len())
                .saturating_add(inner.histograms.len());
            if series >= self.max_series {
                return Err(MetricError::CardinalityExceeded);
            }
        }
        inner.gauges.insert(key, value);
        Ok(())
    }

    /// Record an observation into a histogram.
    pub fn histogram_record(&self, name: &str, labels: &[(&str, &str)], value: f64) {
        if let Err(e) = self.record_histogram(name, labels, value) {
            tracing::debug!(error = %e, "metric histogram rejected");
            self.maybe_bump_rejected();
        }
    }

    fn record_histogram(
        &self,
        name: &str,
        labels: &[(&str, &str)],
        value: f64,
    ) -> Result<(), MetricError> {
        if !value.is_finite() {
            return Err(MetricError::InvalidValue(format!("{value}")));
        }
        self.validate(name, labels)?;
        let key = MetricKey {
            name: name.to_string(),
            labels: LabelSet::new(labels),
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.histograms.contains_key(&key) {
            let series = inner
                .counters
                .len()
                .saturating_add(inner.gauges.len())
                .saturating_add(inner.histograms.len());
            if series >= self.max_series {
                return Err(MetricError::CardinalityExceeded);
            }
        }
        let h = inner.histograms.entry(key).or_insert_with(|| Histogram::new(DEFAULT_BUCKETS));
        h.observe(value);
        Ok(())
    }

    /// Render all metrics in Prometheus exposition format.
    pub fn prometheus(&self) -> String {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = String::new();

        if !inner.counters.is_empty() {
            out.push_str("# HELP moqentra_counters_total Cardinality-limited counters.\n");
            out.push_str("# TYPE moqentra_counters_total counter\n");
        }
        for (key, value) in inner.counters.iter() {
            out.push_str(&format_line(&key.name, &key.labels, *value));
        }

        if !inner.gauges.is_empty() {
            out.push_str("# HELP moqentra_gauges_current Cardinality-limited gauges.\n");
            out.push_str("# TYPE moqentra_gauges_current gauge\n");
        }
        for (key, value) in inner.gauges.iter() {
            out.push_str(&format_line(&key.name, &key.labels, *value));
        }

        for (key, hist) in inner.histograms.iter() {
            out.push_str(&format!("# HELP {name} {name}\n", name = key.name));
            out.push_str(&format!("# TYPE {name} histogram\n", name = key.name));
            out.push_str(&hist.format(&key.name, &key.labels));
        }

        out.push_str("# HELP moqentra_metrics_rejected_total Metrics rejected due to validation or cardinality.\n");
        out.push_str("# TYPE moqentra_metrics_rejected_total counter\n");
        out.push_str(&format!(
            "moqentra_metrics_rejected_total {}\n",
            inner.rejected
        ));

        out
    }
}

#[derive(Debug)]
struct Histogram {
    buckets: Vec<(f64, u64)>,
    sum: f64,
    count: u64,
}

impl Histogram {
    fn new(bounds: &[f64]) -> Self {
        Self {
            buckets: bounds.iter().map(|b| (*b, 0u64)).collect(),
            sum: 0.0,
            count: 0,
        }
    }

    fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count = self.count.saturating_add(1);
        for (bound, count) in &mut self.buckets {
            if value <= *bound {
                *count = count.saturating_add(1);
                break;
            }
        }
    }

    fn format(&self, name: &str, labels: &LabelSet) -> String {
        let mut out = String::new();
        let base = label_string(labels);
        let mut cumulative = 0u64;
        for (bound, count) in &self.buckets {
            cumulative = cumulative.saturating_add(*count);
            let line = if base.is_empty() {
                format!(
                    "{name}_bucket{{le=\"{}\"}} {cumulative}\n",
                    format_f64(*bound)
                )
            } else {
                format!(
                    "{name}_bucket{{{base},le=\"{}\"}} {cumulative}\n",
                    format_f64(*bound)
                )
            };
            out.push_str(&line);
        }
        let inf_line = if base.is_empty() {
            format!("{name}_bucket{{le=\"+Inf\"}} {}\n", self.count)
        } else {
            format!("{name}_bucket{{{base},le=\"+Inf\"}} {}\n", self.count)
        };
        out.push_str(&inf_line);
        if base.is_empty() {
            out.push_str(&format!("{name}_sum {}\n", self.sum));
            out.push_str(&format!("{name}_count {}\n", self.count));
        } else {
            out.push_str(&format!("{name}_sum{{{base}}} {}\n", self.sum));
            out.push_str(&format!("{name}_count{{{base}}} {}\n", self.count));
        }
        out
    }
}

fn validate_metric_name(name: &str) -> Result<(), MetricError> {
    if name.is_empty() {
        return Err(MetricError::InvalidName("empty".into()));
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(MetricError::InvalidName("empty".into()));
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == ':') {
        return Err(MetricError::InvalidName(name.to_string()));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
        return Err(MetricError::InvalidName(name.to_string()));
    }
    Ok(())
}

fn validate_label_name(name: &str, max_len: usize) -> Result<(), MetricError> {
    if name.is_empty() || name.len() > max_len {
        return Err(MetricError::InvalidLabelName(name.to_string()));
    }
    if RESERVED_LABELS.iter().any(|r| name.eq_ignore_ascii_case(r)) {
        return Err(MetricError::ReservedLabel(name.to_string()));
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(MetricError::InvalidLabelName("empty".into()));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(MetricError::InvalidLabelName(name.to_string()));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') {
        return Err(MetricError::InvalidLabelName(name.to_string()));
    }
    Ok(())
}

fn format_line<T: std::fmt::Display>(name: &str, labels: &LabelSet, value: T) -> String {
    let labels = label_string(labels);
    if labels.is_empty() {
        format!("{name} {value}\n")
    } else {
        format!("{name}{{{labels}}} {value}\n")
    }
}

fn label_string(labels: &LabelSet) -> String {
    labels
        .to_pairs()
        .iter()
        .map(|(k, v)| format!("{k}=\"{}\"", escape_label_value(v)))
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_label_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn format_f64(v: f64) -> String {
    if v == v.trunc() {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_and_prometheus_output() {
        let r = MetricsRegistry::default();
        r.counter_inc("moqentra_test_total", &[("method", "GET")], 1);
        let out = r.prometheus();
        assert!(out.contains("moqentra_test_total{method=\"GET\"} 1"));
    }

    #[test]
    fn rejects_reserved_label() {
        let r = MetricsRegistry::default();
        r.counter_inc("x", &[("tenant_id", "t1")], 1);
        let out = r.prometheus();
        assert!(out.contains("moqentra_metrics_rejected_total 1"));
    }

    #[test]
    fn histogram_format() {
        let r = MetricsRegistry::default();
        r.histogram_record(
            "moqentra_http_request_duration_seconds",
            &[("method", "GET")],
            0.05,
        );
        let out = r.prometheus();
        assert!(out.contains(
            "moqentra_http_request_duration_seconds_bucket{method=\"GET\",le=\"0.1\"} 1"
        ));
        assert!(out.contains("moqentra_http_request_duration_seconds_count{method=\"GET\"} 1"));
    }
}
