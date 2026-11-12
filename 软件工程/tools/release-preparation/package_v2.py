"""Copy an explicitly supplied candidate and declared assets into a NEW v2 portable directory.

No build, installer/global setting change, cloud secret, or implicit C57 selection.
"""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import stat
import struct
import zipfile

FORBIDDEN_PARTS = {".git", ".work", ".env", "node_modules", "target", "__pycache__",
                   ".venv", "venv", "webview2", "ebwebview", "credentials", "secrets"}
FORBIDDEN_SUFFIXES = {".sqlite", ".sqlite3", ".db", ".journal", ".pyc", ".pyo", ".pem", ".key", ".pfx"}
RUNTIME_SCRIPTS = {"virtual_device.py", "journal.py", "protocol.py", "devices.demo.json"}


class PackageError(ValueError):
    pass


def require_materialized(source):
    """A tracked LFS pointer is not a distributable runtime resource."""
    with Path(source).open("rb") as stream:
        first_line = stream.readline(200).rstrip(b"\r\n")
        if first_line == b"version https://git-lfs.github.com/spec/v1":
            raise PackageError("Git LFS resource is not materialized: " + str(source))


def candidate_binaries(candidate):
    planned = []
    for source, destination in ((candidate, "superfactory.exe"),
                                (candidate.parent / "superfactory-mcp-bridge.exe", "superfactory-mcp-bridge.exe")):
        if not source.is_file():
            raise PackageError("required candidate executable is missing: " + destination)
        with source.open("rb") as stream:
            if stream.read(2) != b"MZ":
                raise PackageError("candidate is not a Windows executable: " + destination)
        planned.append((source, Path(destination), "explicit_candidate_exe" if destination == "superfactory.exe" else "candidate_sidecar"))
    loader = candidate.parent / "WebView2Loader.dll"
    if loader.is_file():
        planned.append((loader, Path(loader.name), "candidate_sidecar"))
    return planned


def safe_relative(value):
    windows = PureWindowsPath(value)
    path = Path(value)
    if not path.parts or windows.is_absolute() or windows.drive or path.is_absolute() or ".." in windows.parts or ".." in path.parts:
        raise PackageError("resource destination must be a relative path")
    return path


def admissible(relative):
    parts = [part.lower() for part in relative.parts]
    if any(p in FORBIDDEN_PARTS or p.startswith(".env.") for p in parts):
        raise PackageError("private/cache path is not a deliverable")
    name = relative.name.lower()
    if relative.suffix.lower() in FORBIDDEN_SUFFIXES or name.endswith(("-journal", "-wal", "-shm", ".writer.lock")):
        raise PackageError("journal, secret or cache file is not a deliverable")
    if name in {"token.json", "tokens.json", "credentials.json", "config.toml"}:
        raise PackageError("private configuration is not a deliverable")


def collect_tree(source, source_boundary, destination, role):
    source = Path(source)
    if linked(source) or not source.resolve().is_relative_to(source_boundary):
        raise PackageError("resource escapes its declared source root")
    if source.is_file():
        members = [source]
    elif source.is_dir():
        # Do not silently drop an unexpected private file from a declared closure.
        members = []
        for member in source.rglob("*"):
            if linked(member) or not member.resolve().is_relative_to(source_boundary):
                raise PackageError("linked resource is outside the trusted closure")
            if member.is_file():
                members.append(member)
    else:
        raise PackageError("declared resource is missing")
    if not members:
        raise PackageError("declared resource is empty")
    result = []
    for member in sorted(members):
        relative_source = member.relative_to(source_boundary)
        admissible(relative_source)
        admissible(member.resolve().relative_to(source_boundary))
        target = destination / member.relative_to(source) if source.is_dir() else destination
        admissible(target)
        result.append((member, target, role))
    return result


def linked(path):
    # Path.is_junction was added in Python 3.12. Keep the existing no-junction
    # boundary when using the delivery tools' Python 3.11 baseline.
    return path.is_symlink() or getattr(path.lstat(), "st_reparse_tag", 0) == stat.IO_REPARSE_TAG_MOUNT_POINT


def resource_closure(root, base, overlay=None, module_archives=(), project_states=()):
    """Resolve actual startup modules and declared exact dependencies, not imported catalog membership."""
    base = Path(base).resolve(strict=True)
    overlay = Path(overlay).resolve(strict=True) if overlay else None
    roots = [p for p in (overlay, base) if p is not None]
    planned, selected = {}, {}
    catalog, archives = {}, {}
    read = lambda p: json.loads(p.read_text(encoding="utf-8-sig"))
    unwrap = lambda value: value.get("data", value)
    exact = lambda value: (value["id"], value["version"])
    for source_root in reversed(roots):
        for path in sorted((source_root / "modules").rglob("module.json")):
            package = unwrap(read(path))
            catalog[exact(package["definition"])] = path.parent.relative_to(source_root)
    for archive in module_archives:
        archive = Path(archive).resolve(strict=True)
        manifest = unwrap(read(archive / "project.json"))
        for document in manifest["documents"]:
            if document["role"] == "module_definition":
                relative = safe_relative(document["file_ref"]["path"])
                archives[exact(document["module_ref"])] = (archive / relative, archive)

    def source_file(relative):
        relative = safe_relative(str(relative))
        for source_root in roots:
            candidate = source_root / relative
            if candidate.is_file():
                return candidate, source_root
        raise PackageError(f"missing exact resource: {relative.as_posix()}")

    def walk(value):
        if isinstance(value, dict):
            yield value
            for key, child in value.items():
                # Project.module_refs is the imported catalog, not a list of used business objects.
                if key != "module_refs":
                    yield from walk(child)
        elif isinstance(value, list):
            for child in value:
                yield from walk(child)

    def add_file(source, boundary, destination, role, content=None):
        destination = safe_relative(str(destination))
        admissible(destination)
        if destination in planned:
            return
        if not source.is_file():
            raise PackageError(f"missing declared file {source} for {destination}")
        collect_tree(source, boundary, destination, role)  # Retain original private/link/source boundary checks.
        planned[destination] = (content if content is not None else source, destination, role)
        if destination.suffix.lower() == ".glb":
            with source.open("rb") as stream:
                magic, version, length = struct.unpack("<4sII", stream.read(12))
                if magic != b"glTF" or version != 2 or length != source.stat().st_size:
                    raise PackageError(f"invalid declared GLB envelope: {destination}")
                chunk_length, chunk_kind = struct.unpack("<II", stream.read(8))
                if chunk_kind != 0x4E4F534A:
                    raise PackageError(f"missing GLB JSON chunk: {destination}")
                document = json.loads(stream.read(chunk_length))
            for row in document.get("images", []) + document.get("buffers", []):
                uri = row.get("uri")
                if uri and not uri.startswith("data:"):
                    local = safe_relative(uri)
                    add_file(source.parent / local, boundary, destination.parent / local, "glb_external_resource")

    def references(document, source_dir, boundary, destination_dir):
        for value in walk(document):
            if isinstance(value.get("path"), str) and isinstance(value.get("kind"), str):
                relative = safe_relative(value["path"])
                source = source_dir / relative
                destination = destination_dir / relative
                add_file(source, boundary, destination, "declared_file_ref")
            if set(value) == {"id", "version"}:
                reference = exact(value)
                if reference not in catalog:
                    raise PackageError(f"exact module reference is not registered: {reference}")
                module(catalog[reference], reference)

    def module(relative, expected=None):
        relative = safe_relative(str(relative))
        destination = relative / "module.json"
        if destination.as_posix() in selected:
            if expected is not None and exact(selected[destination.as_posix()]) != expected:
                raise PackageError(f"registered directory conflicts with exact module reference: {relative}")
            return
        try:
            source, boundary = source_file(destination)
            document = read(source)
            identity = exact(unwrap(document)["definition"])
        except PackageError:
            if expected not in archives:
                raise
            source, boundary = archives[expected]
            document = read(source)
            identity = exact(unwrap(document)["definition"])
        if expected is not None and identity != expected:
            raise PackageError(f"module directory has wrong exact identity: {relative}")
        origin = "overlay" if overlay and source.is_relative_to(overlay) else "baseline"
        if identity in archives:
            source, boundary = archives[identity]
            document = read(source)
            if exact(unwrap(document)["definition"]) != identity:
                raise PackageError("archive document has wrong exact module identity")
            prefix = source.parent.relative_to(boundary).as_posix() + "/"
            def unbundle(value):
                if isinstance(value, str):
                    return value[len(prefix):] if value.startswith(prefix) else value
                if isinstance(value, list):
                    return [unbundle(v) for v in value]
                if isinstance(value, dict):
                    return {key: unbundle(v) for key, v in value.items()}
                return value
            document = unbundle(document)
            origin = "current_saved_exact_module"
        content = (json.dumps(document, ensure_ascii=False, indent=2) + "\n").encode("utf-8") if origin == "current_saved_exact_module" else None
        add_file(source, boundary, destination, "exact_module:" + origin, content)
        selected[destination.as_posix()] = {"id": identity[0], "version": identity[1], "origin": origin}
        references(document, source.parent, boundary, relative)

    profile_source = next(((source / "modules/startup-library.json", source) for source in roots
                           if (source / "modules/startup-library.json").exists()), None)
    profile = read(profile_source[0]) if profile_source else None
    if profile is not None:
        if profile.get("schema_version") != "1.0" or profile.get("profile") != "paper-packaging-runtime" or not profile.get("register_order"):
            raise PackageError("unsupported or empty explicit startup profile")
        startup = []
    else:
        native = (root / "apps/planner/src-tauri/src/project_api.rs").read_text(encoding="utf-8-sig")
        if "for name in [" not in native.split("pub fn register_bundled_library", 1)[1].split("pub fn new()", 1)[0]:
            native = (root / "packages/commands/src/host_library.rs").read_text(encoding="utf-8-sig")
            native = native.split("fn register_legacy_bundled_library", 1)[1]
        else:
            native = native.split("pub fn register_bundled_library", 1)[1].split("pub fn new()", 1)[0]
        startup = []
        for match in re.finditer(r'for (name|category|relative) in \[(.*?)\] \{(.*?)\n        \}', native, re.S):
            variable, values, body = match.groups()
            values = re.findall(r'"([^"]*)"', values)
            if variable == "relative":
                directories = values
            elif 'join("templates")' in body:
                directories = [f"templates/{name}/candidate" for name in values]
            else:
                joined = re.search(r'join\("([^"]+)"\)', body)
                if not joined:
                    raise PackageError("unsupported Native startup registration")
                directories = [joined[1] + "/" + name for name in values]
            startup.extend("modules/" + directory for directory in directories)
        for families, body in re.findall(r'for suffix in \["-simple", ""\] \{\s*for family in \[(.*?)\] \{(.*?)\n            \}', native, re.S):
            directory = re.search(r'join\("([^"]+)"\)', body)[1]
            startup.extend(f"modules/{directory}/{family}{suffix}" for suffix in ("-simple", "") for family in re.findall(r'"([^"]+)"', families))
        if not startup or 'root.join("media/native-equipment-500")' not in native or 'factory["additional_register_order"]' not in native:
            raise PackageError("Native startup registration contract is unsupported")
    accepted_path = Path("modules/media/native-equipment-500/accepted-registration.json")
    paper_path = Path("modules/templates/paper-packaging-factory/registration.json")
    accepted_source, accepted_root = source_file(accepted_path)
    paper_source, paper_root = source_file(paper_path)
    accepted, paper = read(accepted_source), read(paper_source)
    accepted_entries = [(Path("modules/media/native-equipment-500") / entry["directory"], exact(entry)) for entry in accepted["register_order"]]
    extra_entries = [(Path("modules") / entry["directory"], exact(entry)) for entry in paper["additional_register_order"]]
    # Directory hints come from the actual registry, not from all 847 archived imports.
    for relative, identity in accepted_entries + [(Path("modules") / entry["directory"], exact(entry)) for entry in paper["register_order"]]:
        catalog[identity] = relative
    if profile is not None:
        for entry in profile['register_order']:
            if entry['kind'] not in ('module', 'template'):
                raise PackageError('unsupported explicit startup entry kind')
            relative = Path('modules') / safe_relative(entry['directory'])
            catalog[exact(entry['module_ref'])] = relative
        for entry in profile['register_order']:
            module(catalog[exact(entry['module_ref'])], exact(entry['module_ref']))
        add_file(profile_source[0], profile_source[1], Path('modules/startup-library.json'), 'actual_Native_startup_profile')
    else:
        for relative in startup:
            module(Path(relative))
        for relative, identity in accepted_entries + extra_entries:
            module(relative, identity)
        module(Path("modules/templates/paper-packaging-factory/candidate"), exact(paper["template_ref"]))
    add_file(accepted_source, accepted_root, accepted_path, "native_accepted_registry")
    add_file(paper_source, paper_root, paper_path, "paper_factory_registry")
    for state in project_states:
        document = read(Path(state))
        for value in walk(document):
            if set(value) == {"id", "version"}:
                identity = exact(value)
                if identity not in catalog:
                    raise PackageError(f"project uses an unavailable exact module: {identity}")
                module(catalog[identity], identity)
    for relative in (Path("report-resources/SourceHanSansCN-Regular.otf"), Path("report-resources/LICENSE.SourceHanSans.txt")):
        source, boundary = source_file(relative)
        add_file(source, boundary, relative, "evaluation_font_or_license")
    metadata = {"startup_directories": startup, "accepted_device_count": accepted["device_count"],
                "accepted_media_count": accepted["media_count"], "accepted_entries": len(accepted_entries),
                "additional_startup_entries": len(extra_entries), "modules": selected,
                "catalog_imports_are_not_roots": True, "archived_manifest_relocation_only": True,
                "Native_start_or_save_reopen_verified": False,
                "startup_profile": profile["profile"] if profile is not None else "legacy_default",
                "explicit_startup_entries": len(profile["register_order"]) if profile is not None else None}
    return list(planned.values()), metadata


def assemble(source_root, candidate_exe, device_resources, output, candidate_id, fixture=False, *,
             resource_base=None, resource_overlay=None, module_archives=(), project_states=(), zip_output=None):
    root = Path(source_root).resolve(strict=True)
    candidate = Path(candidate_exe).resolve(strict=True)
    runtime = Path(device_resources).resolve(strict=True)
    output = Path(output).resolve()
    if output.exists():
        raise PackageError("portable output already exists; refusing to overwrite")
    if zip_output is not None:
        archive = Path(zip_output).resolve()
        if archive.exists() or archive.is_relative_to(output):
            raise PackageError("ZIP must be a new path outside the portable directory")
        immutable_inputs = [runtime]
        if resource_base is not None:
            immutable_inputs.extend([root, Path(resource_base), *([Path(resource_overlay)] if resource_overlay else []), *map(Path, module_archives)])
        if any(archive.is_relative_to(source.resolve()) for source in immutable_inputs):
            raise PackageError("ZIP output overlaps immutable resource input")
    if not candidate.is_file() or candidate.suffix.lower() != ".exe":
        raise PackageError("explicit candidate executable is required")
    with candidate.open("rb") as stream:
        magic = stream.read(2)
    if magic != b"MZ":
        raise PackageError("candidate is not a Windows executable")
    if not candidate_id or any(c in candidate_id for c in "\r\n\0"):
        raise PackageError("candidate id is required")
    if output.is_relative_to(runtime) or runtime.is_relative_to(output):
        raise PackageError("portable output overlaps runtime source")
    config = root / "apps/planner/src-tauri/tauri.conf.json"
    mappings = json.loads(config.read_text(encoding="utf-8-sig"))["bundle"]["resources"]
    if not isinstance(mappings, dict):
        raise PackageError("expected explicit Tauri resource mapping")
    planned = candidate_binaries(candidate)
    closure = None
    if resource_base is not None:
        for source in [root, Path(resource_base), *([Path(resource_overlay)] if resource_overlay else []), *map(Path, module_archives)]:
            if output.is_relative_to(source.resolve()) or source.resolve().is_relative_to(output):
                raise PackageError("portable output overlaps immutable resource input")
        resources, closure = resource_closure(root, resource_base, resource_overlay, module_archives, project_states)
        planned.extend(resources)
    for source, destination in (() if closure is not None else mappings.items()):
        # Device runtime is consumed exactly once from the separately fixed installation tree.
        dest = safe_relative(destination)
        if dest.parts and dest.parts[0].lower() == "v2-devices":
            continue
        declared = config.parent / source
        if not declared.resolve().is_relative_to(root):
            raise PackageError("declared resource source escapes project root")
        planned.extend(collect_tree(declared, root, dest, "declared_native_resource"))
    for name in ["runtime.json", "python/python.exe", "python/python314.dll", "python/python314.zip",
                 "python/python314._pth", "licenses/PYTHON-LICENSE.txt"]:
        if not (runtime / name).is_file():
            raise PackageError("embedded device runtime is incomplete")
    pth = (runtime / "python/python314._pth").read_text(encoding="ascii").splitlines()
    if pth != ["python314.zip", ".", "../scripts"]:
        raise PackageError("embedded path configuration is not isolated")
    if {p.name for p in (runtime / "scripts").iterdir()} != RUNTIME_SCRIPTS:
        raise PackageError("unexpected device script or cache in trusted runtime")
    planned.extend(collect_tree(runtime, runtime, Path("v2-devices"), "trusted_embedded_device_runtime"))
    for name in ["NOTICE", "NOTICE.md", "LICENSE", "LICENSE.md"]:
        p = root / name
        if p.is_file():
            planned.append((p, Path(name), "project_notice"))
    guide = root / "docs/05_开源与参赛/release-preparation/v2/当前版本快速开始.md"
    if guide.is_file():
        planned.append((guide, Path("使用说明.md"), "current_release_quickstart"))
    for suffix in ("md", "html"):
        guide = root / f"docs/05_开源与参赛/release-preparation/v2/软件操作指南.{suffix}"
        if guide.is_file():
            planned.append((guide, Path(f"操作指南.{suffix}"), "user_documentation"))
    destinations = set()
    for source, destination, _ in planned:
        if not isinstance(source, bytes):
            require_materialized(source)
        safe_relative(str(destination))
        normalized = str(destination).replace("\\", "/").lower()
        if normalized in destinations:
            raise PackageError("duplicate portable destination")
        destinations.add(normalized)
    # All checks occur before creating the new delivery directory.
    output.mkdir(parents=True, exist_ok=False)
    copied = []
    for source, destination, role in planned:
        target = output / destination
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(source, bytes):
            target.write_bytes(source)
        else:
            shutil.copyfile(source, target)
        copied.append({"path": destination.as_posix(), "bytes": target.stat().st_size, "role": role})
    result = {"candidate_id": candidate_id, "fixture_only": fixture,
              "assembled_at_utc": datetime.now(timezone.utc).isoformat(),
              "candidate_exe_explicitly_supplied": True,
              "candidate_version_asserted_by_caller_not_verified_by_copy": True,
              "actual_v2_gui_https_engine_acceptance_verified": False,
              "tauri_bundle_setting_changed": False, "files": copied,
              "runtime_resource_directory": "v2-devices", "journal_or_credentials_copied": False,
              "note": "Copying is not Native/Engine/cloud acceptance. Keep runtime journal in private appdata."}
    (output / "V2_PORTABLE.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if closure is not None:
        (output / "V2_RESOURCE_CLOSURE.json").write_text(json.dumps(closure, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if zip_output is not None:
        archive = Path(zip_output)
        archive.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=1) as stream:
            for member in sorted(output.rglob("*")):
                if member.is_file():
                    stream.write(member, member.relative_to(output).as_posix())
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--candidate-exe", type=Path, required=True)
    parser.add_argument("--device-resources", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--candidate-id", required=True)
    parser.add_argument("--fixture", action="store_true", help="Mark an independent copy-boundary fixture, never a real v2 product")
    parser.add_argument("--resource-base", type=Path, help="Immutable existing app resources; enable exact startup/FileRef closure")
    parser.add_argument("--resource-overlay", type=Path, help="Explicit current resource overlay")
    parser.add_argument("--module-archive", type=Path, action="append", default=[], help="Read-only saved archive supplying exact current module assets")
    parser.add_argument("--project-state", type=Path, action="append", default=[], help="Read-only actual ProjectState; imported module_refs is not a root set")
    parser.add_argument("--zip-output", type=Path, help="Optional new ZIP path outside the portable output")
    args = parser.parse_args()
    try:
        result = assemble(args.source_root, args.candidate_exe, args.device_resources,
                          args.output, args.candidate_id, args.fixture, resource_base=args.resource_base,
                          resource_overlay=args.resource_overlay, module_archives=args.module_archive,
                          project_states=args.project_state, zip_output=args.zip_output)
    except (PackageError, OSError, ValueError, KeyError) as error:
        parser.exit(2, "portable preparation failed: " + type(error).__name__ + ": " + str(error) + "\n")
    print(json.dumps({"candidate_id": result["candidate_id"], "fixture_only": result["fixture_only"],
                      "files": len(result["files"]), "bytes": sum(f["bytes"] for f in result["files"]),
                      "output": str(args.output)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
