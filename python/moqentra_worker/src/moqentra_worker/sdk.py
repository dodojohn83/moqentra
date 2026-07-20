"""Moqentra Python worker SDK.

The SDK exposes the worker lifecycle and framework adapters. It intentionally
does not access the control-plane database; all credentials are short-lived and
passed through the environment by the Rust control plane.
"""

from __future__ import annotations

import contextlib
import dataclasses
import os
import signal
import sys
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Protocol


class WorkerLifecycle(Protocol):
    """Framework-adapter lifecycle implemented by user training code."""

    def prepare(self, config: Dict[str, Any]) -> None: ...
    def run(self) -> Dict[str, Any]: ...
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
        self._metrics.append(point)

    def report_metrics(self, points: List[MetricPoint]) -> None:
        for point in points:
            self.report_metric(point)

    def save_checkpoint(self, adapter: WorkerLifecycle, path: Optional[Path] = None) -> str:
        if self._cancelled:
            raise RuntimeError("worker has been cancelled")
        target = path or (self.output_dir / "checkpoints" / f"step-{len(self._metrics)}")
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
        attempt_id = config["attempt_id"]
        fencing_token = config["fencing_token"]
        work_dir = Path(config.get("work_dir", "/tmp/moqentra/work"))
        input_dir = Path(config.get("input_dir", "/tmp/moqentra/input"))
        output_dir = Path(config.get("output_dir", "/tmp/moqentra/output"))

        work_dir.mkdir(parents=True, exist_ok=True)
        input_dir.mkdir(parents=True, exist_ok=True)
        output_dir.mkdir(parents=True, exist_ok=True)

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
            result = self.adapter.run()
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

    def run(self) -> Dict[str, Any]:
        return {}

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
