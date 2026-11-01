"""Build the L03 proof input from the existing versioned business declarations."""
import json
from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
seed = json.loads((ROOT / "packages/module-kit/demo-equipment/definitions.design.json").read_text(encoding="utf-8"))
definitions = {d["id"]: d for d in seed["candidate_definitions"]}
for ref in [*seed["existing_definitions"],
            {"manifest": "modules/devices/identity-rework/manifest.json"},
            {"manifest": "modules/devices/carton-pack/manifest.json"}]:
    definition = json.loads((ROOT / ref["manifest"]).read_text(encoding="utf-8"))
    definitions[definition["id"]] = definition
stations = [
    ("ASM01", "四螺钉控制盒装配台", "assembly", "community.control-box-assembly", "assemble", 0, 0),
    ("R01", "单件夹爪搬运机器人", "robot", "superfactory.demo.robot-transfer", "robot", 1.7, -.9),
    ("CV01", "检查前输送机", "conveyor", "superfactory.demo.belt-transfer", "conveyor", 3.3, 0),
    ("V01", "检验与分流台", "inspection", "community.inspect-route", "inspect", 5.6, 0),
    ("RW01", "保留身份返工台", "assembly", "community.identity-rework", "rework", 5.6, 2.6),
    ("CV02", "装箱前输送机", "conveyor", "superfactory.demo.belt-transfer", "conveyor", 7.9, 0),
    ("PK01", "单件装箱工位", "assembly", "community.carton-pack", "pack", 10.3, 0),
    ("CASE01", "胶带封箱机", "sealer", "superfactory.demo.carton-sealer", "seal", 12.7, 0),
]
records = []
for instance_id, title, kind, definition_id, operation, x, z in stations:
    definition = definitions[definition_id]
    behavior = definition["behavior"]
    records.append(dict(id=instance_id, title=title, kind=kind, definition_ref={"id": definition_id, "version": definition["version"]},
                        operation=operation, duration_ms=round(behavior["duration"]["value"] * 1000),
                        behavior=behavior, ports=definition["ports"], resources=[r["role"] for r in behavior["resources"]],
                        placement=[x, z]))
def local_definition(module_id, category, title, field, payload):
    return dict(id=module_id, version="0.1.0", category=category, title=title, definition_state="design",
                contribution=dict(author="本项目自建演示输入", license="待项目许可确认", status="local_proof_only"), **{field: payload})
item_definitions = []
for alias, name, unit in [("shell", "控制盒壳体", "piece"), ("pcb", "控制板", "piece"), ("fastener", "PCB紧固件", "piece"),
                          ("carton", "控制盒包装纸箱", "piece"), ("assembly", "已装配控制盒", "piece"),
                          ("package", "控制盒包件", "piece"), ("sealing_tape", "封箱胶带", "meter")]:
    payload = dict(unit=unit, tracking="individual" if unit == "piece" else "quantity", quantity_is_instance_state=True,
                   specification_basis="自建演示规格；不采用另一壳体样例的100mm规格")
    if alias == "package": payload.update(container="carton", contents="assembly", supported_closure_values=["open", "sealed"], envelope_meter=[.49, .34, .36])
    if alias == "assembly": payload.update(recipe=dict(shell=1, pcb=1, fastener=4), visual="shared_control_box_geometry_1_to_1")
    if alias == "sealing_tape": payload["quantity_resolution_meter"] = .001
    definition = local_definition("superfactory.demo.item." + alias, "items", name, "item_definition", payload)
    item_definitions.append(dict(alias=alias, **definition))
resource_definitions = [json.loads((ROOT / "modules/resources/operator-role/manifest.json").read_text(encoding="utf-8"))]
for role, title in [("inspector", "检查人员"), ("repairer", "返工人员"), ("packer", "包装人员")]:
    resource_definitions.append(local_definition("superfactory.demo.role." + role, "resources", title, "resource_definition",
                                                dict(kind="worker", role=role, capacity=dict(value=1, unit="person"), current_occupation_is_instance_state=True)))
resource_instances = [dict(id="W-" + d["resource_definition"]["role"].upper() + "-01", role=d["resource_definition"]["role"],
                           definition_ref={"id": d["id"], "version": d["version"]}, capacity=1,
                           availability="always_available_in_this_local_proof", calendar_adoption="explicit_override_of_definition_default") for d in resource_definitions]
config = dict(document_kind="L03_local_executable_proof_input_not_product_project", project_id="E01", hall_id="H01", line_id="L03",
              source_seed="packages/module-kit/demo-equipment/definitions.design.json", stations=records,
              item_definitions=item_definitions, resource_definitions=resource_definitions, resource_instances=resource_instances,
              item_bindings={d["alias"]: {"id": d["id"], "version": d["version"]} for d in item_definitions},
              pallet=dict(id="PAL01", category="resources", definition_ref={"id": "superfactory.demo.pallet12", "version": "0.1.0"},
                          capacity=definitions["superfactory.demo.pallet12"]["resource_definition"]["capacity"]["value"], placement=[14.9, 0]),
              order=dict(id="ORDER-001", quantity=6, accepted_unit="sealed_package_at_PAL01"),
              supplies=dict(shell=6, pcb=2, fastener=24, carton=6, tape_meter=3.6),
              arrivals=[dict(id="ARR-PCB-001", at_ms=120000, item="pcb", quantity=4)],
              quality_inputs={"A-003": ["fail", "pass"]},
              default_quality="pass", max_reworks=definitions["community.inspect-route"]["behavior"]["max_reworks"],
              queue_capacity=1, timing_basis="沿用声明工时与自建演示输入，不是实测工业节拍",
              flow=["ASM01", "R01", "CV01", "V01", "CV02", "PK01", "CASE01", "PAL01"],
              rework_flow=["V01", "RW01", "V01"],
              item_visual_spec=dict(assembly="shared_control_box_geometry", package_dimensions_meter=[.49, .34, .36],
                                    pallet_grid=[2, 2, 3]), actual_product_result=None)
(BASE / "line-config.json").write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print("L03 input prepared: 8 device instances + PAL01 carrier resource; declarations remain design.")
