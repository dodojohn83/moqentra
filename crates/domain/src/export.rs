//! Annotation export converters (COCO, VOC, YOLO and native).

use crate::annotation::{Annotation, ExportFormat, Label};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// COCO category entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CocoCategory {
    pub id: u64,
    pub name: String,
    pub supercategory: String,
}

/// COCO image entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CocoImage {
    pub id: u64,
    pub file_name: String,
    pub width: u32,
    pub height: u32,
}

/// COCO annotation entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CocoAnnotationEntry {
    pub id: u64,
    pub image_id: u64,
    pub category_id: u64,
    pub bbox: Vec<f64>,
    pub segmentation: Vec<Vec<f64>>,
    pub area: f64,
    pub iscrowd: u8,
}

fn idx_to_id(idx: usize) -> u64 {
    u64::try_from(idx).unwrap_or(u64::MAX).saturating_add(1)
}

/// COCO dataset envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CocoDataset {
    pub info: Value,
    pub images: Vec<CocoImage>,
    pub annotations: Vec<CocoAnnotationEntry>,
    pub categories: Vec<CocoCategory>,
}

/// Convert a label list into COCO categories.
pub fn labels_to_coco_categories(labels: &[Label]) -> Vec<CocoCategory> {
    labels
        .iter()
        .enumerate()
        .map(|(idx, label)| CocoCategory {
            id: idx_to_id(idx),
            name: label.name.clone(),
            supercategory: label.parent_id.clone().unwrap_or_default(),
        })
        .collect()
}

/// Convert annotations to a COCO dataset envelope.
///
/// `annotations` must be keyed by `asset_id -> [Annotation]` and `asset_sizes`
/// must map `asset_id -> (width, height)`.  Geometry is extracted from the
/// `payload["bbox"]` array `[x, y, w, h]` when present.
pub fn annotations_to_coco(
    annotations: &BTreeMap<String, Vec<Annotation>>,
    asset_sizes: &BTreeMap<String, (u32, u32)>,
    labels: &[Label],
) -> Result<CocoDataset, moqentra_types::Error> {
    let mut category_by_name: BTreeMap<String, u64> = BTreeMap::new();
    for (idx, label) in labels.iter().enumerate() {
        category_by_name.insert(label.name.clone(), idx_to_id(idx));
    }

    let mut images = Vec::new();
    let mut coco_annotations = Vec::new();
    let mut ann_id = 1u64;
    let mut img_id = 1u64;

    for (asset_id, anns) in annotations {
        let (width, height) = asset_sizes.get(asset_id).copied().ok_or_else(|| {
            moqentra_types::Error::invalid_argument(format!("missing asset size for {asset_id}"))
        })?;
        images.push(CocoImage {
            id: img_id,
            file_name: asset_id.clone(),
            width,
            height,
        });

        for ann in anns {
            let label_name = ann
                .payload
                .get("label")
                .and_then(|v| v.as_str())
                .ok_or_else(|| moqentra_types::Error::invalid_argument("missing label"))?;
            let category_id = category_by_name.get(label_name).copied().ok_or_else(|| {
                moqentra_types::Error::invalid_argument(format!("unknown label {label_name}"))
            })?;
            let bbox_arr = ann
                .payload
                .get("bbox")
                .and_then(|v| v.as_array())
                .ok_or_else(|| moqentra_types::Error::invalid_argument("missing bbox"))?;
            let bbox: Vec<f64> =
                bbox_arr.iter().map(|x| x.as_f64()).collect::<Option<Vec<_>>>().ok_or_else(
                    || moqentra_types::Error::invalid_argument("bbox must be numeric"),
                )?;
            if bbox.len() != 4 {
                return Err(moqentra_types::Error::invalid_argument(
                    "bbox must contain exactly 4 values",
                ));
            }
            let (x, y, w, h) = (bbox[0], bbox[1], bbox[2], bbox[3]);
            if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
                return Err(moqentra_types::Error::invalid_argument(
                    "bbox coordinates must be finite",
                ));
            }
            if w <= 0.0 || h <= 0.0 {
                return Err(moqentra_types::Error::invalid_argument(
                    "bbox width and height must be positive",
                ));
            }
            let area = w * h;
            if !area.is_finite() {
                return Err(moqentra_types::Error::invalid_argument(
                    "bbox area overflow",
                ));
            }
            let segmentation =
                if let Some(poly) = ann.payload.get("polygon").and_then(|v| v.as_array()) {
                    let mut values = Vec::with_capacity(poly.len());
                    for v in poly {
                        let coord = v.as_f64().ok_or_else(|| {
                            moqentra_types::Error::invalid_argument(
                                "polygon coordinate must be numeric",
                            )
                        })?;
                        if !coord.is_finite() {
                            return Err(moqentra_types::Error::invalid_argument(
                                "polygon coordinate must be finite",
                            ));
                        }
                        values.push(coord);
                    }
                    if values.len() % 2 != 0 {
                        return Err(moqentra_types::Error::invalid_argument(
                            "polygon must contain an even number of coordinates",
                        ));
                    }
                    if values.is_empty() {
                        vec![]
                    } else {
                        vec![values]
                    }
                } else {
                    vec![]
                };

            coco_annotations.push(CocoAnnotationEntry {
                id: ann_id,
                image_id: img_id,
                category_id,
                bbox,
                segmentation,
                area,
                iscrowd: 0,
            });
            ann_id += 1;
        }
        img_id += 1;
    }

    Ok(CocoDataset {
        info: serde_json::json!({"description":"Moqentra COCO export","version":"1.0"}),
        images,
        annotations: coco_annotations,
        categories: labels_to_coco_categories(labels),
    })
}

/// YOLO line: `<class_id> <x_center> <y_center> <width> <height>` normalized.
pub fn annotation_to_yolo_line(
    label_index: usize,
    bbox: &[f64],
    image_width: f64,
    image_height: f64,
) -> Option<String> {
    if bbox.len() < 4 || image_width == 0.0 || image_height == 0.0 {
        return None;
    }
    let x = bbox[0];
    let y = bbox[1];
    let w = bbox[2];
    let h = bbox[3];
    if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
        return None;
    }
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let x_center = ((x + w / 2.0) / image_width).clamp(0.0, 1.0);
    let y_center = ((y + h / 2.0) / image_height).clamp(0.0, 1.0);
    let nw = (w / image_width).clamp(0.0, 1.0);
    let nh = (h / image_height).clamp(0.0, 1.0);
    Some(format!(
        "{} {:.6} {:.6} {:.6} {:.6}",
        label_index, x_center, y_center, nw, nh
    ))
}

/// Escape text for inclusion in XML element content or attributes.
fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// Convert annotations for a single image into Pascal VOC XML.
///
/// `filename` is written as the VOC `<filename>` value. Image size is taken from
/// `image_size` as `(width, height, depth)` with depth defaulting to 3 when
/// callers pass depth `0`.
///
/// Geometry is read from `payload["bbox"]` as `[x, y, w, h]` (converted to
/// xmin/ymin/xmax/ymax). The object name is taken from `payload["label"]`.
pub fn annotations_to_voc(
    annotations: &[Annotation],
    filename: &str,
    image_size: (u32, u32, u32),
) -> Result<String, moqentra_types::Error> {
    let (width, height, depth_in) = image_size;
    if width == 0 || height == 0 {
        return Err(moqentra_types::Error::invalid_argument(
            "image width and height must be positive",
        ));
    }
    let depth = if depth_in == 0 { 3 } else { depth_in };
    let mut objects = String::new();
    for ann in annotations {
        let label = ann
            .payload
            .get("label")
            .and_then(|v| v.as_str())
            .ok_or_else(|| moqentra_types::Error::invalid_argument("missing label"))?;
        let bbox_arr = ann
            .payload
            .get("bbox")
            .and_then(|v| v.as_array())
            .ok_or_else(|| moqentra_types::Error::invalid_argument("missing bbox"))?;
        if bbox_arr.len() < 4 {
            return Err(moqentra_types::Error::invalid_argument(
                "bbox must contain at least 4 values",
            ));
        }
        let nums: Option<Vec<f64>> = bbox_arr.iter().take(4).map(|x| x.as_f64()).collect();
        let bbox = nums.ok_or_else(|| {
            moqentra_types::Error::invalid_argument("bbox coordinates must be numeric")
        })?;
        let (x, y, w, h) = (bbox[0], bbox[1], bbox[2], bbox[3]);
        if ![x, y, w, h].into_iter().all(f64::is_finite) {
            return Err(moqentra_types::Error::invalid_argument(
                "bbox coordinates must be finite",
            ));
        }
        if w <= 0.0 || h <= 0.0 {
            return Err(moqentra_types::Error::invalid_argument(
                "bbox width and height must be positive",
            ));
        }
        let xmin = x.floor() as i64;
        let ymin = y.floor() as i64;
        let xmax = (x + w).ceil() as i64;
        let ymax = (y + h).ceil() as i64;
        objects.push_str(&format!(
            r#"  <object>
    <name>{name}</name>
    <pose>Unspecified</pose>
    <truncated>0</truncated>
    <difficult>0</difficult>
    <bndbox>
      <xmin>{xmin}</xmin>
      <ymin>{ymin}</ymin>
      <xmax>{xmax}</xmax>
      <ymax>{ymax}</ymax>
    </bndbox>
  </object>
"#,
            name = xml_escape(label),
            xmin = xmin,
            ymin = ymin,
            xmax = xmax,
            ymax = ymax,
        ));
    }

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<annotation>
  <folder>moqentra</folder>
  <filename>{filename}</filename>
  <source>
    <database>Moqentra</database>
  </source>
  <size>
    <width>{width}</width>
    <height>{height}</height>
    <depth>{depth}</depth>
  </size>
  <segmented>0</segmented>
{objects}</annotation>
"#,
        filename = xml_escape(filename),
        width = width,
        height = height,
        depth = depth,
        objects = objects,
    ))
}

/// Native Moqentra JSON envelope for a set of annotations.
pub fn annotations_to_native(
    annotations: &[Annotation],
    labels: &[Label],
) -> Result<Value, moqentra_types::Error> {
    let label_names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();
    Ok(serde_json::json!({
        "schema": "moqentra.annotation.export/v1",
        "labels": label_names,
        "annotations": annotations.iter().map(|a| {
            serde_json::json!({
                "id": a.id.to_string(),
                "task_id": a.task_id.to_string(),
                "asset_id": a.asset_id.to_string(),
                "revision": a.revision,
                "client_update_id": a.client_update_id,
                "actor_id": a.actor_id.to_string(),
                "payload": a.payload,
                "created_at": a.created_at,
                "updated_at": a.updated_at,
            })
        }).collect::<Vec<_>>(),
    }))
}

/// Detect export format by file extension.
pub fn format_by_extension(path: &str) -> Option<ExportFormat> {
    let lower = path.to_lowercase();
    if lower.ends_with(".moqentra.json") {
        Some(ExportFormat::Native)
    } else if lower.ends_with(".coco.json") {
        Some(ExportFormat::Coco)
    } else if lower.ends_with(".xml") {
        Some(ExportFormat::Voc)
    } else if lower.ends_with(".txt") || lower.ends_with(".yolo") {
        Some(ExportFormat::Yolo)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::Label;
    use moqentra_types::{AnnotationId, AnnotationTaskId, AssetId, RandomIdGenerator, UserId};
    use serde_json::json;

    fn make_label(name: &str) -> Label {
        Label {
            id: name.to_string(),
            name: name.to_string(),
            color: "#ff0000".to_string(),
            parent_id: None,
            metadata: serde_json::Value::Null,
        }
    }

    fn make_annotation(label: &str, bbox: Vec<f64>) -> Annotation {
        let gen = RandomIdGenerator;
        Annotation {
            id: AnnotationId::new_v7(&gen),
            task_id: AnnotationTaskId::new_v7(&gen),
            asset_id: AssetId::new_v7(&gen),
            revision: 1,
            client_update_id: "u1".to_string(),
            actor_id: UserId::new_v7(&gen),
            payload: json!({"label": label, "bbox": bbox}),
            created_at: moqentra_types::UtcTimestamp::now(),
            updated_at: moqentra_types::UtcTimestamp::now(),
        }
    }

    #[test]
    fn coco_export_preserves_bbox() {
        let labels = vec![make_label("cat")];
        let ann = make_annotation("cat", vec![10.0, 20.0, 30.0, 40.0]);
        let mut annotations = BTreeMap::new();
        annotations.insert("cat.jpg".to_string(), vec![ann]);
        let mut sizes = BTreeMap::new();
        sizes.insert("cat.jpg".to_string(), (100, 100));

        let coco = annotations_to_coco(&annotations, &sizes, &labels).unwrap();
        assert_eq!(coco.annotations.len(), 1);
        assert_eq!(coco.annotations[0].bbox, vec![10.0, 20.0, 30.0, 40.0]);
        assert_eq!(coco.categories[0].name, "cat");
    }

    #[test]
    fn yolo_line_normalized() {
        let line = annotation_to_yolo_line(0, &[10.0, 20.0, 30.0, 40.0], 100.0, 100.0).unwrap();
        assert!(line.starts_with("0 "));
        assert!(line.contains("0.250000"));
    }

    #[test]
    fn format_detection() {
        assert_eq!(
            format_by_extension("foo.coco.json"),
            Some(ExportFormat::Coco)
        );
        assert_eq!(format_by_extension("bar.xml"), Some(ExportFormat::Voc));
        assert_eq!(
            format_by_extension("yolo/labels.txt"),
            Some(ExportFormat::Yolo)
        );
        assert_eq!(format_by_extension("foo.json"), None);
        assert_eq!(
            format_by_extension("foo.moqentra.json"),
            Some(ExportFormat::Native)
        );
    }

    #[test]
    fn coco_export_rejects_odd_polygon() {
        let labels = vec![make_label("cat")];
        let mut ann = make_annotation("cat", vec![10.0, 20.0, 30.0, 40.0]);
        ann.payload =
            json!({"label": "cat", "bbox": [10.0, 20.0, 30.0, 40.0], "polygon": [1.0, 2.0, 3.0]});
        let mut annotations = BTreeMap::new();
        annotations.insert("cat.jpg".to_string(), vec![ann]);
        let mut sizes = BTreeMap::new();
        sizes.insert("cat.jpg".to_string(), (100, 100));

        assert!(annotations_to_coco(&annotations, &sizes, &labels).is_err());
    }

    #[test]
    fn coco_export_preserves_polygon() {
        let labels = vec![make_label("cat")];
        let mut ann = make_annotation("cat", vec![10.0, 20.0, 30.0, 40.0]);
        ann.payload = json!({"label": "cat", "bbox": [10.0, 20.0, 30.0, 40.0], "polygon": [1.0, 2.0, 3.0, 4.0]});
        let mut annotations = BTreeMap::new();
        annotations.insert("cat.jpg".to_string(), vec![ann]);
        let mut sizes = BTreeMap::new();
        sizes.insert("cat.jpg".to_string(), (100, 100));

        let coco = annotations_to_coco(&annotations, &sizes, &labels).unwrap();
        assert_eq!(
            coco.annotations[0].segmentation,
            vec![vec![1.0, 2.0, 3.0, 4.0]]
        );
    }

    #[test]
    fn voc_export_contains_bndbox_and_escapes() {
        let ann = make_annotation("cat & dog", vec![10.0, 20.0, 30.0, 40.0]);
        let xml = annotations_to_voc(&[ann], "img<>.jpg", (640, 480, 3)).unwrap();
        assert!(xml.contains("<xmin>10</xmin>"));
        assert!(xml.contains("<ymin>20</ymin>"));
        assert!(xml.contains("<xmax>40</xmax>"));
        assert!(xml.contains("<ymax>60</ymax>"));
        assert!(xml.contains("<name>cat &amp; dog</name>"));
        assert!(xml.contains("<filename>img&lt;&gt;.jpg</filename>"));
        assert!(xml.contains("<width>640</width>"));
    }

    #[test]
    fn native_export_preserves_ids() {
        let labels = vec![make_label("cat")];
        let ann = make_annotation("cat", vec![1.0, 2.0, 3.0, 4.0]);
        let id = ann.id.to_string();
        let value = annotations_to_native(&[ann], &labels).unwrap();
        assert_eq!(value["schema"], "moqentra.annotation.export/v1");
        assert_eq!(value["annotations"][0]["id"], id);
        assert_eq!(value["labels"][0], "cat");
    }
}
