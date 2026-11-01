"""Produce the V2-05 full-factory design from the accepted paper equipment.

This stage writes design data and editable draw.io; it does not create Run results.
The later template producer consumes the same positions and exact module refs.
"""
from __future__ import annotations

import argparse
import csv
import itertools
import json
import math
import struct
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DESIGN = ROOT / "modules/templates/paper-packaging-factory/design"
ART = ROOT / "docs/06_v2.0设计与开发控制/整厂模板设计"
CATALOG = ROOT / "modules/media/native-equipment-500/reviews/B01-Q22/paper-packaging-release-catalog.json"
ACCEPTED = ROOT / "modules/media/native-equipment-500/reviews/B01-Q22/paper-packaging-completion-checks.json"

# Different products have their own routes. These are functional groups, not one
# artificial 92-machine serial line. Dimensions below are explicit demo design.
GROUPS = [
    ("原纸准备", "#deecf7", "paper-unwinder paper-rewinder slitter-rewinder rotary-sheet-cutter guillotine-paper-cutter air-table-loader paper-jogger pile-turner suction-separator sheet-feeder sheet-slitter board-edge-trimmer"),
    ("印刷与表面处理", "#dcefe9", "flexo-printer offset-printer screen-print-press pad-print-press film-laminator flute-laminator varnish-coater uv-curer spot-uv-printer hot-foil-stamper"),
    ("压痕与模切", "#f7e8cb", "flexo-slotter corrugated-slotter rotary-scorer platen-creaser platen-die-cutter rotary-die-cutter digital-flatbed-cutter blank-stripper partition-slotter"),
    ("纸箱折糊成型", "#eadff3", "folder-gluer crash-lock-gluer adhesive-dispenser compression-stack carton-stitcher window-patcher tray-former divider-assembler carton-counter-stacker"),
    ("精品盒与纸品装订", "#e4e7f6", "rigid-box-wrapper box-lid-former corner-rounding-machine blind-embosser book-cover-caser paper-perforator paper-drilling-machine saddle-stitcher wire-o-binder spine-gluer three-knife-trimmer sleeve-former"),
    ("纸袋与信封", "#f5dfd7", "paper-bag-tuber paper-bag-bottomer rope-handle-twister bag-handle-attacher envelope-folder mailing-bag-sealer"),
    ("纸杯碗盘", "#e4efd6", "paper-cup-former paper-bowl-former paper-plate-former"),
    ("纸管与防护材料", "#f5edcf", "paper-tube-winder paper-tube-cutoff paper-core-grinder corner-board-former honeycomb-laminator"),
    ("标签生产", "#d8eef0", "digital-label-printer label-die-cutter label-web-inspector label-peeler"),
    ("检验与成品包装", "#dcebd9", "vision-carton-inspector carton-erector case-packer manual-packing tape-case-sealer shrink-wrapper inkjet-marker label-applicator strapper pallet-stretch-wrapper"),
    ("共享搬运与码垛", "#e0e6eb", "belt-conveyor roller-conveyor curved-conveyor lift-conveyor right-angle-transfer pile-lift six-axis-robot scara-robot gantry-pick-place vacuum-palletizer board-pile-palletizer"),
    ("边料回收与运行服务", "#f0e1d8", "paper-waste-baler"),
]


def read(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write(path: Path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def product(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def transform(node):
    if "matrix" in node:
        return [[node["matrix"][j * 4 + i] for j in range(4)] for i in range(4)]
    x, y, z, w = node.get("rotation", [0, 0, 0, 1])
    sx, sy, sz = node.get("scale", [1, 1, 1])
    tx, ty, tz = node.get("translation", [0, 0, 0])
    return [
        [(1-2*y*y-2*z*z)*sx, (2*x*y-2*z*w)*sy, (2*x*z+2*y*w)*sz, tx],
        [(2*x*y+2*z*w)*sx, (1-2*x*x-2*z*z)*sy, (2*y*z-2*x*w)*sz, ty],
        [(2*x*z-2*y*w)*sx, (2*y*z+2*x*w)*sy, (1-2*x*x-2*y*y)*sz, tz],
        [0, 0, 0, 1],
    ]


def asset_bounds(path):
    """World AABB of actual static GLB; a layout envelope, not factory CAD."""
    with path.open("rb") as f:
        magic, version, _ = struct.unpack("<III", f.read(12))
        assert magic == 0x46546C67 and version == 2
        n, kind = struct.unpack("<II", f.read(8))
        assert kind == 0x4E4F534A
        gltf = json.loads(f.read(n))
    low, high = [math.inf]*3, [-math.inf]*3
    identity = [[int(i == j) for j in range(4)] for i in range(4)]

    def visit(index, parent):
        node = gltf["nodes"][index]
        world = product(parent, transform(node))
        if "mesh" in node:
            for prim in gltf["meshes"][node["mesh"]]["primitives"]:
                a = gltf["accessors"][prim["attributes"]["POSITION"]]
                for corner in itertools.product(*zip(a["min"], a["max"])):
                    point = [sum(world[i][j] * (*corner, 1)[j] for j in range(4)) for i in range(3)]
                    for i in range(3):
                        low[i] = min(low[i], point[i]); high[i] = max(high[i], point[i])
        for child in node.get("children", []):
            visit(child, world)

    for node in gltf["scenes"][gltf.get("scene", 0)]["nodes"]:
        visit(node, identity)
    assert all(math.isfinite(x) for x in low+high)
    return {"minimum": [round(v, 4) for v in low], "maximum": [round(v, 4) for v in high],
            "coordinate_frame": "right_handed_y_up", "basis": "current detailed GLB static world AABB; declared demonstration geometry"}


def design():
    accepted = read(ACCEPTED)["per_device"]
    catalog = {(x["id"], x["version"]): x for x in read(CATALOG)["register_order"]}
    target = {x["id"]: x for x in accepted}
    ids = [x for _, _, words in GROUPS for x in words.split()]
    assert len(ids) == len(set(ids)) == 92 and set(ids) == set(target)
    zones, devices = [], []
    for index, (title, color, words) in enumerate(GROUPS):
        zone_id = f"Z{index+1:02}"
        zx, zy = 4 + (index % 4)*25, 18 + (index // 4)*24
        zone = {"id": zone_id, "title": title, "color": color, "x_m": zx, "y_m": zy,
                "width_m": 21, "depth_m": 20, "device_ids": []}
        cx, cy, row_depth = 1.8, 3.2, 0.0
        for ordinal, name in enumerate(words.split()):
            a = target[name]; entry = catalog[(a["module_ref"]["id"], a["module_ref"]["version"])]
            module = read(ROOT/entry["directory"]/"module.json")["data"]["definition"]
            media_ref = module["payload"]["data"]["presentation"]["source"]
            assert media_ref["kind"] == "media_ref"
            media_id = media_ref["data"]["id"]
            media_entry = catalog[(media_id, media_ref["data"]["version"])]
            media = read(ROOT/media_entry["directory"]/"module.json")["data"]["definition"]["payload"]["data"]
            assert media["source"]["kind"] == "asset"
            bounds = asset_bounds(ROOT/media_entry["directory"]/media["source"]["data"]["path"])
            lo, hi = bounds["minimum"], bounds["maximum"]
            width, depth, height = hi[0]-lo[0], hi[2]-lo[2], hi[1]-lo[1]
            assert width < 26 and depth < 18 and height < 8, (name, width, depth, height)
            # 1.8m working gap is a declared demo choice, not a safety certificate.
            if cx + width > 19.2:
                cx = 1.8; cy += row_depth + 1.8; row_depth = 0
            assert cy + depth < zone["depth_m"]-1.2, (zone_id, name, cy, depth)
            # GLB z maps to factory -y, so the lower world-y edge is -max_z.
            px, py = zx + cx - lo[0], zy + cy + hi[2]
            device_id = f"PF-{name}"
            devices.append({"id": device_id, "equipment_id": name, "title": module["title"],
                "zone_id": zone_id, "line_id": f"PF-L{index+1:02}", "module_ref": a["module_ref"],
                "presentation_ref": media_ref["data"],
                "source_directory": entry["directory"], "presentation_directory": media_entry["directory"],
                "pose": {"translation": {"x": round(px,4), "y": round(py,4), "z": 0},
                         "rotation": {"x": 0,"y": 0,"z": 0,"w": 1}, "scale": {"x":1,"y":1,"z":1},
                         "unit": "meter", "coordinate_frame": "right_handed_z_up"},
                "layout_aabb_m": {"x": round(zx+cx,4), "y": round(zy+cy,4), "width": round(width,4), "depth": round(depth,4)},
                "height_m": round(height,4), "asset_bounds": bounds,
                "ports": [{"id": p["id"], "direction": p["direction"]} for p in module["payload"]["data"]["ports"]],
                "behavior_builtin": module["payload"]["data"]["behavior"]["inline"]["builtin"],
                "design_role": title, "route_binding_state": "to_be_bound_from_actual_device_inputs_outputs"})
            zone["device_ids"].append(device_id)
            cx += width + 1.8; row_depth = max(row_depth, depth)
        zones.append(zone)
    # Every rectangle is derived from actual current media bytes. Check only the
    # newly designed layout, not the already accepted modeling pipeline.
    for a, b in itertools.combinations(devices, 2):
        aa, bb = a["layout_aabb_m"], b["layout_aabb_m"]
        overlap = min(aa["x"]+aa["width"],bb["x"]+bb["width"])-max(aa["x"],bb["x"]) > .001 and min(aa["y"]+aa["depth"],bb["y"]+bb["depth"])-max(aa["y"],bb["y"]) > .001
        assert not overlap, (a["id"], b["id"])
    return {"kind": "superfactory_internal_factory_design", "revision": 1,
        "package": "V2-05-G01.layout", "state": "layout_design_not_running_project",
        "title": "纸包装多产品演示工厂", "hall": {"id":"PF-H1","width_m":104,"depth_m":90,"height_m":9},
        "dimension_basis":"Agent-declared demonstration site and working gaps; not manufacturer capacity or certified building design",
        "coordinate_frame":"right_handed_z_up", "front_direction":"negative_y", "views":["top","front_negative_y","right_positive_x"],
        "zones":zones,"devices":devices,
        "front_band":[{"id":"INBOUND","title":"原料收货与待投料","x_m":4,"y_m":4,"width_m":27,"depth_m":10},
                      {"id":"CONTROL","title":"生产观察与订单入口","x_m":35,"y_m":4,"width_m":30,"depth_m":10},
                      {"id":"OUTBOUND","title":"合格成品与发运","x_m":69,"y_m":4,"width_m":31,"depth_m":10}],
        "aisle_width_m":4,"cross_aisles_y_m":[14,38,62,86],"longitudinal_aisles_x_m":[0,25,50,75,100],
        "functional_flows":[
            {"id":"CARTON","title":"纸箱主线","zones":["Z01","Z02","Z03","Z04","Z10"],"destination":"OUTBOUND"},
            {"id":"RIGID","title":"精品盒/纸品支线","zones":["Z01","Z02","Z05","Z10"],"destination":"OUTBOUND"},
            {"id":"BAG","title":"纸袋支线","zones":["Z01","Z02","Z06","Z10"],"destination":"OUTBOUND"},
            {"id":"FOOD","title":"杯碗盘支线","zones":["Z01","Z02","Z07","Z10"],"destination":"OUTBOUND"},
            {"id":"TUBE","title":"纸管/防护材料支线","zones":["Z01","Z08","Z10"],"destination":"OUTBOUND"},
            {"id":"LABEL","title":"标签支线","zones":["Z09","Z10"],"destination":"OUTBOUND"}],
        "shared_services":{"handling_zone":"Z11","recycling_zone":"Z12","quality_and_isolation_zone":"Z10"},
        "acceptance_boundary":"92 exact device assignments and static geometric spacing checked; route material compatibility, real Run, cloud phases and save/reopen remain implementation tasks"}


class Drawing:
    def __init__(self):
        self.file = ET.Element("mxfile", {"host":"app.diagrams.net","type":"device"})
        self.root = None
        self.counter = 0

    def page(self, title, width=2200, height=1800):
        page = ET.SubElement(self.file,"diagram",{"id":str(len(self.file)),"name":title})
        model = ET.SubElement(page,"mxGraphModel",{"grid":"1","gridSize":"10","page":"1","pageScale":"1","pageWidth":str(width),"pageHeight":str(height),"math":"0","shadow":"0"})
        self.root=ET.SubElement(model,"root");ET.SubElement(self.root,"mxCell",{"id":"0"});ET.SubElement(self.root,"mxCell",{"id":"1","parent":"0"})

    def box(self, text, x,y,w,h,fill="#ffffff",font=15,style="",key=None):
        self.counter+=1;identity=key or "s"+str(self.counter)
        c=ET.SubElement(self.root,"mxCell",{"id":identity,"value":text,"style":f"rounded=0;whiteSpace=wrap;html=0;fillColor={fill};strokeColor=#899baa;fontFamily=Microsoft YaHei;fontSize={font};fontColor=#183248;spacing=5;"+style,"vertex":"1","parent":"1"})
        ET.SubElement(c,"mxGeometry",{"x":str(x),"y":str(y),"width":str(w),"height":str(h),"as":"geometry"})
        return identity

    def edge(self, source,target,label="",color="#247b8d"):
        self.counter+=1
        c=ET.SubElement(self.root,"mxCell",{"id":"e"+str(self.counter),"value":label,"style":f"edgeStyle=orthogonalEdgeStyle;rounded=0;html=0;strokeColor={color};strokeWidth=2;endArrow=block;fontSize=14;labelBackgroundColor=#ffffff;","edge":"1","parent":"1","source":source,"target":target})
        ET.SubElement(c,"mxGeometry",{"relative":"1","as":"geometry"})

    def save(self,path):
        path.parent.mkdir(parents=True,exist_ok=True)
        ET.indent(self.file,space="  ")
        ET.ElementTree(self.file).write(path,encoding="utf-8",xml_declaration=True)


def draw(d):
    dr=Drawing(); scale=12; ox,oy=130,160
    def px(x):return ox+x*scale
    def py(y):return oy+(90-y)*scale
    dr.page("01 区域与设备分配 · 92设备",2020,1780)
    dr.box("纸包装多产品演示工厂｜区域与设备分配",130,24,1248,48,"#ffffff",27,"strokeColor=none;fontStyle=1;")
    dr.box("104 × 90 × 9 m · 自建演示场地 · 92种设备 · 12功能区 · 坐标X向右/Y向上/Z为高度",130,82,1248,38,"#ffffff",17,"strokeColor=none;")
    dr.box("",px(0),py(90),104*scale,90*scale,"#f3f5f6",style="strokeWidth=2;")
    for z in d["zones"]:
        dr.box(f'{z["id"]}  {z["title"]} · {len(z["device_ids"])}',px(z["x_m"]),py(z["y_m"]+z["depth_m"]),21*scale,20*scale,z["color"],14,"verticalAlign=top;align=left;fontStyle=1;",z["id"])
    for dev in d["devices"]:
        a=dev["layout_aabb_m"]; number=d["zones"][int(dev["zone_id"][1:])-1]["device_ids"].index(dev["id"])+1
        dr.box(f'{number:02}',px(a["x"]),py(a["y"]+a["depth"]),a["width"]*scale,a["depth"]*scale,"#ffffff",9,"strokeColor=#3c647d;spacing=1;",dev["id"])
    for band in d["front_band"]:
        dr.box(band["title"],px(band["x_m"]),py(band["y_m"]+band["depth_m"]),band["width_m"]*scale,band["depth_m"]*scale,"#e6edf4",19,key=band["id"])
    dr.box("前侧 / Y=0   西南收货门                         订单入口                         东南发运门",130,1270,1248,48,"#ffffff",17,"strokeColor=none;")
    dr.box("设备外包络取当前GLB，区域内编号见分区清单。4m通道、1.8m设备间距为自建演示值。\n设计分配图 → AI俯/正/右三视图 → 同布局真实模板 → 整厂生产与运输联验。",130,1340,1248,75,"#ffffff",17,"strokeColor=none;align=left;")
    for batch in range(3):
        dr.page(f"0{batch+2} 设备清单 · Z{batch*4+1:02}—Z{batch*4+4:02}",2020,1600)
        dr.box("设备、区域与精确版本｜构建时使用相同设备ID",70,25,1870,55,"#ffffff",26,"strokeColor=none;fontStyle=1;")
        for k,z in enumerate(d["zones"][batch*4:batch*4+4]):
            x=70+(k%2)*950;y=110+(k//2)*700
            dr.box(f'{z["id"]} {z["title"]} · {len(z["device_ids"])}台',x,y,910,50,z["color"],22,"fontStyle=1;")
            for j,identity in enumerate(z["device_ids"]):
                dev=next(a for a in d["devices"] if a["id"]==identity)
                short=dev["title"].split("（")[0];a=dev["layout_aabb_m"]
                label=f'{z["id"]}-{j+1:02}  {short}  @{dev["module_ref"]["version"]}\n{dev["id"]}  |  {a["width"]:.1f} × {a["depth"]:.1f} m'
                dr.box(label,x,y+55+j*49,910,46,"#ffffff",14,"align=left;")
    dr.page("05 产品路线与物流分配",2020,1480)
    dr.box("多产品路线与共享物流｜功能分配设计",70,30,1850,55,"#ffffff",27,"strokeColor=none;fontStyle=1;")
    for i,flow in enumerate(d["functional_flows"]):
        y=160+i*155;prev=dr.box(flow["title"],70,y,210,65,"#edf3f8",19)
        for j,zone_id in enumerate(flow["zones"]):
            z=next(z for z in d["zones"] if z["id"]==zone_id)
            cell=dr.box(f'{zone_id}\n{z["title"]}',335+j*265,y,225,65,z["color"],17);dr.edge(prev,cell);prev=cell
        end=dr.box("合格成品 / 发运",1710,y,230,65,"#dcebd9",17);dr.edge(prev,end)
    dr.box("Z11 共享搬运、输送与码垛服务各路线；Z12 负责边料回收；不合格品在Z10隔离。\n箭头是功能分工，不等同已经配置的实际工艺或运输任务。构建阶段须核对每条路线的物料规格、辅助材料、端口及接受口径。",70,1150,1870,105,"#ffffff",18,"align=left;")
    path=ART/"纸包装整厂区域与设备分配.drawio";dr.save(path)
    if (DESIGN/"bag-line-spec.json").exists():
        from prepare_bag_factory_design import attach_diagram
        attach_diagram(path,read(DESIGN/"bag-line-spec.json"))
    if (DESIGN/"preparation-line-spec.json").exists():
        from prepare_preparation_factory_design import attach_diagram as attach_preparation
        attach_preparation(path,read(DESIGN/"preparation-line-spec.json"))
    if (DESIGN/"logistics-line-spec.json").exists():
        from prepare_logistics_factory_design import attach_diagram as attach_logistics
        attach_logistics(path,read(DESIGN/"logistics-line-spec.json"))
    if (DESIGN/"pallet-line-spec.json").exists():
        from prepare_pallet_factory_design import attach_diagram as attach_pallet
        attach_pallet(path,read(DESIGN/"pallet-line-spec.json"))
    return path


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.parse_args()
    d=design();write(DESIGN/"layout.json",d);diagram=draw(d)
    DESIGN.mkdir(parents=True,exist_ok=True)
    with (DESIGN/"equipment-allocation.csv").open("w",encoding="utf-8-sig",newline="") as f:
        w=csv.writer(f);w.writerow(["设备ID","区域","设备名称","模块ID","精确版本","X_m","Y_m","包络宽_m","包络深_m","高度_m","工艺绑定状态"])
        for x in d["devices"]:
            t=x["pose"]["translation"];a=x["layout_aabb_m"]
            w.writerow([x["id"],x["zone_id"],x["title"],x["module_ref"]["id"],x["module_ref"]["version"],t["x"],t["y"],a["width"],a["depth"],x["height_m"],x["route_binding_state"]])
    print(json.dumps({"devices":len(d["devices"]),"zones":len(d["zones"]),"drawing":str(diagram),"layout":str(DESIGN/"layout.json"),"overlap_pairs":0,"running_project_created":False},ensure_ascii=False))


if __name__=="__main__":main()
