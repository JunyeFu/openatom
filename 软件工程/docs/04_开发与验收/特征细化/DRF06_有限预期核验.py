"""DRF06有限设计核验：标准库算术/原独立预期/引用；不运行产品，不写公共源。"""
import heapq
import json
import re
import runpy
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OUTPUT = HERE / "DRF06_静态与有限计算核验.json"


def read(relative):
    return json.loads((ROOT / relative).read_text(encoding="utf-8-sig"))


def union_length(intervals):
    total, left, right = 0, None, None
    for start, end in sorted(intervals):
        if left is None:
            left, right = start, end
        elif start <= right:
            right = max(right, end)
        else:
            total += right - left
            left, right = start, end
    return total + (right - left if left is not None else 0)


def first_fit(at, duration, calendar):
    return next((max(at, a) for a, b in calendar
                 if max(at, a) < b and max(at, a) + duration <= b), None)



def verify_current_delta(timeline):
    """现行字段的有限正反例；只读原生/标准Schema/TS证明，不调用产品。"""
    prefix = "docs/04_开发与验收/特征细化/"
    delta = read(prefix + "DRF06_公共字段合入delta.json")
    schema = read("tools/type-contract-proof/output/schemas/run.schema.json")
    claim_causes = schema["$defs"]["ClaimIntervalEndCause"]["enum"]
    wait_causes = schema["$defs"]["WaitEndCause"]["enum"]
    assert claim_causes == ["released", "observation_end", "computed_boundary"]
    assert wait_causes == ["condition_changed", "resolved", "observation_end", "computed_boundary"]
    assert "interval_end_causes" in schema["$defs"]["ResourceClaim"]["required"]
    rows = {c["id"]: c for c in delta["finite_cases"]}
    result = []

    def check(name, actual):
        assert actual == rows[name]["expected"], (name, actual, rows[name]["expected"])
        result.append({"id": name, "status": "passed_current_contract_finite_case", "calculated": actual})

    def valid_claim(claim):
        return ((claim["kind"] != "phase" or claim["phase_ref"] is not None)
                and len(claim["intervals"]) == len(claim["interval_end_causes"])
                and all(p["start_ms"] < p["end_ms"] for p in claim["intervals"])
                and all(c in claim_causes for c in claim["interval_end_causes"]))

    i = rows["D06-C01"]["input"]
    base = i["valid"]
    check("D06-C01", dict(valid=valid_claim(base),
                         phase_without_phase_ref=valid_claim({**base, "phase_ref": i["invalid_phase_ref"]}),
                         cause_count_mismatch=valid_claim({**base, "interval_end_causes": i["invalid_end_cause_count"]}),
                         unknown_cause_alias=valid_claim({**base, "interval_end_causes": [i["invalid_end_cause_alias"]]})))

    i = rows["D06-C02"]["input"]
    wait, claim = [], []
    for ctx in i["contexts"]:
        sealed = ctx["whole_window_computed"] and ctx["computed_until_ms"] == ctx["window_end_ms"]
        wait.append(("observation_end" if sealed else "computed_boundary")
                    if ctx["still_blocked_or_held"] else "resolved")
        claim.append(("observation_end" if sealed else "computed_boundary")
                     if ctx["still_blocked_or_held"] else "released")
    check("D06-C02", dict(wait_end_causes=wait, claim_end_causes=claim,
                         cannot_infer_from_end_ms=len(set(wait)) > 1))

    i = rows["D06-C03"]["input"]
    def amount(q):
        assert q["value"]["decimal_places"] == 0
        return q["value"]["coefficient"]

    def valid_initial(allocation):
        if allocation["mode"] not in ["soft_plan", "hard_reserved"]:
            return False
        if allocation["demand_ref"] not in i["task_ids"]:
            return False
        if allocation["quantity"]["unit"] != i["source_quantity"]["unit"]:
            return False
        if amount(allocation["quantity"]) > amount(i["source_quantity"]):
            return False
        if len(allocation["item_refs"]) > 1:
            return amount(allocation["quantity"]) == sum(allocation["source_quantities"])
        return allocation["item_refs"] == [i["source_id"]]

    valid = i["valid_allocation"]
    rejected = [valid_initial({**valid, **x}) for x in i["invalids"]]
    reserved, on_hand = amount(valid["quantity"]), amount(i["source_quantity"])
    check("D06-C03", dict(valid=valid_initial(valid), reserved_quantity=reserved,
                         free_ordinary_quantity=on_hand - reserved if i["source_quality"] == "released" else 0,
                         unreserved_on_hand_quantity=on_hand - reserved, quality_not_bypassed=True,
                         invalid_acceptance=rejected))

    i = rows["D06-C04"]["input"]
    children = [i["reserve_quantity"], i["source_quantity"] - i["reserve_quantity"]]
    check("D06-C04", dict(children_quantities=children, split_from_refs=[i["source_id"]] * len(children),
                         source_lifecycle="transformed", source_counts_as_stock=False,
                         on_hand=sum(children), quantity_acquired=i["reserve_quantity"] if i["resource_available"] else 0,
                         invalid_child_sum_11=11 == i["source_quantity"]))

    def paired(intervals, causes):
        assert len(intervals) == len(causes)
        return {"intervals": [{"start_ms": a, "end_ms": b} for a, b in intervals],
                "interval_end_causes": causes}

    i = rows["D06-C05"]["input"]
    remaining, active = i["effective_duration_ms"], []
    for a, b in i["work_calendar_ms"]:
        a = max(a, i["job_start_ms"])
        if a < b and remaining:
            work = min(remaining, b - a)
            active.append([a, a + work])
            remaining -= work
    assert remaining == 0
    end = active[-1][1]
    check("D06-C05", dict(prefix_machine=paired([[i["job_start_ms"], i["prefix_ms"]]], ["computed_boundary"]),
                         prefix_phase_person=paired([active[0]], ["released"]),
                         final_machine=paired([[i["job_start_ms"], end]], ["released"]),
                         final_phase_person=paired(active, ["released"] * len(active)),
                         final_wait={"interval_ms": [active[0][1], active[1][0]], "end_cause": "resolved"},
                         machine_union_ms=end - i["job_start_ms"], person_union_ms=union_length(active)))

    i = rows["D06-C06"]["input"]
    work_end = i["work_start_ms"] + i["work_duration_ms"]
    finished = max(work_end, i["receiver_available_ms"])
    check("D06-C06", dict(phase_completed_at_ms=work_end, job_finished_at_ms=finished,
                         waiting_output_finished_at_ms=None, device_claim_end_cause="released",
                         person_claim_end_cause="released", invalid_finished_at_phase_end=work_end == finished))

    i = rows["D06-C07"]["input"]
    target = i["role_targets"][i["task_target_role"]]
    readiness, job_count = set(), 0
    key = (target, "declared-same-configuration")
    for _ in range(i["repeated_same_configuration_requests"]):
        if key not in readiness:
            job_count += 1
            readiness.add(key)
    check("D06-C07", dict(work_kind="preparation", prepared_at_ms=i["duration_ms"],
                         created_job_count=job_count, created_items=0, initial_items_after=i["initial_items"],
                         readiness_same_target_and_configuration=key in readiness,
                         readiness_other_target=("D-other", key[1]) in readiness,
                         readiness_changed_configuration=(target, "changed") in readiness,
                         missing_explicit_target_mapping=False))

    i = rows["D06-C08"]["input"]
    quantity = min(len(i["initial_source_items"]), i["target_quantity"], i["carrier_capacity"])
    assert i["allow_partial_load"] and i["supply_embedded_in"] == "ScenarioInput.supplies"
    receive = i["depart_ms"] + i["travel_ms"] + i["unload_ms"]
    check("D06-C08", dict(cargo_items=i["initial_source_items"][:quantity], shipped_quantity=quantity,
                         created_items=0, received_ms=receive, returned_ms=receive + i["return_ms"],
                         commit_via="PutObject Scenario", new_EditableProjectObject_Supply_variant=False))

    # 消费者保留原数值别名，协议投影按现行两个数组逐项配对。
    t1 = timeline["fixtures"][0]["expected"]
    for claim in t1["claims"]:
        assert claim["intervals_ms"] == [[p["start_ms"], p["end_ms"]] for p in claim["protocol_intervals"]]
        assert len(claim["protocol_intervals"]) == len(claim["protocol_interval_end_causes"])
        assert all(c in claim_causes for c in claim["protocol_interval_end_causes"])
    def inspect(value):
        if isinstance(value, dict):
            if isinstance(value.get("intervals"), list) and "interval_end_causes" in value:
                assert len(value["intervals"]) == len(value["interval_end_causes"])
                assert all(c in claim_causes for c in value["interval_end_causes"])
                assert all(set(p) == {"start_ms", "end_ms"} for p in value["intervals"])
            if "actual_result" in value:
                assert value["actual_result"] is None
            for child in value.values():
                inspect(child)
        elif isinstance(value, list):
            for child in value:
                inspect(child)
    inspect(delta)
    inspect(timeline)
    contract = read("docs/03_架构与契约/全层类型与工程格式.json")["timeline_runtime_contract"]
    assert "ResourceClaim.interval_end_causes" in contract["fields"]
    claim_TS = (ROOT / "tools/type-contract-proof/output/typescript/ResourceClaim.ts").read_text(encoding="utf-8-sig")
    wait_TS = (ROOT / "tools/type-contract-proof/output/typescript/WaitInterval.ts").read_text(encoding="utf-8-sig")
    assert "interval_end_causes" in claim_TS and "ClaimIntervalEndCause" in claim_TS and "phase_ref" in claim_TS
    assert "end_cause" in wait_TS and "WaitEndCause" in wait_TS
    assert any("源转为transformed" in s and "不再计现有" in s for s in contract["endpoint_rules"])
    native = read("tools/type-contract-proof/output/timeline-contract-proof-result.json")
    assert len(native["cases"]) == 20 and native["Engine_or_Host_executed"] is False
    assert native["actual_product_runtime"] is None and native["status"].startswith("passed")
    algorithm = read("docs/03_架构与契约/调度与状态转换算法.json")
    lowering = read("docs/04_开发与验收/特征细化/DRF03_C01_逐类原子采用.json")
    alg14 = next(r for r in algorithm["rules"] if r["id"] == "ALG-14")
    assert lowering["task_lowering"] and lowering["transport_lowering"]
    assert any("PhaseWorkKind::Preparation" in s for s in alg14["algorithm"])
    assert any("ScenarioInput.supplies" in s for s in alg14["algorithm"])
    assert any("不重复取得" in s for s in alg14["algorithm"])
    result.append({"id": "current_canonical_DTO_and_C01_consumer_adoption",
                   "status": "passed_readonly_schema_source_and_consumer_mapping",
                   "native_owner_finite_cases_read": 20, "Claim_shape": "parallel_arrays",
                   "complete_source_transformed_children_conserved": True,
                   "C01_task_transport_lowerings_aligned": True, "Engine_or_Host_executed": False})
    return result


def verify():
    results = []
    boundary = read("scenarios/真实业务设计/business-rule-boundaries.design.json")
    rows = {row["id"]: row for row in boundary["cases"]}
    assert len(rows) == len(boundary["cases"]) == 18
    assert list(rows) == [f"B{x:02}" for x in range(1, 19)]
    legacy = runpy.run_path(str(ROOT / "scenarios/真实业务设计/复核业务规则.py"))["verify"]()
    assert legacy["actual_runtime"] is None
    results.extend(legacy["cases"])

    def check(case_id, calculated):
        assert calculated == rows[case_id]["expected"], (case_id, calculated, rows[case_id]["expected"])
        results.append({"id": case_id, "status": "passed_independent_finite_calculation",
                        "calculated": calculated})

    i = rows["B07"]["input"]
    reserved = sum(x["quantity"] for x in i["split"] if x["reserved_for"])
    free = sum(x["quantity"] for x in i["split"] if x["reserved_for"] is None)
    assert reserved + free == i["quantity"]
    check("B07", dict(on_hand=reserved + free, free_quantity=free, reserved_quantity=reserved,
                     acquired_quantity=i["required_quantity"] if i["resource_available"] else 0,
                     source_record_counted_as_stock=False, split_children_preserved=True,
                     parent_ref_for_both=i["parent_id"]))

    i = rows["B08"]["input"]
    product_ids = [x["id"] for x in i["contents"] if x["unit"] == i["order"]["unit"]]
    assert len(set(product_ids)) == len(product_ids)
    accepted_ids, commits = set(), 0
    for _ in range(i["repeat_acceptance_attempts"]):
        unused = [x for x in i["contents"] if x["id"] not in accepted_ids]
        if sum(x["quantity"] for x in unused) >= i["order"]["quantity"]:
            accepted_ids.update(x["id"] for x in unused)
            commits += 1
    check("B08", dict(accepted_product_quantity=sum(x["quantity"] for x in i["contents"]
                                                  if x["id"] in accepted_ids),
                     box_count=i["container_quantity"], product_ids=product_ids,
                     acceptance_commit_count=commits, free_loose_product_quantity=0,
                     container_final_holder=i["container_holder_sequence"][-1],
                     contents_final_holder=i["container_id"]))

    i = rows["B09"]["input"]
    worker_open = first_fit(i["ready_at_ms"], 0, i["worker_calendar_ms"])
    boundaries = sorted(set([i["ready_at_ms"], i["pcb_arrival_ms"], worker_open]))
    segments = []
    for a, b in zip(boundaries, boundaries[1:]):
        reasons = (["missing_material"] if a < i["pcb_arrival_ms"] else [])
        reasons += ["calendar"] if a < worker_open else []
        if reasons:
            segments.append(dict(interval_ms=[a, b], reason_kinds=reasons,
                                 end_cause="resolved" if b == max(worker_open, i["pcb_arrival_ms"])
                                 else "condition_changed"))
    check("B09", dict(segments=segments, wait_union_ms=union_length([s["interval_ms"] for s in segments]),
                     ready_at_ms=i["ready_at_ms"], started_at_ms=max(worker_open, i["pcb_arrival_ms"]),
                     resource_claim_before_start=False))

    i = rows["B10"]["input"]
    remaining, progress, active, progress_rows = i["duration_ms"], 0, [], []
    for a, b in i["worker_calendar_ms"]:
        a = max(a, i["start_ms"])
        if a < b and remaining:
            used = min(b - a, remaining)
            active.append([a, a + used])
            progress_rows.append([progress, progress + used])
            progress += used
            remaining -= used
    assert remaining == 0
    check("B10", dict(active_intervals_ms=active, progress_boundaries_ms=progress_rows,
                     end_causes=["interrupted"] * (len(active) - 1) + ["phase_completed"],
                     job_claim_ms=[i["start_ms"], active[-1][1]], phase_claims_ms=active,
                     wait_ms=[active[0][1], active[1][0]], completion_ms=active[-1][1],
                     effective_work_ms=sum(b - a for a, b in active)))

    i = rows["B11"]["input"]
    check("B11", dict(resource_required_start_ms=[first_fit(i["at_ms"], 0, c) for c in i["calendars_ms"]],
                     resource_free_logic_start_ms=i["at_ms"],
                     zero_stage_rendering="point_PhaseCompleted_no_positive_width_work_band"))

    i = rows["B12"]["input"]
    end = i["start_ms"] + i["duration_ms"]
    check("B12", [dict(interval_ms=[i["start_ms"], min(t, end)],
                       effective_progress_ms=min(t, end) - i["start_ms"],
                       end_cause="phase_completed" if t >= end else "computed_boundary",
                       phase_completed=t >= end, unknown_window_ms=[t, end] if t < end else None)
                  for t in i["computed_until_sequence_ms"]])

    i = rows["B13"]["input"]
    arrival = i["departure_ms"] + i["travel_ms"]
    unload_start = max(arrival, i["receiver_ready_ms"])
    receive = unload_start + i["unload_ms"]
    returned = receive + i["return_ms"]
    quantity = min(i["source_stock"], i["carrier_capacity"], i["demand"])
    daily = [quantity if day * i["day_length_ms"] <= receive < (day + 1) * i["day_length_ms"]
             else 0 for day in range(2)]
    check("B13", dict(arrival_ms=arrival, receive_ms=receive, accept_ms=receive,
                     carrier_available_ms=returned, receiver_wait_ms=[arrival, unload_start],
                     carrier_claim_ms=[i["departure_ms"], returned], accepted_by_day=daily,
                     outstanding_at_first_day_end=i["demand"] - daily[0],
                     outstanding_final=i["demand"] - sum(daily),
                     tardiness_ms=max(0, receive - i["due_ms"]), auto_created_stock_at_midnight=0))

    i = rows["B14"]["input"]
    arrival = i["depart_ms"] + i["travel_ms"]
    unload_start = first_fit(arrival, i["unload_ms"], i["unloader_calendar_ms"])
    receive, returned = unload_start + i["unload_ms"], unload_start + i["unload_ms"] + i["return_ms"]
    check("B14", dict(arrived_ms=arrival, unload_interval_ms=[unload_start, receive],
                     received_ms=receive, returned_ms=returned, wait_interval_ms=[arrival, unload_start],
                     wait_reason="calendar", cargo_holder_before_receive="transport",
                     carrier_claim_ms=[i["depart_ms"], returned], unloader_claim_ms=[unload_start, receive]))

    i = rows["B15"]["input"]
    applied, why, released_at = [], [], None
    for decision in i["decisions"]:
        reason = ("not_in_pending_state" if decision["at_ms"] < i["pending_at_ms"] else
                  "inspection_rule_mismatch" if decision["rule"] != i["inspection_rule"] else None)
        applied.append(reason is None)
        why.append(reason)
        if reason is None:
            released_at = decision["at_ms"]
    check("B15", dict(applied=applied, not_applied_reasons=why,
                     release_at_ms=released_at, quality_until_release="pending"))

    i = rows["B16"]["input"]
    received = min(i["demand"], i["destination_capacity"])
    accepted = received if i["allow_partial"] else (i["demand"] if received >= i["demand"] else 0)
    assert not i["terminal_outflow_declared"]
    check("B16", dict(physically_received=received, accepted=accepted, outstanding=i["demand"] - accepted,
                     completion_ms=None, destination_occupied=received, second_cargo_holder="transport",
                     second_wait_ms=[i["second_arrive_ms"], i["observation_end_ms"]],
                     wait_end_cause="observation_end", calculation_can_complete=True, goal_met=False))

    # 只验证这一个有限同刻闭包；不是产品Engine或通用调度器。
    i = rows["B17"]["input"]
    queue = [(e["at_ms"], e["phase"], e["id"]) for e in i["events"]]
    heapq.heapify(queue)
    at, free, stock, output, accepted, processed = queue[0][0], False, 0, 0, 0, []
    while queue and queue[0][0] == at:
        _, _, kind = heapq.heappop(queue)
        processed.append(kind)
        if kind == "release":
            free = True
        elif kind == "arrival":
            stock += i["needed_items"]
        elif kind == "zero_done":
            output = 1
        if free and stock >= i["needed_items"]:
            processed.append("start")
            free, stock = False, stock - i["needed_items"]
            heapq.heappush(queue, (at + i["work_duration_ms"], 0, "zero_done"))
        if output and i["receiver_capacity"]:
            processed.append("transfer")
            accepted += output
            output, free = 0, True
    check("B17", dict(step_at_ms=at, processed=processed, accepted=accepted, resource_free=free,
                     pending_output=output, next_event_at_ms=queue[0][0], acceptance_point_ms=at))

    i = rows["B18"]["input"]
    occupied, workers = [], []
    for claim in i["claims"]:
        job = next(j for j in i["jobs"] if j["id"] == claim["job_id"])
        if claim["resource_id"] == job["capacity_ref"]["id"]:
            occupied.append(job["device_id"])
        else:
            workers.append(claim["resource_id"])
    check("B18", dict(job_device_rows={j["id"]: j["device_id"] for j in i["jobs"]},
                     occupied_devices=occupied, worker_busy_resources=workers,
                     unproven_D1_capacity_occupancy="unknown_not_inferred_from_W1"))

    parallel = read("scenarios/并行汇合与返工/cases.design.json")
    shared = {c["id"]: c for c in parallel["cases"]}
    first_assembly_start = max(30000, 20000)
    first_end = first_assembly_start + 60000
    first_finish = first_end + 10000 + 20000 + 10000 + 15000
    second_finish = max(60000, 40000, first_end) + 60000 + 10000 + 15000
    assert shared["A"]["expected"]["package_finish_seconds"] == [first_finish // 1000, second_finish // 1000]
    assert shared["Q"]["expected"]["package_finish_seconds"] == [second_finish // 1000]
    assert shared["U"]["expected"]["packages"] == 0 and shared["U"]["expected"]["quality_decisions"] == 0
    control = read("scenarios/从零搭建控制盒厂房/expected-timeline.design.json")
    assert [x["expected_completion_ms"] for x in control["cases"]] == [440000, 570000]
    late = control["cases"][1]["transfers"][-1]
    assert [late["depart_ms"], late["arrival_ms"], late["accepted_ms"], late["resource_available_ms"]] == [360000, 370000, 420000, 425000]
    results.append({"id": "protected_parallel_and_control_box",
                    "status": "passed_independent_arithmetic_and_existing_expectation_check",
                    "A_finish_ms": [first_finish, second_finish], "Q_finish_ms": [second_finish],
                    "U_quality_decisions": 0, "control_completion_ms": [440000, 570000],
                    "control_late_transfer": late})

    prefix = "docs/04_开发与验收/特征细化/"
    definition = read(prefix + "DRF06_完整定义.json")
    gaps = read(prefix + "DRF06_缺口与修复.json")
    timeline = read(prefix + "DRF06_时间轴独立预期.json")
    assert len(definition["applicable_views"]) == 6 and len(gaps["public_handoffs"]) == 8
    assert definition["call_flows"][0]["return"].startswith("EditCommit.")
    api = (ROOT / "packages/commands/api.design.rs").read_text(encoding="utf-8-sig")
    assert re.search(r"fn apply_commands[\s\S]*?Result<EditCommit, ApiError>", api)
    assert read("docs/03_架构与契约/公开操作与宿主调用契约.json")["api_version"] == definition["api_and_file_versions"]["api_version"]
    selected = read("docs/00_项目基线/首版任务选择.json")
    choices = read("docs/00_项目基线/全量设计编码确认选择.json")
    assert len(selected["selected"]) == 21
    assert len(selected["scope_freeze"]["included_requirement_ids"]) == 17
    assert len(choices["selected_question_ids"]) == 64
    fixtures = {f["id"]: f for f in timeline["fixtures"]}
    assert len(fixtures) == 8 and all(f["source_kind"] == "manual_expected" for f in fixtures.values())
    t1 = fixtures["DRF06-T01"]["expected"]
    assert sum(x["interval"]["end_ms"] - x["interval"]["start_ms"] for x in t1["phase_work_intervals"]) == t1["effective_work_ms"] == 40000
    assert sum(union_length(c["intervals_ms"]) * c["units"] for c in t1["claims"] if c["resource_id"] == "W-C") == 10000
    assert union_length([w["interval_ms"] for w in t1["waits"]]) == 5000
    assert t1["waits"][0]["phase_ref"] == "unload"
    assert fixtures["DRF06-T02"]["expected"]["partial"]["phase_work_intervals"][0]["effective_progress_end_ms"] == rows["B12"]["expected"][0]["effective_progress_ms"]
    assert fixtures["DRF06-T03"]["expected"]["case_A_phase_rows_ms"][-1][-1] == second_finish
    output_wait = fixtures["DRF06-T07"]["expected"]["output_wait_job"]
    oi = output_wait["input"]
    work_end = oi["start_ms"] + oi["effective_duration_ms"]
    transfer_at = max(work_end, oi["receiver_frees_slot_ms"])
    assert output_wait["phase_work_ms"] == [oi["start_ms"], work_end]
    assert output_wait["wait_ms"] == [work_end, transfer_at]
    assert output_wait["device_claim_ms"] == [oi["start_ms"], transfer_at]
    assert output_wait["person_claim_ms"] == [oi["start_ms"], work_end]
    assert output_wait["before_transfer"]["finished_at_ms"] is None
    assert output_wait["finished_at_ms"] == transfer_at
    assert output_wait["created_quantity"] == output_wait["transfer_commit_count"] == 1
    frozen_business = read("scenarios/真实业务设计/expected-business.design.json")
    for variant in fixtures["DRF06-T04"]["expected"]["variants"]:
        original = next(c for c in frozen_business["cases"] if c["id"] == variant["id"])
        assert variant["orders"] == original["expected_orders"]
        assert variant["worker_a_occupied_ms"] == original["expected_worker_a_occupied_ms"]

    refs_checked = set()
    def walk(value):
        if isinstance(value, dict):
            if "actual_result" in value:
                assert value["actual_result"] is None
            for key, child in value.items():
                if key in ("source_refs", "business_sources", "files") and isinstance(child, list):
                    for ref in child:
                        if "/" in ref:
                            target = ref.split("#", 1)[0]
                            assert (ROOT / target).exists(), target
                            refs_checked.add(target)
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)
    for value in [definition, gaps, timeline, boundary, read("scenarios/真实业务设计/business-inputs.design.json")]:
        walk(value)
    results.append({"id": "DRF06_documents_and_timeline", "status": "passed_static_and_finite_math",
                    "views": 6, "owner_handoffs": 8, "timeline_fixtures": 8,
                    "source_files_and_directories_resolved": len(refs_checked),
                    "original_SF_P_Q_counts": [17, 21, 64],
                    "product_runtime_executed": False})

    domain = (ROOT / "packages/domain/types.design.rs").read_text(encoding="utf-8-sig")
    current_public_field_presence = {
        "JobState.execution_ref": bool(re.search(r"pub struct JobState \{[^}]*execution_ref", domain)),
        "WaitInterval.phase_ref": bool(re.search(r"pub struct WaitInterval \{[^}]*phase_ref", domain)),
        "ResourceClaim.phase_ref": bool(re.search(r"pub struct ResourceClaim \{[^}]*phase_ref", domain)),
        "ResourceClaim.interval_end_causes": bool(re.search(r"pub struct ResourceClaim \{[^}]*interval_end_causes", domain)),
        "RunResult.phase_work_intervals": bool(re.search(r"pub struct RunResult \{[^}]*phase_work_intervals", domain)),
        "RunResult.computed_until_ms": bool(re.search(r"pub struct RunResult \{[^}]*computed_until_ms", domain)),
        "WaitInterval.end_cause": bool(re.search(r"pub struct WaitInterval \{[^}]*end_cause", domain)),
        "InitialConditions.material_allocations": bool(re.search(r"pub struct InitialConditions \{[^}]*material_allocations", domain)),
        "RuntimeItem.split_from_ref": bool(re.search(r"pub struct RuntimeItem \{[^}]*split_from_ref", domain)),
    }
    assert all(current_public_field_presence.values())
    results.extend(verify_current_delta(timeline))
    return {"document_kind": "DRF06_static_and_finite_math_verification_not_product_runtime",
            "date": "2026-10-14", "status": "passed_static_and_finite_math",
            "checks": results, "check_count": len(results),
            "boundary_checks": 18, "legacy_business_variants": 6, "timeline_fixtures": 8,
            "protected_expectations_rewritten": False,
            "current_public_field_presence_readonly_snapshot": current_public_field_presence,
            "shared_merge_status": "canonical_fields_merged_current_parallel_array_and_C01_consumers_checked",
            "refinement_revision": "DRF06-2a", "current_delta_finite_cases": 8,
            "native_owner_finite_cases_readonly": 20,
            "actual_product_runtime": None, "actual_prototype": None,
            "actual_human_walkthrough": None, "actual_result": None}


if __name__ == "__main__":
    result = verify()
    OUTPUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{result['check_count']} static/finite checks passed; 18 boundaries, 6 legacy variants, 8 timeline fixtures. No product runtime.")
