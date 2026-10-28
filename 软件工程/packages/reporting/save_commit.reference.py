"""Finite queue/terminal-state reference only; no filesystem or ReplaceFileW call."""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[2]
LAB = ROOT / "docs/04_开发与验收/特征细化/DRF05_四格式试做"


class QueueReference:
    def __init__(self):
        self.pending = []
        self.ready = set()
        self.committed = []

    def admit(self, revision):
        self.pending.append(revision)

    def prepared(self, revision):
        self.ready.add(revision)
        while self.pending and self.pending[0] in self.ready:
            front = self.pending.pop(0)
            self.ready.remove(front)
            self.committed.append(front)


def terminal(revision, current_revision, replace_ok, restore_ok=None,
             cancel_before_commit=False, commit_wins=False, *,
             write_ok=True, old_root_valid=False):
    if cancel_before_commit and not commit_wins:
        return {"state": "cancelled", "saved_revision": None,
                "dirty": True, "old_root": "retained", "failure": None}
    if replace_ok or commit_wins:
        return {"state": "completed", "saved_revision": revision,
                "dirty": current_revision != revision, "old_root": "superseded", "failure": None}
    if not write_ok:
        branch, old_root = "NFR06-F01", "retained"
    elif old_root_valid:
        branch, old_root = "NFR06-F02", "retained"
    elif restore_ok is True:
        branch, old_root = "NFR06-F03", "restored"
    elif restore_ok is False:
        branch, old_root = "NFR06-F04", "not_claimed_intact"
    else:
        raise ValueError("A replacement failure needs old-root or restoration evidence")
    # This maps supplied outcomes; it does not prove that any file can reopen.
    return {"state": "failed", "saved_revision": None, "dirty": True,
            "old_root": old_root, "failure": "IoFailure", "failure_branch": branch,
            "normal_save_succeeded": False,
            "recoverable_old_file_reported": branch == "NFR06-F04"}


def failure_acceptance(branch, task, *, old_root_complete, retained_root_complete,
                       old_files_preserved, locations_reported, expected_branch=True):
    """Test-record judgments only; booleans come from separate file/open evidence."""
    task_ok = (task["state"] == "failed" and task["failure"] == "IoFailure"
               and task["saved_revision"] is None and task["dirty"] is True
               and task["failure_branch"] == branch)

    def status(*facts):
        if any(fact is False for fact in facts):
            return "fail"
        if any(fact is None for fact in facts):
            return "not_evaluable"
        return "pass"

    retained = retained_root_complete if branch == "NFR06-F04" else old_root_complete
    return {"terminal_status": status(task_ok),
            "fault_handling_status": status(task_ok, old_files_preserved, locations_reported),
            "old_content_retention_status": status(retained),
            "branch_acceptance_status": status(expected_branch, task_ok,
                                                old_files_preserved, locations_reported, retained),
            "project_health_status": status(old_root_complete),
            "retained_content_status": status(retained_root_complete),
            "normal_save_succeeded": False}


def main():
    q = QueueReference()
    q.admit(7); q.admit(8)
    q.prepared(8)
    assert q.committed == []
    after_newer_ready = q.committed.copy()
    q.prepared(7)
    assert q.committed == [7, 8]
    cases = [
        ("save_R7_with_current_R8", (7, 8, True), {"state": "completed", "saved_revision": 7, "dirty": True}),
        ("save_R8_current_R8", (8, 8, True), {"state": "completed", "saved_revision": 8, "dirty": False}),
        ("replacement_failed_restore_succeeded", (7, 8, False, True), {"state": "failed", "old_root": "restored", "dirty": True, "failure": "IoFailure"}),
        ("replacement_failed_restore_failed", (7, 8, False, False), {"state": "failed", "old_root": "not_claimed_intact", "dirty": True, "recoverable_old_file_reported": True}),
        ("cancel_before_commit", (7, 8, False, None, True), {"state": "cancelled", "saved_revision": None, "dirty": True}),
        ("commit_wins_cancel", (7, 8, True, None, True, True), {"state": "completed", "saved_revision": 7, "dirty": True})]
    results = []
    extra_cases = [
        ("write_failed_old_root_retained", {"write_ok": False},
         {"failure_branch": "NFR06-F01", "old_root": "retained"}),
        ("replacement_failed_old_root_still_valid", {"old_root_valid": True},
         {"failure_branch": "NFR06-F02", "old_root": "retained"})]
    for name, inputs, expected in cases:
        actual = terminal(*inputs)
        assert all(actual[k] == v for k, v in expected.items()), name
        results.append({"case": name, "reference_input": list(inputs), "independent_expected": expected,
                        "actual_reference_output": actual, "actual_Windows_output": None})
    for name, options, expected in extra_cases:
        actual = terminal(7, 8, False, **options)
        assert all(actual[k] == v for k, v in expected.items()), name
        results.append({"case": name, "reference_input": options, "independent_expected": expected,
                        "actual_reference_output": actual, "actual_Windows_output": None})
    prior_ref = "tools/type-contract-proof/output/persistence-proof-result.json"
    prior = json.loads((ROOT / prior_ref).read_text(encoding="utf-8"))
    assert len(prior["cases"]) == 12
    evidence = {"document_kind": "finite_save_queue_terminal_reference_not_Windows_fault_test",
                "date": "2026-10-15 Asia/Shanghai", "status": "passed_finite_reference",
                "same_logical_target_queue": {"admission_order": [7, 8], "prepare_order": [8, 7],
                                             "commits_after_R8_ready": after_newer_ready, "actual_reference_commit_order": q.committed},
                "terminal_cases": results, "prior_12_case_evidence": {"path": prior_ref, "case_count": 12,
                    "evidence_kind": prior["evidence_kind"], "rerun_here": False},
                "actual_directory_identity_resolution_tested": False, "ReplaceFileW_called_here": False,
                "Windows_fault_injection_here": False, "new_recovery_file_operations_here": False,
                "actual_Host_or_product_save": None, "actual_cross_machine": None}
    LAB.mkdir(parents=True, exist_ok=True)
    (LAB / "save-reference-result.json").write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("PASS: ordered commit and 8 terminal-state reference cases; no new Windows fault or Host save.")


if __name__ == "__main__":
    main()
