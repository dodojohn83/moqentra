"""Minimal gRPC worker agent client generated from worker.proto.

The client opens a bidirectional stream to the Moqentra control plane,
registers capabilities, sends periodic heartbeats, and dispatches inbound
commands to a user-supplied handler. It intentionally does not access the
control-plane database; all credentials are short-lived and passed through the
environment.
"""

from __future__ import annotations

import contextlib
import os
import queue
import threading
from typing import Any, Callable, Dict, Iterator, Optional

import grpc  # type: ignore[import]

from moqentra.worker.v1 import worker_pb2  # type: ignore[import]
from moqentra.worker.v1 import worker_pb2_grpc  # type: ignore[import]

from .sdk import get_device_info

CommandHandler = Callable[[worker_pb2.Command], Optional[worker_pb2.Result]]


class WorkerAgentClient:
    """Bidirectional worker agent client using generated proto stubs."""

    def __init__(
        self,
        endpoint: str,
        node_id: str,
        agent_version: str = "0.1.0",
        handler: Optional[CommandHandler] = None,
    ) -> None:
        self.endpoint = endpoint
        self.node_id = node_id
        self.agent_version = agent_version
        self.handler = handler
        self._out: "queue.Queue[Optional[worker_pb2.WorkerAgentServiceOpenStreamRequest]]" = queue.Queue(maxsize=64)
        self._stop = threading.Event()

    def run(self) -> None:
        """Connect and block on the worker stream until the server drains us."""
        channel = grpc.insecure_channel(self.endpoint)
        stub = worker_pb2_grpc.WorkerAgentServiceStub(channel)
        self._out.put(
            worker_pb2.WorkerAgentServiceOpenStreamRequest(
                hello=worker_pb2.Hello(
                    node_id=self.node_id,
                    agent_version=self.agent_version,
                    capabilities=self._capabilities(),
                )
            )
        )

        heartbeat = threading.Thread(target=self._heartbeat, daemon=True)
        heartbeat.start()
        try:
            for resp in stub.OpenStream(self._iter_out()):
                self._handle(resp)
        finally:
            self._stop.set()
            with contextlib.suppress(queue.Full):
                self._out.put(None, timeout=1.0)

    def _iter_out(self) -> Iterator[worker_pb2.WorkerAgentServiceOpenStreamRequest]:
        while True:
            req = self._out.get()
            if req is None:
                break
            yield req

    def _heartbeat(self) -> None:
        seq = 0
        while not self._stop.wait(10.0):
            seq += 1
            req = worker_pb2.WorkerAgentServiceOpenStreamRequest(
                heartbeat=worker_pb2.Heartbeat(sequence=seq)
            )
            with contextlib.suppress(queue.Full):
                self._out.put(req, timeout=1.0)

    def _capabilities(self) -> worker_pb2.WorkerCapabilities:
        info: Dict[str, Any] = get_device_info()
        return worker_pb2.WorkerCapabilities(
            agent_build_version=self.agent_version,
            contract_version="1",
            frameworks=[worker_pb2.Framework(name="PyTorch", version="2.6.0")],
            hardware_label=self.node_id,
            device_labels=info.get("device_labels", ["cpu"]),
            driver_version=info.get("driver_version") or "n/a",
            runtime_version=info.get("runtime_version") or "n/a",
            runtimes=info.get("runtimes", ["cpu"]),
            model_formats=[worker_pb2.ModelFormat(name="onnx", version=["1.17"])],
            collective_backend="",
            device_memory_bytes=info.get("device_memory_bytes", 0),
            max_parallelism=info.get("max_parallelism", 1),
            supports_gpu=info.get("supports_gpu", False),
            supports_npu=info.get("supports_npu", False),
        )

    def _handle(self, resp: worker_pb2.WorkerAgentServiceOpenStreamResponse) -> None:
        payload = resp.payload
        which: Any = payload.WhichOneof("payload")
        if which == "command":
            command: Any = payload.command
            ack = worker_pb2.Ack(
                command_id=command.command_id,
                status=worker_pb2.AckStatus.Received,
            )
            self._out.put(worker_pb2.WorkerAgentServiceOpenStreamRequest(ack=ack))
            if self.handler:
                result = self.handler(command)
            else:
                result = worker_pb2.Result(
                    command_id=command.command_id, success=True, payload=b""
                )
            if result is not None:
                self._out.put(
                    worker_pb2.WorkerAgentServiceOpenStreamRequest(result=result)
                )
        elif which == "lease":
            lease: Any = payload.lease
            # A lease is an implicit keepalive; no immediate response required.
            os.environ["MOQENTRA_LEASE_ID"] = lease.lease_id
        elif which == "drain":
            self._stop.set()

