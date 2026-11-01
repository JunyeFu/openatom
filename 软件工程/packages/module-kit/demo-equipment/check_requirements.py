"""Check the local design seed against canonical declarations and actual GLB nodes.

This reads files and calculates finite specification expectations. It is neither
a module registrar nor a production simulator and never fills actual_result.
"""
import json
import math
import struct
from collections import Counter
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def key(record):
    return record["id"], record["version"]


def check(condition, message):
    if not condition:
        raise ValueError(message)


def bound_value(definition, binding):
    value = definition
    for token in binding.replace("[", ".").replace("]", "").split("."):
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def recipe(definition, stock):
    behavior = definition["behavior"]
    ports = {p["id"]: p for p in definition["ports"]}
    count = min(stock[ports[i["port"]]["item"]] // i["quantity"] for i in behavior["inputs"])
    remaining = stock.copy()
    for line in behavior["inputs"]:
        remaining[ports[line["port"]]["item"]] -= count * line["quantity"]
    output = {ports[i["port"]]["item"]: count * i["quantity"] for i in behavior["outputs"]}
    return {"possible_cycles": count, "remaining": remaining, "output": output,
            "working_seconds": behavior["duration"]["value"]}


def main():
    seed = read(BASE / "definitions.design.json")
    requirements = read(BASE / "equipment.design.json")
    contracts = read(ROOT / "packages/module-kit/category-contracts.design.json")
    category_fields = {c["id"]: c["payload_fields"] for c in contracts["categories"]}
    definitions = {}
    for reference in seed["existing_definitions"]:
        definition = read(ROOT / reference["manifest"])
        check(key(definition) == key(reference), f"Definition/version mismatch: {reference['manifest']}")
        check(definition["category"] == reference["category"], f"Category mismatch: {reference['id']}")
        check(key(definition) not in definitions, f"Duplicate definition reference: {key(definition)}")
        definitions[key(definition)] = definition
    for definition in seed["candidate_definitions"]:
        check(key(definition) not in definitions, f"Duplicate definition: {key(definition)}")
        definitions[key(definition)] = definition
    media = {key(m): m for m in requirements["media_definitions"]}
    check(len(media) == len(requirements["media_definitions"]), "Duplicate media identity/version")
    check(not set(media).intersection(definitions), "A business definition and media share identity")

    for definition in [*definitions.values(), *media.values()]:
        check(set(contracts["common_fields"]) <= definition.keys(), f"Missing common fields: {definition['id']}")
        check(all(field in definition for field in category_fields[definition["category"]]),
              f"Missing category payload: {definition['id']}")
        check(definition["definition_state"] == "design", f"Unexpected runtime claim: {definition['id']}")
        check(not {"current_stock", "current_holder", "completed_quantity", "current_occupation"}.intersection(definition),
              f"Runtime state stored in definition: {definition['id']}")
        if definition["category"] != "devices":
            continue
        check(("behavior" in definition) != ("operation_ref" in definition),
              f"Choose one behavior source: {definition['id']}")
        ports = {p["id"]: p for p in definition["ports"]}
        check(len(ports) == len(definition["ports"]), f"Duplicate ports: {definition['id']}")
        behavior = definition["behavior"]
        # Existing inspection declares alternative output routes, not a recipe output.
        outputs = [r for r in behavior["routes"] if "port" in r] if "routes" in behavior else behavior["outputs"]
        for direction, entries in [("in", behavior["inputs"]), ("out", outputs)]:
            for line in entries:
                check(line["port"] in ports and ports[line["port"]]["direction"] == direction,
                      f"Unbound behavior port: {definition['id']}:{line['port']}")
        for parameter in definition["parameters"]:
            bound_value(definition, parameter["binding"])
        if "stages" in behavior:
            check(math.isclose(sum(s["ratio"] for s in behavior["stages"]), 1),
                  f"Stages do not cover the cycle: {definition['id']}")
        if "declaration" in behavior:
            check("builtin" not in behavior and behavior["implementation_status"] in {"required_not_implemented", "local_proof_only_not_product_runtime"},
                  f"New declaration falsely marked executable: {definition['id']}")

    asset_checks = []
    for model in media.values():
        presentation = model["presentation"]
        check(presentation["production_effect"] == "none", f"Media changes production: {model['id']}")
        path = ROOT / presentation["asset_locator"]["path"]
        raw = path.read_bytes()
        check(struct.unpack_from("<4sII", raw) == (b"glTF", 2, len(raw)), f"Invalid GLB: {path.name}")
        length, chunk_type = struct.unpack_from("<I4s", raw, 12)
        check(chunk_type == b"JSON", f"Missing GLB JSON: {path.name}")
        gltf = json.loads(raw[20:20 + length])
        node_counts = Counter(node.get("name") for node in gltf["nodes"])
        for node in model["required_nodes"]:
            check(node_counts[node["name"]] == node["count"],
                  f"GLB node mismatch: {path.name}:{node['name']}, expected {node['count']}, got {node_counts[node['name']]}")
        for motion in model["verified_motion_contract"]:
            matches = [node for node in gltf["nodes"] if node.get("name") == motion["node"]]
            check(len(matches) == 1 and all(matches[0].get("extras", {}).get(k) == v for k, v in motion["extras"].items()),
                  f"Motion metadata mismatch: {path.name}:{motion['node']}")
        asset_checks.append({"file": path.name, "required_nodes_checked": len(model["required_nodes"]),
                             "existing_motion_nodes_checked": len(model["verified_motion_contract"])})

    kinds = set()
    for item in requirements["equipment"]:
        check(item["preview_kind"] not in kinds, f"Duplicate preview kind: {item['preview_kind']}")
        kinds.add(item["preview_kind"])
        definition = definitions[key(item["definition_ref"])]
        check(key(item["media_ref"]) in media, f"Missing media binding: {item['preview_kind']}")
        if "media_ref" in definition["presentation"]:
            check(key(definition["presentation"]["media_ref"]) == key(item["media_ref"]),
                  f"Definition and equipment media disagree: {item['preview_kind']}")
        if definition["category"] == "devices":
            check(set(item["port_roles"]) == {p["id"] for p in definition["ports"]},
                  f"Incomplete port roles: {item['preview_kind']}")
            check(set(item["resource_roles"]) == {r["role"] for r in definition["behavior"]["resources"]},
                  f"Incomplete resource roles: {item['preview_kind']}")
        if "visual_progress_segments" in item:
            cursor = 0
            for segment in item["visual_progress_segments"]:
                check(segment["start"] == cursor and segment["end"] > cursor, f"Visual interval gap: {item['preview_kind']}")
                cursor = segment["end"]
            check(cursor == 1, f"Visual cycle incomplete: {item['preview_kind']}")
    check(kinds == {"assembly", "robot", "conveyor", "inspection", "sealer", "pallet"}, "Six preview models not covered")
    check({r["category"] for r in requirements["nine_category_roles"]} == set(category_fields), "Nine-category responsibility missing")

    cases = {c["id"]: c for c in requirements["finite_expectations"]}
    by_id = {d["id"]: d for d in definitions.values()}
    assembly = by_id["community.control-box-assembly"]
    case = cases["assembly_four_fasteners"]
    check(recipe(assembly, case["input"]) == case["expected"], "Canonical assembly recipe no longer matches expectation")
    reuse = seed["reuse_example"]
    reused = [read(ROOT / d["manifest"]) for d in reuse["definitions"]]
    check(len({key(d) for d in reused}) == 2 and key(reuse["shared_media_ref"]) in media, "Shared geometry merged distinct definitions")
    check(all(key(d) == key(r) for d, r in zip(reused, reuse["definitions"])), "Reuse definition version changed")
    case = cases["same_appearance_different_definition"]
    manual = next(d for d in reused if d["id"] == "community.manual-assembly")
    check(recipe(manual, case["input"])["possible_cycles"] == case["expected"]["manual_possible_cycles"], "Manual assembly must not require fasteners")
    check(recipe(assembly, case["input"])["possible_cycles"] == case["expected"]["four_fastener_possible_cycles"], "Four-fastener assembly accepted missing fasteners")
    check(manual["behavior"]["duration"]["value"] == case["expected"]["manual_working_seconds"], "Manual assembly duration drift")

    robot = by_id["superfactory.demo.robot-transfer"]["behavior"]
    elapsed = 0
    stage_ends = {}
    for stage in robot["stages"]:
        elapsed += stage["ratio"] * robot["duration"]["value"]
        stage_ends[stage["id"]] = elapsed
    expected = cases["robot_identity_and_return"]["expected"]
    for stage, field in [("grip", "robot_holds_at_seconds"), ("release", "delivered_at_seconds"), ("return", "robot_free_at_seconds")]:
        check(math.isclose(stage_ends[stage], expected[field]), f"Robot stage expectation drift: {stage}")
    check(robot["identity_policy"] == "same_item_id_type_quality_and_composition", "Robot must preserve cargo identity")
    conveyor = by_id["superfactory.demo.belt-transfer"]["behavior"]
    check(conveyor["duration"]["value"] == cases["conveyor_output_blocked"]["expected"]["at_seconds"]
          and conveyor["identity_policy"] == robot["identity_policy"], "Conveyor handoff definition drift")
    inspection = by_id["community.inspect-route"]["behavior"]
    check(inspection["duration"]["value"] == cases["inspection_missing_quality"]["expected"]["after_working_seconds"]
          and any(r.get("when") == "unknown_or_missing" and r.get("action") == "hold_item_and_resources" for r in inspection["routes"]),
          "Missing quality must hold the item and inspector")
    sealer_definition = by_id["superfactory.demo.carton-sealer"]
    sealer = sealer_definition["behavior"]
    case = cases["sealing_keeps_container_and_contents"]
    seal_time = sum(s["ratio"] for s in sealer["stages"][:2]) * sealer["duration"]["value"]
    check(seal_time == case["expected"]["seal_at_seconds"] and sealer["duration"]["value"] == case["expected"]["handoff_at_seconds"], "Sealer stage expectation drift")
    check(math.isclose(case["input"]["tape_meter"] - sealer["inputs"][1]["quantity"], case["expected"]["remaining_tape_meter"]), "Tape balance mismatch")
    check(sealer_definition["ports"][1]["unit"] == "meter" and sealer["identity_policy"] == "preserve_package_id_and_all_content_ids", "Sealing units or identity changed")
    pallet = by_id["superfactory.demo.pallet12"]
    case = cases["empty_pallet_then_capacity"]
    capacity = pallet["resource_definition"]["capacity"]["value"]
    check(pallet["category"] == "resources" and capacity == 12, "Pallet must remain a carrier resource with capacity 12")
    check(min(case["input"]["existing_sealed_packages"], capacity) == case["expected"]["accepted_packages"]
          and case["input"]["existing_sealed_packages"] - capacity == case["expected"]["waiting_packages"], "Pallet capacity expectation mismatch")
    check(seed["actual_result"] is None and requirements["actual_result"] is None
          and all(c["actual_result"] is None for c in cases.values()), "Design checks must not fill production actual_result")

    print(json.dumps({"scope": "static_design_and_actual_glb_binding_checks", "business_definitions": len(definitions),
                      "existing_definition_refs": len(seed["existing_definitions"]),
                      "new_design_candidates": len(seed["candidate_definitions"]), "media_definitions": len(media),
                      "asset_checks": asset_checks, "finite_specification_checks": len(cases), "check_result": "passed",
                      "registered_product_execution": "not_run", "local_line_evidence_ref": "tools/3d-proof/output/line-business-checks.json",
                      "catalog_registration": "not_done"}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
