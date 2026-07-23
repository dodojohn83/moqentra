"""ResNet18 image-classification training template."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Tuple

try:
    import torch
    import torch.nn as nn
    import torchvision
    from PIL import Image
    from torch.utils.data import DataLoader, Dataset
    from torchvision import transforms
except ImportError:  # pragma: no cover - optional heavy deps
    torch = None  # type: ignore[assignment]
    nn = type("NnStub", (), {"Module": object})()  # type: ignore[assignment]
    torchvision = None  # type: ignore[assignment]
    Image = None  # type: ignore[assignment, misc]
    DataLoader = None  # type: ignore[assignment, misc]
    Dataset = object  # type: ignore[assignment, misc]
    transforms = None  # type: ignore[assignment]

from moqentra_worker.onnx_validation import (
    OnnxValidationError,
    validate_onnx_against_pytorch,
    write_evaluation_report,
)
from moqentra_worker.sdk import MetricPoint, WorkerLifecycle, WorkerSession

from .common import make_environment, read_annotations, set_seed, sha256_file


class _ClassificationDataset(Dataset):  # type: ignore[valid-type, misc]
    """PIL image dataset backed by fixture annotations."""

    def __init__(self, image_dir: Path, records: List[Dict[str, Any]], transform: Any = None):
        self.image_dir = image_dir
        self.records = records
        self.transform = transform

    def __len__(self) -> int:  # type: ignore[override]
        return len(self.records)

    def __getitem__(self, idx: int) -> Tuple[Any, int]:  # type: ignore[override]
        rec = self.records[idx]
        path = self.image_dir / rec["file_name"]
        img = Image.open(path).convert("RGB")
        if self.transform is not None:
            img = self.transform(img)
        label = int(rec["index"])
        return img, label


class ResNet18ClassificationTemplate(WorkerLifecycle):
    """Train a ResNet18 classifier and export to ONNX.

    Expected config keys:
    - input_dir: directory containing ``images/`` and ``annotations.json``
    - output_dir: writable directory for checkpoints and model.onnx
    - num_classes: override class count; defaults to len(classes) from annotations
    - seed: deterministic seed (default 42)
    - epochs: training epochs (default 2)
    - batch_size: default 4
    - lr: default 1e-3
    - image_size: resize both sides to this value (default 64)
    - checkpoint_interval: save checkpoint every N epochs (default 1)
    """

    def __init__(self) -> None:
        self.config: Dict[str, Any] = {}
        self.model: Any = None
        self.optimizer: Any = None
        self.criterion: Any = None
        self.device: Any = None
        self.classes: List[str] = []
        self.num_classes: int = 0
        self.image_size: int = 64

    def prepare(self, config: Dict[str, Any]) -> None:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for ResNet18 template")
        self.config = config
        seed = int(config.get("seed", 42))
        set_seed(seed)

        input_dir = Path(config["input_dir"])
        annotations = read_annotations(input_dir / "annotations.json")
        self.classes = annotations.get("classes", [])
        self.num_classes = int(
            config.get("num_classes") or annotations.get("num_classes") or len(self.classes)
        )
        self.image_size = int(config.get("image_size", 64))

        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        self.model = torchvision.models.resnet18(
            weights=None, num_classes=self.num_classes
        ).to(self.device)
        self.criterion = nn.CrossEntropyLoss()
        self.optimizer = torch.optim.Adam(self.model.parameters(), lr=float(config.get("lr", 1e-3)))

    def run(self, session: WorkerSession) -> Dict[str, Any]:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for ResNet18 template")
        input_dir = Path(self.config["input_dir"])
        annotations = read_annotations(input_dir / "annotations.json")
        records = annotations.get("records", [])

        transform = transforms.Compose(
            [
                transforms.Resize((self.image_size, self.image_size)),
                transforms.ToTensor(),
            ]
        )
        dataset: Dataset = _ClassificationDataset(input_dir / "images", records, transform)
        batch_size = int(self.config.get("batch_size", 4))
        loader = DataLoader(dataset, batch_size=batch_size, shuffle=True, num_workers=0)

        epochs = int(self.config.get("epochs", 2))
        checkpoint_interval = int(self.config.get("checkpoint_interval", 1))
        final_loss = float("nan")
        final_acc = 0.0

        for epoch in range(1, epochs + 1):
            self.model.train()
            total_loss = 0.0
            correct = 0
            total = 0
            for images, labels in loader:
                images = images.to(self.device)
                labels = labels.to(self.device)
                self.optimizer.zero_grad()
                outputs = self.model(images)
                loss = self.criterion(outputs, labels)
                loss.backward()
                self.optimizer.step()

                total_loss += loss.item() * images.size(0)
                preds = outputs.argmax(dim=1)
                correct += (preds == labels).sum().item()
                total += images.size(0)

            avg_loss = total_loss / total if total else 0.0
            acc = correct / total if total else 0.0
            final_loss = avg_loss
            final_acc = acc
            session.report_metric(
                MetricPoint(step=epoch, name="loss", value=avg_loss, tags={"split": "train"})
            )
            session.report_metric(
                MetricPoint(step=epoch, name="accuracy", value=acc, tags={"split": "train"})
            )

            if checkpoint_interval > 0 and epoch % checkpoint_interval == 0:
                session.save_checkpoint(
                    self,
                    Path(f"checkpoint_epoch_{epoch:04d}.pt"),
                )

        session.save_checkpoint(self, Path("checkpoint_final.pt"))
        return {
            "loss": final_loss,
            "accuracy": final_acc,
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
                "image_size": self.image_size,
            },
            path,
        )
        return sha256_file(path)

    def finalize(self) -> Dict[str, Any]:
        if torch is None or torchvision is None:  # pragma: no cover
            raise RuntimeError("torch and torchvision are required for ResNet18 template")
        output_dir = Path(self.config["output_dir"])
        output_dir.mkdir(parents=True, exist_ok=True)
        onnx_path = output_dir / "model.onnx"
        dummy = torch.randn(1, 3, self.image_size, self.image_size).to(self.device)
        self.model.eval()
        torch.onnx.export(
            self.model,
            dummy,
            onnx_path,
            input_names=["input"],
            output_names=["output"],
            dynamic_axes={"input": {0: "batch_size"}, "output": {0: "batch_size"}},
            opset_version=14,
            dynamo=False,
        )
        onnx_digest = sha256_file(onnx_path)
        try:
            report = validate_onnx_against_pytorch(
                onnx_path, self.model, dummy, tolerance=1e-5
            )
            write_evaluation_report(output_dir, report)
        except OnnxValidationError as exc:  # pragma: no cover - validation failure surfaces as run error
            raise RuntimeError(f"ONNX validation failed: {exc}") from exc

        return {
            "onnx_path": str(onnx_path),
            "onnx_digest": onnx_digest,
            "onnx_evaluation_report": report,
            "environment": make_environment(),
            "num_classes": self.num_classes,
            "classes": self.classes,
        }
