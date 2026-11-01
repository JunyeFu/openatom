"""Read the actual delivery inputs; never build, run, authorize or publish a candidate."""
from __future__ import annotations

import argparse
import csv
import importlib.util
import json
from collections import Counter
from datetime import datetime
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[2]
TEMPLATE_NAMES = ("machining-delivery", "purchased-assembly", "parallel-electromechanical", "carton-factory")
_candidate_spec = importlib.util.spec_from_file_location("candidate_inventory", Path(__file__).with_name("candidate_inventory.py"))
candidate_inventory = importlib.util.module_from_spec(_candidate_spec)
_candidate_spec.loader.exec_module(candidate_inventory)


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def license_state(author, license_text):
    if not author or not license_text or any(s in (str(author) + " " + str(license_text)).lower() for s in ("unknown", "待", "unconfirmed")):
        return "unknown_hold"
    return "declared_requires_notices_and_owner_release"


def asset_rows(package, source, scope, root=None):
    definition = package["definition"]
    contribution = definition.get("contribution") or {}
    rows = [{"source": source, "scope": scope, "module_id": definition["id"],
             "version": definition["version"], "kind": "definition", "asset_id": "",
             "relative_path": "", "resolved_asset_path": "", "file_present": None, "author_or_source": contribution.get("author"),
             "license": contribution.get("license"),
             "redistribution_confirmed": None,
             "state": license_state(contribution.get("author"), contribution.get("license")),
             "hold_reason": "missing_or_unconfirmed_definition_author_or_license" if license_state(contribution.get("author"), contribution.get("license")) == "unknown_hold" else ""}]
    rights = {r["asset_ref"]: r for r in contribution.get("assets", [])}
    package_rights = {r["asset_ref"]: r for r in package.get("rights", [])}
    for asset in package.get("assets", []):
        right = rights.get(asset["id"], {})
        state = license_state(right.get("source"), right.get("license"))
        reasons = []
        if state == "unknown_hold":
            reasons.append("missing_or_unconfirmed_asset_source_or_license")
        if not right.get("redistribution_confirmed"):
            state = "unknown_hold"
            reasons.append("redistribution_not_confirmed")
        other = package_rights.get(asset["id"])
        if other is not None and any(right.get(k) != other.get(k) for k in ("source", "license", "redistribution_confirmed")):
            state = "unknown_hold"
            reasons.append("package_rights_conflict_with_contribution_assets")
        base = Path(source).parent
        if scope == "bundled_support":
            base /= definition["payload"]["category"]
        resolved = base / asset.get("path", "")
        present = None
        if root:
            target = (root / resolved).resolve()
            try:
                target.relative_to(root.resolve())
                present = target.is_file()
            except ValueError:
                present = False
                reasons.append("asset_path_outside_input_root")
            if not present:
                state = "unknown_hold"
                reasons.append("asset_file_missing")
        rows.append({"source": source, "scope": scope, "module_id": definition["id"],
                     "version": definition["version"], "kind": "asset", "asset_id": asset["id"],
                     "relative_path": asset.get("path", ""), "resolved_asset_path": resolved.as_posix(),
                     "file_present": present, "author_or_source": right.get("source"),
                     "license": right.get("license"),
                     "redistribution_confirmed": right.get("redistribution_confirmed"), "state": state,
                     "hold_reason": ";".join(reasons)})
    return rows


def inspect_assets(root):
    rows = []
    for name in TEMPLATE_NAMES:
        base = root / "modules/templates" / name / "candidate"
        for path in sorted(base.rglob("module.json")):
            data = read_json(path)
            package = data.get("data", data)
            if "definition" in package:
                rows += asset_rows(package, path.relative_to(root).as_posix(), "bundled_template", root)
    path = root / "modules/devices/template-support/catalog.json"
    for package in read_json(path):
        rows += asset_rows(package, path.relative_to(root).as_posix(), "bundled_support", root)
    # Current config includes this example; source presence is not staged proof.
    path = root / "modules/devices/manual-assembly/presentation/p303-owned/module.json"
    if path.exists():
        data = read_json(path)
        rows += asset_rows(data.get("data", data), path.relative_to(root).as_posix(), "configured_owned_media_requires_staging_check", root)
    rows += [{"source": "tools/architecture-proof/assets/LICENSE.SourceHanSans.txt",
              "scope": "bundled_report_font", "module_id": "", "version": "",
              "kind": "font", "asset_id": "SourceHanSansCN-Regular.otf",
              "relative_path": "tools/architecture-proof/assets/SourceHanSansCN-Regular.otf",
              "resolved_asset_path": "tools/architecture-proof/assets/SourceHanSansCN-Regular.otf",
              "file_present": (root / "tools/architecture-proof/assets/SourceHanSansCN-Regular.otf").is_file(),
              "author_or_source": "Adobe; exact copyright and reserved name in OFL file",
              "license": "OFL-1.1", "redistribution_confirmed": True,
              "state": "declared_requires_notices_and_owner_release"}]
    return rows


def notice_files(directory):
    return sorted(p for p in directory.rglob("*") if p.is_file() and any(
        token in p.name.upper() for token in ("LICENSE", "NOTICE", "COPYING", "COPYRIGHT"))
        and "node_modules" not in p.relative_to(directory).parts)


def dependencies(root, cargo_registry=None, compiled=None):
    rows = []
    registry = cargo_registry or (root / ".inputs/cargo-registry" if (root / ".inputs/cargo-registry").is_dir() else Path.home() / ".cargo/registry/src")
    caches = [registry] + ([p for p in registry.iterdir() if p.is_dir()] if registry.exists() else [])
    compiled = compiled or {}
    def evidence(path):
        try:
            return path.relative_to(root).as_posix()
        except ValueError:
            return str(path)
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf8"))
    for package in lock["package"]:
        if not package.get("source"):
            continue
        manifest = next((p / f"{package['name']}-{package['version']}/Cargo.toml" for p in caches
                         if (p / f"{package['name']}-{package['version']}/Cargo.toml").exists()), None)
        meta = tomllib.loads(manifest.read_text(encoding="utf8"))["package"] if manifest else {}
        rows.append({"ecosystem": "cargo", "name": package["name"], "version": package["version"],
                     "scope": "locked_superset_includes_build_dev_and_other_targets_not_final_binary_SBOM",
                     "authors_declared": json.dumps(meta.get("authors"), ensure_ascii=False),
                     "repository_declared": meta.get("repository"),
                     "license": meta.get("license"), "license_file": meta.get("license-file"),
                     "evidence": evidence(manifest) if manifest else "frozen_cache_manifest_missing",
                     "metadata_present": manifest is not None,
                     "compiler_frontier_present": (package["name"], package["version"]) in compiled,
                     "compiler_roles": ";".join(sorted(compiled.get((package["name"], package["version"]), []))),
                     "final_binary_linkage": "not_proven_by_compiler_frontier",
                     "notice_files": ";".join(str(p) for p in notice_files(manifest.parent)) if manifest else "",
                     "state": "declared_review_expression_and_notices" if meta.get("license") or meta.get("license-file") else "unknown_hold"})
    npm = read_json(root / "package-lock.json")
    for name, package in npm["packages"].items():
        if "node_modules/" not in name or package.get("link"):
            continue
        local = root / name / "package.json"
        meta = read_json(local) if local.exists() else {}
        rows.append({"ecosystem": "npm", "name": meta.get("name", name.rsplit("node_modules/", 1)[-1]),
                     "version": package.get("version"), "scope": "development" if package.get("dev") else "production_or_transitive",
                     "authors_declared": json.dumps(meta.get("author"), ensure_ascii=False),
                     "repository_declared": json.dumps(meta.get("repository"), ensure_ascii=False),
                     "license": meta.get("license", package.get("license")), "license_file": None,
                     "evidence": evidence(local) if local.exists() else "package-lock.json:" + name,
                     "metadata_present": local.exists(), "compiler_frontier_present": None,
                     "compiler_roles": "", "final_binary_linkage": "embedded_frontend_membership_not_proven",
                     "notice_files": ";".join(str(p) for p in notice_files(local.parent)) if local.exists() else "",
                     "state": "declared_review_expression_and_notices" if meta.get("license", package.get("license")) else "unknown_hold"})
    return rows


def write_csv(path, rows):
    with path.open("w", encoding="utf-8-sig", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(dict.fromkeys(k for row in rows for k in row)))
        w.writeheader()
        w.writerows(rows)


def run(root: Path, output: Path, candidate: Path | None = None, cargo_registry: Path | None = None):
    output.mkdir(parents=True, exist_ok=True)
    config = read_json(root / "apps/planner/src-tauri/tauri.conf.json")
    root_package = read_json(root / "package.json")
    app_package = read_json(root / "apps/planner/package.json")
    selection = read_json(root / "docs/00_项目基线/首版任务选择.json")
    trace = read_json(root / "docs/04_开发与验收/首版需求验收追踪.json")
    nfr = read_json(root / "docs/01_业务需求/非功能需求与测量验收.json")
    packet = read_json(root / "docs/04_开发与验收/整包派发/P5-01_整包执行.json")
    tasks = []
    for task in selection["selected"]:
        path = root / "docs/04_开发与验收/整包派发" / (task["id"] + "_整包执行.json")
        actual = read_json(path) if path.exists() else {}
        tasks.append({"id": task["id"], "selected_scope": task["scope"],
                      "source_status": actual.get("status", task["status"]),
                      "whole_task_done": actual.get("whole_task_done", False),
                      "actual_result_present": actual.get("actual_result") is not None,
                      "implementation_dependencies": task["implementation_dependencies"],
                      "release_dependencies": task["release_dependencies"],
                      "source_ref": path.relative_to(root).as_posix() if path.exists() else "docs/00_项目基线/首版任务选择.json"})
    compiled = candidate_inventory.compiler_packages(candidate) if candidate else {}
    assets = candidate_inventory.inspect_candidate_assets(candidate, asset_rows) if candidate else inspect_assets(root)
    deps = dependencies(root, cargo_registry, compiled)
    write_csv(output / "assets-and-rights.csv", assets)
    write_csv(output / "locked-dependencies.csv", deps)
    candidate_result = None
    if candidate:
        candidate_result, files, mappings = candidate_inventory.candidate_files(root, candidate, config)
        write_csv(output / "candidate-files.csv", files)
        write_csv(output / "resource-mappings.csv", mappings)
        file_rights = candidate_inventory.file_rights(files, assets, candidate)
        write_csv(output / "candidate-file-rights.csv", file_rights)
        notices, expressions = candidate_inventory.copy_notices(root, output, deps, compiled)
        font_notice = candidate / "report-resources/LICENSE.SourceHanSans.txt"
        font_destination = output / "third-party-notices/assets/SourceHanSans/LICENSE.SourceHanSans.txt"
        font_destination.parent.mkdir(parents=True, exist_ok=True)
        font_destination.write_bytes(font_notice.read_bytes())
        notices.append(dict(ecosystem="asset", name="SourceHanSansCN-Regular", version="as_staged", declared_license="OFL-1.1",
            source=font_notice.relative_to(root).as_posix(), destination=font_destination.relative_to(output).as_posix(),
            bytes=font_notice.stat().st_size, copied_verbatim=font_notice.read_bytes() == font_destination.read_bytes(), inclusion_scope="staged_report_font"))
        write_csv(output / "third-party-notices.csv", notices)
        write_csv(output / "compiled-dependencies.csv", [d for d in deps if d["ecosystem"] == "cargo" and d["compiler_frontier_present"]])
        write_csv(output / "frontend-dependency-scope.csv", [d for d in deps if d["ecosystem"] == "npm"])
        write_csv(output / "dependency-license-review.csv", [{**r, "findings": ";".join(r["findings"])} for r in expressions])
        candidate_result.update(compiled_registry_packages=len(compiled),
            compiler_scope="Actual compiler artifacts/build scripts, not an exact final linked binary SBOM",
            copied_notice_texts=len(notices), candidate_file_rights_states=dict(Counter(r["state"] for r in file_rights)),
            known_declaration_categories=dict(Counter(r.get("category", "") for r in assets if r["kind"] == "definition")),
            unique_definition_id_versions=len({(r["module_id"], r["version"]) for r in assets if r["kind"] == "definition"}),
            unique_unconfirmed_definition_id_versions=len({(r["module_id"], r["version"]) for r in assets if r["kind"] == "definition" and r["state"] == "unknown_hold"}),
            unique_unknown_asset_files=sorted({r["resolved_asset_path"] for r in assets if r["kind"] == "asset" and r["state"] == "unknown_hold"}),
            dependency_scope_counts={"cargo_locked_registry":sum(d["ecosystem"] == "cargo" for d in deps),
                "cargo_compiled_registry":len(compiled), "npm_locked_external":sum(d["ecosystem"] == "npm" for d in deps),
                "npm_frozen_external_metadata":sum(d["ecosystem"] == "npm" and d["metadata_present"] for d in deps),
                "npm_lock_production_or_transitive":sum(d["ecosystem"] == "npm" and d["scope"] == "production_or_transitive" for d in deps)},
            named_unresolved_asset_declarations=[r for r in assets if r["state"] == "unknown_hold"],
            named_dependency_reviews=[r for r in expressions if r["findings"]],
            file_scope_reviews=[r["file"] for r in file_rights if r["state"] == "file_scope_requires_owner_review"],
            no_root_license_granted=True, no_runtime_or_human_acceptance_granted=True)
        (output / "candidate-closure.json").write_text(json.dumps(candidate_result, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
    gate_rows = []
    for key, kind in [("functional_requirements", "SF"), ("nonfunctional_requirements", "NFR"), ("integration_checks", "IX")]:
        for row in trace[key]:
            gate_rows.append({"id": row["id"], "kind": kind, "title": row.get("title", row["id"]),
                              "producer_tasks": ";".join(row.get("producer_tasks", [])),
                              "source_ref": "docs/04_开发与验收/首版需求验收追踪.json#" + row["id"],
                              "original_rule": json.dumps(row.get("acceptance", row.get("release_gate", row.get("execution_rule"))), ensure_ascii=False),
                              "release_acceptance": "pending_full_original_evidence_not_closed_by_this_inventory"})
    write_csv(output / "full-acceptance-matrix.csv", gate_rows)
    (output / "full-acceptance-matrix.json").write_text(json.dumps({
        "kind": "original_acceptance_source_snapshot_not_new_contract_or_execution",
        "source": "docs/04_开发与验收/首版需求验收追踪.json",
        "functional_requirements": trace["functional_requirements"],
        "cases": trace["cases"], "nonfunctional_requirements": trace["nonfunctional_requirements"],
        "required_check_groups": nfr["test_execution_contract"]["required_check_groups"],
        "integration_checks": trace["integration_checks"], "tasks": tasks,
        "confirmed_choices": packet["source_task_snapshot"]["confirmed_design_requirements"],
        "whole_release_done": False}, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
    write_csv(output / "confirmed-choices.csv", packet["source_task_snapshot"]["confirmed_design_requirements"])
    resource_rows = [{"source": src, "destination": dst, "source_exists": (root / "apps/planner/src-tauri" / src).exists()}
                     for src, dst in config["bundle"]["resources"].items()]
    root_license = any((root / name).exists() for name in ("LICENSE", "LICENSE.txt", "LICENSE.md"))
    result = {"kind": "read_only_release_preparation_inventory_not_acceptance", "snapshot_at": datetime.now().astimezone().isoformat(timespec="seconds"),
              "version": config["version"], "author_confirmed_for_work_introduction": "傅钧烨",
              "candidate_ready": False, "offline_distribution_verified": False, "whole_release_done": False,
              "staged_candidate": candidate_result,
              "root_code_license_exists": root_license, "root_code_license_choice": "MIT candidate only; no license granted by this tool",
              "configuration": {"bundle_active": config["bundle"]["active"], "dev_url": config["build"]["devUrl"],
                                "frontend_dist": config["build"]["frontendDist"], "app_build_command": app_package["scripts"]["build"],
                                "root_build_command": root_package["scripts"]["build"], "node_npm_required": root_package["engines"],
                                "debug_resources": "CARGO_MANIFEST_DIR source modules and report font", "release_resources": "app.path().resource_dir() modules/report-resources; supplied candidate byte checks are in candidate-closure.json; runtime remains unverified", "resource_mappings": resource_rows},
              "tasks": tasks, "task_counts": dict(Counter(t["source_status"] for t in tasks)),
              "whole_task_done_count": sum(t["whole_task_done"] for t in tasks),
              "acceptance_counts": dict(Counter(x["kind"] for x in gate_rows)),
              "nfr_required_groups": sum(len(groups) for groups in nfr["test_execution_contract"]["required_check_groups"].values()),
              "confirmed_question_ids": packet["source_task_snapshot"]["confirmed_question_ids"],
              "question_count_note": "Dispatch prose says21Q; actual original frozen list has20 explicit IDs. All20 preserved; no invented extra question. W00 should reconcile wording.",
              "assets": {"rows": len(assets), "states": dict(Counter(x["state"] for x in assets)),
                         "scope_counts": dict(Counter(x["scope"] for x in assets)), "listing": "assets-and-rights.csv"},
              "dependencies": {"rows": len(deps), "ecosystems": dict(Counter(x["ecosystem"] for x in deps)),
                               "states": dict(Counter(x["state"] for x in deps)), "listing": "locked-dependencies.csv",
                               "scope": "Actual lock/cache declarations, not legal clearance or final target dependency closure; original text/NOTICE and target build must accompany final candidate."},
              "external_actions": {"registration": None, "review_confirmation": None, "submission": None, "receipt": None, "published_release": None},
              "remaining_hard_gates": ["root standard source license and named asset/dependency/file-scope rights reviews", "source-independent offline runtime and distribution configuration; staged candidate is diagnostic, not final release", "full21 release task closure and17SF/11NFR/3IX", "real Codex SF17 achieved-first and insufficient-first branches", "actual target Windows11 x64 new-machine and third-person adoption/reopen/edit", "actual core workflow video from one identified candidate", "owner authorized external registration/submission only when reached"]}
    (output / "release-readiness.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, help="Explicit frozen staged directory; never run its executables")
    parser.add_argument("--cargo-registry", type=Path, help="Frozen crate metadata/license directory (direct or registry-index layout)")
    args = parser.parse_args()
    result = run(args.root.resolve(), args.output.resolve(), args.candidate.resolve() if args.candidate else None,
                 args.cargo_registry.resolve() if args.cargo_registry else None)
    print(json.dumps({k: result[k] for k in ("candidate_ready", "whole_task_done_count", "assets", "dependencies", "acceptance_counts", "nfr_required_groups")}, ensure_ascii=False, indent=2))
