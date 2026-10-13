"""Check the repaired geometry in the exported files, independently of Three.js."""
import json
import math
import struct
from pathlib import Path

OUTPUT = Path(__file__).resolve().parent / "output"


class Model:
    def __init__(self, name):
        self.path = OUTPUT / f"{name}.glb"
        raw = self.path.read_bytes()
        assert struct.unpack_from("<4sII", raw) == (b"glTF", 2, len(raw))
        json_length, kind = struct.unpack_from("<I4s", raw, 12)
        assert kind == b"JSON"
        self.g = json.loads(raw[20:20 + json_length])
        bin_length, kind = struct.unpack_from("<I4s", raw, 20 + json_length)
        assert kind == b"BIN\x00"
        self.binary = raw[28 + json_length:28 + json_length + bin_length]
        self.parents = {child: i for i, node in enumerate(self.g["nodes"]) for child in node.get("children", [])}
        self.bounds = {}
        for i, node in enumerate(self.g["nodes"]):
            if "mesh" not in node:
                continue
            vertices = []
            for primitive in self.g["meshes"][node["mesh"]]["primitives"]:
                positions = self.accessor(primitive["attributes"]["POSITION"])
                normals = self.accessor(primitive["attributes"]["NORMAL"])
                assert all(math.isfinite(v) for point in positions + normals for v in point), (name, node.get("name"))
                vertices.extend(self.world(i, point) for point in positions)
            self.bounds[i] = [[min(p[a] for p in vertices) for a in range(3)], [max(p[a] for p in vertices) for a in range(3)]]

    def accessor(self, index):
        accessor = self.g["accessors"][index]
        view = self.g["bufferViews"][accessor["bufferView"]]
        assert accessor["componentType"] == 5126 and accessor["type"] == "VEC3"
        offset = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        stride = view.get("byteStride", 12)
        return [struct.unpack_from("<3f", self.binary, offset + i * stride) for i in range(accessor["count"])]

    def world(self, index, point):
        while True:
            node = self.g["nodes"][index]
            if "matrix" in node:
                m = node["matrix"]
                point = [sum(m[c * 4 + r] * point[c] for c in range(3)) + m[12 + r] for r in range(3)]
            else:
                point = [p * s for p, s in zip(point, node.get("scale", [1, 1, 1]))]
                x, y, z, w = node.get("rotation", [0, 0, 0, 1])
                px, py, pz = point
                tx, ty, tz = 2 * (y * pz - z * py), 2 * (z * px - x * pz), 2 * (x * py - y * px)
                point = [px + w * tx + y * tz - z * ty, py + w * ty + z * tx - x * tz, pz + w * tz + x * ty - y * tx]
                point = [p + t for p, t in zip(point, node.get("translation", [0, 0, 0]))]
            if index not in self.parents:
                return point
            index = self.parents[index]

    def nodes(self, name):
        return [i for i, node in enumerate(self.g["nodes"]) if node.get("name") == name]

    def center(self, index):
        return [(a + b) / 2 for a, b in zip(*self.bounds[index])]


def close(a, b, message, tolerance=1e-6):
    assert abs(a - b) <= tolerance, (message, a, b)


def main():
    models = {name: Model(name) for name in ["assembly", "robot", "conveyor", "inspection", "sealer", "pallet"]}
    assets = []
    for name, model in models.items():
        minimum_y = min(bounds[0][1] for bounds in model.bounds.values())
        close(minimum_y, 0, f"{name}: ground contact")
        image_count = len(model.g.get("images", []))
        assert image_count == (0 if name == "pallet" else 3), (name, "shared ORM not duplicated", image_count)
        assets.append({"file": model.path.name, "bytes": model.path.stat().st_size, "mesh_nodes": len(model.bounds), "minimum_y": minimum_y, "images": image_count})

    assembly = models["assembly"]
    bit = assembly.nodes("screwdriver-bit")[0]
    driver_center = assembly.center(bit)
    target = min(assembly.nodes("hex-socket"), key=lambda i: sum((assembly.center(i)[a] - driver_center[a]) ** 2 for a in [0, 2]))
    for a in [0, 2]:
        close(assembly.center(target)[a], driver_center[a], "driver aligned to screw")
    slide = assembly.nodes("z-slide-and-driver")[0]
    travel = assembly.g["nodes"][slide]["extras"]["travel"]
    clearance = assembly.bounds[bit][0][1] - travel - assembly.bounds[target][1][1]
    assert 0 <= clearance < 0.002, ("full stroke clearance", clearance)
    assert all(assembly.parents[i] == slide for i in assembly.nodes("linear-bearing-block"))
    board = assembly.bounds[assembly.nodes("pcb-board")[0]]
    housing = assembly.bounds[assembly.nodes("housing-base")[0]]
    for i in assembly.nodes("pcb-standoff"):
        close(assembly.bounds[i][0][1], housing[1][1], "PCB standoff contacts housing")
        close(assembly.bounds[i][1][1], board[0][1], "PCB standoff contacts board")
    for i in assembly.nodes("pcb-washer"):
        close(assembly.bounds[i][0][1], board[1][1], "washer contacts PCB")
    for i in assembly.nodes("linear-bearing-block"):
        close(assembly.bounds[i][1][2], assembly.bounds[assembly.nodes("moving-carriage")[0]][0][2], "bearing contacts carriage")
    for kind in ["assembly", "inspection", "sealer"]:
        model = models[kind]
        close(model.bounds[model.nodes("lcd-panel")[0]][0][2], model.bounds[model.nodes("panel-housing")[0]][1][2], "LCD contacts panel")
    for name in ["signal-tower-bracket", "drag-chain-fixed-anchor", "drag-chain-return-anchor", "panel-support-post", "transfer-leg"]:
        assert assembly.nodes(name), name

    robot = models["robot"]
    assert robot.nodes("axis1-shoulder-housing") and not robot.nodes("axis1-housing")
    for axis in [2, 3, 5]:
        caps, seals = robot.nodes(f"axis{axis}-cap"), robot.nodes(f"axis{axis}-seal")
        assert len(caps) == len(seals) == 2
        for cap in caps:
            sign = 1 if robot.center(cap)[2] > 0 else -1
            seal = next(i for i in seals if robot.center(i)[2] * sign > 0)
            close(robot.bounds[cap][0 if sign > 0 else 1][2], robot.bounds[seal][1 if sign > 0 else 0][2], "cap touches seal")
    for model in [robot, models["conveyor"]]:
        for head in model.nodes("bolt-head"):
            head_center = model.center(head)
            if head_center[2] >= -0.1:
                continue
            if model is models["conveyor"] and head_center[2] > -0.334:
                continue  # Frame brackets face +Z; only the rear rail faces -Z.
            socket = min(model.nodes("hex-socket"), key=lambda i: sum((model.center(i)[a] - head_center[a]) ** 2 for a in range(3)))
            if abs(model.center(socket)[2] - head_center[2]) > 0.001:
                assert model.center(socket)[2] < head_center[2], "back socket faces outward"
    assert len([n for n in robot.g["nodes"] if n.get("extras", {}).get("motion") == "gripper"]) == 2
    cable = robot.nodes("robot-dress-pack")[0]
    primitive = robot.g["meshes"][robot.g["nodes"][cable]["mesh"]]["primitives"][0]
    vertices = [robot.world(cable, p) for p in robot.accessor(primitive["attributes"]["POSITION"])]
    centers = [[sum(vertices[start + j][a] for j in range(8)) / 8 for a in range(3)] for start in range(0, len(vertices), 9)]
    for clamp in robot.nodes("dress-pack-clamp"):
        center = robot.center(clamp)
        distances = []
        for start, end in zip(centers, centers[1:]):
            delta = [b - a for a, b in zip(start, end)]
            t = max(0, min(1, sum((c - a) * d for c, a, d in zip(center, start, delta)) / sum(d * d for d in delta)))
            distances.append(math.dist(center, [a + t * d for a, d in zip(start, delta)]))
        assert min(distances) < 0.002, ("clamp centered on cable", min(distances))

    pallet = models["pallet"]
    runners_top = max(pallet.bounds[i][1][1] for i in pallet.nodes("pallet-runner"))
    close(min(pallet.bounds[i][0][1] for i in pallet.nodes("pallet-deck")), runners_top, "deck touches runners")
    deck_top = max(pallet.bounds[i][1][1] for i in pallet.nodes("pallet-deck"))
    cartons = sorted((pallet.bounds[i][0][1], pallet.bounds[i][1][1]) for i in pallet.nodes("finished-carton"))
    close(cartons[0][0], deck_top, "cartons touch deck")
    close(cartons[4][0], cartons[0][1], "second layer touches first")
    close(cartons[8][0], cartons[4][1], "third layer touches second")
    result = {"scope": "exported_model_defect_regression_not_production_simulation", "passed": True, "assets": assets, "assembly_full_stroke_clearance_mm": clearance * 1000, "checks": ["finite geometry and normals", "six assets contact ground", "single shared ORM image", "driver targets screw without penetration", "bearing blocks follow and contact slide", "PCB supports and LCD contact their mounts", "continuous shoulder housing", "both joint caps seat on seals", "back bolt sockets face outward", "two independent gripper slides", "clamps centered on cable", "pallet and all carton layers contact"]}
    (OUTPUT / "model-repair-checks.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
