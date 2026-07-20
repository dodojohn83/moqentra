import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from moqentra_worker import MetricPoint, PyTorchAdapter, WorkerRuntime


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
