from pathlib import Path
import json
import os
import unittest

ROOT = Path(os.environ.get("SF_RELEASE_INPUT_ROOT", Path(__file__).resolve().parents[3]))
OUTPUT = Path(os.environ.get("SF_RELEASE_INVENTORY_OUTPUT", ROOT / ".work/whole-packages/window2-release-preparation/inventory"))


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


class MaterialTests(unittest.TestCase):
    def test_all_original_gates_and_questions_remain_without_promoting_acceptance(self):
        matrix = load(OUTPUT / "full-acceptance-matrix.json")
        trace = load(ROOT / "docs/04_开发与验收/首版需求验收追踪.json")
        for key, count in [("functional_requirements", 17), ("nonfunctional_requirements", 11), ("integration_checks", 3)]:
            self.assertEqual(len(matrix[key]), count)
            self.assertEqual(matrix[key], trace[key])
        self.assertEqual(len(matrix["tasks"]), 21)
        self.assertEqual(sum(len(x) for x in matrix["required_check_groups"].values()), 38)
        packet = load(ROOT / "docs/04_开发与验收/整包派发/P5-01_整包执行.json")
        self.assertEqual(matrix["confirmed_choices"], packet["source_task_snapshot"]["confirmed_design_requirements"])
        self.assertFalse(matrix["whole_release_done"])

    def test_offline_introduction_has_embedded_assets_and_cannot_be_a_runtime_receipt(self):
        docs = ROOT / "docs/05_开源与参赛/release-preparation"
        html = (docs / "output/超级工厂_作品介绍_开发候选.html").read_text(encoding="utf8")
        self.assertIn("data:font/otf;base64,", html)
        self.assertEqual(html.count("data:image/png;base64,"), 2)
        self.assertNotIn('src="file:', html)
        self.assertIn("不是完整软件验收", load(docs / "作品介绍候选.json")["release_status"])
        readiness = load(OUTPUT / "release-readiness.json")
        self.assertFalse(readiness["candidate_ready"])
        self.assertFalse(readiness["offline_distribution_verified"])
        self.assertTrue(all(x is None for x in readiness["external_actions"].values()))


if __name__ == "__main__":
    unittest.main()
