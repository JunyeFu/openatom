"""Actual public rendering MCP client, only started by Root's explicit Native mode.

This client creates no Host, caller or grant. Native authorizes exactly this
bridge's OS-verified PID through the real main-window authority command.
"""
import base64
import json
import struct
import subprocess
import sys
import time
from pathlib import Path


def write_new(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)


class Client:
    def __init__(self, bridge, endpoint, host, project, output):
        self.host, self.project, self.output = host, project, output
        self.next_id = 1
        self.stderr = (output / "bridge.stderr.txt").open("xb")
        self.process = subprocess.Popen(
            [bridge, "--host-pipe", endpoint, "--host-instance-id", host],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        # Publish atomically, so Native never reads a partial identity file.
        temporary = output / "bridge-peer.tmp"
        write_new(temporary, {"pid": self.process.pid, "bridge": bridge})
        temporary.rename(output / "bridge-peer.json")

    def send(self, value):
        self.process.stdin.write(json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8") + b"\n")
        self.process.stdin.flush()

    def rpc(self, method, params):
        request_id = self.next_id
        self.next_id += 1
        self.send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        while True:
            line = self.process.stdout.readline()
            assert line, f"Actual bridge disconnected before {method}: {self.process.poll()}"
            response = json.loads(line)
            if response.get("id") == request_id:
                assert "error" not in response, response
                return response["result"]

    def call(self, operation, scope, params):
        arguments = {"api_version": "0.1", "request_id": f"public-render-{self.next_id}",
                     "project_id": self.project, "scope": scope, "operation": operation, "params": params}
        wrapper = self.rpc("tools/call", {"name": "superfactory_" + operation, "arguments": arguments})
        original = wrapper["structuredContent"]
        managed = original["outcome"]["status"] == "managed"
        if managed:
            resources = [block["resource"] for block in wrapper["content"] if block["type"] == "resource"]
            assert len(resources) == 1 and resources[0]["mimeType"] == "application/json"
            resource = resources[0]
            assert resource["uri"] == "superfactory-result://" + original["outcome"]["data"]["result_id"]
            assert len(resource["text"].encode("utf-8")) == original["outcome"]["data"]["byte_length"]
            original = json.loads(resource["text"])
        assert original["request_id"] == arguments["request_id"]
        assert wrapper["isError"] == (original["outcome"]["status"] == "error")
        assert original["outcome"]["status"] == "ok", original
        assert original["context"]["host_instance_id"] == self.host
        return original, wrapper, managed

    def close(self):
        self.process.stdin.close()
        exit_code = self.process.wait()
        self.stderr.close()
        return exit_code


def verify_case(client, case, index):
    scope, params = case["scope"], case["params"]
    admitted, _, _ = client.call("render_view", scope, params)
    handle = admitted["outcome"]["data"]
    assert handle["kind"] == "rendering" and handle["input_revision"] == params["revision"]
    write_new(client.output / f"case-{index:02}-admission.json", admitted)
    while True:
        reply, wrapper, managed = client.call("get_task", scope, {"task_id": handle["id"]})
        task = reply["outcome"]["data"]
        if task["task"]["state"] in ("completed", "failed", "cancelled"):
            break
        time.sleep(0.01)  # Observe the real Task; no speculative timeout or retry.
    write_new(client.output / f"case-{index:02}-task.json", reply)
    write_new(client.output / f"case-{index:02}-mcp.json", wrapper)
    assert task["task"]["state"] == "completed", task
    assert task["task"]["id"] == handle["id"] and task["task"]["scope"] == handle["scope"]
    assert task["originating_operation"] == "render_view" and task["result"]["kind"] == "render"
    rendered = task["result"]["data"]
    assert task["task"]["scope"] == {"kind": "edit", "data": rendered["requested_scope"]}
    if scope["kind"] == "project":
        assert rendered["requested_scope"] == {"kind": "project", "data": client.project}
    else:
        assert rendered["requested_scope"]["kind"] == "definition_draft"
        assert rendered["requested_scope"]["data"]["id"] == scope["data"]
    scene = rendered["scene"]
    assert rendered["requested_scope_revision"] == params["revision"] == scene["scene_revision"]
    # Host canonicalizes target/hall only when absent. All explicitly requested
    # view fields, including camera/LOD/source time, remain exact.
    for key, value in params["view"].items():
        if key not in ("hall_id", "object_ref") or value is not None:
            assert scene["view"][key] == value, (key, scene["view"], params["view"])
    images = [block for block in wrapper["content"] if block["type"] == "image"]
    assert len(images) == 1 and images[0]["mimeType"] == "image/png"
    png = base64.b64decode(images[0]["data"], validate=True)
    assert png[:8] == b"\x89PNG\r\n\x1a\n" and png[12:16] == b"IHDR"
    width, height = struct.unpack(">II", png[16:24])
    assert width > 0 and height > 0
    file = client.output / f"case-{index:02}.png"
    with file.open("xb") as stream:
        stream.write(png)
    return {"name": case["name"], "task_id": handle["id"], "scope": scope,
            "scope_revision": params["revision"], "view": scene["view"],
            "scene": scene, "visible_objects": rendered["visible_objects"],
            "issues": rendered["issues"], "png": str(file), "byte_length": len(png),
            "width": width, "height": height, "managed_json_and_image": managed, "passed": True}


def main():
    bridge, endpoint, host, project, output = sys.argv[1:]
    output = Path(output)
    plan = json.loads((output / "plan.json").read_text(encoding="utf-8"))
    client, results, exit_code = None, [], None
    report = {"passed": False, "controlled_authorization": True, "human_or_Codex_acceptance": False}
    try:
        client = Client(bridge, endpoint, host, project, output)
        initialized = client.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": {"name": "superfactory-public-render-proof", "version": "1"}})
        assert initialized["protocolVersion"] == "2025-06-18"
        client.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        tools = client.rpc("tools/list", {})["tools"]
        assert {"superfactory_render_view", "superfactory_get_task"} <= {tool["name"] for tool in tools}
        for index, case in enumerate(plan["cases"], 1):
            results.append(verify_case(client, case, index))
            print(json.dumps({"completed": index, "name": case["name"]}), flush=True)
        report["passed"] = True
    except Exception as error:
        report["failure"] = str(error)
    finally:
        if client is not None:
            exit_code = client.close()
    report.update({"cases": results, "bridge_exit": exit_code})
    report["passed"] = report["passed"] and exit_code == 0
    write_new(output / "driver-result.json", report)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
