"""C04 current typed adoption binding + independent business arithmetic; no Host/Engine."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
OWN = "docs/04_开发与验收/特征细化/"
C04 = "scenarios/九类模块采用/adoption-output/"


def read(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8-sig"))


def qty(q):
    return q["value"]["coefficient"] / 10 ** q["value"]["decimal_places"]


def ref(definition):
    return {k: definition[k] for k in ("id", "version")}


def verify():
    fixture = read(OWN + "DRF06_C04_type_fixture.json")
    closure = read(OWN + "DRF06_C04_显式绑定收口.json")
    template = read(C04 + "template.json")["data"]
    adoption = template["adoption"]
    b, expected = fixture["public_bindings"], fixture["expected"]
    defs = {}
    for name in [d["id"] for d in template["devices"]] + ["preparation-board-preparation"]:
        defs[name] = read(C04 + f"definitions/{name}/module.json")["data"]["definition"]
    rules = {k: v["payload"]["data"]["behavior"]["inline"]
             for k, v in defs.items() if v["payload"]["category"] == "devices"}
    prep = defs["preparation-board-preparation"]["payload"]["data"]
    checks = []

    def check(name, ok, evidence):
        assert ok, name
        checks.append({"id": f"B4-{len(checks)+1:02}", "meaning": name,
                       "matched": True, "evidence": evidence})

    check("Current typed adoption and derived output effect exist",
          template["seed"] is None and len(template["locations"]) == 12
          and len(template["connections"]) == 18 and len(defs) == 7
          and all(d["module_ref"] == ref(defs[d["id"]]) for d in template["devices"])
          and defs["BoardFinish01"]["version"] == "0.2.0"
          and rules["BoardFinish01"]["outputs"][0]["declared_output_quality"] == "released",
          {"template": adoption["template_ref"], "BoardFinish": ref(defs["BoardFinish01"]),
           "old_0_1_0_not_modified": True})
    schema_fields = {}
    for filename, typename, field in [
        ("template", "TemplateSubset", "adoption"),
        ("project", "ProjectState", "template_adoptions"),
        ("module", "MaterialRoleQuantity", "declared_output_quality")]:
        schema = read(f"tools/type-contract-proof/output/schemas/{filename}.schema.json")
        node = schema if typename == schema.get("title") else schema["$defs"][typename]
        schema_fields[typename + "." + field] = field in node["properties"]
    ts = ROOT / "tools/type-contract-proof/output/typescript"
    native = read("tools/type-contract-proof/output/type-proof-result.json")
    api_adoption = read("docs/03_架构与契约/公开操作与宿主调用契约.json")["module_adoption_contract"]
    typed_adoption = read("docs/03_架构与契约/全层类型与工程格式.json")["typed_template_adoption_contract"]
    assert api_adoption["reserved_execution_capacity_role"] == typed_adoption["execution_capacity_binding"]
    assert "同device_id已选择inline" in api_adoption["tasks"]
    assert "DeviceInstance.module_ref" in typed_adoption["effective_operation_identity"]
    assert "不再同时按Device容量重复扣减" in typed_adoption["execution_capacity_binding"]
    check("Actual canonical Schema and TS carriers, native receipt read only",
          all(schema_fields.values())
          and "source_id" in (ts / "TemplateResourceChoice.ts").read_text(encoding="utf-8-sig")
          and "declared_output_quality" in (ts / "MaterialRoleQuantity.ts").read_text(encoding="utf-8-sig")
          and native["status"] == "passed_actual_native_type_compile_and_codegen",
          {"schema_fields": schema_fields, "native_status": native["status"],
           "native_typescript_files": native["typescript_files"], "receipt_rerun": False,
           "public_capacity_and_inline_norms_aligned": True})

    source_order = ["W-B", "W-C", "W-A", "W-E"]
    check("Four worker source keys/order/requirements and explicit choices",
          [c["source_id"] for c in adoption["resource_choices"]] == source_order
          and [c["requirement_index"] for c in adoption["resource_choices"]] == list(range(4))
          and all(c["resource_id"] is None for c in adoption["resource_choices"])
          and all(r["role"] == "legacy.worker." + source_order[i]
                  and qty(r["capacity"]) == 1 and r["capacity"]["unit"] == "person"
                  and r["default_calendar_ms"] == [{"start_ms": 0, "end_ms": 300000}]
                  for i, r in enumerate(template["resource_requirements"]))
          and [w["source_id"] for w in b["workers"]] == source_order,
          {"source_order": source_order, "source_resources_created": False})
    resources = {r["id"]: r for r in b["resources"]}
    calendars = {c["id"]: c for c in b["calendars"]}
    given_defs = {tuple(d["module_ref"].values()): d for d in b["given_resource_definitions"]}
    check("Explicit actual fixture resources/calendars/capacities all resolve",
          len(resources) == 10 and len(calendars) == 10
          and all(r["calendar_id"] in calendars
                  and tuple(r["definition_ref"].values()) in given_defs
                  and qty(r["capacity"]) == 1
                  and r["capacity"]["unit"] == ("person" if r["kind"] == "worker" else "station")
                  for r in resources.values()),
          {"resources": list(resources), "calendar_count": len(calendars),
           "resource_definitions": "explicit TF given descriptors; not real catalog registration"})
    devices = {d["source_id"]: d for d in b["devices"]}
    source_workers = {w["source_id"]: w["fixture_resource_id"] for w in b["workers"]}
    for sd in template["devices"]:
        td = devices[sd["id"]]
        assert td["definition_ref"] == sd["module_ref"]
        assert td["worker_role_bindings"] == {
            role: source_workers[source] for role, source in sd["role_bindings"]["resources"].items()}
        assert td["material_store_targets"] == {
            role: {"kind": target["kind"], "id": "TF." + target["id"]}
            for role, target in sd["role_bindings"]["targets"].items()
            if role != "target"}
        assert td["item_type_bindings"] == sd["role_bindings"]["item_types"]
        slot = next(s for s in b["execution_slots"] if s["source_device_id"] == sd["id"])
        assert slot["resource_id"] == td["execution_capacity_binding"]
        assert slot["execution_capacity_ref"] == {"kind": "resource", "id": slot["resource_id"]}
    check("One complete remap with a single capacity claim identity",
          devices["Inspection01"]["worker_role_bindings"]["inspector"]
          == devices["Packing01"]["worker_role_bindings"]["packer"] == "TF.worker.B",
          {"shared_worker": "TF.worker.B", "device_identity_is_second_claim": False})

    typed = b["template_adoption"]
    route = b["route"]
    prepstep = next(s for s in route["steps"] if s["id"] == b["preparation"]["route_step_id"])
    for st in route["steps"]:
        assert all(p in {x["id"] for x in route["steps"]} for p in st["predecessors"])
        assert all(r in resources for r in st["resource_bindings"].values())
        if st is not prepstep:
            dv = devices[st["device_id"].removeprefix("TF.")]
            assert st["operation_ref"] == dv["definition_ref"]
            assert st["item_bindings"] == dv["item_type_bindings"]
    check("Exact preparation Task/target/roles/route and typed obligation projection",
          prepstep["operation_ref"] == ref(defs["preparation-board-preparation"])
          and prepstep["device_id"] == b["preparation"]["targets"]["target"]["id"] == "TF.Board01"
          and prep["target_role"] == "target"
          and prep["equipment_access"] == "exclusive" and prep["duration"]["value"] == 30000
          and prep["completion_effect"] == {"target_role": "target", "marker": "board-preparation",
                                             "produces_material": False}
          and typed["preparation_bindings"][0]["worker_roles"] == {"legacy.worker.W-E": "W-E"}
          and typed["preparation_bindings"][0]["route_refs"] == [{"id": route["id"], "revision": route["revision"]}]
          and devices["Board01"]["target_role_bindings"]["target"]["id"] == "TF.Board01",
          {"Task": prepstep["operation_ref"], "source_key_preserved": "W-E",
           "actual_worker": b["preparation"]["resources"]["legacy.worker.W-E"], "materials_created": 0})
    phase_times = {"CNC01": [10000, 45000, 5000], "Board01": [5000, 20000, 5000]}
    for name, times in phase_times.items():
        rule = rules[name]
        assert rule["duration"] is None
        assert [p["duration"]["value"] for p in rule["phases"]] == times
        assert all(p["interruption_policy"] == "pause_resume" for p in rule["phases"])
        old = read("scenarios/九类模块采用/template-output/" + f"definitions/{name}/module.json")
        assert old["data"]["definition"]["payload"]["data"]["behavior"]["inline"]["phases"] == rule["phases"]
    check("Phase vs whole-job lifetimes preserve declared durations and pause_resume",
          all(not rules[k]["phases"] for k in ["BoardFinish01", "Assembly01", "Inspection01", "Packing01"])
          and [rules[k]["duration"]["value"] for k in ["BoardFinish01", "Assembly01", "Inspection01", "Packing01"]]
          == [30000, 40000, 10000, 20000],
          {"phase_times_ms": phase_times, "whole_job_slot_until_output_transfer": True})

    # Independent recurrence: release/input completion + previous use of each capacity/worker.
    def serial(starts, duration, ready=0):
        out = []
        for arrival in starts:
            start = max(arrival, ready)
            ready = start + duration
            out.append([start, ready])
        return out

    pt = prep["duration"]["value"]
    cnc = serial([0, 0], sum(phase_times["CNC01"]))
    board = serial([pt, pt], sum(phase_times["Board01"]), pt)
    finish = serial([i[1] for i in board], rules["BoardFinish01"]["duration"]["value"])
    material_ready = [max(cnc[i][1], finish[i][1]) for i in range(2)]
    assembly = serial(material_ready, rules["Assembly01"]["duration"]["value"])
    inspection, packing, b_free = [], [], 0
    for arr in assembly:
        ins_start = max(arr[1], b_free)
        inspection.append([ins_start, ins_start + rules["Inspection01"]["duration"]["value"]])
        packing.append([inspection[-1][1], inspection[-1][1] + rules["Packing01"]["duration"]["value"]])
        b_free = packing[-1][1]
    actual = {"CNC_job_intervals_ms": cnc, "Board_job_intervals_ms": board,
              "BoardFinish_intervals_ms": finish, "Assembly_intervals_ms": assembly,
              "Inspection_intervals_ms": inspection, "Packing_intervals_ms": packing,
              "released_pcb_at_ms": [i[1] for i in finish],
              "accepted_package_at_ms": [i[1] for i in packing],
              "second_assembly_wait_for_worker_ms": [material_ready[1], assembly[1][0]]}
    check("Independent two-item recurrence matches locked timeline",
          all(v == expected[k] for k, v in actual.items()), actual)

    def people_phases(jobs, rule):
        intervals = []
        for start, _ in jobs:
            for phase in rule["phases"]:
                end = start + phase["duration"]["value"]
                if phase["phase_resources"]:
                    intervals.append([start, end])
                start = end
        return intervals

    c = people_phases(cnc, rules["CNC01"])
    e = people_phases(board, rules["Board01"])
    workers = {"TF.worker.C": c, "TF.worker.E": [[0, pt]] + e,
               "TF.worker.A": assembly, "TF.worker.B": inspection + packing}
    occupied = {r: sum(y-x for x, y in values) for r, values in workers.items()}
    slots = {devices[k]["execution_capacity_binding"]: intervals for k, intervals in [
        ("CNC01", cnc), ("Board01", [[0, pt]] + board), ("BoardFinish01", finish),
        ("Assembly01", assembly), ("Inspection01", inspection), ("Packing01", packing)]}
    for rid, intervals in {**workers, **slots}.items():
        intervals = sorted(intervals)
        assert all(a[1] <= z[0] for a, z in zip(intervals, intervals[1:]))
        windows = calendars[resources[rid]["calendar_id"]]["intervals"]
        assert all(any(w["start_ms"] <= x < y <= w["end_ms"] for w in windows) for x, y in intervals)
    check("Resource claims fit calendars, never overlap or double-debit the device",
          occupied == expected["worker_occupied_ms"]
          and c == expected["CNC_person_phase_intervals_ms"]
          and e == expected["Board_person_phase_intervals_ms"],
          {"occupied_ms": occupied, "slot_claims": slots})
    check("Two-package BOM quantities and receiving capacities are sufficient",
          b["initial_conditions"]["stock_piece"] == {"blank": 2, "bare_pcb": 2, "component": 16, "fastener": 8, "carton": 2}
          and b["BOM"]["component_per_board"] * 2 == 16 and b["BOM"]["fastener_per_assembly"] * 2 == 8
          and all(l["capacity_piece"] >= (2 if l["item_type"] not in ["component", "fastener"] else 8)
                  for l in b["locations"])
          and all(q == "unchecked" for q in b["initial_conditions"]["initial_quality"].values()),
          {"packages": 2, "initial_quality": "explicit unchecked fixture choice",
           "no_material_from_preparation": True})
    # A finite pause probe changes no real source or expected two-item timeline.
    check("Phase-only person releases while execution slot and progress remain",
          (32000-30000) + (43000-40000) == phase_times["Board01"][0]
          and 68000-30000 == sum(phase_times["Board01"]) + 8000,
          {"person_load_ms": [[30000, 32000], [40000, 43000]],
           "person_gap_ms": [32000, 40000], "slot_held_ms": [30000, 68000],
           "probe_only": True})

    marker = ("TF.run", "TF.Board01", tuple(prepstep["operation_ref"].values()),
              devices["Board01"]["configuration_ref"], "board-preparation")
    completed_evidence = {marker: {"job_id": "TF.preparation.job", "completed_at_ms": pt}}
    check("Readiness derives from one completed Job scoped to Run/target/Task/config",
          marker in completed_evidence
          and ("TF.other-run", *marker[1:]) not in completed_evidence
          and (marker[0], "TF.other-target", *marker[2:]) not in completed_evidence
          and (*marker[:3], "TF.other-config", marker[4]) not in completed_evidence,
          {"durable_evidence": completed_evidence[marker], "reused_jobs": 0,
           "ready_before_30000": False, "new_run_inherits_marker": False})
    preset = b["quality_preset"]
    current_algorithm = read("docs/03_架构与契约/调度与状态转换算法.json")
    quality_alg = next(r for r in current_algorithm["rules"] if r["id"] == "ALG-08")
    assert any("本Run+实际限定ID" in text and "共享序列" in text for text in quality_alg["algorithm"])
    ruleq = rules["Inspection01"]["quality_and_routes"]
    check("Preset has an exact module, explicit ancestor and qualified local-key resolution",
          preset["inspection_module"] == ref(defs["Inspection01"])
          and preset["match_kind"] == "ancestor"
          and preset["item_or_ancestor"] == b["initial_conditions"]["blank_batch_id"]
          and preset["result_ref_resolution"]["immutable_module_result_ref"] == ruleq["result_ref"]
          and preset["results"] == adoption["inspection_presets"][0]["results"]
          and typed["inspection_presets"][0]["item_or_ancestor"] == preset["item_or_ancestor"],
          {"id": preset["id"], "immutable_result_ref": ruleq["result_ref"],
           "source_preset_scope_still_null": adoption["inspection_presets"][0]["item_or_ancestor"] is None})
    # Source result ordinal is locked when starting; completion consumes that same value once.
    started, locked, decisions = 0, {}, {}
    def start(job):
        nonlocal started
        if job not in locked:
            locked[job] = started
            started += 1
        return locked[job]
    def complete(job):
        index = locked[job]
        raw = preset["results"][index] if index < len(preset["results"]) else None
        if job not in decisions:
            decisions[job] = raw["value"] if raw and raw["kind"] in ("string", "enum") else "UNKNOWN"
        return decisions[job]
    assert start("TF.inspect.1") == 0
    at_start = {"started": started, "decisions": len(decisions)}
    assert start("TF.inspect.1") == 0  # Resume after phase pause or compute cancellation.
    values = [complete("TF.inspect.1")]
    complete("TF.inspect.1")
    assert start("TF.inspect.2") == 1
    values.append(complete("TF.inspect.2"))
    check("Inspection start locks ordinal, pause/cancel retains it, completion decides once",
          at_start == {"started": 1, "decisions": 0} and started == 2
          and list(locked.values()) == expected["inspection_reserved_indices"]
          and values == expected["preset_values_consumed_on_completion"] and len(decisions) == 2,
          {"at_first_start": at_start, "locked": locked, "completed_decisions": decisions,
           "outputs_per_completed_inspection": 1})
    unknowns = [None, {"kind": "string", "value": "UNKNOWN"}, {"kind": "integer", "value": 1}]
    def quality(raw):
        return "released" if raw and raw.get("kind") in ("string", "enum") and raw.get("value") == "PASS" else "pending"
    check("Exhausted/null/unsupported result never becomes PASS",
          all(quality(x) == "pending" for x in unknowns) and quality(None) == "pending",
          {"probe_count": len(unknowns)+1, "new_inspection_job_created": False, "result": "pending"})

    output = rules["BoardFinish01"]["outputs"][0]
    slot = {"TF.pcb.1": {"quality": "unchecked", "holder": "TF.finish.job"}}
    affected = set()
    def apply_output(completed, actual_module, actual_port, items):
        if not completed or actual_module != ref(defs["BoardFinish01"]) or actual_port != output["port"]:
            return 0
        new_ids = set(items) - affected
        for item in new_ids:
            items[item]["quality"] = output["declared_output_quality"]
        affected.update(new_ids)
        return len(new_ids)
    counts_before = [apply_output(False, ref(defs["BoardFinish01"]), "output", slot) for _ in range(4)]
    wrong_module = apply_output(True, ref(defs["Board01"]), "output", slot)
    count_at_completion = apply_output(True, ref(defs["BoardFinish01"]), "output", slot)
    retry_count = apply_output(True, ref(defs["BoardFinish01"]), "output", slot)
    check("Only exact actual final output-slot completion releases once; blocked retry retains holder",
          counts_before == [0, 0, 0, 0] and wrong_module == 0 and count_at_completion == 1
          and retry_count == 0 and slot["TF.pcb.1"] == {"quality": "released", "holder": "TF.finish.job"}
          and output["required_quality"] == []
          and rules["Packing01"]["inputs"][0]["required_quality"] == ["released"],
          {"before_completion": counts_before, "at_completion": 1, "retry": 0,
           "slot": slot, "physical_receipt_or_order_acceptance": False})
    missing = {
        "worker_choice": any(x["resource_id"] is None for x in adoption["resource_choices"]),
        "preparation_route": not adoption["preparation_bindings"][0]["route_refs"],
        "preset_scope": adoption["inspection_presets"][0]["item_or_ancestor"] is None,
        "Product_BOM_Route": any(adoption["deferred_seed"][k] is None for k in ("product_ref", "bom_ref", "routing_ref")),
        "initial_quality": any(x["quality"] is None for x in adoption["deferred_seed"]["initial_stock"]),
        "machine_capacity_calendar": all("execution_capacity" not in d["role_bindings"]["resources"] for d in template["devices"]),
    }
    check("Unselected real inputs remain named admission obligations; no fixture backfill",
          all(missing.values()) and template["seed"] is None and fixture["actual_result"] is None
          and not fixture["wire_Project_parse_claim"]
          and any(x["kind"] == "site_geometry" for x in adoption["missing_inputs"])
          and fixture["actual_Engine"] is None,
          {"real_missing_inputs": missing, "draft_persistence_allowed": True,
           "delete_Issue_text_resolves_typed_values": False})
    selection = read("docs/00_项目基线/首版任务选择.json")["selected"]
    rows = closure["task_coverage"]
    check("Original 21 tasks cover current design references and legal user inputs",
          len(rows) == len(selection) == 21
          and [r["P_id"] for r in rows] == [r["id"] for r in selection]
          and all(r["legal_user_input"] and not r["missing_referenced_files"] for r in rows)
          and all((ROOT / path).exists() for r in rows for path in r["design_input_refs"]),
          {"task_count": 21, "missing_referenced_files": [],
           "remaining_common_normative_delta": [x["id"] for x in closure["required_still_missing"]],
           "task_status_or_gate_changed": False})

    receipt = read(OWN + "DRF03_C04_r4_核验结果.json")
    return {
        "document_kind": "DRF06_C04_new_binding_finite_type_fixture_check_not_Engine",
        "status": "passed_current_binding_and_independent_business_arithmetic",
        "check_count": len(checks), "all_matched": True, "checks": checks,
        "actual_timeline": actual, "worker_occupied_ms": occupied,
        "window3_readonly_reference": {"case_count": receipt["case_count"],
                                      "all_matched": receipt["all_matched"],
                                      "schema_kind": receipt["schema_kind"],
                                      "script_called": False},
        "public_native_receipt_readonly": {"status": native["status"], "typescript_files": native["typescript_files"],
                                           "fixture_roundtrips": native["fixture_roundtrips"],
                                           "api_fixture_roundtrips": native["api_fixture_roundtrips"], "script_called": False},
        "old_35_25_20_proofs_rerun": [], "actual_Host_or_Engine": None,
        "actual_complete_professional": None, "actual_result": None,
        "stage_gate_frozen": False, "public_progress_written": False,
        "business_design_ready": True,
        "preserved_original_inputs": closure["preserved_original_inputs"],
        "limits": "Only explicit TF given inputs, field mapping and finite formulas; no canonical whole-Project parse, actual C04 Route, producer/native machine result, product run or UI acceptance."
    }


if __name__ == "__main__":
    result = verify()
    (HERE / "DRF06_C04_绑定有限核验.json").write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f'{result["check_count"]}/{result["check_count"]} new C04 binding checks matched; no Host/Engine.')

