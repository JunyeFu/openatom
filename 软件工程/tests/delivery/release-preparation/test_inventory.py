import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("release_inventory", ROOT / "tools/release-preparation/inventory.py")
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)


class RightsTests(unittest.TestCase):
    def test_unknown_author_or_license_remains_on_hold(self):
        for author, license_text in [(None, "MIT"), ("known", None), ("known", "待声明"), ("待填写", "MIT")]:
            self.assertEqual(inventory.license_state(author, license_text), "unknown_hold")

    def test_module_license_cannot_authorize_unconfirmed_asset(self):
        package = {"definition": {"id": "m", "version": "1.0.0", "contribution": {
            "author": "author", "license": "MIT", "assets": [{"asset_ref": "image", "source": "source", "license": "MIT", "redistribution_confirmed": False}]}},
            "assets": [{"id": "image", "path": "image.png"}]}
        self.assertEqual(inventory.asset_rows(package, "module.json", "fixture")[1]["state"], "unknown_hold")

    def test_declared_rights_are_not_final_release_approval(self):
        self.assertEqual(inventory.license_state("known", "OFL-1.1"), "declared_requires_notices_and_owner_release")


if __name__ == "__main__":
    unittest.main()
