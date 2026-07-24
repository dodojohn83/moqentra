//! LabelU native format import/export.
//!
//! This mirrors the JSON boundary defined by `apps/web/src/annotation/LabelUAdapter.ts`.

use moqentra_domain::annotation::{Annotation, AnnotationTask, AnnotationTaskState, TaskType};
use moqentra_domain::dataset::AssetRef;
use moqentra_types::{
    AnnotationId, AnnotationProjectId, AnnotationTaskId, AssetId, RandomIdGenerator, TenantId,
    UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single LabelU annotation object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LabelUAnnotation {
    pub id: String,
    #[serde(rename = "type")]
    pub annotation_type: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<Vec<f64>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A LabelU project configuration (one per annotation project).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs, non_snake_case)]
pub struct LabelUProjectConfig {
    pub version: String,
    pub mediaType: String,
    pub tools: Vec<LabelUToolConfig>,
}

/// A LabelU tool config entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LabelUToolConfig {
    pub tool: String,
    pub config: LabelUToolConfigEntry,
}

/// Inner config of a LabelU tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LabelUToolConfigEntry {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<LabelULabel>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A Label in LabelU.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LabelULabel {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// A LabelU native dataset document (project config + per-asset annotations).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LabelUDataset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<LabelUProjectConfig>,
    pub annotations: BTreeMap<String, Vec<LabelUAnnotation>>,
}

impl LabelUDataset {
    /// Export a project's tasks/annotations to LabelU native JSON.
    pub fn export(
        project_id: AnnotationProjectId,
        task_type: TaskType,
        tasks: &[AnnotationTask],
        annotations_by_task: &BTreeMap<AnnotationTaskId, Vec<Annotation>>,
        assets: &[AssetRef],
    ) -> Result<Self, moqentra_types::Error> {
        let config = Some(LabelUProjectConfig {
            version: "v1".to_string(),
            mediaType: task_type_to_media_type(task_type).to_string(),
            tools: vec![LabelUToolConfig {
                tool: task_type_to_tool(task_type).to_string(),
                config: LabelUToolConfigEntry {
                    label: "moqentra".to_string(),
                    color: None,
                    labels: None,
                    extra: BTreeMap::new(),
                },
            }],
        });

        let asset_by_id: BTreeMap<AssetId, &AssetRef> = assets.iter().map(|a| (a.id, a)).collect();
        let mut result = BTreeMap::new();
        for task in tasks {
            if task.project_id != project_id || task.asset_ids.len() != 1 {
                continue;
            }
            let asset_id = task.asset_ids.first().copied().ok_or_else(|| {
                moqentra_types::Error::internal("missing asset id after length check")
            })?;
            let asset = asset_by_id
                .get(&asset_id)
                .ok_or_else(|| moqentra_types::Error::not_found("asset for task"))?;
            let annotations =
                annotations_by_task.get(&task.id).map(|v| v.as_slice()).unwrap_or(&[]);
            let mut list = Vec::with_capacity(annotations.len());
            for a in annotations {
                list.push(annotation_to_labelu(a, task_type)?);
            }
            result.insert(asset.name.clone(), list);
        }

        Ok(LabelUDataset {
            config,
            annotations: result,
        })
    }

    /// Import LabelU native annotations into tasks.
    pub fn import(
        &self,
        project_id: AnnotationProjectId,
        tenant_id: TenantId,
        task_type: TaskType,
        assets: &[AssetRef],
    ) -> Result<Vec<(AnnotationTask, Vec<Annotation>)>, moqentra_types::Error> {
        let gen = RandomIdGenerator;
        let asset_by_name: BTreeMap<String, &AssetRef> =
            assets.iter().map(|a| (a.name.clone(), a)).collect();
        let mut pairs = Vec::with_capacity(self.annotations.len());

        for (file_name, labelu_annotations) in &self.annotations {
            let asset = asset_by_name
                .get(file_name)
                .ok_or_else(|| moqentra_types::Error::not_found(format!("asset {}", file_name)))?;
            let task_id = AnnotationTaskId::new_v7(&gen);
            let mut task = AnnotationTask::new(task_id, project_id, tenant_id, vec![asset.id])?;
            task.state = AnnotationTaskState::Approved;

            let mut annotations = Vec::with_capacity(labelu_annotations.len());
            for l in labelu_annotations {
                let payload = labelu_to_payload(l, task_type)?;
                let now = moqentra_types::UtcTimestamp::now();
                annotations.push(Annotation {
                    id: AnnotationId::new_v7(&gen),
                    task_id,
                    asset_id: asset.id,
                    revision: 0,
                    client_update_id: format!("labelu-import-{}", l.id),
                    actor_id: UserId::new_v7(&gen),
                    payload,
                    created_at: now,
                    updated_at: now,
                });
            }
            pairs.push((task, annotations));
        }

        Ok(pairs)
    }
}

fn task_type_to_media_type(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::VideoClassification | TaskType::ObjectTracking => "video",
        _ => "image",
    }
}

fn task_type_to_tool(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::ImageClassification | TaskType::VideoClassification => "imageClassification",
        TaskType::BoundingBox => "rectTool",
        TaskType::Polygon => "polygonTool",
        TaskType::KeyPoint => "pointTool",
        TaskType::ObjectTracking => "frameRectTool",
        TaskType::Relation => "lineTool",
    }
}

fn annotation_to_labelu(
    a: &Annotation,
    task_type: TaskType,
) -> Result<LabelUAnnotation, moqentra_types::Error> {
    let mut points = None;
    let mut extra = BTreeMap::new();

    match task_type {
        TaskType::BoundingBox => {
            if let Some(bbox) = a.payload.get("bbox").and_then(|v| v.as_array()) {
                let nums: Vec<f64> = bbox.iter().filter_map(|v| v.as_f64()).collect();
                if let [x, y, w, h] = nums.as_slice() {
                    points = Some(vec![*x, *y, *x + *w, *y + *h]);
                } else {
                    extra.insert("bbox".to_string(), serde_json::json!(nums));
                }
            }
        }
        TaskType::Polygon => {
            if let Some(poly) = a.payload.get("segmentation").or_else(|| a.payload.get("polygon")) {
                if let Some(arr) = poly.as_array() {
                    points = Some(arr.iter().filter_map(|v| v.as_f64()).collect());
                } else {
                    extra.insert("segmentation".to_string(), poly.clone());
                }
            }
        }
        _ => {}
    }

    let label = a
        .payload
        .get("category_name")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| a.payload.get("label").and_then(|v| v.as_str()).unwrap_or(""))
        .to_string();

    let annotation_type = task_type_to_tool(task_type).to_string();

    Ok(LabelUAnnotation {
        id: a.client_update_id.clone(),
        annotation_type,
        label,
        tool: Some(task_type_to_tool(task_type).to_string()),
        frame: None,
        points,
        extra,
    })
}

fn labelu_to_payload(
    l: &LabelUAnnotation,
    task_type: TaskType,
) -> Result<serde_json::Value, moqentra_types::Error> {
    let mut payload = serde_json::Map::new();
    payload.insert("label".to_string(), serde_json::json!(l.label));

    match task_type {
        TaskType::BoundingBox => {
            if let Some(points) = &l.points {
                if let [x1, y1, x2, y2, ..] = points.as_slice() {
                    payload.insert(
                        "bbox".to_string(),
                        serde_json::json!(vec![*x1, *y1, *x2 - *x1, *y2 - *y1]),
                    );
                } else {
                    payload.insert("bbox".to_string(), serde_json::json!(points));
                }
            }
        }
        TaskType::Polygon => {
            if let Some(points) = &l.points {
                payload.insert("polygon".to_string(), serde_json::json!(points));
            }
        }
        _ => {}
    }

    if l.frame.is_some() {
        payload.insert("frame".to_string(), serde_json::json!(l.frame));
    }
    for (k, v) in &l.extra {
        payload.insert(k.clone(), v.clone());
    }

    Ok(serde_json::Value::Object(payload))
}
