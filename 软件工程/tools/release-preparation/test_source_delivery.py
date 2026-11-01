"""Source delivery boundaries and an actual archive/extraction round trip."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location("source_delivery", Path(__file__).with_name("source_delivery.py"))
delivery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(delivery)


def put(root, name, value="source"):
    p = root / name
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(value) if isinstance(value, dict) else value, encoding="utf-8")


def fixture(root):
    for name in delivery.REQUIRED:
        put(root, name)
    put(root, "Cargo.toml", '[workspace]\nmembers=["packages/example"]\n[workspace.package]\nversion="0.1.0"\n')
    put(root, "packages/example/Cargo.toml", "[package]\nname='example'\nversion='0.1.0'\n")
    put(root, "package.json", {"version": "0.1.0", "workspaces": ["apps/planner"], "engines": {"node": "24.15.0"}})
    put(root, "apps/planner/package.json", {})
    put(root, "apps/planner/src-tauri/tauri.conf.json", {"version": "0.1.0", "bundle": {"resources": {"../../../tools/font.otf": "font.otf"}, "icon": ["icon.png"]}})
    put(root, "apps/planner/src-tauri/icon.png")
    put(root, "tools/font.otf")
    put(root, "modules/catalog.json", {"modules": []})
    for folder in ("machining-delivery", "purchased-assembly", "parallel-electromechanical", "carton-factory"):
        for name in ("module.json", "template.json", "example-project.json", "definitions.json"):
            put(root, f"modules/templates/{folder}/candidate/{name}", {})
    put(root, "scenarios/非功能基准/load.json", {})


class SourceDeliveryTests(unittest.TestCase):
    def test_pruned_historical_referrer_cannot_keep_orphaned_inputs_after_round_trip(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "source"
            fixture(root)
            prefix = "scenarios/媒体表现/runtime/"
            put(root, prefix + "runner.rs", '#[path="needed/first.rs"] mod first;')
            put(root, prefix + "needed/first.rs", 'const DATA: &str = include_str!("second.json");')
            put(root, prefix + "needed/second.json", {"needed": True})
            put(root, prefix + "history/old.rs", 'const OLD: &str = include_str!("orphan.json");')
            put(root, prefix + "history/orphan.json", {"historical": True})
            output = Path(temp) / "source.zip"
            result = delivery.package(root, output, "stable-source", extract_to=Path(temp) / "clean")
            self.assertTrue(result["extracted_bytes_verified"])
            with zipfile.ZipFile(output) as archive:
                self.assertIn(prefix + "needed/second.json", archive.namelist())
                self.assertNotIn(prefix + "history/old.rs", archive.namelist())
                self.assertNotIn(prefix + "history/orphan.json", archive.namelist())

    def test_v2_device_sources_and_demo_survive_archive_extraction(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "source"
            fixture(root)
            config_path = root / "apps/planner/src-tauri/tauri.conf.json"
            config = json.loads(config_path.read_text(encoding="utf-8"))
            config["bundle"]["resources"]["../../../resources/v2-devices/"] = "v2-devices/"
            put(root, "apps/planner/src-tauri/tauri.conf.json", config)
            kept = ("resources/v2-devices/scripts/virtual_device.py", "resources/v2-devices/runtime.json",
                    "resources/v2-devices/python/python.exe", "resources/v2-devices/python/python314.dll",
                    "resources/v2-devices/python/python314.zip",
                    "resources/v2-demo/geometry-v4/project/project.json")
            for name in kept:
                put(root, name, {"actual_input": name})
            output = Path(temp) / "source.zip"
            result = delivery.package(root, output, "v2-test", extract_to=Path(temp) / "clean")
            self.assertTrue(result["extracted_bytes_verified"])
            with zipfile.ZipFile(output) as archive:
                for name in kept:
                    self.assertEqual(archive.read(name), (root / name).read_bytes())

    def test_actual_new_source_and_resource_survive_zip_without_cache_or_credentials(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "source"
            fixture(root)
            put(root, "apps/planner/src/not-yet-tracked.ts", "export const value = 7;")
            for name in (".work/private.json", "packages/example/target/cache", "apps/planner/.env.local", "tools/client.key", "scenarios/媒体表现/model-quality-20261019/old.png"):
                put(root, name, "must not ship")
            output = Path(temp) / "source.zip"
            result = delivery.package(root, output, "test-candidate", extract_to=Path(temp) / "clean")
            self.assertTrue(result["extracted_bytes_verified"])
            with zipfile.ZipFile(output) as archive:
                names = archive.namelist()
                self.assertIn("apps/planner/src/not-yet-tracked.ts", names)
                self.assertIn("tools/font.otf", names)
                self.assertFalse(any("must not ship" in archive.read(n).decode("utf-8", errors="ignore") for n in names))

    def test_missing_native_resource_rejects_before_creating_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "source"
            fixture(root)
            (root / "tools/font.otf").unlink()
            output = Path(temp) / "source.zip"
            with self.assertRaisesRegex(ValueError, "Native resource absent"):
                delivery.package(root, output, "broken")
            self.assertFalse(output.exists())

    def test_frozen_supplement_adds_missing_input_but_cannot_replace_source(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "source"
            supplement = Path(temp) / "inputs"
            fixture(root)
            put(supplement, "community/CONTRIBUTING.md", "existing contribution rules")
            files, _ = delivery.collect(root, supplement)
            self.assertIn("community/CONTRIBUTING.md", files)
            put(supplement, "README.md", "different source")
            with self.assertRaisesRegex(ValueError, "Supplement conflicts"):
                delivery.collect(root, supplement)

    def test_npm_auth_setting_is_rejected_instead_of_printed_or_archived(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixture(root)
            put(root, ".npmrc", "//registry.example/:_authToken=private-test-value\n")
            with self.assertRaisesRegex(ValueError, "Credential-bearing"):
                delivery.collect(root)

    def test_frontend_json_import_must_be_in_source_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixture(root)
            put(root, "apps/planner/src/example.ts", "import frames from '../../../scenarios/媒体表现/runtime/frames.json';")
            files, _ = delivery.collect(root)
            with self.assertRaisesRegex(ValueError, "Frontend JSON input absent"):
                delivery.closure(files)

    def test_declared_cargo_target_source_must_be_in_archive(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixture(root)
            put(root, "packages/example/Cargo.toml", "[package]\nname='example'\nversion='0.1.0'\n[[test]]\nname='external'\npath='../../scenarios/external.rs'\n")
            files, _ = delivery.collect(root)
            with self.assertRaisesRegex(ValueError, "Declared Cargo target absent"):
                delivery.closure(files)

    def test_external_rust_path_module_is_packed_and_missing_source_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixture(root)
            put(root, "packages/example/src/lib.rs", '#[path = "../../../integrations/agent/runtime/contract.rs"]\nmod contract;')
            put(root, "integrations/agent/runtime/contract.rs", "pub const VERSION: u8 = 1;")
            files, _ = delivery.collect(root)
            self.assertIn("integrations/agent/runtime/contract.rs", files)
            delivery.closure(files)
            (root / "integrations/agent/runtime/contract.rs").unlink()
            files, _ = delivery.collect(root)
            with self.assertRaisesRegex(ValueError, "Rust path module absent"):
                delivery.closure(files)

    def test_confirmed_generated_outputs_are_omitted_without_removing_real_assets(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixture(root)
            omitted = [
                "scenarios/媒体表现/runtime-l03/borrowed-key-core-sdk/deps/rust_fontconfig-test.lib",
                "scenarios/媒体表现/runtime-l03/borrowed-key-core-sdk/.fingerprint/result.json",
                "scenarios/媒体表现/runtime-l03/variants/sealer-output-wait/project-moved/project.json",
                "scenarios/媒体表现/runtime-l03/styles/gif/old-run/project.json",
                "scenarios/媒体表现/runtime-l03/execution-1792179087534697200/project.json",
                "scenarios/媒体表现/runtime-l03/native-20261017-051541/project.json",
                "scenarios/媒体表现/runtime-item-layout-consumer/browser.json",
                "scenarios/媒体表现/runtime-item-layout-consumer/browser-context-warning-before.json",
                "scenarios/媒体表现/model-quality-20261022/old.png",
            ]
            kept = [
                "modules/media/example/model.glb", "modules/media/example/texture.png",
                "modules/media/example/browser.json", "tools/font.otf",
                "scenarios/媒体表现/runtime-l03/engine-input.json",
                "scenarios/媒体表现/runtime/frames.json",
                "scenarios/Agent共建/runtime-native-product-capture/frame-key-original-browser.json",
                "scenarios/非功能基准/input.json",
            ]
            for name in omitted + kept:
                put(root, name)
            files, _ = delivery.collect(root)
            for name in omitted:
                self.assertNotIn(name, files, name)
            for name in kept:
                self.assertIn(name, files, name)
            delivery.closure(files)

    def test_literal_rust_compile_fixture_remains_required(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixture(root)
            put(root, "packages/example/src/lib.rs", 'pub const DATA: &str = include_str!("../../../scenarios/fixture.json");')
            files, _ = delivery.collect(root)
            with self.assertRaisesRegex(ValueError, "Rust include input absent"):
                delivery.closure(files)
            put(root, "scenarios/fixture.json", {})
            files, _ = delivery.collect(root)
            delivery.closure(files)


class CleanCandidateTests(unittest.TestCase):
    def test_old_runtime_reports_and_archives_are_excluded_but_imported_input_survives(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);fixture(root)
            put(root,'scenarios/媒体表现/runtime/actual-frames.json',{'frozen':'input'})
            put(root,'apps/planner/src/reader.ts',"import frames from '../../../scenarios/媒体表现/runtime/actual-frames.json';")
            for path in ['scenarios/媒体表现/runtime/browser.json','scenarios/媒体表现/runtime/session/frame.png','docs/old/output/report.pdf','docs/old/archive.zip','tests/view/webview-1/cache.bin','tools/design-quality-audit/prototype-parallel/window1/after.json','tools/design-quality-audit/report-linked/window2/report.json']:
                put(root,path,'old output')
            files,_=delivery.collect(root)
            self.assertIn('scenarios/媒体表现/runtime/actual-frames.json',files)
            self.assertFalse(any(path.endswith(('report.pdf','archive.zip','cache.bin','frame.png','browser.json','after.json','report.json')) for path in files))
            delivery.closure(files)

    def test_licenses_are_retained_and_named_caches_pruned(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);fixture(root);put(root,'licenses/third-party/react/LICENSE','original license')
            for path in ['tools/probe/.cache/data','tools/probe/npm-cache/data','tests/view/webview2-profile/data']:
                put(root,path,'cache')
            files,_=delivery.collect(root)
            self.assertIn('licenses/third-party/react/LICENSE',files)
            self.assertFalse(any(path.endswith('/data') for path in files))


if __name__ == "__main__":
    unittest.main()
