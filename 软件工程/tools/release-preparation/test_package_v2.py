"""Candidate closure checks; fixtures do not claim a runnable Native build."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import json
import zipfile
from unittest.mock import Mock

spec = importlib.util.spec_from_file_location("package_v2", Path(__file__).with_name("package_v2.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)
native_spec = importlib.util.spec_from_file_location("native_delivery", Path(__file__).with_name("native_delivery.py"))
native_delivery = importlib.util.module_from_spec(native_spec)
native_spec.loader.exec_module(native_delivery)


def portable_fixture(root):
    source, native, runtime = (root / name for name in ("source", "native", "runtime"))
    for directory in (source, native, runtime):
        directory.mkdir()
    for name in ("superfactory.exe", "superfactory-mcp-bridge.exe"):
        (native / name).write_bytes(b"MZfixture")
    config = source / "apps/planner/src-tauri/tauri.conf.json"
    config.parent.mkdir(parents=True)
    config.write_text(json.dumps({"bundle": {"resources": {}}}), encoding="utf-8")
    for relative in package.AGENT_ENTRY_FILES:
        entry = source / relative
        entry.parent.mkdir(parents=True, exist_ok=True)
        entry.write_text("fixture source: " + relative, encoding="utf-8")
    for relative in ("runtime.json", "python/python.exe", "python/python314.dll", "python/python314.zip",
                     "licenses/PYTHON-LICENSE.txt"):
        entry = runtime / relative
        entry.parent.mkdir(parents=True, exist_ok=True)
        entry.write_bytes(b"fixture")
    (runtime / "python/python314._pth").write_text("python314.zip\n.\n../scripts\n", encoding="ascii")
    (runtime / "scripts").mkdir()
    for name in package.RUNTIME_SCRIPTS:
        (runtime / "scripts" / name).write_text("fixture", encoding="utf-8")
    return source, native / "superfactory.exe", runtime


class CandidateClosureTests(unittest.TestCase):
    def test_agent_entry_survives_actual_portable_zip_roundtrip(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, native, runtime = portable_fixture(root)
            output, archive = root / "portable", root / "fixture.zip"
            result = package.assemble(source, native, runtime, output, "fixture", fixture=True, zip_output=archive)
            self.assertTrue(result["fixture_only"])
            with zipfile.ZipFile(archive) as stream:
                self.assertIsNone(stream.testzip())
                for relative in package.AGENT_ENTRY_FILES:
                    target = "agent-entry/" + relative
                    self.assertEqual(stream.read(target), (source / relative).read_bytes())
                    self.assertTrue(any(row["path"] == target and row["role"] == "agent_consumer_source" for row in result["files"]))

    def test_missing_agent_entry_rejected_before_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, native, runtime = portable_fixture(root)
            (source / "integrations/agent/stdio_client.py").unlink()
            with self.assertRaisesRegex(package.PackageError, "Agent consumer source is missing"):
                package.assemble(source, native, runtime, root / "portable", "fixture", fixture=True)
            self.assertFalse((root / "portable").exists())

    def test_lfs_resource_rejected_before_any_portable_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, native, runtime = portable_fixture(root)
            config = source / "apps/planner/src-tauri/tauri.conf.json"
            config.write_text(json.dumps({"bundle": {"resources": {"font.bin": "report-resources/font.bin"}}}), encoding="utf-8")
            config.with_name("font.bin").write_bytes(b"version https://git-lfs.github.com/spec/v1\noid sha256:fixture\nsize 5\n")
            with self.assertRaisesRegex(package.PackageError, "LFS resource is not materialized"):
                package.assemble(source, native, runtime, root / "portable", "fixture", fixture=True)
            self.assertFalse((root / "portable").exists())

    def test_native_delivery_cannot_silently_omit_agent_bridge(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "superfactory.exe").write_bytes(b"MZfixture")
            output = root / "portable"
            with self.assertRaisesRegex(ValueError, "required candidate executable.*bridge"):
                native_delivery.assemble(root, root, root, output, root / "candidate.zip", "source-fixture")
            self.assertFalse(output.exists())

    def test_real_resource_tree_on_python311(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assets = root / "assets"
            assets.mkdir()
            member = assets / "image.bin"
            member.write_bytes(b"actual bytes")
            self.assertEqual(package.collect_tree(assets, root, Path("resources"), "asset"),
                             [(member, Path("resources/image.bin"), "asset")])

    def test_junction_tag_remains_rejected(self):
        junction = Mock()
        junction.is_symlink.return_value = False
        junction.lstat.return_value.st_reparse_tag = package.stat.IO_REPARSE_TAG_MOUNT_POINT
        self.assertTrue(package.linked(junction))

    def test_missing_bridge_rejects_before_creating_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            native = root / "superfactory.exe"
            native.write_bytes(b"MZfixture")
            config = root / "apps/planner/src-tauri/tauri.conf.json"
            config.parent.mkdir(parents=True)
            config.write_text('{"bundle":{"resources":{}}}', encoding="utf-8")
            output = root / "portable"
            runtime = root / "runtime"
            runtime.mkdir()
            with self.assertRaisesRegex(package.PackageError, "required candidate executable.*bridge"):
                package.assemble(root, native, runtime, output, "source-fixture", fixture=True)
            self.assertFalse(output.exists())

    def test_invalid_bridge_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            native = root / "superfactory.exe"
            native.write_bytes(b"MZfixture")
            (root / "superfactory-mcp-bridge.exe").write_bytes(b"not an executable")
            with self.assertRaisesRegex(package.PackageError, "not a Windows executable.*bridge"):
                package.candidate_binaries(native)

    def test_required_bridge_and_optional_loader_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ("superfactory.exe", "superfactory-mcp-bridge.exe", "WebView2Loader.dll"):
                (root / name).write_bytes(b"MZfixture")
            planned = package.candidate_binaries(root / "superfactory.exe")
            self.assertEqual([destination.name for _, destination, _ in planned],
                             ["superfactory.exe", "superfactory-mcp-bridge.exe", "WebView2Loader.dll"])

    def test_lfs_pointer_with_either_line_ending_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            asset = Path(temporary) / "font.otf"
            for newline in (b"\n", b"\r\n"):
                with self.subTest(newline=newline):
                    asset.write_bytes(newline.join((b"version https://git-lfs.github.com/spec/v1",
                                                   b"oid sha256:" + b"a" * 64, b"size 123", b"")))
                    with self.assertRaisesRegex(package.PackageError, "LFS resource is not materialized"):
                        package.require_materialized(asset)

    def test_real_resource_is_accepted_without_modification(self):
        with tempfile.TemporaryDirectory() as temporary:
            asset = Path(temporary) / "image.bin"
            original = b"\x00\xffactual asset\n"
            asset.write_bytes(original)
            package.require_materialized(asset)
            self.assertEqual(asset.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
