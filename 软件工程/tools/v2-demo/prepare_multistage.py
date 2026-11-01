"""Prepare new module source and ordinary Host commands; never rewrite saved inputs."""
from pathlib import Path
import copy
import json
import shutil
from prepare_geometry import layout, pose, read, write

ROOT = Path(__file__).resolve().parents[2]
FINISH = {"id": "p201.task8.finish", "version": "1.0.0"}


def prepare(bundle: Path):
    original = ROOT / "resources/v2-demo/geometry-v4/project"
    staged = bundle / "source"
    if staged.exists():
        raise FileExistsError("A fresh source directory is required")
    bundle.mkdir(parents=True, exist_ok=True)
    shutil.copytree(original, staged)
    manifest = read(staged / "project.json")
    entries = manifest["data"]["documents"]
    entry = next(e for e in entries if e.get("module_ref") == {"id": "p201.b60.station", "version": "1.0.6"})
    package = read(staged / entry["file_ref"]["path"])
    package["data"]["definition"]["id"] = FINISH["id"]
    package["data"]["definition"]["version"] = FINISH["version"]
    package["data"]["definition"]["title"] = "第二工序 · 精整"
    filename = "module-task8-finish-1.0.0.json"
    write(staged / filename, package)
    new_entry = copy.deepcopy(entry)
    new_entry["module_ref"] = FINISH
    new_entry["object_ref"]["id"] = FINISH["id"]
    new_entry["file_ref"]["path"] = filename
    entries.append(new_entry)
    manifest["data"]["module_refs"].append(FINISH)
    write(staged / "project.json", manifest)
    project_entry = next(e for e in entries if e["role"] == "active_project")
    scenario_entry = next(e for e in entries if e["role"] == "scenario_input")
    project = read(staged / project_entry["file_ref"]["path"])["data"]
    scenario = read(staged / scenario_entry["file_ref"]["path"])["data"]
    commands = [{"kind": "adopt_module", "data": {"module_ref": FINISH, "targets": [], "bindings": {"item_types": {}, "resources": {}, "targets": {}, "ports": {}}, "new_instances": [], "explicit_seed": None}}]
    commands.append({"kind": "set_project_settings", "data": {
        "title": "Task8 · 双工序四设备 · 12 件成品",
        "time_reference": project["time_reference"],
        "generation_context": project["generation_context"],
        "sources": project["sources"],
        "setup_policies": project.get("setup_policies"),
    }})

    def put(kind, data):
        commands.append({"kind": "put_object", "data": {"kind": kind, "data": data}})

    line = copy.deepcopy(project["lines"][0])
    line["title"] = "双工序生产 · 粗加工 → 精整"
    line["members"] = ["L1-D01", "L1-D02", "L1-D03", "L1-D04"]
    line["region_polygon_m"] = [{"x": 4, "y": 1}, {"x": 15, "y": 1}, {"x": 15, "y": 12}, {"x": 4, "y": 12}]
    put("line", line)
    for index, location in enumerate(project["locations"]):
        location = copy.deepcopy(location)
        location["item_layout"] = layout(12)
        location["position"] = pose(2 if index == 0 else 18, 6, 0)
        location["title"] = "投入区 · 12 件原料" if index == 0 else "成品区 · 目标 12 件"
        put("location", location)
    buffer = copy.deepcopy(project["locations"][0])
    buffer.update(id="L1-BUFFER", title="工序间缓存 · 在制品", purpose="buffer", position=pose(9, 6, 0), item_layout=layout(12))
    put("location", buffer)
    for index, original_device in enumerate(project["devices"]):
        device = copy.deepcopy(original_device)
        device["pose"]["translation"].update(x=5.5, y=3 if index == 0 else 10)
        device["title"] = f"粗加工 {'A' if index == 0 else 'B'} · {10 if index == 0 else 15} 秒"
        put("device", device)
        finishing = copy.deepcopy(device)
        finishing["id"] = f"L1-D0{index + 3}"
        finishing["module_ref"] = FINISH
        finishing["route_operation_refs"] = [FINISH]
        finishing["role_bindings"]["resources"]["execution_capacity"] = finishing["id"] + "-CAP"
        finishing["pose"]["translation"]["x"] = 13
        finishing["title"] = f"精整 {'C' if index == 0 else 'D'} · {10 if index == 0 else 15} 秒"
        put("device", finishing)
        resource = copy.deepcopy(project["resources"][index])
        resource["id"] = finishing["id"] + "-CAP"
        resource["title"] = finishing["id"] + " 单件容量"
        put("resource", resource)
        commands.append({"kind": "set_declared_parameter", "data": {"target": {"kind": "device", "id": finishing["id"]}, "parameter_id": "processing_sim_seconds", "value": finishing["configuration"]["processing_sim_seconds"], "basis": "user_provided"}})
    for original_connection in project["connections"]:
        connection = copy.deepcopy(original_connection)
        data = connection["data"]
        if data["id"].endswith("-output"):
            data["destination"]["data"] = "L1-BUFFER"
            data["destination_store"] = "L1-BUFFER"
        put("connection", connection)
        connection = copy.deepcopy(original_connection)
        data = connection["data"]
        for old, new in [("L1-D01", "L1-D03"), ("L1-D02", "L1-D04")]:
            data["id"] = data["id"].replace(old, new)
        for endpoint in [data["source"], data["destination"]]:
            if endpoint["kind"] == "port":
                endpoint["data"]["module_ref"] = FINISH
                endpoint["data"]["instance_id"] = endpoint["data"]["instance_id"].replace("L1-D01", "L1-D03").replace("L1-D02", "L1-D04")
        if data["id"].endswith("-input"):
            data["source"]["data"] = "L1-BUFFER"
            data["destination_store"] = "L1-BUFFER"
        put("connection", connection)
    route = copy.deepcopy(project["routes"][0])
    route["revision"] = "r2"
    second = copy.deepcopy(route["steps"][0])
    second.update(id="L1-FINISH-step", operation_ref=FINISH, device_id="L1-D03", predecessors=[route["steps"][0]["id"]])
    route["steps"].append(second)
    put("route", route)
    product = copy.deepcopy(project["products"][0])
    product["revision"] = "r2"
    product["routing_ref"]["revision"] = "r2"
    put("product", product)
    scenario["title"] = "双工序 · 12 件成品 · 设备维护与自动改派"
    scenario["window"]["end_ms"] = 240000
    scenario["acceptance_goal"]["observation_window"]["end_ms"] = 240000
    scenario["acceptance_goal"]["product_family"] = "两道工序与四台可替代设备的 12 件试件"
    scenario["acceptance_goal"]["acceptance_scope"]["product_ref"]["revision"] = "r2"
    template = scenario["initial_conditions"]["items"][0]
    scenario["initial_conditions"]["items"] = [dict(copy.deepcopy(template), id=f"blank-{index}") for index in range(12)]
    scenario["production_tasks"][0]["routing_ref"]["revision"] = "r2"
    scenario["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = 12
    put("scenario", scenario)
    write(bundle / "multistage-edit.json", {"base_revision": project["revision"], "commands": commands})
    return staged


if __name__ == "__main__":
    import sys
    print(prepare(Path(sys.argv[1])))
