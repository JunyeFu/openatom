"""Regression checks for actual staged scope and frozen license inputs."""
import importlib.util
import csv
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

CODE_ROOT = Path(__file__).resolve().parents[3]
ROOT = Path(os.environ.get("SF_RELEASE_INPUT_ROOT", CODE_ROOT))
spec = importlib.util.spec_from_file_location("release_inventory_candidate", CODE_ROOT / "tools/release-preparation/inventory.py")
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)
CANDIDATE = ROOT / ".inputs/candidate85"
OUTPUT = Path(os.environ.get("SF_RELEASE_INVENTORY_OUTPUT", ROOT / ".work/whole-packages/window2-release-preparation/inventory"))


class CandidateTests(unittest.TestCase):
    def test_all_mapped_resources_match_staged_bytes(self):
        data = inventory.read_json(OUTPUT / "candidate-closure.json")
        build = inventory.read_json(CANDIDATE / "portable-build.json")
        self.assertEqual(data["matched_resource_bytes"], len(build["resource_files"]))
        self.assertEqual(data["resource_mappings"], build["resource_mappings"])
        self.assertEqual(data["file_issues"], [])
        self.assertTrue(data["portable_entry_bytes_match"])
        self.assertEqual(data["actual_candidate_files"], sum(p.is_file() for p in CANDIDATE.rglob("*")))

    def test_changed_and_unmapped_staged_files_are_reported(self):
        with tempfile.TemporaryDirectory(dir=ROOT / ".work", prefix="release-test-") as temporary:
            root = Path(temporary)
            source = root / "apps/planner/src-tauri/inputs"
            source.mkdir(parents=True)
            (source / "asset.txt").write_bytes(b"right")
            candidate = root / "candidate"
            (candidate / "modules").mkdir(parents=True)
            (candidate / "modules/asset.txt").write_bytes(b"wrong")
            (candidate / "modules/unmapped.txt").write_bytes(b"extra")
            (candidate / "portable-build.json").write_text(json.dumps(dict(status="built_and_staged", entry="superfactory.exe", entry_bytes=1,
                resource_files=[dict(file="modules\\asset.txt", bytes=5)])), encoding="utf8")
            result, files, _ = inventory.candidate_inventory.candidate_files(root, candidate,
                {"bundle": {"resources": {"inputs": "modules"}}})
            issues = {f["file"]: f["issues"] for f in result["file_issues"]}
            self.assertIn("source_bytes_differ", issues["modules/asset.txt"])
            self.assertIn("not_in_frozen_resource_mapping", issues["modules/unmapped.txt"])

    def test_p303_is_actually_bundled_not_an_unbundled_example(self):
        rows = inventory.candidate_inventory.inspect_candidate_assets(CANDIDATE, inventory.asset_rows)
        own = [r for r in rows if "/p303-owned/" in r["source"]]
        self.assertTrue(own)
        self.assertTrue(all(r["scope"] == "candidate85_staged_package" for r in own))
        self.assertTrue(any(r["relative_path"].endswith(".glb") for r in own))

    def test_catalog_asset_paths_use_the_category_directory(self):
        packages = inventory.read_json(CANDIDATE / "modules/devices/template-support/catalog.json")
        package = next(p for p in packages if p["definition"]["id"] == "p401.support.devices")
        row = inventory.asset_rows(package, "modules/devices/template-support/catalog.json", "bundled_support", CANDIDATE)[1]
        self.assertIn("template-support/devices/sources/", row["resolved_asset_path"])
        self.assertTrue(row["file_present"])
        self.assertEqual(row["state"], "unknown_hold")

    def test_missing_asset_and_conflicting_rights_stay_on_hold(self):
        right = dict(asset_ref="image", source="author source", license="MIT", redistribution_confirmed=True)
        package = dict(definition=dict(id="m", version="1", contribution=dict(author="author", license="MIT", assets=[right])),
                       assets=[dict(id="image", path="missing.png")], rights=[dict(right, license="CC0-1.0")])
        with tempfile.TemporaryDirectory(dir=ROOT / ".work", prefix="release-test-") as temporary:
            row = inventory.asset_rows(package, "module.json", "fixture", Path(temporary))[1]
        self.assertEqual(row["state"], "unknown_hold")
        self.assertIn("asset_file_missing", row["hold_reason"])
        self.assertIn("package_rights_conflict", row["hold_reason"])
        self.assertEqual(package["definition"]["contribution"]["assets"][0]["license"], "MIT")

    def test_frozen_direct_registry_does_not_read_home_or_claim_linkage(self):
        compiled = inventory.candidate_inventory.compiler_packages(CANDIDATE)
        with patch.object(Path, "home", side_effect=AssertionError("live cache is not an input")):
            rows = inventory.dependencies(ROOT, ROOT / ".inputs/cargo-registry", compiled)
        cargo = [r for r in rows if r["ecosystem"] == "cargo"]
        self.assertGreater(len(cargo), len(compiled))
        self.assertEqual(sum(r["compiler_frontier_present"] for r in cargo), len(compiled))
        self.assertTrue(all(r["final_binary_linkage"] == "not_proven_by_compiler_frontier" for r in cargo))
        self.assertTrue(all(r["metadata_present"] for r in cargo))
        self.assertTrue(any("license-mit" in r["notice_files"] for r in cargo))

    def test_nine_categories_and_unresolved_rights_are_separate_from_acceptance(self):
        data = inventory.read_json(OUTPUT / "candidate-closure.json")
        expected = {"items", "devices", "resources", "tasks", "operations", "transport", "templates", "professional", "media"}
        self.assertTrue(expected.issubset(data["known_declaration_categories"]))
        self.assertTrue(data["named_unresolved_asset_declarations"])
        self.assertFalse(data["whole_release_done"])
        self.assertIsNone(data["actual_runtime"])
        self.assertTrue(data["diagnostic_devtools_candidate"])

    def test_notice_pool_preserves_texts_without_granting_clearance(self):
        with (OUTPUT / "third-party-notices.csv").open(encoding="utf-8-sig") as source:
            rows = list(csv.DictReader(source))
        data = inventory.read_json(OUTPUT / "candidate-closure.json")
        self.assertEqual(len(rows), data["copied_notice_texts"])
        self.assertTrue(all(r["copied_verbatim"] == "True" for r in rows))
        for ecosystem in ("cargo", "npm", "asset"):
            row = next(r for r in rows if r["ecosystem"] == ecosystem)
            self.assertEqual((ROOT / row["source"]).read_bytes(), (OUTPUT / row["destination"]).read_bytes())
        self.assertTrue(any("license_text_missing" in r["findings"] for r in data["named_dependency_reviews"]))
        self.assertTrue(data["no_root_license_granted"])


if __name__ == "__main__":
    unittest.main()
