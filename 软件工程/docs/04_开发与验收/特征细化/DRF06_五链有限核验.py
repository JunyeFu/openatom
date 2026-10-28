"""S06-05五链业务侧有限核验；标准库只读源，不运行Engine/Host，不改原预期。"""
import copy
import json
import re
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def read(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8-sig"))


def intersect_length(interval, calendar):
    a, b = interval
    return sum(max(0, min(b, y) - max(a, x)) for x, y in calendar)


def finish_in_calendar(start, amount, calendar):
    remaining, intervals = amount, []
    for a, b in calendar:
        a = max(a, start)
        if a < b and remaining:
            taken = min(remaining, b - a)
            intervals.append([a, a + taken])
            remaining -= taken
    assert remaining == 0
    return intervals


def verify():
    data = read("docs/04_开发与验收/特征细化/DRF06_五链贯通核对.json")
    checks = []
    original_flows = read("docs/02_交互设计/连续任务与原型制作依据.json")["continuous_flows"]
    selection = read("docs/00_项目基线/首版任务选择.json")["selected"]
    assert [f["id"] for f in data["flows"]] == [f["id"] for f in original_flows]
    assert set(data["coverage"]["P_refs"]) == {t["id"] for t in selection}
    assert len(data["coverage"]["SF_refs"]) == 17 and len(data["coverage"]["NFR_refs"]) == 11
    assert len(data["coverage"]["IX"]) == 3 and data["stage_gate_frozen"] is False
    for f in data["flows"]:
        assert f["steps"] and f["phase_wait_claim"] and f["output_report"] and f["cancel_failure_return"]
        assert f["actual_result"] is None and all(not step["called_this_round"] for step in f["steps"])
        original = next(x for x in original_flows if x["id"] == f["id"])
        assert f["title"] == original["title"] and f["original_frames"] == original["frames"]
    checks.append({"id": "original_five_chain_coverage", "status": "passed_design_contract",
                   "flows": 5, "P": 21, "SF": 17, "NFR": 11, "IX": 3,
                   "actual_stage_frozen": False})

    scenarios = {x["id"]: x for x in data["finite_cross_feature_scenarios"]}
    assert len(scenarios) == 3
    x = scenarios["X01"]
    i, e = x["input"], x["expected"]
    eligible_quality = read("packages/domain/business-objects.design.json")["policies"]["stock_eligibility"]["quality_if_inspection_required"]
    eligible_before = len(i["material_ids"]) if i["quality_initial"] in eligible_quality else 0
    assert eligible_before == e["ordinary_issuable_before_release_piece"] == 0
    assert len(i["material_ids"]) * i["quantity_each_piece"] == i["required_input_piece"]
    active = finish_in_calendar(i["declared_quality_release_ms"], i["duration_ms"], i["worker_calendar_ms"])
    assert active == e["work_intervals_ms"]
    assert all(intersect_length(p, i["machine_calendar_ms"]) == p[1] - p[0] for p in active)
    work_end = active[-1][1]
    finish = max(work_end, i["destination_occupied_until_ms"])
    machine_claim = [active[0][0], finish]
    held = machine_claim[1] - machine_claim[0]
    open_held = intersect_length(machine_claim, i["machine_calendar_ms"])
    available = intersect_length(i["window_ms"], i["machine_calendar_ms"])
    worker = sum(b - a for a, b in active)
    assert [active[0][1], active[1][0]] == e["calendar_wait_ms"]
    assert [work_end, finish] == e["output_wait_ms"]
    assert work_end == e["phase_completed_ms"] and finish == e["job_finished_ms"]
    assert machine_claim == e["machine_claim_ms"] and active == e["worker_claims_ms"]
    assert held == e["machine_total_capacity_ms"] == 35000
    assert open_held == e["machine_open_calendar_capacity_ms"] == 25000
    assert held - open_held == e["machine_closed_held_capacity_ms"] == 10000
    assert available == e["machine_available_capacity_ms"] == 30000
    ratio = Fraction(open_held, available)
    assert [ratio.numerator, ratio.denominator] == e["machine_utilization_fraction"] == [5, 6]
    assert worker == e["worker_capacity_ms"] == 8000
    assert max(0, finish - i["due_ms"]) == e["tardiness_ms"] == 0
    cost = Fraction(held * i["prices"]["machine"]["rate"], i["prices"]["machine"]["rate_denominator"])
    cost += Fraction(worker * i["prices"]["worker"]["rate"], i["prices"]["worker"]["rate_denominator"])
    assert cost == e["known_cost_subtotal_CNY"] == 51
    assert e["complete_cost_CNY"] is None and e["unit_cost"] is None
    checks.append({"id": "X01_complete_pending_calendar_output",
                   "status": "passed_independent_finite_arithmetic",
                   "active_intervals_ms": active, "output_wait_ms": [work_end, finish],
                   "held_open_closed_capacity_ms": [held, open_held, held - open_held],
                   "utilization_fraction": [ratio.numerator, ratio.denominator],
                   "known_subtotal_CNY": int(cost), "complete_total_CNY": None})

    p = e["prefix"]
    boundary = i["prefix_pause_at_ms"]
    assert boundary == active[0][1] == 10000
    prefix_machine = boundary - active[0][0]
    prefix_worker = sum(max(0, min(b, boundary) - a) for a, b in active if a < boundary)
    prefix_cost = Fraction(prefix_machine * i["prices"]["machine"]["rate"], i["prices"]["machine"]["rate_denominator"])
    prefix_cost += Fraction(prefix_worker * i["prices"]["worker"]["rate"], i["prices"]["worker"]["rate_denominator"])
    assert prefix_cost == p["known_cost_subtotal_CNY"] == 15
    assert p["machine_claim_ms"] == p["worker_claim_ms"] == [5000, 10000]
    assert p["machine_claim_end"] == "computed_boundary" and p["worker_claim_end"] == "released"
    assert p["effective_progress_ms"] == 5000 and p["accepted_piece"] == 0
    assert p["completion_ms"] is None and p["goal_met"] is None and p["unit_cost"] is None
    assert p["complete_cost_CNY"] is None and p["material_rate_unknown_amount"] is None
    assert p["explicit_zero_material_rate_if_selected_amount"] == 0
    assert p["positive_calendar_wait_interval_yet"] is False
    checks.append({"id": "X01_prefix_known_zero_unknown_cost",
                   "status": "passed_independent_finite_arithmetic",
                   "prefix_ms": boundary, "known_acceptance_piece": 0,
                   "machine_end": "computed_boundary", "worker_end": "released",
                   "known_subtotal_CNY": int(prefix_cost), "unit_cost": None,
                   "unknown_material_rate_is_not_zero": True})

    i, e = scenarios["X02"]["input"], scenarios["X02"]["expected"]
    marker, prepared_jobs = set(), 0
    for _ in range(i["same_configuration_requests"]):
        key = (i["target_device"], i["selected_configuration"])
        if key not in marker:
            marker.add(key)
            prepared_jobs += 1
    quantity = min(len(i["source_released_cargo_ids"]), i["declared_target_piece"], i["carrier_capacity_piece"])
    cargo = i["source_released_cargo_ids"][:quantity]
    arrival = i["departure_start_ms"] + i["travel_ms"]
    receive = arrival + i["unload_ms"]
    returned = receive + i["return_ms"]
    assert prepared_jobs == e["prepared_job_count"] == 1
    assert i["preparation_ms"] == e["prepared_work_ms"] == 2000
    assert e["prepared_claims_repeated"] == e["prepared_creates_items"] == 0
    assert cargo == e["cargo_ids_all_stages"]
    assert [arrival, receive, returned] == [e["arrival_ms"], e["receive_and_accept_ms"], e["return_complete_ms"]]
    assert quantity == e["accepted_piece"] == i["declared_target_piece"] == 3
    assert e["carrier_claim_ms"] == [i["departure_start_ms"], returned]
    assert receive == e["end_point_query_ms"][1] == i["due_ms"]
    assert e["end_point_query_accepts_piece"] == 3
    assert e["compare_runs_calls"] == e["repair_rounds"] == 0 and e["formal_run_count"] == 1
    assert e["report_scenario_comparison"] is None and e["unit_cost"] is None
    assert e["destination_occupied_after_accept_piece"] == 3
    checks.append({"id": "X02_prepare_cargo_accept_endpoint_direct_delivery",
                   "status": "passed_independent_finite_arithmetic",
                   "prepared_jobs": prepared_jobs, "cargo_ids": cargo,
                   "arrival_receive_return_ms": [arrival, receive, returned],
                   "accepted_piece": quantity, "required_comparisons": 0})

    template = read("modules/templates/parallel-electromechanical/project.design.json")
    stores = {s["id"]: s for s in template["stores"]}
    devices = {d["id"]: d for d in template["devices"]}
    manifests = {}
    base = ROOT / "modules/templates/parallel-electromechanical"
    for ref in template["module_refs"]:
        m = json.loads((base / ref["manifest"]).read_text(encoding="utf-8-sig"))
        assert m["id"] == ref["id"] and m["version"] == ref["version"]
        manifests[m["id"]] = m
    assert len(stores) == len(data["C04"]["locations"]) == 12
    assert len(template["connections"]) == len(data["C04"]["connections"]) == 18
    assert template["halls"][0]["height_m"] is None
    assert template["preparation_tasks"][0]["duration_ms"] == 30000
    assert devices["Inspection01"]["parameters"]["max_reworks"] == 0

    def endpoint(text):
        if text in stores:
            return {"kind": "location", "data": text}
        dev, port = text.split(".")
        m = manifests[devices[dev]["module"]]
        assert any(p["id"] == port for p in m["ports"])
        return {"kind": "port", "data": {"module_ref": {"id": m["id"], "version": m["version"]},
                                        "instance_id": dev, "port_id": port}}

    def port_decl(end):
        return next(p for p in manifests[end["data"]["module_ref"]["id"]]["ports"]
                    if p["id"] == end["data"]["port_id"])

    def type_at(end):
        if end["kind"] == "location":
            return stores[end["data"]]["item_type"]
        d, port = devices[end["data"]["instance_id"]], port_decl(end)
        return port["item"] if "item" in port else d["parameters"][port["item_parameter"]]

    for original, row in zip(template["connections"], data["C04"]["connections"]):
        src, dst = endpoint(original["from"]), endpoint(original["to"])
        receiver = dst["data"] if dst["kind"] == "location" else devices[dst["data"]["instance_id"]]["port_bindings"][dst["data"]["port_id"]]
        assert src == row["canonical_source"] and dst == row["canonical_destination"]
        assert receiver == row["destination_store"] and stores[receiver]["capacity"] == row["capacity_piece"]
        assert type_at(src) == type_at(dst) == original["item_type"] == row["item_type"]
        if src["kind"] == "port":
            assert port_decl(src)["direction"] == "out"
        if dst["kind"] == "port":
            assert port_decl(dst)["direction"] == "in"
        assert stores[receiver]["unit"] == row["unit"] == "piece"
        assert row["initial_stock_created"] == 0 and row["actual_result"] is None
        checks.append({"id": f"C04-edge-{row['source_ordinal']:02}",
                       "status": "passed_actual_source_direction_type_receiver_capacity",
                       "from": original["from"], "to": original["to"], "destination_store": receiver})

    i, e = scenarios["X03"]["input"], scenarios["X03"]["expected"]
    def accepts_connection(row, grants, binding_overrides):
        src, dst = row["canonical_source"], row["canonical_destination"]
        recv = row["destination_store"]
        if recv not in stores:
            return False
        if any((ref["kind"], ref["id"]) not in grants for ref in row["authorization_refs"]):
            return False
        for end, direction in [(src, "out"), (dst, "in")]:
            if end["kind"] == "port" and port_decl(end)["direction"] != direction:
                return False
        if dst["kind"] == "location":
            if recv != dst["data"]:
                return False
        else:
            key = (dst["data"]["instance_id"], dst["data"]["port_id"])
            binding = binding_overrides[key] if key in binding_overrides else devices[key[0]]["port_bindings"][key[1]]
            if binding != recv:
                return False
        return type_at(src) == type_at(dst) == stores[recv]["item_type"] == row["item_type"]

    grants = {(ref["kind"], ref["id"]) for row in data["C04"]["connections"] for ref in row["authorization_refs"]}
    incoming = copy.deepcopy(data["C04"]["connections"][0])
    outgoing = copy.deepcopy(data["C04"]["connections"][1])
    bad_direction = copy.deepcopy(outgoing)
    bad_direction["canonical_source"]["data"]["port_id"] = "input"
    bad_store = copy.deepcopy(outgoing)
    bad_store["destination_store"] = "store-pcb"
    actual_rejections = {
        "missing_receiver_binding_admitted": accepts_connection(incoming, grants, {("CNC01", "input"): None}),
        "wrong_direction_admitted": accepts_connection(bad_direction, grants, {}),
        "destination_location_store_mismatch_admitted": accepts_connection(bad_store, grants, {}),
        "out_of_scope_adoption_admitted": accepts_connection(incoming, grants - {("location", "store-blank")}, {}),
    }
    assert all(accepts_connection(row, grants, {}) for row in data["C04"]["connections"])
    assert all(value == e[name] is False for name, value in actual_rejections.items())
    checks.append({"id": "C04_endpoint_semantic_and_authorization_counterexamples",
                   "status": "passed_finite_constraint_model", "rejected": actual_rejections,
                   "actual_Host_admission": None})
    admitted = [outcome == "Pass" and ack and match and i["consumer_job_complete_for_binding"]
                and i["binding_payload_scope_matches"]
                for outcome, ack, match in zip(i["artifact_outcomes"], i["ack_trusted"], i["subject_version_matches"])]
    assert admitted == e["binding_admitted"] == [True, False, False, False]
    assert e["existing_stock_after_insertion"] == i["existing_stock_example"]
    assert i["existing_project_selected_seed"] is False and e["insertion_created_demo_stock"] == 0
    assert e["site_height_pass"] is False and e["isolated_ordinary_issue"] is False
    assert e["wrong_direction_admitted"] is False and e["out_of_scope_adoption_admitted"] is False
    assert e["missing_receiver_binding_admitted"] is False and e["destination_location_store_mismatch_admitted"] is False
    assert e["missing_machine_calendar_admitted"] is False and e["multisource_ambiguous_partial_reservation_admitted"] is False
    isolated = next(x for x in data["C04"]["locations"] if x["id"] == "store-isolate")
    assert isolated["ordinary_issue_enabled"] is False and isolated["capacity_piece"] == 2
    checks.append({"id": "X03_template_seed_site_binding_negative_paths",
                   "status": "passed_finite_contract_counterexamples",
                   "binding_admitted": admitted, "created_seed_stock": 0, "site_pass": False})

    # 当前公共DTO、操作和派生字段只读核对；不是完整业务Run执行。
    types = (ROOT / "packages/domain/types.design.rs").read_text(encoding="utf-8-sig")
    API = (ROOT / "packages/commands/api.design.rs").read_text(encoding="utf-8-sig")
    assert re.search(r"pub enum MaterialEndpoint[\s\S]*?Port\(PortRef\)[\s\S]*?Location\(ObjectId\)", types)
    assert re.search(r"pub struct MaterialConnection \{[^}]*source: MaterialEndpoint[^}]*destination: MaterialEndpoint", types)
    assert re.search(r"pub struct TemplateSubset \{[^}]*locations: Vec<StockLocation>", types)
    assert re.search(r"pub struct HallDesign \{[^}]*height_m: Option<FiniteNumber>", types)
    assert "cost_inputs" in types and "interval_end_causes" in types
    assert "scenario_comparison: Option<RunComparison>" in types
    for op in ["apply_commands", "run_scenario", "get_run", "get_task", "compare_runs",
               "compute_packaging", "adopt_artifact", "save_project", "export_evaluation", "import_package"]:
        assert f"fn {op}(" in API
    ts = ROOT / "tools/type-contract-proof/output/typescript"
    assert "locations" in (ts / "TemplateSubset.ts").read_text(encoding="utf-8-sig")
    assert "location" in (ts / "MaterialEndpoint.ts").read_text(encoding="utf-8-sig")
    assert "null" in (ts / "HallDesign.ts").read_text(encoding="utf-8-sig")
    algorithm = read("docs/03_架构与契约/调度与状态转换算法.json")
    text = "\n".join(s for r in algorithm["rules"] for s in r["algorithm"])
    for required in ["MaterialEndpoint.location", "destination_store", "ordinary", "non_preemptive"]:
        if required != "ordinary":
            assert required in text
    assert "utilization=open_calendar_occupied_capacity_ms/available_capacity_ms" in text
    assert "scenario_comparison=None" in text and "机位" in text
    reporting = read("packages/reporting/evaluation-report.contract.design.json")
    assert "open-calendar held / available" in reporting["metric_definitions"]["resource_utilization"]["formula"]
    assert "null" in reporting["metric_definitions"]["unit_cost"]["unknown"]
    assert all(x["actual_result"] is None for x in read("scenarios/真实业务设计/expected-business.design.json")["cases"])
    assert len(read("scenarios/真实业务设计/business-rule-boundaries.design.json")["cases"]) == 18
    timeline = read("docs/04_开发与验收/特征细化/DRF06_时间轴独立预期.json")
    assert len(timeline["fixtures"]) == 8 and all(x["source_kind"] == "manual_expected" and x["actual_result"] is None for x in timeline["fixtures"])
    checks.append({"id": "current_DTO_API_derived_consumer_and_preserved_expectations",
                   "status": "passed_readonly_static_source", "C04_carriers": "canonical",
                   "B_cases": 18, "manual_expected_timeline": 8, "public_API_called": False})

    return {"document_kind": "DRF06_five_chain_business_static_and_finite_verification",
            "status": "passed_business_design_and_finite_checks", "date": "2026-10-14",
            "check_count": len(checks), "checks": checks, "flows": 5,
            "original_coverage": {"P": 21, "SF": 17, "NFR": 11, "IX": 3},
            "cross_feature_cases": 3, "locations": 12, "connections": 18,
            "original_expectations_rewritten": False, "actual_Engine": None,
            "actual_Host": None, "actual_product_runtime": None, "actual_result": None,
            "stage_gate_frozen": False}


if __name__ == "__main__":
    result = verify()
    (HERE / "DRF06_五链有限核验.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{result['check_count']} five-chain static/finite checks passed; 3 composite cases; C04 12 locations/18 connections. No Engine/Host execution or gate freeze.")
