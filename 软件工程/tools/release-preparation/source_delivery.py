"""Pack the actual Windows source tree, without relying on Git's tracked list.

Python 3.11+, standard library only. This does not install, build or start software.
"""
import argparse
import json
import os
from pathlib import Path
import re
import stat
import tomllib
import zipfile


ROOT_FILES = (
    "README.md", "CONTEXT.md", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    "package.json", "package-lock.json", ".npmrc", ".gitignore", "LICENSE", "NOTICE", "LICENSE_STATUS.md",
)
ROOT_DIRS = ("apps", "packages", "integrations", "assets", "modules", "resources", "scenarios", "community", "docs", "tests", "tools", "licenses")
OMIT_DIRS = {".work", ".git", "node_modules", "target", "__pycache__", ".venv", "tmp", "temp", ".cache", ".vite", ".pytest_cache", "npm-cache", "webview2-profile"}
OMIT_ROOTS = (
    # Observed historical audit snapshots and reports; not product source or fixtures.
    "tools/design-quality-audit/prototype-parallel",
    "tools/design-quality-audit/prototype-logic",
    "tools/design-quality-audit/report-linked",
    # Observed C12 historical npm/WebView2 caches, not source inputs.
    "docs/04_开发与验收/特征细化/DRF04_S05-desktop-proof/npm-cache",
    "docs/04_开发与验收/特征细化/DRF04_S05-desktop-proof/output/webview2-profile",
    "apps/planner/dist", "apps/planner/src-tauri/gen",
    # Historical image/video measurements, not module sources or benchmark inputs.
    "scenarios/媒体表现/model-quality-20261019", "scenarios/媒体表现/native50-20261021",
    "scenarios/媒体表现/model-quality-20261022",
    # Observed Native500 acceptance screenshots (over 5 GiB), not runtime/model inputs.
    "scenarios/媒体表现/native500-20261024",
    "scenarios/媒体表现/native500-quality-20261025",
    # Submission PDF/material work belongs to its own window, outside this code delivery.
    "docs/05_开源与参赛/release-preparation/v2/作品介绍",
    # Repeated L03 execution/visual-measurement archives; original inputs stay.
    "scenarios/媒体表现/runtime-l03/variants", "scenarios/媒体表现/runtime-l03/styles",
)
REQUIRED = ("README.md", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "package.json", "package-lock.json")


def json_read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def excluded(relative):
    name = relative.name.lower()
    parts = relative.parts
    device_runtime_binary = parts[:3] == ("resources", "v2-devices", "python") and relative.suffix.lower() in {".exe", ".dll", ".zip"}
    scenario_runtime = len(parts) > 2 and parts[0] == "scenarios" and parts[2].startswith("runtime")
    generated_runtime = scenario_runtime and (
        any(part.endswith("-core-sdk") for part in parts[3:])
        or (parts[1:3] == ("媒体表现", "runtime-l03") and len(parts) > 3
            and re.fullmatch(r"(?:execution-\d+|native-\d{8}-\d{6})", parts[3]))
        or (relative.suffix.lower() == ".json" and (name == "browser.json" or name.startswith("browser-")))
    )
    return (
        generated_runtime or bool(set(relative.parts) & OMIT_DIRS)
        or any(relative.as_posix() == p or relative.as_posix().startswith(p + "/") for p in OMIT_ROOTS)
        or name in {"current_task.md", "isolated_task.md", "isolated_task.json", ".ds_store", "thumbs.db"}
        or name == ".env" or name.startswith(".env.")
        or name.startswith(("credentials.", "secrets.", "token."))
        or (not device_runtime_binary and relative.suffix.lower() in {".zip", ".7z", ".tar", ".gz", ".pyc", ".pyo", ".log", ".tmp", ".pdb", ".exe", ".dll", ".obj", ".rmeta", ".rlib", ".pem", ".key", ".pfx", ".p12"})
    )


def delivery_output(relative):
    parts = relative.parts
    if len(parts) > 2 and parts[0] == "scenarios" and parts[2].startswith("runtime"):
        # Authored entry scripts remain; literal/Cargo-referenced inputs are kept below.
        return not (len(parts) == 4 and (relative.suffix in {".rs", ".py", ".mjs", ".ts", ".tsx", ".ps1"} or relative.name in {"engine-input.json", "frames.json", "frame-key-original-browser.json"}))
    if parts and parts[0] in {"docs", "tools", "tests"}:
        if any(p.lower() in {"output", "outputs", "captures", "screenshots", "actual-inputs"} or p.lower().startswith("webview-") for p in parts):
            return True
        if relative.suffix.lower() in {".pdf", ".mp4", ".webm", ".jsonl"}:
            return True
    return False


def referenced_inputs(files):
    required = set()
    for name, path in files.items():
        if path.suffix not in {".rs", ".ts", ".tsx", ".mjs", ".js"}:
            continue
        if delivery_output(Path(name)) and path.suffix != ".rs":
            continue
        source = path.read_text(encoding="utf-8-sig")
        patterns = [r'#\[path\s*=\s*"([^"]+)"\]', r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"'] if path.suffix == ".rs" else [r'''\bfrom\s*['"]([^'"]+\.json(?:\?raw)?)['"]''', r'''new URL\(['"]([^'"]+\.json)['"],\s*import.meta.url\)''']
        for pattern in patterns:
            for relative in re.findall(pattern, source):
                if not relative.startswith(".") and path.suffix != ".rs":
                    continue
                key = Path(os.path.normpath(str(Path(name).parent / relative.removesuffix("?raw")))).as_posix()
                if key in files:
                    required.add(key)
    return required


def is_link(path):
    info = path.lstat()
    return path.is_symlink() or bool(getattr(info, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT)


def collect(root, supplement=None):
    """A supplement freezes inputs omitted from a prepared copy; it cannot overwrite it."""
    files = {}
    omitted = []
    for base in [root] + ([supplement] if supplement else []):
        candidates = [base / p for p in ROOT_FILES if (base / p).is_file()]
        for directory in ROOT_DIRS:
            entry = base / directory
            if not entry.exists():
                continue
            if is_link(entry):
                omitted.append(directory + "/ (link)")
                continue
            for current, dirs, names in os.walk(entry, followlinks=False):
                parent = Path(current)
                for name in list(dirs):
                    p = parent / name
                    rel = p.relative_to(base)
                    if excluded(rel) or is_link(p):
                        dirs.remove(name)
                        omitted.append(rel.as_posix() + "/")
                candidates.extend(parent / n for n in names)
        for p in candidates:
            rel = p.relative_to(base)
            if excluded(rel) or is_link(p):
                omitted.append(rel.as_posix())
                continue
            key = rel.as_posix()
            if key in files and files[key].read_bytes() != p.read_bytes():
                raise ValueError(f"Supplement conflicts with frozen source: {key}")
            if p.name == ".npmrc" and re.search(r"_auth|token|password|username|https?://[^/\s]+@", p.read_text(encoding="utf-8-sig"), re.I):
                raise ValueError("Credential-bearing .npmrc must not be distributed")
            files[key] = p
    while True:
        required = referenced_inputs(files)
        # Declared Cargo targets remain source even beside old runtime outputs.
        for name, path in files.items():
            if path.name == "Cargo.toml":
                manifest = tomllib.loads(path.read_text(encoding="utf-8-sig"))
                for kind in ("test", "bin", "example", "bench"):
                    for target in manifest.get(kind, []):
                        if "path" in target:
                            required.add(Path(os.path.normpath(str(Path(name).parent / target["path"]))).as_posix())
        removed = [name for name in files if delivery_output(Path(name)) and name not in required]
        if not removed:
            break
        for name in removed:
            omitted.append(name)
            del files[name]
    return dict(sorted(files.items())), sorted(set(omitted))


def closure(files):
    missing = [p for p in REQUIRED if p not in files]
    if missing:
        raise ValueError(f"Required source files missing: {missing}")
    cargo = tomllib.loads(files["Cargo.toml"].read_text(encoding="utf-8-sig"))
    npm = json_read(files["package.json"])
    config_path = "apps/planner/src-tauri/tauri.conf.json"
    config = json_read(files[config_path])
    mappings = []
    for source, destination in config["bundle"]["resources"].items():
        # Resolve relative to the native manifest, including ../../../.
        parts = []
        for part in (Path(config_path).parent / source).as_posix().split("/"):
            if part == "..":
                if not parts:
                    raise ValueError(f"Native resource escapes source root: {source}")
                parts.pop()
            elif part not in {"", "."}:
                parts.append(part)
        key = "/".join(parts)
        selected = [p for p in files if p == key or p.startswith(key + "/")]
        if not selected:
            raise ValueError(f"Native resource absent from delivery: {key}")
        mappings.append({"source": key, "destination": destination, "files": len(selected)})
    for icon in config["bundle"]["icon"]:
        key = (Path(config_path).parent / icon).as_posix()
        if key not in files:
            raise ValueError(f"Native icon absent: {key}")
    declared_cargo_sources = set()
    for member in cargo["workspace"]["members"]:
        if member + "/Cargo.toml" not in files:
            raise ValueError(f"Cargo member absent: {member}")
        manifest = tomllib.loads(files[member + "/Cargo.toml"].read_text(encoding="utf-8-sig"))
        for kind in ("test", "bin", "example", "bench"):
            for target in manifest.get(kind, []):
                if "path" in target:
                    entry = Path(os.path.normpath(str(Path(member) / target["path"]))).as_posix()
                    if entry not in files:
                        raise ValueError(f"Declared Cargo target absent: {entry}")
                    declared_cargo_sources.add(entry)
    for member in npm["workspaces"]:
        if member + "/package.json" not in files:
            raise ValueError(f"npm workspace absent: {member}")
    catalog = json_read(files["modules/catalog.json"])
    for module in catalog["modules"]:
        for key in ("manifest", "readme"):
            if key in module and "modules/" + module[key] not in files:
                raise ValueError(f"Catalog input absent: {module[key]}")
    rust_path_modules = 0
    rust_include_inputs = 0
    for name, path in files.items():
        if path.suffix == ".rs" and (name.startswith(("apps/", "packages/", "integrations/")) or name in declared_cargo_sources):
            for relative in re.findall(r'#\[path\s*=\s*"([^"]+)"\]', path.read_text(encoding="utf-8-sig")):
                entry = Path(os.path.normpath(str(Path(name).parent / relative))).as_posix()
                if entry not in files:
                    raise ValueError(f"Rust path module absent: {name} -> {entry}")
                rust_path_modules += 1
            for relative in re.findall(r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"', path.read_text(encoding="utf-8-sig")):
                entry = Path(os.path.normpath(str(Path(name).parent / relative))).as_posix()
                if entry not in files:
                    raise ValueError(f"Rust include input absent: {name} -> {entry}")
                rust_include_inputs += 1
    json_imports = 0
    for name, path in files.items():
        if name.startswith(("apps/", "packages/")) and path.suffix in {".ts", ".tsx"}:
            for relative in re.findall(r'''\bfrom\s*['"]([^'"]+\.json)['"]''', path.read_text(encoding="utf-8-sig")):
                if relative.startswith("."):
                    entry = Path(os.path.normpath(str(Path(name).parent / relative))).as_posix()
                    if entry not in files:
                        raise ValueError(f"Frontend JSON input absent: {name} -> {entry}")
                    json_imports += 1
    for folder in ("machining-delivery", "purchased-assembly", "parallel-electromechanical", "carton-factory"):
        for name in ("module.json", "template.json", "example-project.json", "definitions.json"):
            key = f"modules/templates/{folder}/candidate/{name}"
            if key not in files:
                raise ValueError(f"Built-in template input absent: {key}")
    benchmark = [p for p in files if p.startswith("scenarios/非功能基准/")]
    if not benchmark:
        raise ValueError("NFR benchmark inputs absent")
    return {
        "cargo_version": cargo["workspace"]["package"]["version"],
        "npm_version": npm["version"], "native_version": config["version"],
        "node_engines": npm["engines"], "cargo_members": cargo["workspace"]["members"],
        "native_resource_mappings": mappings, "benchmark_files": len(benchmark),
        "catalog_modules": len(catalog["modules"]), "frontend_json_imports": json_imports,
        "rust_path_modules": rust_path_modules,
        "rust_literal_include_inputs": rust_include_inputs,
        "root_license_present": "LICENSE" in files,
        "boundary": "Source/resource closure only; not Native runtime or permission to publish.",
    }


def package(root, output, candidate_id, supplement=None, extract_to=None):
    files, omitted = collect(root, supplement)
    summary = closure(files)
    manifest = {
        "document_kind": "source_delivery", "candidate_id": candidate_id,
        "version": summary["npm_version"], "git_commit": None,
        "source_state": "actual_directory_files_including_untracked_sources",
        "closure": summary, "excluded": omitted,
        "exclusion_policy": {"directories": sorted(OMIT_DIRS), "historical_outputs": list(OMIT_ROOTS),
                             "trusted_device_runtime": "Keep the existing resources/v2-devices/python distribution, including interpreter/DLL/stdlib ZIP; do not add external interpreters or caches.",
                             "delivery_outputs": "Historical runtime captures/reports removed; authored entry scripts and literal/Cargo source inputs retained",
                             "scenario_runtime_outputs": ["*-core-sdk trees", "runtime-l03/execution-<digits> and native-YYYYMMDD-HHMMSS trees", "browser.json and browser-*.json measurements"],
                             "credentials": "Excluded by filename; credential-bearing .npmrc rejects packaging."},
        "files": [{"path": p, "bytes": f.stat().st_size} for p, f in files.items()],
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    # Exclusive creation avoids replacing a previously delivered candidate.
    with zipfile.ZipFile(output, "x", zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
        for name, path in files.items():
            archive.write(path, name)
        archive.writestr("SOURCE_DELIVERY.json", json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    output.with_suffix(".manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if extract_to:
        extract_to.mkdir(parents=True, exist_ok=False)
        with zipfile.ZipFile(output) as archive:
            archive.extractall(extract_to)
        staged, _ = collect(extract_to)
        if files.keys() != staged.keys() or any(p.read_bytes() != staged[k].read_bytes() for k, p in files.items()):
            raise ValueError("Extracted source differs from packed input")
        closure(staged)
    return {"candidate_id": candidate_id, "source_files": len(files), "source_bytes": sum(p.stat().st_size for p in files.values()), "archive_bytes": output.stat().st_size, "archive": str(output), "extracted_source": str(extract_to) if extract_to else None, "extracted_bytes_verified": bool(extract_to), "closure": summary}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--supplement-root", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--candidate-id")
    parser.add_argument("--extract-to", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        files, _ = collect(args.root, args.supplement_root)
        result = closure(files)
    else:
        if not args.output or not args.candidate_id:
            parser.error("--output and --candidate-id are required for packaging")
        result = package(args.root, args.output, args.candidate_id, args.supplement_root, args.extract_to)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
