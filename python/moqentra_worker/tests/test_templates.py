import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

try:
    import torch
    import torchvision

    TORCH_AVAILABLE = True
except ImportError:
    TORCH_AVAILABLE = False

from moqentra_worker import (
    WorkerRuntime,
    generate_classification_fixture,
    generate_detection_fixture,
    generate_segmentation_fixture,
)
from moqentra_worker.templates import (
    DeepLabV3SegmentationTemplate,
    ResNet18ClassificationTemplate,
    SSDLite320DetectionTemplate,
)


@unittest.skipUnless(TORCH_AVAILABLE, "torch/torchvision not installed")
class TestTemplates(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def _base_config(self, **kwargs):
        config = {
            "attempt_id": "attempt-test",
            "fencing_token": "token-test",
            "worker_root": str(self.root),
            "output_dir": str(self.root / "output"),
            "seed": 11,
            "epochs": 1,
        }
        config.update(kwargs)
        return config

    def test_resnet18_classification(self):
        generate_classification_fixture(self.root, n=4, seed=11, split="train")
        config = self._base_config(
            input_dir=str(self.root / "train"),
            num_classes=3,
            batch_size=2,
            image_size=32,
        )
        result = WorkerRuntime(ResNet18ClassificationTemplate()).run(config)
        self.assertNotIn("error", result, result.get("error"))
        manifest = Path(self.root / "output" / "manifest.json")
        self.assertTrue(manifest.exists())
        self.assertTrue((self.root / "output" / "model.onnx").exists())
        self.assertTrue((self.root / "output" / "onnx_evaluation_report.json").exists())
        onnx_eval = result["manifest"].get("onnx_evaluation_report", {})
        self.assertTrue(onnx_eval.get("passed"), onnx_eval)

    def test_ssdlite320_detection(self):
        generate_detection_fixture(self.root, n=4, seed=11, split="train")
        config = self._base_config(
            input_dir=str(self.root / "train"),
            batch_size=2,
            lr=0.005,
        )
        result = WorkerRuntime(SSDLite320DetectionTemplate()).run(config)
        self.assertNotIn("error", result, result.get("error"))
        manifest = Path(self.root / "output" / "manifest.json")
        self.assertTrue(manifest.exists())
        self.assertTrue((self.root / "output" / "model.onnx").exists())

    def test_deeplabv3_segmentation(self):
        generate_segmentation_fixture(self.root, n=4, seed=11, split="train")
        config = self._base_config(
            input_dir=str(self.root / "train"),
            batch_size=2,
        )
        result = WorkerRuntime(DeepLabV3SegmentationTemplate()).run(config)
        self.assertNotIn("error", result, result.get("error"))
        manifest = Path(self.root / "output" / "manifest.json")
        self.assertTrue(manifest.exists())
        self.assertTrue((self.root / "output" / "model.onnx").exists())
        self.assertTrue((self.root / "output" / "onnx_evaluation_report.json").exists())
        onnx_eval = result["manifest"].get("onnx_evaluation_report", {})
        self.assertTrue(onnx_eval.get("passed"), onnx_eval)
        self.assertTrue((self.root / "output" / "mask_preview.png").exists())


if __name__ == "__main__":
    unittest.main()
