"""Shared helpers for PyTorch training templates."""

from __future__ import annotations

import hashlib
import json
import os
import random
from pathlib import Path
from typing import Any, Dict

try:
    import numpy as np
except ImportError:  # pragma: no cover
    np = None  # type: ignore[assignment]

try:
    import torch
except ImportError:  # pragma: no cover - optional heavy dep
    torch = None  # type: ignore[assignment]

try:
    import torchvision
except ImportError:  # pragma: no cover - optional heavy dep
    torchvision = None  # type: ignore[assignment]


def set_seed(seed: int) -> None:
    """Set deterministic seeds for Python, NumPy and PyTorch."""
    random.seed(seed)
    if np is not None:
        np.random.seed(seed)
    if torch is not None:
        torch.manual_seed(seed)
        if torch.cuda.is_available():
            torch.cuda.manual_seed_all(seed)
            torch.backends.cudnn.deterministic = True  # type: ignore[attr-defined]
            torch.backends.cudnn.benchmark = False  # type: ignore[attr-defined]


def sha256_file(path: Path) -> str:
    """Compute a canonical sha256 digest for a file."""
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return f"sha256:{h.hexdigest()}"


def make_environment() -> Dict[str, Any]:
    """Collect deterministic environment metadata for an artifact manifest."""
    env: Dict[str, Any] = {
        "python_version": os.sys.version.split()[0],
        "pytorch_version": torch.__version__ if torch is not None else None,
        "torchvision_version": torchvision.__version__ if torchvision is not None else None,
        "cuda_available": torch.cuda.is_available() if torch is not None else False,
        "device_count": torch.cuda.device_count() if torch is not None else 0,
        "driver_version": None,
        "runtime_version": None,
    }
    if torch is not None and torch.cuda.is_available():
        try:
            env["driver_version"] = torch._C._cuda_getDriverVersion()  # type: ignore[attr-defined]
        except Exception:
            pass
    return env


def read_annotations(path: Path) -> Dict[str, Any]:
    """Load an annotations JSON file produced by the fixture generators."""
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)
