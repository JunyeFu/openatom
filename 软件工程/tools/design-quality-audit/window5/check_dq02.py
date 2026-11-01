"""DQ-02 finite outcomes and isolated file checks; no product/ReplaceFileW call."""
from copy import deepcopy
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import json
import os
import shutil
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def load_module(name, path):
    spec = spec_from_file_location(name, path)
    module = module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# Reuse the established schemas/fixture; its main() is never called.
io = load_module("dq02_existing_persistence", ROOT / "tools/type-contract-proof/check_persistence.py")
ref = load_module("dq02_save_reference", ROOT / "packages/reporting/save_commit.reference.py")
FILES, MANIFEST = io.package_fixture()
IDENTITY = {key: MANIFEST["data"][key]
            for key in ("project_id", "generation", "project_revision")}


def local(path):
    return path.relative_to(ROOT).as_posix()


def write(path, value):
    io.write(path, value)


def make_fixture(folder):
    for path, value in FILES.items():
        write(folder / path, value)
    write(folder / "project.json", MANIFEST)
    pending = deepcopy(MANIFEST)
    pending["data"].update(project_revision=2, generation=2)
    active = next(e for e in pending["data"]["documents"] if e["role"] == "active_project")
    active.update(revision=2, file_ref={"path": "data/g000002/project-state.json", "kind": "project"})
    project = deepcopy(FILES["data/g000001/project-state.json"])
    project["data"]["revision"] = 2
    write(folder / active["file_ref"]["path"], project)
    write(folder / "project.r2.pending.json", pending)


def file_refs(value):
    if isinstance(value, dict):
        if "path" in value and "kind" in value:
            yield value["path"]
        for item in value.values():
            yield from file_refs(item)
    elif isinstance(value, list):
        for item in value:
            yield from file_refs(item)


def inspect_candidate(folder, candidate, inspection):
    """Read the real candidate at the original base, then reopen an isolated copy."""
    try:
        manifest = io.read(candidate)
        io.SCHEMAS["manifest"].validate(manifest)
        assert {key: manifest["data"][key] for key in IDENTITY} == IDENTITY, "wrong old identity"
        # Equality to the frozen self-built baseline checks identities/objects/references,
        # without inventing a general loader or introducing checksums.
        assert manifest == MANIFEST, "old manifest differs from frozen baseline"
        for entry in manifest["data"]["documents"]:
            path = io.safe_member(entry["file_ref"]["path"])
            source = (folder / path).resolve()
            assert source.is_relative_to(folder.resolve()), "reference escaped original base"
            data = io.read(source)
            io.SCHEMAS[io.ROLE_SCHEMA[entry["role"]]].validate(data)
            for nested in file_refs(data):
                target = (folder / io.safe_member(nested)).resolve()
                assert target.is_relative_to(folder.resolve()), "nested reference escaped original base"
                assert target.is_file(), "missing nested reference: " + nested
            assert data == FILES[path], "old document differs from frozen baseline: " + path
            destination = inspection / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
        shutil.copyfile(candidate, inspection / "project.json")
        root, project, loaded = io.open_staged(inspection)
        assert root == MANIFEST and loaded == FILES
        return {"status": "pass", "candidate": local(candidate), "path_base": local(folder),
                "identity": IDENTITY, "required_documents_opened": len(loaded),
                "nested_file_references_opened": sum(len(list(file_refs(v))) for v in loaded.values()),
                "all_frozen_content_equal": True, "isolated_reopen_root": local(inspection),
                "active_revision": project["revision"], "product_open_executed": False}
    except (OSError, ValueError, AssertionError, io.ValidationError) as error:
        return {"status": "fail", "candidate": local(candidate), "path_base": local(folder),
                "error_type": type(error).__name__, "reason": str(error),
                "product_open_executed": False}


def preserved_files(folder):
    return all(io.read(folder / path) == value for path, value in FILES.items())


def main():
    runs = HERE / "output"
    runs.mkdir(exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="dq02-", dir=runs))
    branches = []
    for number in range(1, 5):
        branch = f"NFR06-F0{number}"
        folder = run / branch
        make_fixture(folder)
        current, retained = folder / "project.json", folder / "project.old-0001.json"
        initial = inspect_candidate(folder, current, run / "open_checks" / (branch + "-before"))
        assert initial["status"] == "pass"
        error_record = None
        options = {}
        if number == 1:
            (folder / "blocked").write_text("file blocks child creation", encoding="utf-8")
            try:
                write(folder / "blocked/new.json", {})
            except OSError as error:
                error_record = {"stage": "write", "error_type": type(error).__name__,
                                "winerror": getattr(error, "winerror", None)}
            else:
                raise AssertionError("write failure was not observed")
            options = {"write_ok": False}
            origin = "actual isolated write failure before any commit"
        elif number == 2:
            options = {"old_root_valid": True}
            origin = "constructed replacement-failure postcondition with untouched old root"
        else:
            # Construct the postcondition; this is NOT a failed ReplaceFileW execution.
            current.rename(retained)
            retained_before = inspect_candidate(folder, retained, run / "open_checks" / (branch + "-retained-before"))
            assert retained_before["status"] == "pass"
            if number == 4:
                current.mkdir()  # Deliberately block only the isolated restoration destination.
            try:
                os.replace(retained, current)
            except OSError as error:
                assert number == 4
                error_record = {"stage": "isolated_restore", "error_type": type(error).__name__,
                                "winerror": getattr(error, "winerror", None)}
                options = {"restore_ok": False}
            else:
                assert number == 3
                options = {"restore_ok": True}
            origin = "constructed displaced-root postcondition; actual isolated os.replace restoration attempt"
        old = inspect_candidate(folder, current, run / "open_checks" / (branch + "-after"))
        backup = (inspect_candidate(folder, retained, run / "open_checks" / (branch + "-retained-after"))
                  if retained.is_file() else None)
        assert old["status"] == ("fail" if number == 4 else "pass")
        assert number != 4 or backup["status"] == "pass"
        task = ref.terminal(2, 3, False, **options)
        assert task["failure_branch"] == branch
        locations = [{"path": p.relative_to(folder).as_posix(), "kind": "directory" if p.is_dir() else "file"}
                     for p in (current, retained, folder / "project.r2.pending.json") if p.exists()]
        judgments = ref.failure_acceptance(branch, task, old_root_complete=old["status"] == "pass",
                    retained_root_complete=None if backup is None else backup["status"] == "pass",
                    old_files_preserved=preserved_files(folder), locations_reported=bool(locations))
        assert judgments["branch_acceptance_status"] == "pass"
        assert judgments["project_health_status"] == ("fail" if number == 4 else "pass")
        branches.append({"branch_id": branch, "fixture_origin": origin, "observed_error": error_record,
                         "terminal_reference_output": task, "original_root_open": old,
                         "retained_root_open": backup, "reported_relative_locations_reference": locations,
                         "judgments": judgments, "actual_product_task_or_message": None})

    rejected = []
    for label in ("missing_required_document", "wrong_old_identity", "missing_nested_reference"):
        folder = run / label
        make_fixture(folder)
        retained = folder / "project.old-0001.json"
        (folder / "project.json").rename(retained)
        if label == "missing_required_document":
            (folder / "data/g000001/scenario.json").unlink()
        elif label == "wrong_old_identity":
            bad = deepcopy(MANIFEST)
            bad["data"]["project_id"] = "OTHER"
            write(retained, bad)
        else:
            bad = deepcopy(FILES["data/g000001/run.json"])
            bad["data"]["input_ref"]["frozen_project_file"]["path"] = "data/g000001/absent.json"
            write(folder / "data/g000001/run.json", bad)
        outcome = inspect_candidate(folder, retained, run / "open_checks" / label)
        assert retained.is_file() and outcome["status"] == "fail"
        rejected.append({"case": label, "backup_exists": True, "open_evidence": outcome})

    task = branches[-1]["terminal_reference_output"]
    common = dict(old_root_complete=False, old_files_preserved=True, locations_reported=True)
    missing = ref.failure_acceptance("NFR06-F04", task, retained_root_complete=False, **common)
    unknown = ref.failure_acceptance("NFR06-F04", task, retained_root_complete=None, **common)
    unexpected = ref.failure_acceptance("NFR06-F04", task, retained_root_complete=True,
                                       expected_branch=False, **common)
    assert missing["fault_handling_status"] == "pass" and missing["branch_acceptance_status"] == "fail"
    assert unknown["branch_acceptance_status"] == "not_evaluable"
    assert unexpected["branch_acceptance_status"] == "fail"
    terminal_cases = [
        ref.terminal(7, 8, True), ref.terminal(8, 8, True),
        ref.terminal(7, 8, False, True), ref.terminal(7, 8, False, False),
        ref.terminal(7, 8, False, None, True), ref.terminal(7, 8, True, None, True, True)]
    assert [t["state"] for t in terminal_cases] == ["completed", "completed", "failed", "failed", "cancelled", "completed"]
    assert terminal_cases[0]["dirty"] and not terminal_cases[1]["dirty"]
    queue = ref.QueueReference()
    queue.admit(7); queue.admit(8); queue.prepared(8)
    assert queue.committed == []
    queue.prepared(7)
    assert queue.committed == [7, 8]
    result = {"document_kind": "DQ02_finite_reference_and_isolated_file_evidence", "date": "2026-10-15",
              "status": "passed_with_stated_scope", "fixture_root": local(run),
              "branches": branches, "backup_exists_but_reopen_rejected": rejected,
              "judgment_negative_cases": {"missing_old_content": missing, "unproven_old_content": unknown,
                                          "unexpected_restore_failure": unexpected},
              "existing_six_terminal_cases": terminal_cases, "queue_commit_order": queue.committed,
              "ReplaceFileW_called": False, "Windows_1176_1177_injected": False,
              "product_Save_Open_or_Host_executed": False, "actual_result_written": False,
              "fixture_scope": "20 established JSON documents, nine module definitions, six nested FileRefs; no binary asset fixture or product editing/rerun",
              "public_required_check_groups_added": 0}
    write(HERE / "output/dq02-result.json", result)
    print(json.dumps({"status": result["status"], "branches": len(branches), "rejected_backups": len(rejected),
                      "result": local(HERE / "output/dq02-result.json"), "fixture_root": local(run)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
