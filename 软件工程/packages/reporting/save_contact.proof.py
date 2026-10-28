"""Two isolated Windows file contacts; not product Save/Open or recovery."""
from concurrent.futures import ThreadPoolExecutor
from copy import deepcopy
from ctypes import wintypes
from pathlib import Path
from threading import Condition, Event
import ctypes
import json
import platform
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
LAB = ROOT / "docs/04_开发与验收/特征细化/DRF05_四格式试做"
TYPE_PROOF = ROOT / "tools/type-contract-proof"
sys.path.insert(0, str(TYPE_PROOF / "python-libs"))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


kernel = ctypes.WinDLL("kernel32", use_last_error=True)
replace = kernel.ReplaceFileW
replace.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR,
                   wintypes.DWORD, wintypes.LPVOID, wintypes.LPVOID]
replace.restype = wintypes.BOOL
open_file = kernel.CreateFileW
open_file.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                      wintypes.LPVOID, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
open_file.restype = wintypes.HANDLE
close_file = kernel.CloseHandle
close_file.argtypes = [wintypes.HANDLE]
close_file.restype = wintypes.BOOL
VALIDATORS = {name: Draft202012Validator(read(TYPE_PROOF / f"output/schemas/{name}.schema.json"))
              for name in ("project", "manifest")}
PROJECT = read(TYPE_PROOF / "fixtures/project.json")
PROJECT["data"].update(id="TF-SAVE-DRF05", title="self-built isolated save contact")
for name in ("devices", "resources", "calendars", "products", "boms", "routes",
             "module_refs", "scenario_refs", "bindings"):
    PROJECT["data"][name] = []
MANIFEST = read(TYPE_PROOF / "fixtures/manifest.json")
MANIFEST["data"].update(project_id="TF-SAVE-DRF05", title="self-built isolated save contact",
                       module_refs=[], documents=MANIFEST["data"]["documents"][:1])
MANIFEST["data"]["documents"][0]["object_ref"]["id"] = "TF-SAVE-DRF05"


def prepare(folder, revision):
    project = deepcopy(PROJECT)
    project["data"]["revision"] = revision
    project_path = f"data/g{revision:06d}/project-state.json"
    manifest = deepcopy(MANIFEST)
    manifest["data"].update(project_revision=revision, generation=revision)
    manifest["data"]["documents"][0].update(revision=revision,
        file_ref={"path": project_path, "kind": "project"})
    VALIDATORS["project"].validate(project)
    VALIDATORS["manifest"].validate(manifest)
    write(folder / project_path, project)
    pending = folder / f"project.r{revision}.pending.json"
    write(pending, manifest)
    return pending


def reopen(folder):
    manifest = read(folder / "project.json")
    VALIDATORS["manifest"].validate(manifest)
    entry = manifest["data"]["documents"][0]
    project = read(folder / entry["file_ref"]["path"])
    VALIDATORS["project"].validate(project)
    assert project["data"]["id"] == manifest["data"]["project_id"]
    assert project["data"]["revision"] == manifest["data"]["project_revision"] == entry["revision"]
    return manifest["data"]["project_revision"]


def main():
    contacts = LAB / "save-contacts"
    contacts.mkdir(exist_ok=True)
    folder = Path(tempfile.mkdtemp(prefix="windows-", dir=contacts))
    queue_folder = folder / "ordered-commit"
    prepare(queue_folder, 6).rename(queue_folder / "project.json")
    assert reopen(queue_folder) == 6
    ready8 = Event()
    turn = Condition()
    next_revision = 7
    preparation_order, commits = [], []

    def worker(revision):
        nonlocal next_revision
        if revision == 7:
            ready8.wait()
        pending = prepare(queue_folder, revision)
        with turn:
            preparation_order.append(revision)
            if revision == 8:
                ready8.set()
            turn.wait_for(lambda: next_revision == revision)
            old_revision = reopen(queue_folder)
            backup = queue_folder / f"project.r{old_revision}.backup.json"
            if not replace(str(queue_folder / "project.json"), str(pending), str(backup), 0, None, None):
                raise ctypes.WinError(ctypes.get_last_error())
            saved = reopen(queue_folder)
            assert saved == revision
            commits.append({"saved_revision": saved, "current_revision": 8,
                            "still_dirty": saved != 8, "old_backup_revision": read(backup)["data"]["project_revision"]})
            next_revision += 1
            turn.notify_all()

    with ThreadPoolExecutor(max_workers=2) as pool:
        futures = [pool.submit(worker, revision) for revision in (7, 8)]
        for future in futures:
            future.result()
    assert preparation_order == [8, 7]
    assert [row["saved_revision"] for row in commits] == [7, 8]
    assert commits[0]["still_dirty"] and not commits[1]["still_dirty"]
    assert reopen(queue_folder) == 8

    denied_folder = folder / "replacement-denied"
    prepare(denied_folder, 7).rename(denied_folder / "project.json")
    pending = prepare(denied_folder, 8)
    current = denied_folder / "project.json"
    old_root = current.read_bytes()
    # Real open handle allows read/write sharing, deliberately excludes FILE_SHARE_DELETE.
    handle = open_file(str(current), 0x80000000, 0x1 | 0x2, None, 3, 0x80, None)
    if handle == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        success = bool(replace(str(current), str(pending), str(denied_folder / "old.backup.json"), 0, None, None))
        error = ctypes.get_last_error() if not success else 0
        assert not success and error in (5, 32), (success, error)
        assert current.read_bytes() == old_root
        assert reopen(denied_folder) == 7 and pending.exists()
    finally:
        if not close_file(handle):
            raise ctypes.WinError(ctypes.get_last_error())

    result = {"document_kind": "two_real_isolated_Windows_file_contacts_not_product_Save",
              "date": "2026-10-14 Asia/Shanghai", "environment": platform.platform(),
              "evidence_origin": "type_fixture", "case_count": 2, "isolated_root": str(folder),
              "ordered_commit": {"status": "passed", "admission_order": [7, 8],
                                 "actual_preparation_order": preparation_order, "actual_ReplaceFileW_commits": commits,
                                 "final_root_and_project_revision": reopen(queue_folder),
                                 "same_literal_directory": str(queue_folder),
                                 "Host_actual_directory_key_resolution": False},
              "controlled_replace_denial": {"status": "passed", "injection": "CreateFileW read handle without FILE_SHARE_DELETE",
                                            "actual_ReplaceFileW_return": success, "GetLastError": error,
                                            "old_root_bytes_unchanged": True, "old_root_and_project_revision": 7,
                                            "R8_pending_retained": True, "handle_closed": True,
                                            "state_mapping_reference": "Failed/IoFailure; saved revision 7; current revision 8 remains dirty",
                                            "product_task_state_observed": False,
                                            "1176_1177_or_restore_executed": False},
              "existing_6_logic_cases_rerun_or_relabelled": False, "prior_12_IO_cases_rerun_or_changed": False,
              "Engine_Host_Commands_called": False, "product_Save_Open": False,
              "actual_cross_machine": None, "actual_user_walkthrough": None,
              "retained": "All isolated generation, backup and pending files retained for inspection."}
    write(LAB / "save-contact-result.json", result)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
