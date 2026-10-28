"""Inspect an explicitly supplied staged candidate, without executing it."""
from pathlib import Path, PureWindowsPath
from collections import Counter, defaultdict
import json
import shutil


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def relative_file(value):
    path = PureWindowsPath(value)
    if path.is_absolute() or path.drive or ".." in path.parts:
        raise ValueError(f"Expected a candidate-relative path: {value}")
    return Path(*path.parts)


def compiler_packages(candidate):
    """Compilation records are evidence of compilation, not final linkage."""
    packages = defaultdict(set)
    for record in read(candidate / "build-proof/compiler-frontier.json"):
        identity = record.get("package_id", "")
        if not identity.startswith("registry+"):
            continue
        name, version = identity.rsplit("#", 1)[1].rsplit("@", 1)
        kinds = (record.get("target") or {}).get("kind", [])
        packages[name, version].update(kinds or ["build-script-executed"])
    return packages


def candidate_files(root, candidate, config):
    build = read(candidate / "portable-build.json")
    declared = {relative_file(r["file"]).as_posix(): r["bytes"] for r in build["resource_files"]}
    mapped, mappings = {}, []
    for source, destination in config["bundle"]["resources"].items():
        src = (root / "apps/planner/src-tauri" / source).resolve()
        src.relative_to(root)
        dst = relative_file(destination)
        files = sorted(src.rglob("*")) if src.is_dir() else [src]
        files = [f for f in files if f.is_file()]
        mappings.append(dict(source=source, destination=dst.as_posix(),
                             source_exists=src.exists(), source_files=len(files)))
        for file in files:
            target = dst / file.relative_to(src) if src.is_dir() else dst
            key = target.as_posix()
            if key in mapped:
                raise ValueError(f"Overlapping resource destination: {key}")
            mapped[key] = file
    actual = {p.relative_to(candidate).as_posix(): p for p in candidate.rglob("*") if p.is_file()}
    rows = []
    for key in sorted(actual.keys() | mapped.keys() | declared.keys()):
        file, source = actual.get(key), mapped.get(key)
        kind = ("mapped_resource" if key in mapped or key in declared or key.startswith(("modules/", "report-resources/")) else
                "native_executable" if key in ("superfactory.exe", "superfactory-mcp-bridge.exe") else
                "candidate_instructions_or_build_proof")
        same = file.read_bytes() == source.read_bytes() if file and source else None
        expected = declared.get(key)
        problems = []
        if kind == "mapped_resource":
            if not source:
                problems.append("not_in_frozen_resource_mapping")
            if not file:
                problems.append("staged_file_missing")
            if expected is None:
                problems.append("not_in_portable_manifest")
            elif file and file.stat().st_size != expected:
                problems.append("manifest_size_mismatch")
            if same is False:
                problems.append("source_bytes_differ")
        rows.append(dict(file=key, kind=kind, present=bool(file),
                         source=source.relative_to(root).as_posix() if source else "",
                         declared_bytes=expected, actual_bytes=file.stat().st_size if file else None,
                         source_bytes_equal=same, issues=";".join(problems)))
    missing_exes = [name for name in ("superfactory.exe", "superfactory-mcp-bridge.exe") if name not in actual]
    result = dict(
        input=candidate.relative_to(root).as_posix(), recorded_status=build["status"],
        actual_candidate_files=len(actual), file_kinds=dict(Counter(r["kind"] for r in rows)),
        resource_mappings=len(mappings), portable_declared_resources=len(declared),
        mapped_source_files=len(mapped), matched_resource_bytes=sum(r["source_bytes_equal"] is True for r in rows),
        file_issues=[r for r in rows if r["issues"]], missing_executables=missing_exes,
        portable_entry_bytes_match=actual.get(build["entry"]) is not None and actual[build["entry"]].stat().st_size == build["entry_bytes"],
        native_features=build.get("native_features"),
        diagnostic_devtools_candidate=build.get("devtools_measurement_candidate"),
        frontend=build.get("frontend"), WebView2=build.get("WebView2"),
        actual_runtime=build.get("actual_runtime"), installer_created=build.get("installer_created"),
        human_acceptance=build.get("human_acceptance"), whole_release_done=False,
        boundary="File/byte inventory only. Compiler records do not establish final linkage; staged files do not prove runtime, offline delivery or rights clearance.")
    return result, rows, mappings


def package_documents(value, pointer=""):
    if isinstance(value, dict):
        if isinstance(value.get("definition"), dict) and "id" in value["definition"]:
            yield pointer, value
        elif all(k in value for k in ("id", "version", "payload", "contribution")):
            yield pointer, dict(definition=value, assets=[])
        else:
            for key, child in value.items():
                yield from package_documents(child, pointer + "/" + str(key))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from package_documents(child, pointer + "/" + str(index))


def inspect_candidate_assets(candidate, make_rows):
    rows = []
    for path in sorted((candidate / "modules").rglob("*.json")):
        # Every physically staged JSON is inspected, including fixture packages.
        for pointer, package in package_documents(read(path)):
            source = path.relative_to(candidate).as_posix()
            scope = "bundled_support" if source.endswith("template-support/catalog.json") else "candidate85_staged_package"
            current = make_rows(package, source, scope, candidate)
            for row in current:
                row["declaration_pointer"] = pointer
                row["category"] = package["definition"].get("payload", {}).get("category", "")
            rows.extend(current)
    rows.append(dict(source="report-resources/LICENSE.SourceHanSans.txt", scope="bundled_report_font", module_id="", version="",
                     kind="font", asset_id="SourceHanSansCN-Regular.otf", relative_path="report-resources/SourceHanSansCN-Regular.otf",
                     resolved_asset_path="report-resources/SourceHanSansCN-Regular.otf",
                     file_present=(candidate / "report-resources/SourceHanSansCN-Regular.otf").is_file(),
                     author_or_source="Adobe; copyright/reserved names in staged OFL text", license="OFL-1.1",
                     redistribution_confirmed=True, state="declared_requires_notices_and_owner_release",
                     hold_reason="", declaration_pointer="", category="report_font"))
    return rows


def file_rights(files, assets, candidate):
    declared = defaultdict(list)
    for row in assets:
        if row.get("resolved_asset_path"):
            declared[row["resolved_asset_path"]].append(row)
    result = []
    for file in files:
        key = file["file"]
        declarations = declared[key]
        present = file["present"]
        if declarations:
            states = {d["state"] for d in declarations}
            terms = {(str(d["license"]), str(d["redistribution_confirmed"])) for d in declarations}
            state = "unknown_hold" if "unknown_hold" in states or len(terms) > 1 else "declared_requires_notices_and_owner_release"
            reason = "conflicting_file_declarations" if len(terms) > 1 else ";".join(sorted({d.get("hold_reason", "") for d in declarations} - {""}))
        elif any(token in Path(key).name.upper() for token in ("LICENSE", "COPYING", "NOTICE")):
            state, reason = "preserved_license_text", "Original staged license/notice text; does not grant the project code a license"
        else:
            # A module license is not silently promoted to permission for every
            # generator, test file, screenshot or other file in its directory.
            state, reason = "file_scope_requires_owner_review", "No individual asset declaration; review package/source notice scope and root code license"
        result.append(dict(file=key, kind=file["kind"], state=state, reason=reason,
                           declaration_refs=";".join(d["source"] + "#" + d.get("declaration_pointer", "") for d in declarations),
                           licenses=";".join(sorted({str(d["license"]) for d in declarations})), present=present))
    return result


def copy_notices(root, output, deps, compiled):
    """Copy verbatim frozen texts; do not choose SPDX alternatives or clear rights."""
    rows, reviews = [], []
    for dep in deps:
        key = dep["name"], dep["version"]
        cargo = dep["ecosystem"] == "cargo"
        if cargo and key not in compiled:
            continue
        # All frozen npm metadata enters a conservative notice pool. A dev flag
        # or a library being installed is not proof of embedding/exclusion.
        if not cargo and dep["metadata_present"] is False and dep["scope"] == "development":
            continue
        files = [Path(p) for p in dep["notice_files"].split(";") if p]
        license_file = dep.get("license_file")
        manifest = root / dep["evidence"] if dep["metadata_present"] else None
        missing_declared_file = False
        if license_file and manifest:
            declared = (manifest.parent / license_file).resolve()
            declared.relative_to(root)
            if declared.is_file():
                files.append(declared)
            else:
                missing_declared_file = True
        expressions = str(dep.get("license") or "")
        findings = []
        if not files:
            findings.append("license_text_missing")
        if missing_declared_file:
            findings.append("declared_license_file_missing")
        if not expressions and not license_file:
            findings.append("license_declaration_missing")
        if any(term in expressions for term in (" OR ", " AND ", " WITH ", "/", "MPL-", "Unicode", "LGPL", "GPL-")):
            findings.append("expression_and_distribution_obligations_require_owner_review")
        for source in sorted(set(files)):
            source.relative_to(root)
            destination = output / "third-party-notices" / dep["ecosystem"] / dep["name"].replace("@", "").replace("/", "_") / dep["version"] / source.name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
            text = source.read_text(encoding="utf-8", errors="replace")
            if source.name.lower().endswith(".spdx") and len(text.strip()) < 200:
                findings.append("spdx_identifier_text_not_full_license_terms")
            rows.append(dict(ecosystem=dep["ecosystem"], name=dep["name"], version=dep["version"],
                             declared_license=dep.get("license"), source=source.relative_to(root).as_posix(),
                             destination=destination.relative_to(output).as_posix(), bytes=source.stat().st_size,
                             copied_verbatim=source.read_bytes() == destination.read_bytes(),
                             inclusion_scope="compiled_not_proven_linked" if cargo else "conservative_npm_pool_not_proven_embedded"))
        reviews.append(dict(ecosystem=dep["ecosystem"], name=dep["name"], version=dep["version"], license=dep.get("license"),
                            lock_scope=dep["scope"], available_texts=len(set(files)), findings=sorted(set(findings)),
                            inclusion_scope="compiled_not_proven_linked" if cargo else "conservative_npm_pool_not_proven_embedded"))
    return rows, reviews
