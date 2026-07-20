"""Moqentra Python worker SDK.

The SDK exposes the worker lifecycle and framework adapters. It intentionally
does not access the control-plane database; all credentials are short-lived and
passed through the environment by the Rust control plane.
"""

from __future__ import annotations

import contextlib
import dataclasses
import math
import os
import signal
import sys
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Protocol, Union


def _is_finite(value: Union[int, float]) -> bool:
    """Return True for finite ints and floats; reject NaN and infinities."""
    try:
        return math.isfinite(value)
    except (TypeError, ValueError):
        return False


def _is_allowed_path(path: Path, base: Path) -> bool:
    """Reject paths that are relative, contain traversal components, include null bytes, or escape base."""
    s = str(path)
    if "\x00" in s:
        return False
    if not path.is_absolute():
        return False
    if ".." in path.parts:
        return False
    try:
        resolved = path.resolve()
        base_resolved = base.resolve()
    except (OSError, RuntimeError):
        return False
    if resolved == base_resolved.parent or not resolved.is_relative_to(base_resolved):
        return False
    return True


class WorkerLifecycle(Protocol):
    """Framework-adapter lifecycle implemented by user training code."""

    def prepare(self, config: Dict[str, Any]) -> None: ...
    def run(self, session: WorkerSession) -> Dict[str, Any]: ...
    def save_checkpoint(self, path: Path) -> str: ...
    def finalize(self) -> Dict[str, Any]: ...


@dataclasses.dataclass(frozen=True)
class MetricPoint:
    step: int
    name: str
    value: float
    tags: Dict[str, str] = dataclasses.field(default_factory=dict)


class WorkerSession:
    """A single worker execution session bound to an attempt lease."""

    def __init__(
        self,
        attempt_id: str,
        fencing_token: str,
        work_dir: Path,
        input_dir: Path,
        output_dir: Path,
    ) -> None:
        self.attempt_id = attempt_id
        self.fencing_token = fencing_token
        self.work_dir = work_dir
        self.input_dir = input_dir
        self.output_dir = output_dir
        self._metrics: List[MetricPoint] = []
        self._cancelled = False

    def report_metric(self, point: MetricPoint) -> None:
        if self._cancelled:
            raise RuntimeError("worker has been cancelled")
        if not _is_finite(point.value):
            raise ValueError(f"metric value must be finite: {point.name}")
        self._metrics.append(point)

    def report_metrics(self, points: List[MetricPoint]) -> None:
        for point in points:
            self.report_metric(point)

    def save_checkpoint(self, adapter: WorkerLifecycle, path: Optional[Path] = None) -> str:
        if self._cancelled:
            raise RuntimeError("worker has been cancelled")
        target = path or (self.output_dir / "checkpoints" / f"step-{len(self._metrics)}")
        if not (
            _is_allowed_path(target, self.work_dir)
            or _is_allowed_path(target, self.output_dir)
        ):
            raise ValueError(f"checkpoint path outside of work/output directories: {target}")
        target.parent.mkdir(parents=True, exist_ok=True)
        digest = adapter.save_checkpoint(target)
        return digest

    def cancel(self) -> None:
        self._cancelled = True

    def is_cancelled(self) -> bool:
        return self._cancelled


class WorkerRuntime:
    """Minimal runtime that dispatches the adapter lifecycle."""

    def __init__(
        self,
        adapter: WorkerLifecycle,
        signal_handler: Optional[Callable[[int, Any], None]] = None,
    ) -> None:
        self.adapter = adapter
        self._signal_handler = signal_handler
        self._session: Optional[WorkerSession] = None

    def _handle_signal(self, signum: int, frame: Any) -> None:
        if self._session is not None:
            self._session.cancel()
        if self._signal_handler:
            self._signal_handler(signum, frame)

    def run(self, config: Dict[str, Any]) -> Dict[str, Any]:
        attempt_id = config.get("attempt_id")
        fencing_token = config.get("fencing_token")
        if not attempt_id or not fencing_token:
            return {
                "attempt_id": attempt_id or "",
                "fencing_token": fencing_token or "",
                "error": "missing attempt_id or fencing_token",
            }
        base_dir = Path(
            config.get("worker_root")
            or os.environ.get("MOQENTRA_WORKER_ROOT")
            or "/tmp/moqentra"
        )
        if not _is_allowed_path(base_dir, base_dir):
            raise ValueError(f"invalid worker root: {base_dir}")
        real_base = Path(os.path.realpath(str(base_dir)))
        if real_base != base_dir:
            raise ValueError(f"worker root must not contain symlinks: {base_dir}")

        work_dir = Path(config.get("work_dir") or str(base_dir / "work"))
        input_dir = Path(config.get("input_dir") or str(base_dir / "input"))
        output_dir = Path(config.get("output_dir") or str(base_dir / "output"))

        for path in (work_dir, input_dir, output_dir):
            if not _is_allowed_path(path, base_dir):
                raise ValueError(f"invalid worker path: {path}")

        work_dir.mkdir(parents=True, exist_ok=True)
        input_dir.mkdir(parents=True, exist_ok=True)
        output_dir.mkdir(parents=True, exist_ok=True)

        if input_dir.resolve().is_relative_to(base_dir.resolve()):
            with contextlib.suppress(OSError):
                os.chmod(input_dir, 0o555)

        self._session = WorkerSession(
            attempt_id=attempt_id,
            fencing_token=fencing_token,
            work_dir=work_dir,
            input_dir=input_dir,
            output_dir=output_dir,
        )

        signal.signal(signal.SIGTERM, self._handle_signal)
        signal.signal(signal.SIGINT, self._handle_signal)

        try:
            self.adapter.prepare(config)
            result = self.adapter.run(self._session)
            manifest = self.adapter.finalize()
            return {
                "attempt_id": attempt_id,
                "fencing_token": fencing_token,
                "result": result,
                "manifest": manifest,
                "metrics": [dataclasses.asdict(m) for m in self._session._metrics],
            }
        except Exception as exc:
            return {
                "attempt_id": attempt_id,
                "fencing_token": fencing_token,
                "error": str(exc),
                "metrics": [dataclasses.asdict(m) for m in self._session._metrics],
            }


class PyTorchAdapter(WorkerLifecycle):
    """Stub PyTorch adapter."""

    def __init__(self, train_fn: Callable[[WorkerSession], Dict[str, Any]]) -> None:
        self.train_fn = train_fn

    def prepare(self, config: Dict[str, Any]) -> None:
        pass

    def run(self, session: WorkerSession) -> Dict[str, Any]:
        return self.train_fn(session)

    def save_checkpoint(self, path: Path) -> str:
        path.write_text("checkpoint")
        return "sha256:checkpoint"

    def finalize(self) -> Dict[str, Any]:
        return {"status": "ok"}


def get_device_info() -> Dict[str, Any]:
    """Report worker device/capability metadata."""
    return {
        "framework": "pytorch",
        "accelerator": None,
        "device_count": 0,
        "driver_version": None,
        "collective_backend": None,
    }
