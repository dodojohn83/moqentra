"""DeepLabV3 MobileNetV3 semantic-segmentation training template."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Tuple

try:
    import numpy as np
    import torch
    import torch.nn as nn
    import torch.nn.functional as F
    import torchvision
    from PIL import Image
    from torch.utils.data import DataLoader, Dataset
    from torchvision import transforms
except ImportError:  # pragma: no cover - optional heavy deps
    np = None  # type: ignore[assignment]
    torch = None  # type: ignore[assignment]
    nn = None  # type: ignore[assignment]
    F = None  # type: ignore[assignment]
    torchvision = None  # type: ignore[assignment]
    Image = None  # type: ignore[assignment, misc]
    DataLoader = None  # type: ignore[assignment, misc]
    Dataset = None  # type: ignore[assignment, misc]
    transforms = None  # type: ignore[assignment]

from moqentra_worker.onnx_validation import (
    OnnxValidationError,
    validate_onnx_against_pytorch,
    write_evaluation_report,
)
from moqentra_worker.sdk import MetricPoint, WorkerLifecycle, WorkerSession

from .common import make_environment, read_annotations, set_seed, sha256_file


class _DeepLabOutputWrapper(nn.Module):  # type: ignore[valid-type, misc]
    """Wrap DeepLabV3 to return the 'out' tensor for ONNX export."""

    def __init__(self, model: Any):
        super().__init__()
        self.model = model

    def forward(self, x: Any) -> Any:
        return self.model(x)["out"]


class _SegmentationDataset(Dataset):  # type: ignore[valid-type, misc]
    """Segmentation dataset backed by fixture images and masks."""

    def __init__(self, image_dir: Path, mask_dir: Path, records: int, transform: Any = None):
        self.image_dir = image_dir
        self.mask_dir = mask_dir
        self.records = records
        self.transform = transform

    def __len__(self) -> int:  # type: ignore[override]
        return self.records

    def __getitem__(self, idx: int) -> Tuple[Any, Any]:  # type: ignore[override]
        name = f"train_{idx:05d}.png"
        img = Image.open(self.image_dir / name).convert("RGB")
        mask = Image.open(self.mask_dir / name).convert("L")
        if self.transform is not None:
            img = self.transform(img)
        mask_t = torch.from_numpy(np.array(mask, dtype=np.int64))
        return img, mask_t


def _compute_miou(
    model: Any,
    dataset: Dataset,
    device: Any,
    num_classes: int,
    max_images: int = 20,
) -> float:
    """Compute mean intersection-over-union over a subset of the dataset."""
    if torch is None or np is None:
        return 0.0  # pragma: no cover
    model.eval()
    ious: List[float] = []
    with torch.no_grad():
        n = min(len(dataset), max_images)  # type: ignore[arg-type]
        for i in range(n):
            img, target = dataset[i]
            img_t = img.to(device).unsqueeze(0)
            target_t = target.to(device)
            output = model(img_t)["out"]
            pred = output.argmax(dim=1).squeeze(0)

            # Resize prediction to match target resolution if needed.
            if pred.shape != target_t.shape:
                pred = F.interpolate(
                    pred.unsqueeze(0).unsqueeze(0).float(),
                    size=target_t.shape,
                    mode="nearest",
                ).squeeze().long()

            for c in range(num_classes):
                pred_c = pred == c
                target_c = target_t == c
                intersection = (pred_c & target_c).sum().item()
                union = (pred_c | target_c).sum().item()
                if union > 0:
                    ious.append(intersection / union)
    return sum(ious) / len(ious) if ious else 0.0


class DeepLabV3SegmentationTemplate(WorkerLifecycle):
    """Train a DeepLabV3 MobileNetV3 semantic-segmentation model and export to ONNX.

    Expected config keys mirror the other templates. The fixture must contain
    ``images/`` and ``masks/`` plus ``annotations.json`` with ``classes``.
    """

    def __init__(self) -> None:
        self.config: Dict[str, Any] = {}
        self.model: Any = None
        self.optimizer: Any = None
        self.criterion: Any = None
        self.device: Any = None
        self.classes: List[str] = []
        self.num_classes: int = 0

    def prepare(self, config: Dict[str, Any]) -> None:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for DeepLabV3 template")
        self.config = config
        seed = int(config.get("seed", 42))
        set_seed(seed)

        input_dir = Path(config["input_dir"])
        annotations = read_annotations(input_dir / "annotations.json")
        self.classes = annotations.get("classes", [])
        # Channel 0 reserved for background.
        self.num_classes = int(config.get("num_classes") or len(self.classes) + 1)

        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        self.model = torchvision.models.segmentation.deeplabv3_mobilenet_v3_large(
            weights=None,
            num_classes=self.num_classes,
        ).to(self.device)
        self.criterion = torch.nn.CrossEntropyLoss(ignore_index=255)
        self.optimizer = torch.optim.Adam(self.model.parameters(), lr=float(config.get("lr", 1e-3)))

    def run(self, session: WorkerSession) -> Dict[str, Any]:
        if torch is None or torchvision is None or np is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for DeepLabV3 template")
        input_dir = Path(self.config["input_dir"])
        annotations = read_annotations(input_dir / "annotations.json")
        records = int(annotations.get("records", 0))

        transform = transforms.ToTensor()
        dataset: Dataset = _SegmentationDataset(
            input_dir / "images",
            Path(annotations["mask_dir"]),
            records,
            transform,
        )
        batch_size = int(self.config.get("batch_size", 4))
        loader = DataLoader(dataset, batch_size=batch_size, shuffle=True, num_workers=0)

        epochs = int(self.config.get("epochs", 2))
        final_loss = float("nan")
        for epoch in range(1, epochs + 1):
            self.model.train()
            total_loss = 0.0
            total = 0
            for images, masks in loader:
                images = images.to(self.device)
                masks = masks.to(self.device)
                outputs = self.model(images)["out"]
                resized_masks = F.interpolate(
                    masks.unsqueeze(1).float(),
                    size=outputs.shape[2:],
                    mode="nearest",
                ).squeeze(1).long()
                loss = self.criterion(outputs, resized_masks)
                self.optimizer.zero_grad()
                loss.backward()
                self.optimizer.step()

                total_loss += loss.item() * images.size(0)
                total += images.size(0)

            avg_loss = total_loss / total if total else 0.0
            final_loss = avg_loss
            session.report_metric(
                MetricPoint(step=epoch, name="loss", value=avg_loss, tags={"split": "train"})
            )
            session.save_checkpoint(self, Path(f"checkpoint_epoch_{epoch:04d}.pt"))

        session.save_checkpoint(self, Path("checkpoint_final.pt"))
        miou = _compute_miou(self.model, dataset, self.device, self.num_classes, max_images=20)
        session.report_metric(MetricPoint(step=epochs, name="miou", value=miou, tags={"split": "val"}))

        return {
            "loss": final_loss,
            "miou": miou,
            "epochs": epochs,
            "num_classes": self.num_classes,
            "classes": self.classes,
        }

    def save_checkpoint(self, path: Path) -> str:
        if torch is None:  # pragma: no cover
            raise RuntimeError("torch is required to save checkpoints")
        path.parent.mkdir(parents=True, exist_ok=True)
        torch.save(
            {
                "model_state_dict": self.model.state_dict(),
                "optimizer_state_dict": self.optimizer.state_dict(),
                "classes": self.classes,
                "num_classes": self.num_classes,
            },
            path,
        )
        return sha256_file(path)

    def finalize(self) -> Dict[str, Any]:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for DeepLabV3 template")
        output_dir = Path(self.config["output_dir"])
        output_dir.mkdir(parents=True, exist_ok=True)
        onnx_path = output_dir / "model.onnx"
        self.model.eval()
        dummy = torch.randn(1, 3, 64, 64).to(self.device)
        wrapped = _DeepLabOutputWrapper(self.model).to(self.device).eval()
        torch.onnx.export(
            wrapped,
            dummy,
            onnx_path,
            input_names=["input"],
            output_names=["output"],
            dynamic_axes={
                "input": {0: "batch_size", 2: "height", 3: "width"},
                "output": {0: "batch_size", 2: "height", 3: "width"},
            },
            opset_version=14,
            dynamo=False,
        )
        onnx_digest = sha256_file(onnx_path)

        try:
            report = validate_onnx_against_pytorch(
                onnx_path, wrapped, dummy, tolerance=1e-5
            )
            write_evaluation_report(output_dir, report)
        except OnnxValidationError as exc:  # pragma: no cover - validation failure surfaces as run error
            raise RuntimeError(f"ONNX validation failed: {exc}") from exc

        # Generate a small mask preview for the first training image.
        preview_path = output_dir / "mask_preview.png"
        self._save_preview(preview_path)

        return {
            "onnx_path": str(onnx_path),
            "onnx_digest": onnx_digest,
            "onnx_evaluation_report": report,
            "preview_path": str(preview_path),
            "environment": make_environment(),
            "num_classes": self.num_classes,
            "classes": self.classes,
        }

    def _save_preview(self, path: Path) -> None:
        if torch is None or Image is None or np is None:
            return  # pragma: no cover
        input_dir = Path(self.config["input_dir"])
        try:
            img = Image.open(input_dir / "images" / "train_00000.png").convert("RGB")
            transform = transforms.ToTensor()
            tensor = transform(img).to(self.device).unsqueeze(0)
            with torch.no_grad():
                output = self.model(tensor)["out"].argmax(dim=1).squeeze(0).cpu().numpy()
            preview = (output * (255 // max(self.num_classes, 1))).astype(np.uint8)
            Image.fromarray(preview).save(path)
        except Exception:
            pass
