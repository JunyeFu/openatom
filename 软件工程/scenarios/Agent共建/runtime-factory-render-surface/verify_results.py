import json
from pathlib import Path

here = Path(__file__).resolve().parent
root = here.parents[2]
load = lambda path: json.loads(path.read_text(encoding="utf-8-sig"))
browser = load(here / "browser-frozen.json")
actual = load(here / "actual-frozen.json")
labels = load(here / "labels-regression/browser-result.json")
old_canvas = load(root / "scenarios/媒体表现/runtime-item-layout-consumer/old-factory/browser-green.json")
assert len(browser["checks"]) == 11
assert all(row["status"] == "passed" for row in browser["checks"])
for row in browser["checks"]:
    assert row["errors"] == row["expected_errors"]
    assert row["warnings"] == []
assert len(actual["checks"]) == 5
assert all(row["status"] == "passed" for row in actual["checks"])
assert actual["errors"] == actual["warnings"] == []
assert len(labels["checks"]) == 5
assert not any(row.get("status") == "failed" for row in labels["checks"])
assert labels["errors"] == labels["warnings"] == []
assert len(old_canvas["checks"]) == 7
assert all(row["status"] == "passed" for row in old_canvas["checks"])
result = {
    "new_browser_groups": 11,
    "real_frozen_Host_consumer_groups": 5,
    "old_label_regression_groups": 5,
    "own_unique_groups": 21,
    "original_canvas_7_external_owner_verified": True,
    "normal_success_console_errors": 0,
    "warnings": 0,
    "expected_fault_injection_console_errors": 1,
    "actual_Host_cut": actual["producer_cut"],
    "current_source_commands": [
        {"command": "node tests/planner/render-surface/browser.mjs frozen", "exit_code": 0},
        {"command": "node tests/planner/render-surface/actual.mjs frozen", "exit_code": 0},
        {"command": "node scenarios/Agent共建/runtime-factory-render-surface/labels-regression.mjs", "exit_code": 0},
        {"command": "node node_modules/typescript/bin/tsc -p tests/planner/render-surface/tsconfig.json", "exit_code": 0},
        {"command": "npm --prefix apps/planner run typecheck", "exit_code": 0},
        {"command": "node tests/planner/render-surface/build.mjs", "exit_code": 0},
    ],
    "new_listener_started": False,
    "Native_started": False,
    "Cargo_run_in_this_Surface_batch": False,
    "CapturePreview_run": False,
}
(here / "verification.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps(result, ensure_ascii=False, indent=2))
