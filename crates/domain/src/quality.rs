//! Data quality, auto-annotation, review and multimodal extension domain.

use crate::annotation::Annotation;
use moqentra_types::{
    AnnotationId, AutoLabelJobId, DatasetVersionId, ModelVersionId, QualityRunId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Severity of a quality violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single quality rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityRule {
    MissingLabel,
    OutOfBoundsBox,
    SelfIntersectingPolygon,
    IllegalClass { allowed: BTreeSet<String> },
    DuplicateObjects { threshold: u64 },
    FrameRange { min: u64, max: u64 },
    ClassDistribution { min_count_per_class: u64 },
    SampleReview { ratio_permille: u64 },
}

/// A quality violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityViolation {
    pub severity: Severity,
    pub asset_id: String,
    pub annotation_id: Option<String>,
    pub rule: String,
    pub message: String,
    pub evidence: Value,
}

/// State of a quality run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityRunState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Immutable quality report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityReport {
    pub total_assets: u64,
    pub total_annotations: u64,
    pub violations: Vec<QualityViolation>,
}

/// A data quality run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityRun {
    pub id: QualityRunId,
    pub dataset_version_id: DatasetVersionId,
    pub rule_version: String,
    pub seed: u64,
    pub rules: Vec<QualityRule>,
    pub state: QualityRunState,
    pub report: Option<QualityReport>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl QualityRun {
    pub fn new(
        id: QualityRunId,
        dataset_version_id: DatasetVersionId,
        rule_version: impl Into<String>,
        seed: u64,
        rules: Vec<QualityRule>,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            dataset_version_id,
            rule_version: rule_version.into(),
            seed,
            rules,
            state: QualityRunState::Pending,
            report: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn start(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, QualityRunState::Pending) {
            return Err(moqentra_types::Error::conflict(
                "quality run cannot be started",
            ));
        }
        self.state = QualityRunState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn complete(&mut self, report: QualityReport) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, QualityRunState::Running) {
            return Err(moqentra_types::Error::conflict(
                "quality run is not running",
            ));
        }
        self.report = Some(report);
        self.state = QualityRunState::Completed;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn fail(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, QualityRunState::Running) {
            return Err(moqentra_types::Error::conflict(
                "quality run is not running",
            ));
        }
        self.state = QualityRunState::Failed;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn evaluate(&self, annotations: &BTreeMap<String, Vec<Annotation>>) -> QualityReport {
        let mut total_annotations = 0u64;
        let mut violations = Vec::new();
        let mut all_classes: BTreeMap<String, u64> = BTreeMap::new();

        for (asset_id, anns) in annotations {
            total_annotations += anns.len() as u64;
            for ann in anns {
                for rule in &self.rules {
                    if let Some(v) = self.check(rule, asset_id, ann) {
                        if let Some(label) = ann.payload.get("label").and_then(|v| v.as_str()) {
                            *all_classes.entry(label.to_string()).or_insert(0) += 1;
                        }
                        violations.push(v);
                    }
                }
            }
        }

        for rule in &self.rules {
            if let QualityRule::ClassDistribution {
                min_count_per_class,
            } = rule
            {
                for (class, count) in &all_classes {
                    if *count < *min_count_per_class {
                        violations.push(QualityViolation {
                            severity: Severity::Warning,
                            asset_id: String::new(),
                            annotation_id: None,
                            rule: "ClassDistribution".to_string(),
                            message: format!("class '{}' has {} samples", class, count),
                            evidence: serde_json::json!({"class": class, "count": count, "min": min_count_per_class}),
                        });
                    }
                }
            }
        }

        QualityReport {
            total_assets: annotations.len() as u64,
            total_annotations,
            violations,
        }
    }

    fn check(
        &self,
        rule: &QualityRule,
        asset_id: &str,
        ann: &Annotation,
    ) -> Option<QualityViolation> {
        match rule {
            QualityRule::MissingLabel => {
                if ann.payload.get("label").is_none() {
                    Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "MissingLabel".to_string(),
                        message: "annotation has no label".to_string(),
                        evidence: ann.payload.clone(),
                    })
                } else {
                    None
                }
            }
            QualityRule::OutOfBoundsBox => {
                let bbox = ann.payload.get("bbox").and_then(|v| v.as_array())?;
                if bbox.len() < 4 {
                    return Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "OutOfBoundsBox".to_string(),
                        message: "bbox array too short".to_string(),
                        evidence: ann.payload.clone(),
                    });
                }
                None
            }
            QualityRule::IllegalClass { allowed } => {
                let label = ann.payload.get("label").and_then(|v| v.as_str())?;
                if !allowed.contains(label) {
                    Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "IllegalClass".to_string(),
                        message: format!("label '{}' not in ontology", label),
                        evidence: ann.payload.clone(),
                    })
                } else {
                    None
                }
            }
            QualityRule::SelfIntersectingPolygon => {
                // Exact geometry check left to geometry crate; stub heuristic.
                if ann.payload.get("polygon").is_some()
                    && ann.payload.get("self_intersect").and_then(|v| v.as_bool()).unwrap_or(false)
                {
                    Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "SelfIntersectingPolygon".to_string(),
                        message: "polygon appears self-intersecting".to_string(),
                        evidence: ann.payload.clone(),
                    })
                } else {
                    None
                }
            }
            QualityRule::DuplicateObjects { threshold } => {
                // Simplified: flag if payload marks duplicate.
                if ann.payload.get("duplicate").and_then(|v| v.as_bool()).unwrap_or(false) {
                    Some(QualityViolation {
                        severity: Severity::Warning,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "DuplicateObjects".to_string(),
                        message: format!("duplicate object count exceeds threshold {}", threshold),
                        evidence: ann.payload.clone(),
                    })
                } else {
                    None
                }
            }
            QualityRule::FrameRange { min, max } => {
                let frame = ann.payload.get("frame").and_then(|v| v.as_u64())?;
                if frame < *min || frame > *max {
                    Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "FrameRange".to_string(),
                        message: format!("frame {} outside [{}, {}]", frame, min, max),
                        evidence: ann.payload.clone(),
                    })
                } else {
                    None
                }
            }
            QualityRule::SampleReview { .. } | QualityRule::ClassDistribution { .. } => None,
        }
    }
}

/// State of an auto-label job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoLabelJobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A suggestion produced by an auto-label job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoLabelSuggestion {
    pub id: AnnotationId,
    pub asset_id: String,
    pub label: String,
    pub confidence: f64,
    pub geometry: Value,
    pub model_version_id: ModelVersionId,
    pub inference_params: Value,
}

/// Auto-label job producing suggestion layer.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoLabelJob {
    pub id: AutoLabelJobId,
    pub dataset_version_id: DatasetVersionId,
    pub model_version_id: ModelVersionId,
    pub confidence_threshold: f64,
    pub state: AutoLabelJobState,
    pub suggestions: Vec<AutoLabelSuggestion>,
    pub accepted_count: u64,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl AutoLabelJob {
    pub fn new(
        id: AutoLabelJobId,
        dataset_version_id: DatasetVersionId,
        model_version_id: ModelVersionId,
        confidence_threshold: f64,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            dataset_version_id,
            model_version_id,
            confidence_threshold,
            state: AutoLabelJobState::Pending,
            suggestions: Vec::new(),
            accepted_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn start(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, AutoLabelJobState::Pending) {
            return Err(moqentra_types::Error::conflict(
                "auto-label job cannot be started",
            ));
        }
        self.state = AutoLabelJobState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn add_suggestions(
        &mut self,
        suggestions: Vec<AutoLabelSuggestion>,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, AutoLabelJobState::Running) {
            return Err(moqentra_types::Error::conflict(
                "auto-label job is not running",
            ));
        }
        self.suggestions.extend(suggestions);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, AutoLabelJobState::Running) {
            return Err(moqentra_types::Error::conflict(
                "auto-label job is not running",
            ));
        }
        self.state = AutoLabelJobState::Completed;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn accept(
        &mut self,
        suggestion_id: AnnotationId,
    ) -> Result<AutoLabelSuggestion, moqentra_types::Error> {
        let pos = self
            .suggestions
            .iter()
            .position(|s| s.id == suggestion_id)
            .ok_or_else(|| moqentra_types::Error::not_found("suggestion"))?;
        self.accepted_count += 1;
        self.updated_at = UtcTimestamp::now();
        Ok(self.suggestions.remove(pos))
    }
}

/// Review decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewDecision {
    Pending,
    Approved,
    Rejected,
}

/// Review queue entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItem {
    pub annotation_id: AnnotationId,
    pub annotator_id: moqentra_types::UserId,
    pub decision: ReviewDecision,
    pub reason: Option<String>,
    pub rework_task_id: Option<moqentra_types::AnnotationTaskId>,
    pub reviewed_at: Option<UtcTimestamp>,
}

impl ReviewItem {
    pub fn new(annotation_id: AnnotationId, annotator_id: moqentra_types::UserId) -> Self {
        Self {
            annotation_id,
            annotator_id,
            decision: ReviewDecision::Pending,
            reason: None,
            rework_task_id: None,
            reviewed_at: None,
        }
    }

    pub fn approve(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.decision, ReviewDecision::Pending) {
            return Err(moqentra_types::Error::conflict(
                "review item is not pending",
            ));
        }
        self.decision = ReviewDecision::Approved;
        self.reviewed_at = Some(UtcTimestamp::now());
        Ok(())
    }

    pub fn reject(
        &mut self,
        reason: impl Into<String>,
        rework_task_id: moqentra_types::AnnotationTaskId,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(self.decision, ReviewDecision::Pending) {
            return Err(moqentra_types::Error::conflict(
                "review item is not pending",
            ));
        }
        self.decision = ReviewDecision::Rejected;
        self.reason = Some(reason.into());
        self.rework_task_id = Some(rework_task_id);
        self.reviewed_at = Some(UtcTimestamp::now());
        Ok(())
    }
}

/// Multimodal asset metadata placeholders for audio, text and point cloud.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultimodalMeta {
    pub media_type: String,
    pub sample_rate: Option<u64>,
    pub duration_ms: Option<u64>,
    pub text_language: Option<String>,
    pub point_count: Option<u64>,
}

/// Unified annotation payload for image, video, audio, text and point-cloud.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultimodalAnnotation {
    pub time_range: Option<(f64, f64)>,
    pub frame_range: Option<(u64, u64)>,
    pub transcript: Option<String>,
    pub tokens: Vec<(u64, u64, String)>,
    pub spatial_indices: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Annotation;
    use moqentra_types::{
        AnnotationId, AnnotationTaskId, AssetId, DatasetVersionId, ModelVersionId,
        RandomIdGenerator, UserId,
    };
    use serde_json::json;

    fn make_annotation(label: Option<&str>, payload_extra: Value) -> Annotation {
        let gen = RandomIdGenerator;
        Annotation {
            id: AnnotationId::new_v7(&gen),
            task_id: AnnotationTaskId::new_v7(&gen),
            asset_id: AssetId::new_v7(&gen),
            revision: 1,
            client_update_id: "u1".to_string(),
            actor_id: UserId::new_v7(&gen),
            payload: if let Some(l) = label {
                let mut p = payload_extra.as_object().cloned().unwrap_or_default();
                p.insert("label".to_string(), json!(l));
                Value::Object(p)
            } else {
                payload_extra
            },
            created_at: UtcTimestamp::now(),
            updated_at: UtcTimestamp::now(),
        }
    }

    #[test]
    fn quality_run_detects_missing_label() {
        let gen = RandomIdGenerator;
        let mut run = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            42,
            vec![QualityRule::MissingLabel],
        );
        run.start().unwrap();
        let ann = make_annotation(None, json!({"bbox": [0.0, 0.0, 1.0, 1.0]}));
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![ann]);
        let report = run.evaluate(&annotations);
        run.complete(report.clone()).unwrap();
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "MissingLabel");
    }

    #[test]
    fn quality_run_detects_illegal_class() {
        let gen = RandomIdGenerator;
        let mut run = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            42,
            vec![QualityRule::IllegalClass {
                allowed: ["cat".to_string()].into_iter().collect(),
            }],
        );
        run.start().unwrap();
        let ann = make_annotation(Some("dog"), json!({}));
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![ann]);
        let report = run.evaluate(&annotations);
        run.complete(report.clone()).unwrap();
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "IllegalClass");
    }

    #[test]
    fn auto_label_job_state_machine() {
        let gen = RandomIdGenerator;
        let mut job = AutoLabelJob::new(
            AutoLabelJobId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            ModelVersionId::new_v7(&gen),
            0.5,
        );
        job.start().unwrap();
        let suggestion = AutoLabelSuggestion {
            id: AnnotationId::new_v7(&gen),
            asset_id: "a1".to_string(),
            label: "cat".to_string(),
            confidence: 0.9,
            geometry: json!({"bbox": [0.0, 0.0, 1.0, 1.0]}),
            model_version_id: job.model_version_id,
            inference_params: json!({"threshold": 0.5}),
        };
        job.add_suggestions(vec![suggestion.clone()]).unwrap();
        job.complete().unwrap();
        assert_eq!(job.suggestions.len(), 1);
    }

    #[test]
    fn review_item_reject_requires_reason_and_rework() {
        let gen = RandomIdGenerator;
        let mut item = ReviewItem::new(AnnotationId::new_v7(&gen), UserId::new_v7(&gen));
        item.reject("wrong label", AnnotationTaskId::new_v7(&gen)).unwrap();
        assert!(matches!(item.decision, ReviewDecision::Rejected));
        assert!(item.rework_task_id.is_some());
    }
}
