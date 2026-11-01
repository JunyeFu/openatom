"""Actual stdio process consumer; launched by --p3-04-mcp-smoke only.

The desktop authorizes actual verified peers through its native trusted entry.
This client never constructs a Host, Engine or grant and makes no model call.
"""
import json
import subprocess
import sys
import time
from pathlib import Path

bridge, endpoint, host, project_id, output = sys.argv[1:]
output = Path(output)
checks = []
admin_index = 0


def check(name, condition):
    assert condition, name
    checks.append(name)


def admin(kind, session, task=None):
    """Startup harness asks actual native main-window commands, never MCP grants."""
    global admin_index
    admin_index += 1
    action = {"kind": kind, "session_id": session}
    if task is not None:
        action["task_id"] = task
    temporary = output / f"admin-request-{admin_index}.tmp"
    temporary.write_text(json.dumps(action), encoding="utf-8")
    temporary.replace(output / f"admin-request-{admin_index}.json")
    response = output / f"admin-response-{admin_index}.json"
    for _ in range(400):
        if response.exists():
            try:
                text = response.read_text(encoding="utf-8")
            except PermissionError:
                # Windows can expose the published name before the writer's
                # rename handle is released. Keep the original wait budget.
                time.sleep(0.005)
                continue
            return json.loads(text)["result"]
        time.sleep(0.005)
    raise AssertionError(f"native main-window action did not finish: {kind}")


def eof_removed(session):
    for _ in range(100):
        result = admin("probe_eof_revoked", session)
        if result["host_denied"] and result["context_null"]:
            return True
        time.sleep(0.005)
    return False


class Client:
    def __init__(self, ordinal):
        self.stderr = (output / f"bridge-{ordinal}.stderr.txt").open("wb")
        self.process = subprocess.Popen(
            [bridge, "--host-pipe", endpoint, "--host-instance-id", host],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        self.next_id = 1
        initialized = self.rpc("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "superfactory-actual-process-proof", "version": "1"},
        })
        check(f"stdio_initialize_{ordinal}", "result" in initialized)
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def send(self, value):
        self.process.stdin.write(json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8") + b"\n")
        self.process.stdin.flush()

    def rpc(self, method, params):
        request_id = self.next_id
        self.next_id += 1
        self.send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        while True:
            line = self.process.stdout.readline()
            assert line, f"bridge disconnected before {method}, exit={self.process.poll()}"
            response = json.loads(line)
            if response.get("id") == request_id:
                return response

    def call(self, operation, params, project=project_id, **extra):
        arguments = {"api_version": "0.1", "request_id": f"actual-{self.next_id}",
                     "project_id": project, "scope": {"kind": "workspace" if project is None else "project"},
                     "operation": operation, "params": params, **extra}
        response = self.rpc("tools/call", {"name": f"superfactory_{operation}", "arguments": arguments})
        if "error" in response:
            return response, None
        result = response["result"]
        original = result["structuredContent"]
        if original["outcome"]["status"] == "managed":
            resources = [c["resource"] for c in result["content"] if c["type"] == "resource"]
            check("actual_managed_json_resource_present", len(resources) == 1 and resources[0]["mimeType"] == "application/json")
            raw = resources[0]["text"]
            check("actual_managed_frozen_resource_URI", resources[0]["uri"] == "superfactory-result://" + original["outcome"]["data"]["result_id"])
            check("actual_managed_exact_byte_length", len(raw.encode("utf-8")) == original["outcome"]["data"]["byte_length"])
            original = json.loads(raw)
            check("actual_managed_full_original_response", original["request_id"] == arguments["request_id"] and original["context"]["host_instance_id"] == host)
        check("mcp_error_flag_matches_original", result["isError"] == (original["outcome"]["status"] == "error"))
        return original, result

    def close(self):
        self.process.stdin.close()
        check("bridge_clean_eof_exit", self.process.wait(timeout=5) == 0)
        self.stderr.close()


def object_read(client, kind="project", object_id=project_id):
    return client.call("get_object", {"object_id": {"kind": kind, "id": object_id}, "source": {"kind": "draft"}, "node_index": None})


def error_code(response):
    return response.get("outcome", {}).get("data", {}).get("code")


wrong = subprocess.run([bridge, "--host-pipe", endpoint, "--host-instance-id", "desktop-host-0"], capture_output=True, creationflags=subprocess.CREATE_NO_WINDOW, timeout=5)
check("endpoint_alone_is_not_host_authority", wrong.returncode == 2 and b"selected desktop Host" in wrong.stderr)
first = Client(1)
catalog = first.rpc("tools/list", {})["result"]["tools"]
check("readonly_catalog_excludes_apply", "superfactory_apply_commands" not in {t["name"] for t in catalog})
original, wrapper = object_read(first)
check("real_large_project_managed", wrapper["structuredContent"]["outcome"]["status"] == "managed" and len(original["outcome"]["data"]["data"]["title"]) == 1_200_000)
project = original["outcome"]["data"]["data"]
settings = {key: project[key] for key in ["title", "time_reference", "generation_context", "sources"]}
settings["title"] = "Edited by actual MCP subprocess"
edit = {"kind": "set_project_settings", "data": settings}
denied, _ = first.call("apply_commands", {"base_revision": 1, "commands": [edit]})
check("real_host_denies_no_edit_grant", error_code(denied) == "PermissionDenied")
forged, _ = first.call("get_context", {"selection": []}, caller={"origin": "interactive_ui"})
check("caller_cannot_be_injected_in_json", forged.get("error", {}).get("code") == -32602)
context, _ = first.call("get_context", {"selection": []})
session1 = context["context"]["session_id"]
wrong_project, _ = first.call("get_object", {"object_id": {"kind": "project", "id": project_id}, "source": {"kind": "draft"}, "node_index": None}, project="ungranted-project")
check("actual_scope_authority_rejects_other_project", error_code(wrong_project) == "PermissionDenied")
first.close()
check("EOF_removes_first_session_from_shared_Host", eof_removed(session1))

second = Client(2)
catalog = second.rpc("tools/list", {})["result"]["tools"]
check("authorized_catalog_includes_apply", "superfactory_apply_commands" in {t["name"] for t in catalog})
capabilities, _ = second.call("describe_capabilities", {}, project=None)
capabilities = capabilities["outcome"]["data"]
library, _ = second.call("query_module_library", {"category_filters": [], "industry": None, "product_family": None, "site_envelope": None, "source_scope": ["project_packages"]})
check("actual_library_response_is_typed", library["outcome"]["status"] == "ok")
context, _ = second.call("get_context", {"selection": []})
session2 = context["context"]["session_id"]
check("reconnect_has_fresh_host_session", session1 != session2)
selection = json.loads((output / "selection.json").read_text(encoding="utf-8"))
check("selected_file_token_owned_by_current_peer_session", selection["session_id"] == session2)
admission, _ = second.call("save_project", {"revision": 1, "selected_directory_ref": selection["selection"]})
check("real_long_save_admitted_before_cancel", admission["outcome"]["status"] == "ok")
cancelled_task_id = admission["outcome"]["data"]["id"]
cancel_race, _ = second.call("cancel_task", {"task_id": cancelled_task_id, "action": "cancel"})
check("actual_business_cancel_reports_acceptance_or_terminal_winner", cancel_race["outcome"]["status"] == "ok" and (cancel_race["outcome"]["data"]["action_accepted"] or cancel_race["outcome"]["data"]["terminal_won"]))
for _ in range(200):
    cancelled_task, _ = second.call("get_task", {"task_id": cancelled_task_id})
    if cancelled_task["outcome"]["data"]["task"]["state"] in {"completed", "failed", "cancelled"}:
        break
    time.sleep(0.005)
check("cancelled_or_completed_is_actual_terminal_task", cancelled_task["outcome"]["data"]["task"]["state"] in {"completed", "cancelled"})
committed, _ = second.call("apply_commands", {"base_revision": 1, "commands": [edit]})
check("same_host_edit_commits_revision_two", committed["outcome"]["status"] == "ok" and committed["outcome"]["data"]["new_revision"] == 2)
stale, _ = second.call("apply_commands", {"base_revision": 1, "commands": [edit]})
check("actual_revision_conflict_does_not_overwrite", error_code(stale) == "RevisionConflict")
new_item = {"kind": "put_object", "data": {"kind": "item_type", "data": {"id": "mcp-created-item", "revision": "r1", "title": "MCP native item", "unit": "piece", "tracking": "individual", "spec_ref": None, "attributes": {}, "attribute_schema": []}}}
created, _ = second.call("apply_commands", {"base_revision": 2, "commands": [new_item]})
check("actual_authorized_native_object_creation", created["outcome"]["status"] == "ok" and created["outcome"]["data"]["new_revision"] == 3)
item, _ = object_read(second, "item_type", "mcp-created-item")
check("actual_created_object_readback", item["outcome"]["data"]["data"]["title"] == "MCP native item")
bad = json.loads(json.dumps(edit))
bad["data"]["time_reference"]["timezone"] = "UTC"
atomic, _ = second.call("apply_commands", {"base_revision": 3, "commands": [edit, bad]})
check("whole_batch_denied_for_ungranted_field", error_code(atomic) == "PermissionDenied")
new_item["data"]["data"]["id"] = "mcp-budget-exhausted"
budget, _ = second.call("apply_commands", {"base_revision": 3, "commands": [new_item]})
check("actual_cumulative_create_budget", error_code(budget) == "PermissionDenied")
readback, _ = object_read(second)
check("denied_batches_leave_original_revision_and_timezone", readback["outcome"]["data"]["data"]["revision"] == 3 and readback["outcome"]["data"]["data"]["time_reference"]["timezone"] == "Asia/Shanghai")
scenario = {"id": "mcp-empty-scenario", "revision": 1, "project_revision": 3, "title": "Actual IPC empty Scenario fixture", "time_reference": project["time_reference"], "window": {"start_ms": 0, "end_ms": 1000}, "quality_presets": [], "initial_conditions": {"items": [], "initial_setup_families": {}, "declared_quality_decisions": [], "material_allocations": []}, "orders": [], "production_tasks": [], "arrivals": [], "supplies": [], "parameter_overrides": [], "scope": [], "issues": [], "cost_inputs": []}
scenario_created, _ = second.call("apply_commands", {"base_revision": 3, "commands": [{"kind": "put_object", "data": {"kind": "scenario", "data": scenario}}]})
check("actual_scenario_fixture_created: " + json.dumps(scenario_created)[:600], scenario_created["outcome"]["status"] == "ok")
run_admission, _ = second.call("run_scenario", {"scenario_id": scenario["id"], "input_revision": 4, "observation_window": scenario["window"], "intent": {"kind": "project_scenario"}})
check("actual_engine_task_through_mcp: " + json.dumps(run_admission)[:600], run_admission["outcome"]["status"] == "ok")
run_task_id = run_admission["outcome"]["data"]["id"]
for _ in range(200):
    run_task, _ = second.call("get_task", {"task_id": run_task_id})
    if run_task["outcome"]["data"]["task"]["state"] in {"completed", "failed", "cancelled"}:
        break
    time.sleep(0.005)
check("actual_empty_scenario_engine_completed: " + json.dumps(run_task)[:600], run_task["outcome"]["data"]["task"]["state"] == "completed")
run_id = run_task["outcome"]["data"]["result"]["data"]["run_id"]
summary, _ = second.call("get_run", {"run_id": run_id, "query": {"kind": "summary"}})
full, _ = second.call("get_run", {"run_id": run_id, "query": {"kind": "full_result"}})
check("real_run_summary_and_full_keep_same_frozen_input", summary["outcome"]["data"]["data"]["input"] == full["outcome"]["data"]["data"]["input_ref"] and full["outcome"]["data"]["data"]["id"] == run_id)
events, _ = second.call("get_run", {"run_id": run_id, "query": {"kind": "events", "data": {"interval": None, "page": {"after_index": None, "limit": 100}}}})
objects, _ = second.call("get_run", {"run_id": run_id, "query": {"kind": "objects_at", "data": {"at_ms": 0, "objects": [], "page": {"after_index": None, "limit": 100}}}})
check("actual_timeline_queries_keep_run_and_requested_time", events["outcome"]["data"]["data"]["run_id"] == run_id and objects["outcome"]["data"]["data"]["run_id"] == run_id and objects["outcome"]["data"]["data"]["at_ms"] == 0)
saved, _ = second.call("save_project", {"revision": 4, "selected_directory_ref": selection["selection"]})
check("save_returns_actual_host_task_admission", saved["outcome"]["status"] == "ok")
task_id = saved["outcome"]["data"]["id"]
for _ in range(200):
    task, _ = second.call("get_task", {"task_id": task_id})
    if task["outcome"]["data"]["task"]["state"] in {"completed", "failed", "cancelled"}:
        break
    time.sleep(0.005)
check("actual_save_task_completion", task["outcome"]["data"]["task"]["state"] == "completed")
cancel, _ = second.call("cancel_task", {"task_id": task_id, "action": "cancel"})
check("business_cancel_preserves_actual_terminal_winner", cancel["outcome"]["status"] == "ok" and cancel["outcome"]["data"]["terminal_won"] is True)
secondary_update = admin("secondary_update", session2)
check("actual_secondary_window_cannot_update_authority", not secondary_update["ok"] and "Only the main desktop window" in secondary_update["error"])
check("actual_main_window_updates_same_session", admin("restore", session2)["ok"])
retained_budget, _ = second.call("apply_commands", {"base_revision": 4, "commands": [new_item]})
check("same_session_update_retains_spent_create_ledger", error_code(retained_budget) == "PermissionDenied")
check("actual_main_window_removes_fields", admin("remove_fields", session2)["ok"])
removed_field, _ = second.call("apply_commands", {"base_revision": 4, "commands": [edit]})
check("removed_field_authority_denies_atomically", error_code(removed_field) == "PermissionDenied")
check("actual_main_window_downgrades_operations", admin("drop_operations", session2)["ok"])
limited_tools = {tool["name"] for tool in second.rpc("tools/list", {})["result"]["tools"]}
check("current_tools_list_reflects_operation_downgrade", not {"superfactory_get_object", "superfactory_apply_commands", "superfactory_get_task", "superfactory_cancel_task"}.intersection(limited_tools))
removed_read, _ = object_read(second)
check("removed_operation_is_denied_by_shared_Host", error_code(removed_read) == "PermissionDenied" and removed_read["context"] is None)
check("actual_main_window_restores_allowed_operations", admin("restore", session2)["ok"])
restored_budget, _ = second.call("apply_commands", {"base_revision": 4, "commands": [new_item]})
check("restoring_operations_never_refunds_creation_budget", error_code(restored_budget) == "PermissionDenied")
restored, _ = object_read(second)
check("native_updates_keep_original_session_and_revision", restored["context"]["session_id"] == session2 and restored["outcome"]["data"]["data"]["revision"] == 4)
disconnected_admission, _ = second.call("save_project", {"revision": 4, "selected_directory_ref": selection["selection"]})
check("accepted_task_handle_retained_before_disconnect", disconnected_admission["outcome"]["status"] == "ok")
disconnected_task_id = disconnected_admission["outcome"]["data"]["id"]
disconnected_run_admission, _ = second.call("run_scenario", {"scenario_id": scenario["id"], "input_revision": 4, "observation_window": scenario["window"], "intent": {"kind": "project_scenario"}})
check("actual_Engine_task_admitted_before_disconnect", disconnected_run_admission["outcome"]["status"] == "ok")
disconnected_run_task_id = disconnected_run_admission["outcome"]["data"]["id"]
second.close()
check("EOF_removes_admitted_task_owner_session", eof_removed(session2))

third = Client(3)
context, _ = third.call("get_context", {"selection": []})
check("second_reconnect_fresh_session", context["context"]["session_id"] not in {session1, session2})
old_token, _ = third.call("save_project", {"revision": 4, "selected_directory_ref": selection["selection"]})
check("old_selected_file_token_not_restored", error_code(old_token) == "PermissionDenied")
old_task, _ = third.call("get_task", {"task_id": task_id})
check("current_host_requires_explicit_old_task_reauthorization", error_code(old_task) == "PermissionDenied")
session3 = context["context"]["session_id"]
secondary_task = admin("secondary_task", session3, disconnected_task_id)
check("actual_secondary_window_cannot_grant_old_task", not secondary_task["ok"] and "Only the main desktop window" in secondary_task["error"])
check("actual_main_window_grants_exact_original_task_readonly", admin("task_readonly", session3, disconnected_task_id)["ok"])
reconnected, _ = third.call("get_task", {"task_id": disconnected_task_id})
for _ in range(200):
    if reconnected["outcome"]["data"]["task"]["state"] in {"completed", "failed", "cancelled"}:
        break
    time.sleep(0.005)
    reconnected, _ = third.call("get_task", {"task_id": disconnected_task_id})
reconnected_state = reconnected["outcome"]["data"]
check("accepted_task_completion_observed_after_new_OS_session_read_grant", reconnected_state["task"]["state"] == "completed" and reconnected_state["originating_operation"] == "save_project")
check("reconnect_preserves_original_task_ID_scope_and_input_revision", all(reconnected_state["task"][key] == disconnected_admission["outcome"]["data"][key] for key in ["id", "scope", "kind", "input_revision"]) and reconnected["context"]["session_id"] == session3)
check("readonly_task_grant_exposes_no_controls", reconnected_state["allowed_controls"] == [])
readonly_cancel, _ = third.call("cancel_task", {"task_id": disconnected_task_id, "action": "cancel"})
check("readonly_old_task_grant_denies_business_cancel", error_code(readonly_cancel) == "PermissionDenied")
check("actual_main_window_explicitly_grants_task_controls", admin("task_controls", session3, disconnected_task_id)["ok"])
controlled, _ = third.call("cancel_task", {"task_id": disconnected_task_id, "action": "cancel"})
check("explicit_control_grant_preserves_actual_terminal_winner", controlled["outcome"]["status"] == "ok" and controlled["outcome"]["data"]["terminal_won"])
unsupported_control, _ = third.call("cancel_task", {"task_id": disconnected_task_id, "action": "pause_calculation"})
check("explicit_task_control_grant_does_not_add_unsupported_controls", error_code(unsupported_control) == "UnsupportedCapability")
check("actual_main_window_downgrades_old_task_to_readonly", admin("task_readonly", session3, disconnected_task_id)["ok"])
downgraded_cancel, _ = third.call("cancel_task", {"task_id": disconnected_task_id, "action": "cancel"})
check("old_task_control_downgrade_is_immediate", error_code(downgraded_cancel) == "PermissionDenied")
check("task_access_still_allows_current_operation_downgrade", admin("drop_operations", session3)["ok"])
op_denied, _ = third.call("get_task", {"task_id": disconnected_task_id})
check("granted_task_cannot_bypass_current_operations", error_code(op_denied) == "PermissionDenied")
check("restore_same_session_operations_for_task", admin("restore", session3)["ok"])
check("native_authority_can_change_project_scope", admin("wrong_project", session3)["ok"])
wrong_scope, _ = third.call("get_task", {"task_id": disconnected_task_id})
check("granted_original_task_cannot_bypass_current_project_scope", error_code(wrong_scope) == "PermissionDenied")
wrong_grant = admin("task_controls", session3, disconnected_task_id)
check("native_main_window_cannot_grant_task_outside_current_scope", not wrong_grant["ok"])
check("native_main_window_restores_explicit_project_scope", admin("restore", session3)["ok"])
token_after_grant, _ = third.call("save_project", {"revision": 4, "selected_directory_ref": selection["selection"]})
check("task_and_scope_grants_never_restore_original_file_token", error_code(token_after_grant) == "PermissionDenied")
ungranted_run, _ = third.call("get_task", {"task_id": disconnected_run_task_id})
check("grant_of_one_TaskId_does_not_authorize_another_Engine_task", error_code(ungranted_run) == "PermissionDenied")
check("main_window_grants_original_Engine_TaskId_to_fresh_session", admin("task_readonly", session3, disconnected_run_task_id)["ok"])
for _ in range(200):
    resumed_run, _ = third.call("get_task", {"task_id": disconnected_run_task_id})
    if resumed_run["outcome"]["data"]["task"]["state"] in {"completed", "failed", "cancelled"}:
        break
    time.sleep(0.005)
resumed_run_state = resumed_run["outcome"]["data"]
check("original_Engine_task_completes_across_native_disconnect_without_resubmit", resumed_run_state["task"]["state"] == "completed" and resumed_run_state["originating_operation"] == "run_scenario" and all(resumed_run_state["task"][key] == disconnected_run_admission["outcome"]["data"][key] for key in ["id", "scope", "input_revision"]))
resumed_run_id = resumed_run_state["result"]["data"]["run_id"]
resumed_full, _ = third.call("get_run", {"run_id": resumed_run_id, "query": {"kind": "full_result"}})
check("reconnected_Engine_result_keeps_its_original_frozen_input", resumed_full["outcome"]["data"]["data"]["id"] == resumed_run_id and resumed_full["outcome"]["data"]["data"]["input_ref"] == resumed_run_state["result"]["data"]["input"])
third.close()
check("EOF_revokes_reconnected_session_and_task_access", eof_removed(session3))
fourth = Client(4)
revoke_context, _ = fourth.call("get_context", {"selection": []})
revoke_session = revoke_context["context"]["session_id"]
not_inherited, _ = fourth.call("get_task", {"task_id": disconnected_task_id})
check("next_OS_peer_never_inherits_previous_explicit_task_access", error_code(not_inherited) == "PermissionDenied")
(output / "request-revoke.json").write_text(json.dumps(revoke_session), encoding="utf-8")
for _ in range(200):
    if (output / "revoked.json").exists():
        break
    time.sleep(0.005)
check("native_desktop_revoke_performed", (output / "revoked.json").exists())
fourth.send({"jsonrpc": "2.0", "id": fourth.next_id, "method": "tools/call", "params": {"name": "superfactory_get_context", "arguments": {"api_version": "0.1", "request_id": "after-revoke", "project_id": project_id, "scope": {"kind": "project"}, "operation": "get_context", "params": {"selection": []}}}})
check("revoked_connection_cannot_dispatch", fourth.process.stdout.readline() == b"" and fourth.process.wait(timeout=5) == 76)
fourth.process.stdin.close()
fourth.stderr.close()
check("explicit_native_revoke_removes_shared_Host_session", eof_removed(revoke_session))
fifth = Client(5)
switch_context, _ = fifth.call("get_context", {"selection": []})
switch_session = switch_context["context"]["session_id"]
check("pre_switch_actual_peer_reads_original_project", switch_context["outcome"]["status"] == "ok" and switch_context["context"]["project_id"] == project_id)
check("pre_switch_main_grants_original_task", admin("task_readonly", switch_session, disconnected_task_id)["ok"])
switch_task, _ = fifth.call("get_task", {"task_id": disconnected_task_id})
check("pre_switch_actual_peer_reads_explicit_original_task", switch_task["outcome"]["status"] == "ok" and switch_task["outcome"]["data"]["task"]["id"] == disconnected_task_id)
switched = admin("switch_project", switch_session)
check("pre_switch_native_original_project_readback", switched["before"]["outcome"]["data"]["data"]["title"] == "Edited by actual MCP subprocess" and switched["before"]["context"]["project_revision"] == 4)
check("actual_native_switch_after_original_tasks_terminal", switched["ok"])
next_project = switched["switch"]["outcome"]["data"]["project_id"]
check("switch_created_distinct_project_at_revision_one", next_project != project_id and switched["switch"]["outcome"]["data"]["new_revision"] == 1)
post_switch_replies = {}
for label, operation, params, target in [
    ("old_project_context", "get_context", {"selection": []}, project_id),
    ("new_project_context", "get_context", {"selection": []}, next_project),
    ("old_task_old_project", "get_task", {"task_id": disconnected_task_id}, project_id),
    ("old_task_new_project", "get_task", {"task_id": disconnected_task_id}, next_project),
    ("new_project_object", "get_object", {"object_id": {"kind": "project", "id": next_project}, "source": {"kind": "draft"}, "node_index": None}, next_project),
    ("new_project_edit", "apply_commands", {"base_revision": 1, "commands": [{"kind": "set_project_settings", "data": {**settings, "title": "Old authority must not change B"}}]}, next_project),
]:
    reply, _ = fifth.call(operation, params, project=target)
    post_switch_replies[label] = reply
    check(f"project_switch_revokes_same_OS_peer_{label}", error_code(reply) == "PermissionDenied" and reply["context"] is None)
(output / "project-switch-replies.json").write_text(json.dumps({"session_id": switch_session, "before": switch_context, "original_task": switch_task, "native_switch": switched, "replies": post_switch_replies}, ensure_ascii=False, indent=2), encoding="utf-8")
fifth.close()
print(json.dumps({"passed": True, "checks": checks, "task_id": task_id,
                  "accepted_task_at_disconnect": disconnected_task_id,
                  "accepted_Engine_task_at_disconnect": disconnected_run_task_id,
                  "reconnected_original_RunId": resumed_run_id,
                  "cancel_race_actual_state": cancelled_task["outcome"]["data"]["task"]["state"],
                  "empty_fixture_run_id": run_id, "actual_capabilities": {key: capabilities[key] for key in ["implemented_operations", "category_ids", "media_kinds", "providers", "native_primitives", "limitations"]},
                  "actual_sessions": [session1, session2, context["context"]["session_id"]],
                  "project_switch_old_session": switch_session,
                  "project_switch_new_project": next_project,
                  "shared_host_lifecycle_consumers": "connected and verified through actual native main-window administration",
                  "remaining_shared_host_gap": None}, ensure_ascii=False))
