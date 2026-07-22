import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from moqentra_worker import (
    generate_classification_fixture,
    generate_detection_fixture,
    generate_segmentation_fixture,
)


class TestFixtures(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_classification_deterministic(self):
        generate_classification_fixture(self.root, n=10, seed=123, split="test")
        first = list((self.root / "test" / "images").glob("*.png"))
        self.assertEqual(len(first), 10)
        ann = (self.root / "test" / "annotations.json").read_text()
        self.assertIn("circle", ann)

    def test_classification_reproducible(self):
        root1 = Path(tempfile.mkdtemp())
        root2 = Path(tempfile.mkdtemp())
        try:
            generate_classification_fixture(root1, n=10, seed=7, split="s")
            generate_classification_fixture(root2, n=10, seed=7, split="s")
            for a, b in zip(
                sorted((root1 / "s" / "images").glob("*.png")),
                sorted((root2 / "s" / "images").glob("*.png")),
            ):
                self.assertEqual(a.read_bytes(), b.read_bytes())
        finally:
            shutil.rmtree(root1, ignore_errors=True)
            shutil.rmtree(root2, ignore_errors=True)

    def test_detection_fixture(self):
        generate_detection_fixture(self.root, n=5, seed=1, split="train")
        self.assertEqual(len(list((self.root / "train" / "images").glob("*.png"))), 5)
        ann = (self.root / "train" / "annotations.json").read_text()
        self.assertIn("annotations", ann)

    def test_segmentation_fixture(self):
        generate_segmentation_fixture(self.root, n=5, seed=2, split="train")
        self.assertTrue((self.root / "train" / "masks").exists())
        ann = (self.root / "train" / "annotations.json").read_text()
        self.assertIn("mask_dir", ann)


if __name__ == "__main__":
    unittest.main()
