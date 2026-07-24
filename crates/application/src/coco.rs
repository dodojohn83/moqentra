//! COCO format import/export for annotation projects.
//!
//! Supports image-level categories, bounding boxes and polygons.  Segmentation
//! masks and keypoints are represented as placeholders so the round-trip does
//! not silently lose information.

#![allow(missing_docs)]

use moqentra_domain::annotation::{Annotation, AnnotationTask, AnnotationTaskState, TaskType};
use moqentra_domain::dataset::AssetRef;
use moqentra_types::{
    AnnotationId, AnnotationProjectId, AnnotationTaskId, AssetId, RandomIdGenerator, TenantId,
    UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// COCO image entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoImage {
    pub id: u64,
    pub file_name: String,
    pub width: u64,
    pub height: u64,
}

/// COCO category entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoCategory {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supercategory: Option<String>,
}

/// COCO annotation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoAnnotation {
    pub id: u64,
    pub image_id: u64,
    pub category_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segmentation: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keypoints: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iscrowd: Option<u64>,
}

/// A COCO dataset document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CocoDataset {
    pub images: Vec<CocoImage>,
    pub categories: Vec<CocoCategory>,
    pub annotations: Vec<CocoAnnotation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
}

impl CocoDataset {
    /// Create an empty COCO document.
    pub fn empty() -> Self {
        Self {
            images: Vec::new(),
            categories: Vec::new(),
            annotations: Vec::new(),
            info: None,
        }
    }

    /// Export a set of annotation tasks and their logs to COCO.
    ///
    /// Each exported task must annotate exactly one asset.  Image dimensions are
    /// read from `assets` when the asset id is present, otherwise left as zero.
    pub fn export(
        project_id: AnnotationProjectId,
        task_type: TaskType,
        tasks: &[AnnotationTask],
        annotations_by_task: &BTreeMap<AnnotationTaskId, Vec<Annotation>>,
        assets: &[AssetRef],
    ) -> Result<Self, moqentra_types::Error> {
        let mut images = Vec::with_capacity(tasks.len());
        let mut image_id_by_asset: BTreeMap<AssetId, u64> = BTreeMap::new();
        let mut next_image_id: u64 = 1;

        for task in tasks {
            if task.project_id != project_id {
                continue;
            }
            if task.asset_ids.len() != 1 {
                return Err(moqentra_types::Error::invalid_argument(
                    "COCO export supports one asset per task",
                ));
            }
            let asset_id = task.asset_ids[0];
            let asset = assets
                .iter()
                .find(|a| a.id == asset_id)
                .ok_or_else(|| moqentra_types::Error::not_found("asset for task"))?;

            let image_id = next_image_id;
            next_image_id += 1;
            image_id_by_asset.insert(asset_id, image_id);
            let (width, height) = match parse_media_dimensions(asset) {
                Some((w, h)) => (w, h),
                None => (0, 0),
            };
            images.push(CocoImage {
                id: image_id,
                file_name: asset.name.clone(),
                width,
                height,
            });
        }

        // Build categories from ontology labels.  This crate does not depend on
        // domain ontology internals, so callers must supply categories.
        let categories = Vec::new();
        let mut annotations: Vec<CocoAnnotation> = Vec::new();
        let mut next_annotation_id: u64 = 1;

        for task in tasks {
            if task.project_id != project_id {
                continue;
            }
            let asset_id = task.asset_ids[0];
            let image_id = image_id_by_asset[&asset_id];
            let task_annotations =
                annotations_by_task.get(&task.id).map(|v| v.as_slice()).unwrap_or(&[]);
            for a in task_annotations {
                let mut coco = payload_to_coco(&a.payload, task_type)?;
                coco.id = next_annotation_id;
                next_annotation_id += 1;
                coco.image_id = image_id;
                annotations.push(coco);
            }
        }

        Ok(CocoDataset {
            images,
            categories,
            annotations,
            info: Some(serde_json::json!({
                "description": "Exported from Moqentra annotation project",
                "project_id": project_id.to_string(),
            })),
        })
    }

    /// Import COCO annotations for an active project.
    ///
    /// Returns a list of (task, annotations) pairs.  One task is created per
    /// COCO image that maps to an asset in `assets`.  `image_id` values are
    /// mapped to the order of `assets`; if an explicit `id` does not match, the
    /// `file_name` is used as fallback.
    pub fn import(
        &self,
        project_id: AnnotationProjectId,
        tenant_id: TenantId,
        task_type: TaskType,
        assets: &[AssetRef],
    ) -> Result<Vec<(AnnotationTask, Vec<Annotation>)>, moqentra_types::Error> {
        let gen = RandomIdGenerator;
        let mut asset_by_name: BTreeMap<String, &AssetRef> = BTreeMap::new();
        let mut asset_by_id: BTreeMap<u64, &AssetRef> = BTreeMap::new();
        for (idx, a) in assets.iter().enumerate() {
            asset_by_name.insert(a.name.clone(), a);
            let id = u64::try_from(idx)
                .map_err(|_| moqentra_types::Error::internal("asset index overflow"))?
                + 1;
            asset_by_id.insert(id, a);
        }

        let mut image_to_asset: BTreeMap<u64, &AssetRef> = BTreeMap::new();
        for img in &self.images {
            let asset = asset_by_id
                .get(&img.id)
                .or_else(|| asset_by_name.get(&img.file_name))
                .copied()
                .ok_or_else(|| {
                    moqentra_types::Error::not_found(format!("asset for image {}", img.id))
                })?;
            image_to_asset.insert(img.id, asset);
        }

        let mut category_by_id: BTreeMap<u64, &CocoCategory> = BTreeMap::new();
        for c in &self.categories {
            category_by_id.insert(c.id, c);
        }

        let mut annotations_by_image: BTreeMap<u64, Vec<&CocoAnnotation>> = BTreeMap::new();
        for a in &self.annotations {
            annotations_by_image.entry(a.image_id).or_default().push(a);
        }

        let mut result = Vec::with_capacity(image_to_asset.len());
        let actor = UserId::new_v7(&gen); // Placeholder; callers can rewrite actor_id.

        for (image_id, asset) in image_to_asset {
            let task_id = AnnotationTaskId::new_v7(&gen);
            let mut task = AnnotationTask::new(task_id, project_id, tenant_id, vec![asset.id])?;
            task.state = AnnotationTaskState::Approved;

            let coco_annotations =
                annotations_by_image.get(&image_id).map(|v| v.as_slice()).unwrap_or(&[]);
            let mut annotations = Vec::with_capacity(coco_annotations.len());
            for coco in coco_annotations {
                let category = category_by_id
                    .get(&coco.category_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                let payload = coco_to_payload(coco, task_type, &category)?;
                let now = moqentra_types::UtcTimestamp::now();
                annotations.push(Annotation {
                    id: AnnotationId::new_v7(&gen),
                    task_id,
                    asset_id: asset.id,
                    revision: 0,
                    client_update_id: format!("coco-import-{}", coco.id),
                    actor_id: actor,
                    payload,
                    created_at: now,
                    updated_at: now,
                });
            }
            result.push((task, annotations));
        }

        Ok(result)
    }
}

fn parse_media_dimensions(asset: &AssetRef) -> Option<(u64, u64)> {
    if let Some(meta) = asset.metadata.as_object() {
        if let (Some(w), Some(h)) = (meta.get("width"), meta.get("height")) {
            if let (Some(w), Some(h)) = (w.as_u64(), h.as_u64()) {
                return Some((w, h));
            }
        }
    }
    None
}

fn payload_to_coco(
    payload: &serde_json::Value,
    task_type: TaskType,
) -> Result<CocoAnnotation, moqentra_types::Error> {
    let mut coco = CocoAnnotation {
        id: 0,
        image_id: 0,
        category_id: 0,
        bbox: None,
        segmentation: None,
        keypoints: None,
        area: None,
        iscrowd: Some(0),
    };

    match task_type {
        TaskType::BoundingBox => {
            if let Some(bbox) = payload.get("bbox").and_then(|v| v.as_array()) {
                let nums: Vec<f64> = bbox.iter().filter_map(|v| v.as_f64()).collect();
                if nums.len() == 4 {
                    coco.bbox = Some(nums);
                }
            }
        }
        TaskType::Polygon => {
            if let Some(points) = payload.get("polygon").or_else(|| payload.get("segmentation")) {
                coco.segmentation = Some(points.clone());
            }
        }
        TaskType::KeyPoint => {
            if let Some(kps) = payload.get("keypoints") {
                coco.keypoints = Some(kps.clone());
            }
        }
        _ => {}
    }

    if let Some(label) = payload.get("label").and_then(|v| v.as_u64()) {
        coco.category_id = label;
    }

    Ok(coco)
}

fn coco_to_payload(
    coco: &CocoAnnotation,
    task_type: TaskType,
    category_name: &str,
) -> Result<serde_json::Value, moqentra_types::Error> {
    let mut payload = serde_json::Map::new();
    payload.insert("label".to_string(), serde_json::json!(coco.category_id));
    if !category_name.is_empty() {
        payload.insert(
            "category_name".to_string(),
            serde_json::json!(category_name),
        );
    }

    match task_type {
        TaskType::BoundingBox => {
            if let Some(bbox) = &coco.bbox {
                payload.insert("bbox".to_string(), serde_json::json!(bbox));
            }
        }
        TaskType::Polygon => {
            if let Some(seg) = &coco.segmentation {
                payload.insert("segmentation".to_string(), seg.clone());
            }
        }
        TaskType::KeyPoint => {
            if let Some(kps) = &coco.keypoints {
                payload.insert("keypoints".to_string(), kps.clone());
            }
        }
        _ => {}
    }

    Ok(serde_json::Value::Object(payload))
}
