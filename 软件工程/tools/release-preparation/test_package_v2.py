"""Candidate closure checks; fixtures do not claim a runnable Native build."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("package_v2", Path(__file__).with_name("package_v2.py"))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class CandidateClosureTests(unittest.TestCase):
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
