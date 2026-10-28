"""Call existing finite save consumer without main, filesystem writes or bytecode."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
source = ROOT / "packages/reporting/save_commit.reference.py"
ns = {"__file__": str(source), "__name__": "window6_readonly_consumer"}
exec(compile(source.read_text(encoding="utf-8-sig"), str(source), "exec"), ns)
terminal = ns["terminal"]
acceptance = ns["failure_acceptance"]
cases = []


def record(name, actual, expected):
    matched = all(actual.get(k) == v for k, v in expected.items())
    cases.append({"id": name, "actual": actual, "independent_expected": expected, "matched": matched})


inputs = [
    ("NFR06-F01", {"write_ok": False}, "retained"),
    ("NFR06-F02", {"old_root_valid": True}, "retained"),
    ("NFR06-F03", {"restore_ok": True}, "restored"),
    ("NFR06-F04", {"restore_ok": False}, "not_claimed_intact"),
]
for branch, options, old_root in inputs:
    actual = terminal(7, 8, False, **options)
    record(branch, actual, {"state": "failed", "failure": "IoFailure", "dirty": True,
                            "saved_revision": None, "failure_branch": branch,
                            "normal_save_succeeded": False, "old_root": old_root})
    facts = {"old_root_complete": branch != "NFR06-F04",
             "retained_root_complete": True if branch == "NFR06-F04" else None,
             "old_files_preserved": True, "locations_reported": True}
    verdict = acceptance(branch, actual, **facts)
    record(branch + "-acceptance", verdict,
           {"fault_handling_status": "pass", "branch_acceptance_status": "pass",
            "project_health_status": "fail" if branch == "NFR06-F04" else "pass",
            "normal_save_succeeded": False})
f4 = terminal(7, 8, False, False)
for name, retained, old_files, locations, expected in [
    ("F04-retained-missing", False, True, True, "fail"),
    ("F04-retained-unverified", None, True, True, "not_evaluable"),
    ("F04-old-files-lost", True, False, True, "fail"),
    ("F04-no-location", True, True, False, "fail"),
]:
    actual = acceptance("NFR06-F04", f4, old_root_complete=False,
                        retained_root_complete=retained, old_files_preserved=old_files,
                        locations_reported=locations)
    record(name, actual, {"branch_acceptance_status": expected,
                         "project_health_status": "fail", "normal_save_succeeded": False})
q = ns["QueueReference"]()
q.admit(7); q.admit(8); q.prepared(8)
before = list(q.committed)
q.prepared(7)
record("same-target-order", {"before": before, "after": q.committed},
       {"before": [], "after": [7, 8]})
record("saved-R7-current-R8", terminal(7, 8, True),
       {"state": "completed", "saved_revision": 7, "dirty": True})
record("cancel-before-commit", terminal(7, 8, False, cancel_before_commit=True),
       {"state": "cancelled", "saved_revision": None, "dirty": True})
record("commit-wins-cancel", terminal(7, 8, True, cancel_before_commit=True, commit_wins=True),
       {"state": "completed", "saved_revision": 7, "dirty": True})
result = {"document_kind": "window6_independent_four_exit_save_consumer_probe",
          "source_ref": source.relative_to(ROOT).as_posix(),
          "cases": cases, "failed_ids": [c["id"] for c in cases if not c["matched"]],
          "source_main_called": False, "ReplaceFileW_called": False,
          "actual_file_reopen_evidence_produced_here": False,
          "input_evidence_kind": "explicit synthetic evidence flags for existing finite consumer",
          "actual_Host_or_product_save": None,
          "limits": "No real files, Windows fault injection or public output rewritten; actual complete/open evidence belongs to window5 and future Windows tests."}
target = Path(__file__).with_name("save-independent-result.json")
target.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"cases": len(cases), "failed": result["failed_ids"]}))

