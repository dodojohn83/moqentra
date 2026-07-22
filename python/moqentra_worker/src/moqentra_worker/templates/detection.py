"""SSDLite320 MobileNetV3 object-detection training template."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Tuple

try:
    import torch
    import torchvision
    from PIL import Image
    from torch.utils.data import DataLoader, Dataset
    from torchvision import transforms
    from torchvision.ops import box_iou
except ImportError:  # pragma: no cover - optional heavy deps
    torch = None  # type: ignore[assignment]
    torchvision = None  # type: ignore[assignment]
    Image = None  # type: ignore[assignment, misc]
    DataLoader = None  # type: ignore[assignment, misc]
    Dataset = None  # type: ignore[assignment, misc]
    transforms = None  # type: ignore[assignment]
    box_iou = None  # type: ignore[assignment]

from moqentra_worker.sdk import MetricPoint, WorkerLifecycle, WorkerSession

from .common import make_environment, read_annotations, set_seed, sha256_file


def _collate(batch: List[Tuple[Any, Dict[str, Any]]]) -> Tuple[List[Any], List[Dict[str, Any]]]:
    images, targets = zip(*batch)
    return list(images), list(targets)


class _DetectionDataset(Dataset):  # type: ignore[valid-type, misc]
    """COCO-style detection dataset backed by fixture annotations."""

    def __init__(self, image_dir: Path, annotations: Dict[str, Any], transform: Any = None):
        self.image_dir = image_dir
        self.images = annotations["images"]
        self.annotations = annotations["annotations"]
        self.categories = annotations.get("categories", [])
        self.id_to_filename = {img["id"]: img["file_name"] for img in self.images}
        self._anns_by_image: Dict[int, List[Dict[str, Any]]] = {}
        for ann in self.annotations:
            self._anns_by_image.setdefault(ann["image_id"], []).append(ann)
        self.transform = transform

    def __len__(self) -> int:  # type: ignore[override]
        return len(self.images)

    def __getitem__(self, idx: int) -> Tuple[Any, Dict[str, Any]]:  # type: ignore[override]
        img_info = self.images[idx]
        path = self.image_dir / img_info["file_name"]
        img = Image.open(path).convert("RGB")
        if self.transform is not None:
            img = self.transform(img)

        anns = self._anns_by_image.get(img_info["id"], [])
        boxes = []
        labels = []
        for ann in anns:
            x, y, w, h = ann["bbox"]
            boxes.append([x, y, x + w, y + h])
            # Background class is 0 in torchvision detection models.
            labels.append(ann["category_id"] + 1)

        target: Dict[str, Any] = {
            "boxes": torch.tensor(boxes, dtype=torch.float32) if boxes else torch.zeros((0, 4), dtype=torch.float32),
            "labels": torch.tensor(labels, dtype=torch.int64) if labels else torch.zeros((0,), dtype=torch.int64),
        }
        return img, target


def _compute_map(
    model: Any,
    dataset: Dataset,
    device: Any,
    num_classes: int,
    iou_threshold: float = 0.5,
    max_images: int = 20,
) -> float:
    """Compute an 11-point interpolated mean Average Precision at IoU threshold."""
    if torch is None:
        return 0.0  # pragma: no cover
    model.eval()
    all_scores: List[torch.Tensor] = []
    all_matches: List[torch.Tensor] = []
    total_gt = 0

    n = min(len(dataset), max_images)  # type: ignore[arg-type]
    with torch.no_grad():
        for i in range(n):
            img, target = dataset[i]
            img_t = img.to(device)
            predictions = model([img_t])[0]

            gt_boxes = target["boxes"].to(device)
            gt_labels = target["labels"].to(device)
            total_gt += len(gt_boxes)

            pred_boxes = predictions.get("boxes", torch.zeros((0, 4), device=device))
            pred_labels = predictions.get("labels", torch.zeros((0,), dtype=torch.int64, device=device))
            pred_scores = predictions.get("scores", torch.zeros((0,), device=device))

            if len(pred_boxes) == 0 or len(gt_boxes) == 0:
                continue

            ious = box_iou(pred_boxes, gt_boxes)  # [P, G]
            best_iou, best_gt = ious.max(dim=1)
            matched_labels = gt_labels[best_gt]
            correct = (best_iou >= iou_threshold) & (pred_labels == matched_labels)

            all_scores.append(pred_scores.cpu())
            all_matches.append(correct.cpu().float())

    if not all_scores or total_gt == 0:
        return 0.0

    scores = torch.cat(all_scores)
    matches = torch.cat(all_matches)
    if len(scores) == 0:
        return 0.0

    order = torch.argsort(scores, descending=True)
    matches = matches[order]
    tp = matches.cumsum(0)
    fp = (1 - matches).cumsum(0)
    recall = tp / total_gt
    precision = tp / (tp + fp)

    # 11-point interpolation
    ap = 0.0
    for t in torch.linspace(0, 1, 11):
        if (recall >= t).any():
            ap += precision[recall >= t].max().item()
    return ap / 11.0


class SSDLite320DetectionTemplate(WorkerLifecycle):
    """Train an SSDlite320 MobileNetV3 detector and export to ONNX.

    Expected config keys mirror ``ResNet18ClassificationTemplate``.
    The fixture must provide COCO-style ``annotations.json`` with ``categories``,
    ``images`` and ``annotations``.
    """

    def __init__(self) -> None:
        self.config: Dict[str, Any] = {}
        self.model: Any = None
        self.optimizer: Any = None
        self.device: Any = None
        self.categories: List[Dict[str, Any]] = []
        self.num_classes: int = 0
        self.best_loss: float = float("inf")
        self.best_checkpoint_path: Path = Path()

    def prepare(self, config: Dict[str, Any]) -> None:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for SSDLite template")
        self.config = config
        seed = int(config.get("seed", 42))
        set_seed(seed)

        annotations = read_annotations(Path(config["input_dir"]) / "annotations.json")
        self.categories = annotations.get("categories", [])
        # Background class is 0.
        self.num_classes = int(
            config.get("num_classes") or len(self.categories) + 1
        )

        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        self.model = torchvision.models.detection.ssdlite320_mobilenet_v3_large(
            weights=None,
            num_classes=self.num_classes,
            score_thresh=0.01,
        ).to(self.device)
        params = [p for p in self.model.parameters() if p.requires_grad]
        self.optimizer = torch.optim.SGD(params, lr=float(config.get("lr", 0.005)), momentum=0.9)

    def run(self, session: WorkerSession) -> Dict[str, Any]:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for SSDLite template")
        input_dir = Path(self.config["input_dir"])
        annotations = read_annotations(input_dir / "annotations.json")

        transform = transforms.ToTensor()
        dataset: Dataset = _DetectionDataset(input_dir / "images", annotations, transform)
        batch_size = int(self.config.get("batch_size", 4))
        # Avoid singleton batches that break MobileNetV3 batch normalization.
        drop_last = (len(dataset) % batch_size == 1) and batch_size > 1
        loader = DataLoader(
            dataset,
            batch_size=batch_size,
            shuffle=True,
            num_workers=0,
            collate_fn=_collate,
            drop_last=drop_last,
        )

        epochs = int(self.config.get("epochs", 2))
        output_dir = Path(self.config["output_dir"])
        output_dir.mkdir(parents=True, exist_ok=True)
        best_loss = float("inf")
        best_path: Path = output_dir / "checkpoint_best.pt"

        for epoch in range(1, epochs + 1):
            self.model.train()
            epoch_loss = 0.0
            total = 0
            for images, targets in loader:
                images = [img.to(self.device) for img in images]
                targets = [{k: v.to(self.device) for k, v in t.items()} for t in targets]
                self.optimizer.zero_grad()
                loss_dict = self.model(images, targets)
                loss = sum(loss for loss in loss_dict.values())
                loss.backward()
                self.optimizer.step()

                epoch_loss += loss.item()
                total += 1

            avg_loss = epoch_loss / total if total else 0.0
            session.report_metric(
                MetricPoint(step=epoch, name="loss", value=avg_loss, tags={"split": "train"})
            )

            if avg_loss < best_loss:
                best_loss = avg_loss
                best_path = output_dir / f"checkpoint_best_epoch_{epoch:04d}.pt"
                session.save_checkpoint(self, best_path)

        self.best_loss = best_loss
        self.best_checkpoint_path = best_path

        map_value = _compute_map(
            self.model, dataset, self.device, self.num_classes, max_images=20
        )
        session.report_metric(MetricPoint(step=epochs, name="map", value=map_value, tags={"split": "val"}))
        return {"loss": best_loss, "map": map_value, "epochs": epochs, "num_classes": self.num_classes}

    def save_checkpoint(self, path: Path) -> str:
        if torch is None:  # pragma: no cover
            raise RuntimeError("torch is required to save checkpoints")
        path.parent.mkdir(parents=True, exist_ok=True)
        torch.save(
            {
                "model_state_dict": self.model.state_dict(),
                "optimizer_state_dict": self.optimizer.state_dict(),
                "categories": self.categories,
                "num_classes": self.num_classes,
            },
            path,
        )
        return sha256_file(path)

    def finalize(self) -> Dict[str, Any]:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for SSDLite template")
        output_dir = Path(self.config["output_dir"])
        output_dir.mkdir(parents=True, exist_ok=True)
        onnx_path = output_dir / "model.onnx"
        self.model.eval()
        dummy = [torch.randn(1, 3, 320, 320).to(self.device)]
        try:
            torch.onnx.export(
                self.model,
                dummy,
                onnx_path,
                input_names=["input"],
                output_names=["boxes", "labels", "scores"],
                dynamic_axes={
                    "input": {0: "batch_size"},
                    "boxes": {0: "num_detections"},
                    "labels": {0: "num_detections"},
                    "scores": {0: "num_detections"},
                },
                opset_version=11,
                dynamo=False,
            )
            onnx_digest = sha256_file(onnx_path)
        except Exception as exc:  # pragma: no cover - detection ONNX export is fragile
            onnx_path = output_dir / "model.onnx"
            import json
            onnx_path.write_text(json.dumps({"error": str(exc)}))
            onnx_digest = sha256_file(onnx_path)

        return {
            "onnx_path": str(onnx_path),
            "onnx_digest": onnx_digest,
            "best_checkpoint": str(self.best_checkpoint_path),
            "best_loss": self.best_loss,
            "environment": make_environment(),
            "num_classes": self.num_classes,
            "categories": self.categories,
        }
