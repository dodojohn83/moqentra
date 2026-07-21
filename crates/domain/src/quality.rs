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
    ) -> Result<Self, moqentra_types::Error> {
        let rule_version = rule_version.into();
        if rule_version.trim().is_empty() || rule_version.len() > 64 {
            return Err(moqentra_types::Error::invalid_argument(
                "rule_version must be non-empty and at most 64 characters",
            ));
        }
        if rules.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "quality run must have at least one rule",
            ));
        }
        for rule in &rules {
            match rule {
                QualityRule::DuplicateObjects { threshold } => {
                    if *threshold == 0 {
                        return Err(moqentra_types::Error::invalid_argument(
                            "DuplicateObjects threshold must be greater than zero",
                        ));
                    }
                }
                QualityRule::FrameRange { min, max } => {
                    if min > max {
                        return Err(moqentra_types::Error::invalid_argument(
                            "FrameRange min cannot exceed max",
                        ));
                    }
                }
                QualityRule::ClassDistribution {
                    min_count_per_class,
                } => {
                    if *min_count_per_class == 0 {
                        return Err(moqentra_types::Error::invalid_argument(
                            "ClassDistribution min_count_per_class must be greater than zero",
                        ));
                    }
                }
                QualityRule::SampleReview { ratio_permille } => {
                    if *ratio_permille == 0 || *ratio_permille > 1000 {
                        return Err(moqentra_types::Error::invalid_argument(
                            "SampleReview ratio_permille must be between 1 and 1000",
                        ));
                    }
                }
                _ => {}
            }
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            dataset_version_id,
            rule_version,
            seed,
            rules,
            state: QualityRunState::Pending,
            report: None,
            created_at: now,
            updated_at: now,
        })
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

    /// Evaluate annotations against configured rules.
    ///
    /// `asset_sizes` maps `asset_id -> (width, height)` and is required for
    /// accurate `OutOfBoundsBox` checks. When a size is missing, dimensions may
    /// still be read from the annotation payload (`image_width` / `image_height`).
    pub fn evaluate(
        &self,
        annotations: &BTreeMap<String, Vec<Annotation>>,
        asset_sizes: &BTreeMap<String, (u32, u32)>,
    ) -> QualityReport {
        let mut total_annotations = 0u64;
        let mut violations = Vec::new();
        let mut all_classes: BTreeMap<String, u64> = BTreeMap::new();

        for (asset_id, anns) in annotations {
            total_annotations += u64::try_from(anns.len()).unwrap_or(u64::MAX);
            for ann in anns {
                if let Some(label) = ann.payload.get("label").and_then(|v| v.as_str()) {
                    *all_classes.entry(label.to_string()).or_insert(0) += 1;
                }
                for rule in &self.rules {
                    if let Some(v) = self.check(rule, asset_id, ann, asset_sizes) {
                        violations.push(v);
                    }
                }
            }

            for rule in &self.rules {
                if let QualityRule::DuplicateObjects { threshold } = rule {
                    violations.extend(detect_duplicate_objects(asset_id, anns, *threshold));
                }
            }
        }

        for rule in &self.rules {
            match rule {
                QualityRule::ClassDistribution {
                    min_count_per_class,
                } => {
                    for (class, count) in &all_classes {
                        if *count < *min_count_per_class {
                            violations.push(QualityViolation {
                                severity: Severity::Warning,
                                asset_id: String::new(),
                                annotation_id: None,
                                rule: "ClassDistribution".to_string(),
                                message: format!("class '{}' has {} samples", class, count),
                                evidence: serde_json::json!({
                                    "class": class,
                                    "count": count,
                                    "min": min_count_per_class
                                }),
                            });
                        }
                    }
                }
                QualityRule::SampleReview { ratio_permille } => {
                    violations.extend(sample_review_violations(
                        annotations,
                        *ratio_permille,
                        self.seed,
                    ));
                }
                _ => {}
            }
        }

        QualityReport {
            total_assets: u64::try_from(annotations.len()).unwrap_or(u64::MAX),
            total_annotations,
            violations,
        }
    }

    fn check(
        &self,
        rule: &QualityRule,
        asset_id: &str,
        ann: &Annotation,
        asset_sizes: &BTreeMap<String, (u32, u32)>,
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
            QualityRule::OutOfBoundsBox => check_out_of_bounds_box(asset_id, ann, asset_sizes),
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
                let polygon = ann.payload.get("polygon")?;
                let points = parse_polygon_points(polygon)?;
                if points.len() < 3 {
                    return Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "SelfIntersectingPolygon".to_string(),
                        message: "polygon has fewer than 3 vertices".to_string(),
                        evidence: ann.payload.clone(),
                    });
                }
                if polygon_self_intersects(&points) {
                    Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "SelfIntersectingPolygon".to_string(),
                        message: "polygon self-intersects".to_string(),
                        evidence: ann.payload.clone(),
                    })
                } else {
                    None
                }
            }
            // Handled per-asset after scanning all annotations.
            QualityRule::DuplicateObjects { .. } => None,
            QualityRule::FrameRange { min, max } => {
                let frame_i = ann.payload.get("frame").and_then(|v| v.as_i64());
                if let Some(frame) = frame_i {
                    let frame_u = u64::try_from(frame).unwrap_or(0);
                    if frame < 0 || frame_u < *min || frame_u > *max {
                        return Some(QualityViolation {
                            severity: Severity::Error,
                            asset_id: asset_id.to_string(),
                            annotation_id: Some(ann.id.to_string()),
                            rule: "FrameRange".to_string(),
                            message: format!("frame {} outside [{}, {}]", frame, min, max),
                            evidence: ann.payload.clone(),
                        });
                    }
                }
                let frame_u = ann.payload.get("frame").and_then(|v| v.as_u64());
                if let Some(frame) = frame_u {
                    if frame < *min || frame > *max {
                        return Some(QualityViolation {
                            severity: Severity::Error,
                            asset_id: asset_id.to_string(),
                            annotation_id: Some(ann.id.to_string()),
                            rule: "FrameRange".to_string(),
                            message: format!("frame {} outside [{}, {}]", frame, min, max),
                            evidence: ann.payload.clone(),
                        });
                    }
                }
                if frame_i.is_none() && frame_u.is_none() {
                    return Some(QualityViolation {
                        severity: Severity::Error,
                        asset_id: asset_id.to_string(),
                        annotation_id: Some(ann.id.to_string()),
                        rule: "FrameRange".to_string(),
                        message: "frame value is not an integer".to_string(),
                        evidence: ann.payload.clone(),
                    });
                }
                None
            }
            QualityRule::SampleReview { .. } | QualityRule::ClassDistribution { .. } => None,
        }
    }
}

fn parse_bbox(payload: &Value) -> Option<[f64; 4]> {
    let bbox = payload.get("bbox")?.as_array()?;
    if bbox.len() < 4 {
        return None;
    }
    let x = bbox[0].as_f64()?;
    let y = bbox[1].as_f64()?;
    let w = bbox[2].as_f64()?;
    let h = bbox[3].as_f64()?;
    if ![x, y, w, h].into_iter().all(f64::is_finite) {
        return None;
    }
    Some([x, y, w, h])
}

fn resolve_image_size(
    asset_id: &str,
    ann: &Annotation,
    asset_sizes: &BTreeMap<String, (u32, u32)>,
) -> Option<(f64, f64)> {
    if let Some((w, h)) = asset_sizes.get(asset_id) {
        return Some((f64::from(*w), f64::from(*h)));
    }
    let w = ann
        .payload
        .get("image_width")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))?;
    let h = ann
        .payload
        .get("image_height")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|n| n as f64)))?;
    if w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite() {
        Some((w, h))
    } else {
        None
    }
}

fn check_out_of_bounds_box(
    asset_id: &str,
    ann: &Annotation,
    asset_sizes: &BTreeMap<String, (u32, u32)>,
) -> Option<QualityViolation> {
    let bbox_arr = ann.payload.get("bbox").and_then(|v| v.as_array())?;
    if bbox_arr.len() < 4 {
        return Some(QualityViolation {
            severity: Severity::Error,
            asset_id: asset_id.to_string(),
            annotation_id: Some(ann.id.to_string()),
            rule: "OutOfBoundsBox".to_string(),
            message: "bbox array too short".to_string(),
            evidence: ann.payload.clone(),
        });
    }
    let Some([x, y, w, h]) = parse_bbox(&ann.payload) else {
        return Some(QualityViolation {
            severity: Severity::Error,
            asset_id: asset_id.to_string(),
            annotation_id: Some(ann.id.to_string()),
            rule: "OutOfBoundsBox".to_string(),
            message: "bbox coordinates must be finite numbers".to_string(),
            evidence: ann.payload.clone(),
        });
    };
    if w <= 0.0 || h <= 0.0 {
        return Some(QualityViolation {
            severity: Severity::Error,
            asset_id: asset_id.to_string(),
            annotation_id: Some(ann.id.to_string()),
            rule: "OutOfBoundsBox".to_string(),
            message: "bbox width and height must be positive".to_string(),
            evidence: ann.payload.clone(),
        });
    }
    let Some((img_w, img_h)) = resolve_image_size(asset_id, ann, asset_sizes) else {
        return Some(QualityViolation {
            severity: Severity::Warning,
            asset_id: asset_id.to_string(),
            annotation_id: Some(ann.id.to_string()),
            rule: "OutOfBoundsBox".to_string(),
            message: "missing image dimensions for out-of-bounds check".to_string(),
            evidence: ann.payload.clone(),
        });
    };
    let right = x + w;
    let bottom = y + h;
    if x < 0.0 || y < 0.0 || right > img_w || bottom > img_h {
        return Some(QualityViolation {
            severity: Severity::Error,
            asset_id: asset_id.to_string(),
            annotation_id: Some(ann.id.to_string()),
            rule: "OutOfBoundsBox".to_string(),
            message: format!(
                "bbox [{}, {}, {}, {}] exceeds image bounds {}x{}",
                x, y, w, h, img_w, img_h
            ),
            evidence: ann.payload.clone(),
        });
    }
    None
}

/// Parse polygon as flat `[x,y,...]` or array of `[x,y]` / `{x,y}` points.
fn parse_polygon_points(value: &Value) -> Option<Vec<(f64, f64)>> {
    let arr = value.as_array()?;
    if arr.is_empty() {
        return Some(Vec::new());
    }
    if arr[0].as_array().is_some() || arr[0].as_object().is_some() {
        let mut points = Vec::with_capacity(arr.len());
        for p in arr {
            if let Some(pair) = p.as_array() {
                if pair.len() < 2 {
                    return None;
                }
                let x = pair[0].as_f64()?;
                let y = pair[1].as_f64()?;
                if !x.is_finite() || !y.is_finite() {
                    return None;
                }
                points.push((x, y));
            } else if let Some(obj) = p.as_object() {
                let x = obj.get("x")?.as_f64()?;
                let y = obj.get("y")?.as_f64()?;
                if !x.is_finite() || !y.is_finite() {
                    return None;
                }
                points.push((x, y));
            } else {
                return None;
            }
        }
        return Some(points);
    }
    if arr.len() % 2 != 0 {
        return None;
    }
    let mut points = Vec::with_capacity(arr.len() / 2);
    let mut i = 0;
    while i + 1 < arr.len() {
        let x = arr[i].as_f64()?;
        let y = arr[i + 1].as_f64()?;
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        points.push((x, y));
        i += 2;
    }
    Some(points)
}

fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.1 - a.1) * (c.0 - b.0) - (b.0 - a.0) * (c.1 - b.1)
}

fn on_segment(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> bool {
    b.0 <= a.0.max(c.0) && b.0 >= a.0.min(c.0) && b.1 <= a.1.max(c.1) && b.1 >= a.1.min(c.1)
}

fn segments_intersect(p1: (f64, f64), q1: (f64, f64), p2: (f64, f64), q2: (f64, f64)) -> bool {
    let o1 = orient(p1, q1, p2);
    let o2 = orient(p1, q1, q2);
    let o3 = orient(p2, q2, p1);
    let o4 = orient(p2, q2, q1);

    const EPS: f64 = 1e-9;
    let s1 = o1.abs() < EPS;
    let s2 = o2.abs() < EPS;
    let s3 = o3.abs() < EPS;
    let s4 = o4.abs() < EPS;

    if !s1 && !s2 && !s3 && !s4 {
        return (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0);
    }
    // Collinear special cases (shared endpoints of adjacent edges are ignored by caller).
    if s1 && on_segment(p1, p2, q1) {
        return true;
    }
    if s2 && on_segment(p1, q2, q1) {
        return true;
    }
    if s3 && on_segment(p2, p1, q2) {
        return true;
    }
    if s4 && on_segment(p2, q1, q2) {
        return true;
    }
    false
}

fn polygon_self_intersects(points: &[(f64, f64)]) -> bool {
    let n = points.len();
    if n < 4 {
        // Triangle cannot properly self-intersect without collinear overlap.
        return false;
    }
    for i in 0..n {
        let a1 = points[i];
        let a2 = points[(i + 1) % n];
        for j in (i + 1)..n {
            // Skip adjacent edges (including first/last which share a vertex).
            if j == i || (j + 1) % n == i || (i + 1) % n == j {
                continue;
            }
            let b1 = points[j];
            let b2 = points[(j + 1) % n];
            if segments_intersect(a1, a2, b1, b2) {
                return true;
            }
        }
    }
    false
}

fn bbox_iou(a: [f64; 4], b: [f64; 4]) -> f64 {
    let (ax, ay, aw, ah) = (a[0], a[1], a[2], a[3]);
    let (bx, by, bw, bh) = (b[0], b[1], b[2], b[3]);
    let ax2 = ax + aw;
    let ay2 = ay + ah;
    let bx2 = bx + bw;
    let by2 = by + bh;
    let ix1 = ax.max(bx);
    let iy1 = ay.max(by);
    let ix2 = ax2.min(bx2);
    let iy2 = ay2.min(by2);
    let iw = (ix2 - ix1).max(0.0);
    let ih = (iy2 - iy1).max(0.0);
    let inter = iw * ih;
    let union = aw * ah + bw * bh - inter;
    if union <= 0.0 || !union.is_finite() || !inter.is_finite() {
        return 0.0;
    }
    inter / union
}

fn detect_duplicate_objects(
    asset_id: &str,
    anns: &[Annotation],
    threshold: u64,
) -> Vec<QualityViolation> {
    // Pairwise IoU: a pair with IoU >= 0.9 counts as a duplicate pair.
    // Violation fires when the number of such pairs is >= threshold.
    const IOU_DUP: f64 = 0.9;
    let mut boxes: Vec<(usize, [f64; 4])> = Vec::new();
    for (idx, ann) in anns.iter().enumerate() {
        if let Some(b) = parse_bbox(&ann.payload) {
            if b[2] > 0.0 && b[3] > 0.0 {
                boxes.push((idx, b));
            }
        }
    }
    let mut dup_pairs = 0u64;
    let mut evidence_pairs = Vec::new();
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let iou = bbox_iou(boxes[i].1, boxes[j].1);
            if iou >= IOU_DUP {
                dup_pairs = dup_pairs.saturating_add(1);
                evidence_pairs.push(serde_json::json!({
                    "a": anns[boxes[i].0].id.to_string(),
                    "b": anns[boxes[j].0].id.to_string(),
                    "iou": iou,
                }));
            }
        }
    }
    if dup_pairs >= threshold && threshold > 0 {
        vec![QualityViolation {
            severity: Severity::Warning,
            asset_id: asset_id.to_string(),
            annotation_id: None,
            rule: "DuplicateObjects".to_string(),
            message: format!(
                "found {} near-duplicate pairs (threshold {})",
                dup_pairs, threshold
            ),
            evidence: serde_json::json!({
                "pairs": evidence_pairs,
                "threshold": threshold
            }),
        }]
    } else {
        Vec::new()
    }
}

fn sample_review_violations(
    annotations: &BTreeMap<String, Vec<Annotation>>,
    ratio_permille: u64,
    seed: u64,
) -> Vec<QualityViolation> {
    let mut asset_ids: Vec<&String> = annotations.keys().collect();
    asset_ids.sort();
    if asset_ids.is_empty() {
        return Vec::new();
    }
    let total = asset_ids.len() as u64;
    let mut sample_count = total.saturating_mul(ratio_permille) / 1000;
    if sample_count == 0 {
        sample_count = 1;
    }
    sample_count = sample_count.min(total);

    // Deterministic selection: hash(seed, asset_id) and take lowest N.
    let mut scored: Vec<(u64, &String)> = asset_ids
        .into_iter()
        .map(|id| {
            let mut h = seed;
            for b in id.as_bytes() {
                h = h.wrapping_mul(0x0000_0100_0000_01b3).wrapping_add(u64::from(*b));
            }
            (h, id)
        })
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    scored
        .into_iter()
        .take(sample_count as usize)
        .map(|(_, asset_id)| QualityViolation {
            severity: Severity::Info,
            asset_id: asset_id.clone(),
            annotation_id: None,
            rule: "SampleReview".to_string(),
            message: format!(
                "asset selected for sample review ({}‰, seed {})",
                ratio_permille, seed
            ),
            evidence: serde_json::json!({
                "ratio_permille": ratio_permille,
                "seed": seed
            }),
        })
        .collect()
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
    ) -> Result<Self, moqentra_types::Error> {
        if !(0.0..=1.0).contains(&confidence_threshold) {
            return Err(moqentra_types::Error::invalid_argument(
                "confidence_threshold must be finite and in [0, 1]",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            dataset_version_id,
            model_version_id,
            confidence_threshold,
            state: AutoLabelJobState::Pending,
            suggestions: Vec::new(),
            accepted_count: 0,
            created_at: now,
            updated_at: now,
        })
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
        for suggestion in &suggestions {
            if suggestion.label.trim().is_empty() {
                return Err(moqentra_types::Error::invalid_argument(
                    "suggestion label is empty",
                ));
            }
            if !(0.0..=1.0).contains(&suggestion.confidence) {
                return Err(moqentra_types::Error::invalid_argument(
                    "suggestion confidence must be finite and in [0, 1]",
                ));
            }
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
        if !matches!(self.state, AutoLabelJobState::Completed) {
            return Err(moqentra_types::Error::conflict(
                "auto-label job is not completed",
            ));
        }
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
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "rejection reason must be non-empty",
            ));
        }
        self.decision = ReviewDecision::Rejected;
        self.reason = Some(reason);
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
        )
        .unwrap();
        run.start().unwrap();
        let ann = make_annotation(None, json!({"bbox": [0.0, 0.0, 1.0, 1.0]}));
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![ann]);
        let sizes = BTreeMap::new();
        let report = run.evaluate(&annotations, &sizes);
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
        )
        .unwrap();
        run.start().unwrap();
        let ann = make_annotation(Some("dog"), json!({}));
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![ann]);
        let sizes = BTreeMap::new();
        let report = run.evaluate(&annotations, &sizes);
        run.complete(report.clone()).unwrap();
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "IllegalClass");
    }

    #[test]
    fn quality_run_detects_out_of_bounds_box() {
        let gen = RandomIdGenerator;
        let mut run = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            7,
            vec![QualityRule::OutOfBoundsBox],
        )
        .unwrap();
        run.start().unwrap();
        let ann = make_annotation(Some("cat"), json!({"bbox": [90.0, 90.0, 20.0, 20.0]}));
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![ann]);
        let mut sizes = BTreeMap::new();
        sizes.insert("asset1".to_string(), (100u32, 100u32));
        let report = run.evaluate(&annotations, &sizes);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "OutOfBoundsBox");
    }

    #[test]
    fn quality_run_detects_self_intersecting_polygon() {
        let gen = RandomIdGenerator;
        let mut run = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            7,
            vec![QualityRule::SelfIntersectingPolygon],
        )
        .unwrap();
        run.start().unwrap();
        // Classic bow-tie: (0,0)-(1,1)-(1,0)-(0,1)
        let ann = make_annotation(
            Some("poly"),
            json!({"polygon": [0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0]}),
        );
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![ann]);
        let report = run.evaluate(&annotations, &BTreeMap::new());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "SelfIntersectingPolygon");
    }

    #[test]
    fn quality_run_detects_duplicate_objects_by_iou() {
        let gen = RandomIdGenerator;
        let mut run = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            7,
            vec![QualityRule::DuplicateObjects { threshold: 1 }],
        )
        .unwrap();
        run.start().unwrap();
        let a = make_annotation(Some("cat"), json!({"bbox": [10.0, 10.0, 20.0, 20.0]}));
        // Nearly identical box — IoU well above the 0.9 duplicate threshold.
        let b = make_annotation(Some("cat"), json!({"bbox": [10.0, 10.0, 20.0, 19.5]}));
        let mut annotations = BTreeMap::new();
        annotations.insert("asset1".to_string(), vec![a, b]);
        let report = run.evaluate(&annotations, &BTreeMap::new());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "DuplicateObjects");
    }

    #[test]
    fn quality_run_sample_review_is_deterministic() {
        let gen = RandomIdGenerator;
        let rules = vec![QualityRule::SampleReview {
            ratio_permille: 500,
        }];
        let mut run_a = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            99,
            rules.clone(),
        )
        .unwrap();
        let mut run_b = QualityRun::new(
            QualityRunId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            "1.0",
            99,
            rules,
        )
        .unwrap();
        run_a.start().unwrap();
        run_b.start().unwrap();
        let mut annotations = BTreeMap::new();
        for name in ["a", "b", "c", "d"] {
            annotations.insert(
                name.to_string(),
                vec![make_annotation(Some("x"), json!({}))],
            );
        }
        let ra = run_a.evaluate(&annotations, &BTreeMap::new());
        let rb = run_b.evaluate(&annotations, &BTreeMap::new());
        assert_eq!(ra.violations.len(), 2);
        assert_eq!(
            ra.violations.iter().map(|v| v.asset_id.clone()).collect::<Vec<_>>(),
            rb.violations.iter().map(|v| v.asset_id.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn auto_label_job_state_machine() {
        let gen = RandomIdGenerator;
        let mut job = AutoLabelJob::new(
            AutoLabelJobId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            ModelVersionId::new_v7(&gen),
            0.5,
        )
        .unwrap();
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
