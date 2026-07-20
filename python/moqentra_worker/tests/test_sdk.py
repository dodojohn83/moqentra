import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from moqentra_worker import MetricPoint, PyTorchAdapter, WorkerRuntime


def _make_config(root):
    return {
        "attempt_id": "attempt-1",
        "fencing_token": "token-1",
        "worker_root": str(root),
        "work_dir": str(root / "work"),
        "input_dir": str(root / "input"),
        "output_dir": str(root / "output"),
    }


def test_metric_and_cancel():
    session = None

    def train_fn(sess):
        nonlocal session
        session = sess
        sess.report_metric(MetricPoint(step=1, name="loss", value=0.5))
        return {"accuracy": 0.9}

    adapter = PyTorchAdapter(train_fn)
    runtime = WorkerRuntime(adapter)
    config = {
        "attempt_id": "attempt-1",
        "fencing_token": "token-1",
        "worker_root": "/tmp/moqentra-test",
        "work_dir": "/tmp/moqentra-test/work",
        "input_dir": "/tmp/moqentra-test/input",
        "output_dir": "/tmp/moqentra-test/output",
    }
    result = runtime.run(config)
    assert result["attempt_id"] == "attempt-1"
    assert len(result["metrics"]) == 1
    assert result["metrics"][0]["name"] == "loss"


def test_device_info():
    from moqentra_worker import get_device_info

    info = get_device_info()
    assert "framework" in info


def test_save_checkpoint_relative_path_resolves_to_output_dir():
    root = Path("/tmp/moqentra-test-checkpoint")
    if root.exists():
        shutil.rmtree(root)

    def train_fn(sess):
        digest = sess.save_checkpoint(adapter, Path("rel-ckpt.bin"))
        assert digest == "sha256:checkpoint"
        assert (root / "output" / "rel-ckpt.bin").exists()
        return {}

    adapter = PyTorchAdapter(train_fn)
    runtime = WorkerRuntime(adapter)
    runtime.run(_make_config(root))
    shutil.rmtree(root, ignore_errors=True)


def test_save_checkpoint_rejects_outside_path():
    root = Path("/tmp/moqentra-test-outside")
    if root.exists():
        shutil.rmtree(root)

    def train_fn(sess):
        try:
            sess.save_checkpoint(adapter, Path("/etc/passwd"))
            raise AssertionError("outside path should be rejected")
        except ValueError:
            pass
        return {}

    adapter = PyTorchAdapter(train_fn)
    runtime = WorkerRuntime(adapter)
    runtime.run(_make_config(root))
    shutil.rmtree(root, ignore_errors=True)
