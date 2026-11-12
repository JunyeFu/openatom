"""Candidate closure checks; fixtures do not claim a runnable Native build."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock

spec = importlib.util.spec_from_file_location("package_v2", Path(__file__).with_name("package_v2.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)
native_spec = importlib.util.spec_from_file_location("native_delivery", Path(__file__).with_name("native_delivery.py"))
native_delivery = importlib.util.module_from_spec(native_spec)
native_spec.loader.exec_module(native_delivery)


class CandidateClosureTests(unittest.TestCase):
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
