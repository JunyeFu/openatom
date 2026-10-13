"""Summarize the actual renderer captures and the exported GLB geometry."""
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUTPUT = ROOT / "output"
CASES = [("device", "high"), ("line", "high"), ("load", "high"), ("load", "low")]


def inspect_glb(path):
    raw = path.read_bytes()
    magic, version, total = struct.unpack_from("<4sII", raw)
    assert magic == b"glTF" and version == 2 and total == len(raw), path
    chunk_length, chunk_type = struct.unpack_from("<I4s", raw, 12)
    assert chunk_type == b"JSON", path
    gltf = json.loads(raw[20:20 + chunk_length])
    triangles_per_mesh = []
    for mesh in gltf.get("meshes", []):
        count = 0
        for primitive in mesh["primitives"]:
            assert primitive.get("mode", 4) == 4, path
            accessor = primitive.get("indices", primitive["attributes"]["POSITION"])
            count += gltf["accessors"][accessor]["count"] // 3
        triangles_per_mesh.append(count)
    mesh_nodes = [node for node in gltf["nodes"] if "mesh" in node]
    return {"file": path.name, "bytes": len(raw), "mesh_nodes": len(mesh_nodes), "triangles_across_nodes": sum(triangles_per_mesh[node["mesh"]] for node in mesh_nodes), "materials": len(gltf.get("materials", [])), "embedded_images": len(gltf.get("images", [])), "named_motion_nodes": [node.get("name") for node in gltf["nodes"] if node.get("extras", {}).get("motion")]}


def main():
    assets = [inspect_glb(path) for path in sorted(OUTPUT.glob("*.glb"))]
    cases = []
    for scene, quality in CASES:
        candidates = sorted(OUTPUT.glob(f"{scene}-assembly-{quality}-*.json"), key=lambda path: path.stat().st_mtime, reverse=True)
        candidates = [path for path in candidates if json.loads(path.read_text(encoding="utf-8")).get("material_profile")]
        if not candidates:
            continue
        path = candidates[0]
        data = json.loads(path.read_text(encoding="utf-8"))
        matching = [run for run in data["runs"] if run["scene"] == data["scene"] and run["quality"] == data["quality"]]
        if matching:
            geometry_matches = all(data["assets"][asset["file"].removesuffix(".glb")]["triangles"] == asset["triangles_across_nodes"] and data["assets"][asset["file"].removesuffix(".glb")]["editable_mesh_nodes"] == asset["mesh_nodes"] for asset in assets)
            cases.append({"file": path.name, "geometry_matches_current_glb": geometry_matches, "material_profile": data.get("material_profile"), "render_buffer": data["render_buffer"], "renderer": data["renderer"], "geometry_bytes": data["model_geometry_bytes"], "texture_estimate_bytes": data["model_texture_estimate_bytes"], "render_target_minimum_bytes": data["render_target_minimum_bytes"], "startup_ms": data["startup_ms"], "run": matching[-1]})
    result = {"scope": "standalone_3d_proof_not_full_product_acceptance", "cases": cases, "glb_assets": assets, "texture_arithmetic": {"one_1024_RGBA_with_mips_MiB": 1024 * 1024 * 4 * 4 / 3 / 1048576, "one_2048_RGBA_with_mips_MiB": 2048 * 2048 * 4 * 4 / 3 / 1048576, "twelve_unique_assets_each_3x1k_maps_MiB": 12 * 3 * 1024 * 1024 * 4 * 4 / 3 / 1048576, "ten_assets_3x1k_plus_two_assets_3x2k_maps_MiB": (10 * 3 * 1024 * 1024 * 4 * 4 / 3 + 2 * 3 * 2048 * 2048 * 4 * 4 / 3) / 1048576}}
    (OUTPUT / "measurements-summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
