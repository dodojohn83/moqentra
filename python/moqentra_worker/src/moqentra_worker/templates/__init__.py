"""PyTorch training templates for the R1 vertical slice."""

from __future__ import annotations

from typing import Any, Dict, List, TYPE_CHECKING

if TYPE_CHECKING:
    from pathlib import Path

try:
    import torch
    import torchvision
except ImportError:  # pragma: no cover - optional heavy deps
    torch = None  # type: ignore[assignment]
    torchvision = None  # type: ignore[assignment]

from .classification import ResNet18ClassificationTemplate
from .common import make_environment, sha256_file, set_seed
from .detection import SSDLite320DetectionTemplate
from .segmentation import DeepLabV3SegmentationTemplate

__all__ = [
    "DeepLabV3SegmentationTemplate",
    "ResNet18ClassificationTemplate",
    "SSDLite320DetectionTemplate",
    "make_environment",
    "set_seed",
    "sha256_file",
]


def _environment() -> Dict[str, Any]:
    return make_environment()


def available_templates() -> List[str]:
    """Return names of templates whose runtime dependencies are installed."""
    templates: List[str] = []
    if torch is not None and torchvision is not None:
        templates.extend(
            [
                "resnet18-classification",
                "ssdlite320-detection",
                "deeplabv3-segmentation",
            ]
        )
    return templates
