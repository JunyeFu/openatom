"""Read-only source/lock inventory; no build, app launch, listener or screenshot."""
from pathlib import Path
import datetime
import json
import os
import re
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
def read(path):
    return (ROOT / path).read_text(encoding="utf-8-sig")

models = read("packages/domain/src/models.rs")
api = read("packages/commands/src/api.rs")
host = read("packages/commands/src/project_host.rs")
native = read("apps/planner/src-tauri/src/project_api.rs")
pipe = read("apps/planner/src-tauri/src/agent_pipe.rs")
decode = read("apps/mcp-bridge/src/decode.rs")
lock = tomllib.loads(read("Cargo.lock"))
versions = {x["name"]: x["version"] for x in lock["package"]
            if x["name"] in {"tauri", "wry", "webview2-com", "windows", "windows-core"}}
enum = re.search(r"pub enum RenderViewKind \{([^}]+)\}", models).group(1)
snake = lambda v: re.sub(r"(?<!^)(?=[A-Z])", "_", v).lower()
kinds = [snake(x) for x in re.findall(r"^\s*(\w+),", enum, re.M)]
matrix = json.loads((HERE / "view-matrix.json").read_text(encoding="utf-8"))
assert kinds == [x["kind"] for x in matrix["views"]]
assert len(kinds) == 9
registry = next((Path(os.environ["USERPROFILE"]) / ".cargo/registry/src").glob("index.*"))
local = {
    "controller": (registry / "tauri-2.12.3/src/webview/mod.rs").read_text(encoding="utf-8"),
    "capture": (registry / "webview2-com-sys-0.39.1/src/bindings.rs").read_text(encoding="utf-8"),
    "callback": (registry / "webview2-com-0.39.1/src/callback.rs").read_text(encoding="utf-8"),
}
native_cargo = tomllib.loads(read("apps/planner/src-tauri/Cargo.toml"))["dependencies"]
render_params = re.search(r"model!\(RenderViewParams \{([^}]+)\}\);", api).group(1)
checks = {
    "canonical_nine_kinds_match_matrix": len(kinds) == 9,
    "RenderDelivery_has_both_modes": "CaptureOnly" in api and "ShowAndCapture" in api,
    "RenderOutput_typed_scope_and_visible_refs": all(x in api for x in ["requested_scope: EditScope", "requested_scope_revision: Revision", "visible_objects: Vec<ObjectRef>"]),
    "native_one_Host_new_in_actual_constructor": native[native.index("pub fn new() -> Self"):native.index("pub(super) fn dispatch_ui")].count("ProjectHost::new(") == 1,
    "worker_runs_outside_Host_lock": native.index("let completion = work.run();") < native.index("host.finish_io(completion);"),
    "current_pipe_has_no_render_asset_publication": '"managed_assets":null' in pipe,
    "bridge_has_actual_render_attachment_validation": all(x in decode for x in ["render_view", "rendering", "requested_scope_revision", "image/png", "image/jpeg"]),
    "current_Host_has_no_RenderView_dispatch_arm": not re.search(r"call!\(r,\s*OperationName::RenderView", host),
    "current_Native_has_no_CapturePreview_call": not any(".CapturePreview(" in p.read_text(encoding="utf-8-sig") for p in (ROOT / "apps/planner/src-tauri/src").glob("*.rs")),
    "Tauri_controller_type_is_real_WebView2": "pub fn controller(" in local["controller"] and "ICoreWebView2Controller" in local["controller"],
    "locked_binding_has_actual_CapturePreview": "pub unsafe fn CapturePreview<P1, P2>" in local["capture"],
    "locked_callback_available": "pub struct CapturePreviewCompletedHandler" in local["callback"],
    "public_render_params_lack_exact_ModuleRef": "ModuleRef" not in render_params,
    "public_render_params_lack_comparison_selection": "CompareRunsParams" not in render_params,
    "FactoryScene_no_requested_camera_prop_yet": "camera: CameraState" not in read("apps/planner/src/scene/FactoryScene.tsx"),
    "timeline_preferences_can_write_localStorage": "localStorage.setItem" in read("apps/planner/src/features/timeline/preferences.ts"),
    "manufacturing_declaration_form_is_not_image_renderer": "<img" not in read("apps/planner/src/features/packaging/ManufacturingExportForm.tsx") and "<svg" not in read("apps/planner/src/features/packaging/ManufacturingExportForm.tsx"),
}
assert all(checks.values()), checks
output = {
    "status": "passed_source_inventory_only_not_implementation",
    "at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "locked_versions": versions,
    "Native_direct_dependencies": {x: x in native_cargo for x in ["tauri", "webview2-com", "windows", "windows-sys"]},
    "RenderViewParams_current_fields": render_params.strip(),
    "views": kinds,
    "checks": checks,
    "actual_Cargo_Native_GPU_capture_or_MCP_image_run": False,
    "production_modified_by_this_script": False,
}
(HERE / "source-inventory.json").write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"status": output["status"], "checks": len(checks), "views": len(kinds)}, ensure_ascii=False))
