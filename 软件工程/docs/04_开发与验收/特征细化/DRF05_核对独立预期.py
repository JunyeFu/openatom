"""DRF05: read-only static/finite mathematics; no product/exporter/Windows run."""
from decimal import Decimal
from fractions import Fraction
from pathlib import Path
import json
import re

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]


def read(ref):
    return json.loads((ROOT / ref).read_text(encoding="utf-8-sig"))


def union_length(intervals):
    points = sorted({x for interval in intervals for x in interval})
    return sum(b - a for a, b in zip(points, points[1:])
               if any(s <= a and b <= e for s, e in intervals))


def pointer(document, path):
    for token in path.strip("/").split("/"):
        document = document[int(token)] if isinstance(document, list) else document[token]
    return document


def main():
    contract = read("packages/reporting/evaluation-report.contract.design.json")
    complete = read("docs/04_开发与验收/特征细化/DRF05_完整定义.json")
    gaps = read(complete["gap_ref"])
    fixtures = read(complete["finite_cases_ref"])
    handoff = read("scenarios/模板与交接/expected-template-handoff.design.json")
    cases = {c["id"]: c for c in fixtures["cases"]}
    assert len(cases) == 12
    assert len(handoff["cases"]) == 10 and len(handoff["drf05_handoff_cases"]) == 16
    assert len(gaps["handoffs"]) == 8 and len(complete["six_views"]) == 6
    assert complete["coding_package_state"] == "draft_not_dispatchable"
    assert contract["runtime_available"] is False
    for case in fixtures["cases"] + handoff["cases"] + handoff["drf05_handoff_cases"]:
        assert case["actual_result"] is None
    checked_refs = set()
    for ref in complete["source_refs"]:
        assert (ROOT / ref).is_file(), ref
        checked_refs.add(ref)
    for gap in gaps["handoffs"]:
        assert gap["public_source_written_by_this_window"] is False
        for ref in gap["source_files"]:
            assert (ROOT / ref).is_file(), ref
            checked_refs.add(ref)
    scheduler = read("docs/03_架构与契约/调度与状态转换算法.json")
    assert len(gaps["handoffs"][0]["scheduler_exact_records"]) == 2
    for record in gaps["handoffs"][0]["scheduler_exact_records"]:
        assert pointer(scheduler, record["json_pointer"]) == record["rule"]
    rust = (ROOT / "packages/domain/types.design.rs").read_text(encoding="utf-8")
    report_body = re.search(r"pub struct EvaluationReport \{(.*?)^\}", rust, re.S | re.M).group(1)
    current_fields = set(re.findall(r"pub (\w+):", report_body))
    mapped_fields = {r["logical_field"] for r in contract["public_field_mapping"]}
    assert mapped_fields == current_fields, (mapped_fields - current_fields, current_fields - mapped_fields)
    assert contract["format_version_mapping"]["report_id_label"].endswith("data.id")
    assert len(contract["csv_projection"]["tables"]) == 15
    for table in contract["csv_projection"]["tables"]:
        columns = contract["csv_projection"]["prefix_columns"] + table["columns"]
        assert len(columns) == len(set(columns)), table["file"]
    mathematical = []

    def fixture(number):
        case = cases[f"DRF05-E{number:02}"]
        return case["input"], case["independent_expected"]

    inp, exp = fixture(1)
    events = [e for e in inp["events"] if inp["window"][0] <= e["at_ms"] <= inp["window"][1]]
    assert sum(e["quantity"] for e in events) == exp["accepted_quantity"]
    assert [e["sequence"] for e in events] == exp["included_sequences"]
    assert inp["duration_interval"][1] - inp["duration_interval"][0] == exp["duration_ms"]
    mathematical.append("DRF05-E01")

    inp, exp = fixture(2)
    assert len([t for t in inp["accepted_at_ms"] if t <= inp["computed_until_ms"]]) == exp["known_accepted"]
    assert inp["computed_until_ms"] < inp["requested_window"][1]
    assert exp["whole_window_accepted"] is None and exp["whole_window_goal_met"] is None
    mathematical.append("DRF05-E02")

    inp, exp = fixture(3)
    accepted = 0
    completion = None
    for event in sorted(inp["acceptance"], key=lambda e: e["at_ms"]):
        accepted += event["quantity"]
        if completion is None and accepted >= inp["demand"]:
            completion = event["at_ms"]
    assert completion == exp["completion_ms"]
    assert max(0, completion - inp["due_at_ms"]) == exp["tardiness_ms"]
    mathematical.append("DRF05-E03")

    inp, exp = fixture(4)
    claims = inp["claims"]
    held = sum(c["units"] * (c["interval"][1] - c["interval"][0]) for c in claims)
    available = inp["capacity"] * union_length(inp["calendar"])
    open_held = sum(c["units"] * max(0, min(c["interval"][1], e) - max(c["interval"][0], s))
                    for c in claims for s, e in inp["calendar"])
    assert held == exp["occupied_capacity_ms"] and available == exp["available_capacity_ms"]
    assert open_held == exp["open_calendar_occupied_capacity_ms"]
    assert held - open_held == exp["closed_calendar_held_capacity_ms"]
    assert union_length([c["interval"] for c in claims]) == exp["any_busy_ms"]
    assert Fraction(open_held, available) == Fraction(str(exp["utilization"]))
    mathematical.append("DRF05-E04")

    inp, exp = fixture(5)
    assert union_length([w["interval"] for w in inp["waits"]]) == exp["waiting_ms"]
    for reason, duration in exp["by_reason_ms"].items():
        assert union_length([w["interval"] for w in inp["waits"] if reason in w["reasons"]]) == duration
    assert sum(exp["by_reason_ms"].values()) > exp["waiting_ms"]
    mathematical.append("DRF05-E05")

    inp, exp = fixture(6)
    assert union_length(inp["phase_work"]) == exp["effective_work_ms"]
    assert union_length(inp["waiting"]) == exp["waiting_ms"]
    assert inp["computed_until_ms"] - inp["true_job_start_ms"] == exp["elapsed_ms"]
    assert exp["finished_at_ms"] is None and exp["end_known"] is False
    mathematical.append("DRF05-E06")

    inp, exp = fixture(7)
    labor = Decimal(inp["occupied_ms"]) * Decimal(inp["rate_per_hour"]) / Decimal(3600000)
    paid = Decimal(inp["paid_calendar_ms"]) * Decimal(inp["rate_per_hour"]) / Decimal(3600000)
    material = Decimal(inp["material_quantity"]) * Decimal(inp["material_unit_rate"])
    assert labor == Decimal(exp["occupied_cost"]) and paid == Decimal(exp["paid_calendar_cost"])
    assert material == Decimal(exp["material_cost"]) and labor + material == Decimal(exp["known_subtotal"])
    assert inp["transport_rate"] is None and exp["complete_total"] is None and exp["unit_cost"] is None
    mathematical.append("DRF05-E07")

    inp, exp = fixture(8)
    template = read("modules/templates/parallel-electromechanical/project.design.json")
    dev = {d["id"]: d for d in template["devices"]}
    cnc_cycle = sum(p["duration_ms"] for p in dev["CNC01"]["phases"])
    board_cycle = sum(p["duration_ms"] for p in dev["Board01"]["phases"])
    def ends(prep):
        assembly_free = pack_free = 0
        accepted = []
        for n in range(1, 3):
            ready = max(cnc_cycle * n, prep + board_cycle * n + dev["BoardFinish01"]["parameters"]["cycle_seconds"] * 1000)
            assembly_free = max(ready, assembly_free) + dev["Assembly01"]["parameters"]["cycle_seconds"] * 1000
            pack_free = max(assembly_free, pack_free) + (dev["Inspection01"]["parameters"]["cycle_seconds"] + dev["Packing01"]["parameters"]["cycle_seconds"]) * 1000
            accepted.append(pack_free)
        return accepted
    assert ends(inp["before"]) == exp["baseline_accept_ms"]
    assert ends(inp["after"]) == exp["candidate_accept_ms"]
    legacy = {c["id"]: c["manual_expected"] for c in handoff["cases"]}
    assert legacy[inp["baseline_case"]]["accepted_at_ms"] == ends(inp["before"])
    assert legacy[inp["candidate_case"]]["accepted_at_ms"] == ends(inp["after"])
    delta = ends(inp["after"])[-1] - ends(inp["before"])[-1]
    assert delta == exp["completion_delta_ms"]
    assert Fraction(delta * 100, ends(inp["before"])[-1]) == exp["completion_change_percent"]
    mathematical.append("DRF05-E08")

    variants = read("scenarios/两厂房装配包装/variants.design.json")["variants"]
    assert variants[0]["changes"] == [] and len(variants[1]["changes"]) == 1
    assert variants[1]["changes"][0]["operation"] == "add_initial_item"
    assert fixture(9)[1]["same_initial_conditions"] is False
    inp, exp = fixture(10)
    value = Fraction(inp["runs"][0]["accepted_coefficient"], 10 ** inp["runs"][0]["decimal_places"])
    assert value == Fraction(exp["decimal_display"])
    assert len({(r["run_id"], r["order_id"]) for r in inp["runs"]}) == exp["run_rows"]
    assert all(r["input_project_revision"] != inp["export_project_revision"] for r in inp["runs"])
    mathematical.append("DRF05-E10")
    assert fixture(11)[0]["A"]["stage"] != fixture(11)[0]["B"]["stage"]
    assert fixture(11)[1]["delta"] is None and fixture(12)[1]["unit_cost"] is None
    result = {"document_kind": "DRF05_actual_static_and_finite_math_check_not_product_runtime",
              "date": "2026-10-14 Asia/Shanghai", "status": "passed_static_and_finite_math",
              "command": "python docs/04_开发与验收/特征细化/DRF05_核对独立预期.py",
              "public_field_mapping_count": len(mapped_fields), "csv_table_definition_count": 15,
              "source_ref_count": len(checked_refs), "terminal_rule_pointer_count": 2,
              "finite_expectation_count": 12, "independently_recomputed_cases": mathematical,
              "handoff_scenario_count": 16, "original_case_count_preserved": 10,
              "handoff_scenario_evidence": "structure/definition only; no actual transitions tested",
              "actual_exporter_result": None, "actual_windows_save_open_result": None,
              "actual_product_runtime": None, "actual_prototype": None,
              "actual_user_or_cross_machine_walkthrough": None, "actual_publication": None,
              "public_progress_changed": False}
    (BASE / "DRF05_验证结果.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"PASS: {len(mapped_fields)} mappings,15 CSV table definitions,12 cases ({len(mathematical)} recomputed),16 handoff definitions; original10 retained. No product/exporter/Windows run.")


if __name__ == "__main__":
    main()
