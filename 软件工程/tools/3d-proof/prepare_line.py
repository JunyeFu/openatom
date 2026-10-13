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
config = dict(document_kind="L03_local_executable_proof_input_not_product_project", project_id="E01", hall_id="H01", line_id="L03",
              source_seed="packages/module-kit/demo-equipment/definitions.design.json", stations=records,
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
