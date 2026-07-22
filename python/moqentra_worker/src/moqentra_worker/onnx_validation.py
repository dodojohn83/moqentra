"""ONNX Runtime validation helpers for exported training templates."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Tuple

try:
    import numpy as np
except ImportError:  # pragma: no cover
    np = None  # type: ignore[assignment]

try:
    import onnxruntime as ort
except ImportError:  # pragma: no cover
    ort = None  # type: ignore[assignment]

try:
    import torch
except ImportError:  # pragma: no cover
    torch = None  # type: ignore[assignment]


class OnnxValidationError(Exception):
    """Raised when an ONNX model fails loading, shape validation or numeric comparison."""

    pass


def _numpyify(x: Any) -> Any:
    """Convert PyTorch tensors to NumPy arrays recursively."""
    if torch is not None and isinstance(x, torch.Tensor):
        return x.detach().cpu().numpy()
    if isinstance(x, (list, tuple)):
        return [_numpyify(v) for v in x]
    if isinstance(x, dict):
        return {k: _numpyify(v) for k, v in x.items()}
    return x


def _max_abs_diff(a: Any, b: Any) -> float:
    """Recursively compute the maximum absolute difference between two structures."""
    if np is None:  # pragma: no cover
        raise OnnxValidationError("numpy is required for diff computation")

    if isinstance(a, np.ndarray):
        return float(np.max(np.abs(a.astype(np.float64) - b.astype(np.float64))))
    if isinstance(a, (list, tuple)):
        return max((_max_abs_diff(x, y) for x, y in zip(a, b)), default=0.0)
    if isinstance(a, dict):
        return max(
            (_max_abs_diff(a[k], b[k]) for k in a if k in b),
            default=0.0,
        )
    raise OnnxValidationError(f"unsupported output type for comparison: {type(a)}")


def _shape_of(x: Any) -> Any:
    """Return shapes for tensors, lists/tuples or dicts of tensors."""
    if isinstance(x, np.ndarray):
        return list(x.shape)
    if torch is not None and isinstance(x, torch.Tensor):
        return list(x.shape)
    if isinstance(x, (list, tuple)):
        return [_shape_of(v) for v in x]
    if isinstance(x, dict):
        return {k: _shape_of(v) for k, v in x.items()}
    return None


def validate_onnx_against_pytorch(
    onnx_path: Path,
    model: Any,
    dummy_input: Any,
    tolerance: float = 1e-5,
) -> Dict[str, Any]:
    """Load an ONNX model with ONNX Runtime and compare outputs to PyTorch.

    Parameters
    ----------
    onnx_path:
        Path to the exported ONNX model.
    model:
        PyTorch model in eval mode.
    dummy_input:
        Tensor, tuple or list of tensors passed to ``torch.onnx.export``.
    tolerance:
        Maximum allowed per-element absolute difference between PyTorch and ONNX outputs.

    Returns
    -------
    A dictionary with ``input_shapes``, ``output_shapes``, ``pytorch_output``,
    ``onnx_output``, ``max_abs_diff`` and ``passed``.
    """
    if torch is None:
        raise OnnxValidationError("torch is required")
    if np is None:
        raise OnnxValidationError("numpy is required")
    if ort is None:
        raise OnnxValidationError("onnxruntime is required")

    if not onnx_path.is_file():
        raise OnnxValidationError(f"ONNX file missing: {onnx_path}")

    with onnx_path.open("rb") as f:
        first = f.read(1).strip()
    if first == b"{":
        raise OnnxValidationError(f"ONNX file is a JSON placeholder: {onnx_path}")

    model.eval()
    with torch.no_grad():
        pytorch_output = _numpyify(model(dummy_input))

    providers = (
        ["CPUExecutionProvider"]
        if "CPUExecutionProvider" in ort.get_available_providers()
        else ort.get_available_providers()
    )
    session = ort.InferenceSession(str(onnx_path), providers=providers)
    input_meta = session.get_inputs()
    if len(input_meta) != 1:
        raise OnnxValidationError("Only single-input ONNX models are supported")

    ort_input = _numpyify(dummy_input)
    if isinstance(ort_input, list):
        ort_inputs = {input_meta[0].name: np.concatenate(ort_input, axis=0)}
    elif isinstance(ort_input, tuple):
        ort_inputs = {input_meta[0].name: np.array(ort_input[0])}
    else:
        ort_inputs = {input_meta[0].name: ort_input}

    onnx_output = session.run(None, ort_inputs)

    # Normalize a single output to match PyTorch output shape.
    if len(onnx_output) == 1:
        onnx_output = onnx_output[0]

    diff = _max_abs_diff(pytorch_output, onnx_output)
    passed = diff <= tolerance

    return {
        "input_shapes": _shape_of(dummy_input),
        "output_shapes": _shape_of(onnx_output),
        "pytorch_output": _json_serializable(pytorch_output),
        "onnx_output": _json_serializable(onnx_output),
        "max_abs_diff": diff,
        "tolerance": tolerance,
        "passed": passed,
    }


def _json_serializable(x: Any) -> Any:
    """Turn NumPy/torch structures into JSON-serializable plain Python objects."""
    if isinstance(x, np.ndarray):
        return x.tolist()
    if isinstance(x, (list, tuple)):
        return [_json_serializable(v) for v in x]
    if isinstance(x, dict):
        return {k: _json_serializable(v) for k, v in x.items()}
    if isinstance(x, (np.floating, float)):
        return float(x)
    if isinstance(x, (np.integer, int)):
        return int(x)
    return x


def write_evaluation_report(output_dir: Path, report: Dict[str, Any]) -> Path:
    """Write an ONNX evaluation report to ``onnx_evaluation_report.json``."""
    output_dir.mkdir(parents=True, exist_ok=True)
    path = output_dir / "onnx_evaluation_report.json"
    with path.open("w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, sort_keys=True)
    return path
