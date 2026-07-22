"""Deterministic visual fixture generators for R1 training templates.

All generation is seeded and self-contained; no external downloads are used.
Fixtures are written as PNG images plus a JSON manifest describing the seed,
schema and license.
"""

from __future__ import annotations

import json
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Tuple

import numpy as np
from PIL import Image


def _rng(seed: int) -> np.random.Generator:
    return np.random.default_rng(seed)


def _save_image(path: Path, arr: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if arr.ndim == 2:
        Image.fromarray((arr * 255).astype(np.uint8), mode="L").save(path)
    else:
        Image.fromarray((arr * 255).astype(np.uint8), mode="RGB").save(path)


def _license() -> Dict[str, Any]:
    return {
        "spdx": "CC0-1.0",
        "url": "https://creativecommons.org/publicdomain/zero/1.0/",
        "generated": True,
        "source": "moqentra deterministic fixture generator",
    }


def _write_manifest(
    root: Path,
    kind: str,
    seed: int,
    split: str,
    schema: str,
    records: int,
    extra: Dict[str, Any],
) -> None:
    manifest: Dict[str, Any] = {
        "apiVersion": "moqentra.io/v1",
        "kind": "FixtureManifest",
        "metadata": {
            "id": str(uuid.uuid4()),
            "name": f"{kind}-{split}",
            "createdAt": datetime.now(timezone.utc).isoformat(),
        },
        "spec": {
            "kind": kind,
            "split": split,
            "records": records,
            "seed": seed,
            "schema": schema,
            "license": _license(),
        },
        "extra": extra,
    }
    root.mkdir(parents=True, exist_ok=True)
    tmp = root / ".manifest.json.tmp"
    tmp.write_text(json.dumps(manifest, indent=2, sort_keys=True))
    tmp.replace(root / "manifest.json")


def generate_classification_fixture(
    root: Path,
    n: int = 50,
    seed: int = 42,
    image_size: int = 64,
    split: str = "train",
) -> Path:
    """Generate a deterministic image-classification fixture.

    Each image contains a single colored shape on a gray background. Labels are
    one of ``circle``, ``square``, ``triangle``.
    """
    rng = _rng(seed)
    classes = ["circle", "square", "triangle"]
    image_dir = root / split / "images"
    labels: List[Dict[str, Any]] = []

    for i in range(n):
        label_idx = int(rng.integers(0, len(classes)))
        arr = _draw_shape(rng, image_size, classes[label_idx])
        name = f"{split}_{i:05d}.png"
        path = image_dir / name
        _save_image(path, arr)
        labels.append({"file_name": name, "label": classes[label_idx], "index": label_idx})

    annotations = {
        "classes": classes,
        "records": labels,
        "image_size": image_size,
        "seed": seed,
    }
    ann_path = root / split / "annotations.json"
    ann_path.parent.mkdir(parents=True, exist_ok=True)
    ann_path.write_text(json.dumps(annotations, indent=2, sort_keys=True))

    _write_manifest(
        root / split,
        "classification",
        seed,
        split,
        "https://moqentra.io/schemas/classification-fixture/v1",
        n,
        {"image_dir": str(image_dir), "image_size": image_size},
    )
    return root / split


def _draw_shape(rng: np.random.Generator, size: int, shape: str) -> np.ndarray:
    arr = np.full((size, size, 3), 0.85, dtype=np.float32)
    color = rng.random(3)
    y = int(rng.integers(20, size - 20))
    x = int(rng.integers(20, size - 20))
    radius = int(rng.integers(8, 16))

    yy, xx = np.mgrid[0:size, 0:size]
    if shape == "circle":
        mask = (yy - y) ** 2 + (xx - x) ** 2 <= radius**2
    elif shape == "square":
        mask = (np.abs(xx - x) <= radius) & (np.abs(yy - y) <= radius)
    else:  # triangle
        mask = (
            (yy >= y - radius)
            & (yy <= y + radius)
            & (np.abs(xx - x) <= (yy - (y - radius)) * radius / (2 * radius) + 1)
        )

    arr[mask] = color
    return arr


def generate_detection_fixture(
    root: Path,
    n: int = 50,
    seed: int = 42,
    image_size: int = 64,
    split: str = "train",
) -> Path:
    """Generate a deterministic object-detection fixture in COCO-like format.

    Each image contains up to three axis-aligned bounding boxes of the same
    three classes used for classification.
    """
    rng = _rng(seed)
    classes = ["circle", "square", "triangle"]
    image_dir = root / split / "images"
    annotations: List[Dict[str, Any]] = []
    categories = [{"id": i, "name": c} for i, c in enumerate(classes)]

    for i in range(n):
        arr = np.full((image_size, image_size, 3), 0.85, dtype=np.float32)
        num_boxes = int(rng.integers(1, 4))
        boxes: List[List[int]] = []
        labels: List[int] = []
        for _ in range(num_boxes):
            label = int(rng.integers(0, len(classes)))
            h = int(rng.integers(10, 20))
            w = int(rng.integers(10, 20))
            y1 = int(rng.integers(0, image_size - h))
            x1 = int(rng.integers(0, image_size - w))
            color = rng.random(3)
            arr[y1 : y1 + h, x1 : x1 + w] = color
            boxes.append([x1, y1, x1 + w, y1 + h])
            labels.append(label)

        name = f"{split}_{i:05d}.png"
        path = image_dir / name
        _save_image(path, arr)
        annotations.append(
            {
                "file_name": name,
                "image_id": i,
                "height": image_size,
                "width": image_size,
                "objects": [
                    {"bbox": box, "category_id": label}
                    for box, label in zip(boxes, labels)
                ],
            }
        )

    coco = {
        "categories": categories,
        "images": [
            {"id": i, "file_name": a["file_name"], "height": image_size, "width": image_size}
            for i, a in enumerate(annotations)
        ],
        "annotations": [
            {
                "id": int(rng.integers(0, 2**31)),
                "image_id": i,
                "category_id": obj["category_id"],
                "bbox": [obj["bbox"][0], obj["bbox"][1], obj["bbox"][2] - obj["bbox"][0], obj["bbox"][3] - obj["bbox"][1]],
            }
            for i, a in enumerate(annotations)
            for obj in a["objects"]
        ],
    }
    ann_path = root / split / "annotations.json"
    ann_path.parent.mkdir(parents=True, exist_ok=True)
    ann_path.write_text(json.dumps(coco, indent=2, sort_keys=True))

    _write_manifest(
        root / split,
        "detection",
        seed,
        split,
        "https://moqentra.io/schemas/coco-detection-fixture/v1",
        n,
        {"image_dir": str(image_dir), "image_size": image_size},
    )
    return root / split


def generate_segmentation_fixture(
    root: Path,
    n: int = 50,
    seed: int = 42,
    image_size: int = 64,
    split: str = "train",
) -> Path:
    """Generate a deterministic segmentation fixture.

    Each image has a single foreground shape; the corresponding PNG mask uses
    the class index as the grayscale value.
    """
    rng = _rng(seed)
    classes = ["circle", "square", "triangle"]
    image_dir = root / split / "images"
    mask_dir = root / split / "masks"

    for i in range(n):
        label = int(rng.integers(0, len(classes)))
        arr = _draw_shape(rng, image_size, classes[label])
        name = f"{split}_{i:05d}.png"
        _save_image(image_dir / name, arr)

        # Derive a binary mask from the colored shape by distance from gray bg.
        gray = np.full((image_size, image_size, 3), 0.85, dtype=np.float32)
        mask = np.any(arr != gray, axis=2).astype(np.uint8) * (label + 1)
        _save_image(mask_dir / name, mask)

    ann_path = root / split / "annotations.json"
    ann_path.parent.mkdir(parents=True, exist_ok=True)
    ann_path.write_text(
        json.dumps(
            {
                "classes": classes,
                "records": n,
                "image_size": image_size,
                "seed": seed,
                "mask_dir": str(mask_dir),
            },
            indent=2,
            sort_keys=True,
        )
    )

    _write_manifest(
        root / split,
        "segmentation",
        seed,
        split,
        "https://moqentra.io/schemas/segmentation-fixture/v1",
        n,
        {"image_dir": str(image_dir), "mask_dir": str(mask_dir), "image_size": image_size},
    )
    return root / split
