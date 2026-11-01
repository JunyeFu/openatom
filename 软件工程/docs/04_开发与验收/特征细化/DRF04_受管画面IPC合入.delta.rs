//! Adopted fragment reference: window1 merged/compiled/derived the canonical carrier.
//! This reference is not the actual Host/Renderer implementation.
//! No new business tool or client file path/grant parameter. Uses existing model!.

model!(IpcManagedAssetAttachment {
    task_id: TaskId,
    asset_id: AssetId,
    scope: EditScope,
    scope_revision: Revision,
    mime_type: String,
    byte_length: SafeUint,
    // Windows handle already duplicated into the authenticated bridge process.
    // Host-to-bridge output only; never a path or client-supplied read capability.
    handle_value: SafeUint,
});

// Proposed optional field on IpcResponseFrame<R>:
// pub managed_assets: Option<Vec<IpcManagedAssetAttachment>>,
// Existing non-image replies use None. Actual attachment availability belongs in
// actual implemented capability/limitations; no claim of support from this delta.
// Keep API version and all 18 business operations unchanged. W00 owns IPC source,
// builder and generated fixture/schema updates before native contract admission.

// Host creates an attachment only after get_task resolves a completed RenderOutput
// under the real session/scope. Open the managed image from Host's own result map;
// DuplicateHandle with read-only access to the OS-verified bridge PID, non-inherited.
// Match task/asset/expected requested scope revision and actual scene revision.
// Bridge accepts attachment handles only from the authenticated matching Host frame,
// reads exactly the declared bytes, returns MCP image content plus canonical typed
// Response in structuredContent, then closes the owned handle. No FileRef.path open.
// Captured old revision remains valid for its original task; never overlays a newer
// editor revision. Disconnect/parse/consume failure closes authenticated owned handles.
