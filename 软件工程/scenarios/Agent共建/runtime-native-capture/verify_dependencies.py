import json
import sys
import tomllib
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[2]
before = tomllib.loads((here / "Cargo.lock.before").read_text(encoding="utf-8"))
after = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))

def package_key(package):
    return package["name"], package["version"], package.get("source")

old_packages = {package_key(p): p for p in before["package"]}
new_packages = {package_key(p): p for p in after["package"]}
assert old_packages.keys() == new_packages.keys(), "root package/version set changed"
changed = []
for key, old in old_packages.items():
    new = new_packages[key]
    if old == new:
        continue
    assert key[:2] == ("sf-desktop-host", "0.1.0"), f"unexpected package change: {key}"
    expected = dict(old)
    expected["dependencies"] = sorted(old["dependencies"] + ["webview2-com", "windows"])
    assert new == expected, "Native dependency delta differs from the approved two entries"
    changed.append(key[0])
assert changed == ["sf-desktop-host"], "missing or extra root lock change"

manifest = tomllib.loads((root / "apps/planner/src-tauri/Cargo.toml").read_text(encoding="utf-8"))
native_before = tomllib.loads((here / "native-Cargo.toml.before").read_text(encoding="utf-8"))
expected_manifest = dict(native_before)
expected_manifest["target"] = {
    "cfg(windows)": {
        "dependencies": {
            "webview2-com": "=0.39.1",
            "windows": {"version": "=0.62.2", "features": [
                "Win32_Foundation", "Win32_System_Com", "Win32_System_Com_StructuredStorage"
            ]},
        }
    }
}
assert manifest == expected_manifest, "Native manifest has an unexpected change"

result = {
    "root_package_versions_unchanged": True,
    "root_package_count": len(new_packages),
    "root_changed_packages": changed,
    "Native_dependency_additions": ["webview2-com =0.39.1", "windows =0.62.2"],
    "Native_manifest_delta_exact": True,
}
if "--harness" in sys.argv:
    harness = tomllib.loads((here / "compile-harness/Cargo.lock").read_text(encoding="utf-8"))
    harness_packages = {package_key(p): p for p in harness["package"]}
    unexpected = [key for key in harness_packages if key not in new_packages
                  and key[:2] != ("sf-native-render-capture-compile", "0.1.0")]
    assert not unexpected, f"harness resolved packages outside the existing lock: {unexpected}"
    result["harness_uses_only_existing_locked_versions"] = True
    result["harness_package_count"] = len(harness_packages)
print(json.dumps(result, ensure_ascii=False, indent=2))
