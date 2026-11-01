"""Real stdio client versus the actual Native main-window command entry.

Private acceptance input drives UI commands, not human gestures. This is not
the installed Codex-client acceptance and does not grant permissions over MCP.
"""
import copy
import json
from pathlib import Path
import subprocess
import sys
import time


bridge, endpoint, host, project_id, directory = sys.argv[1:]
output = Path(directory)
checks = []
requests = []
ordinal = 0


def record(name, condition):
    if not condition:
        raise AssertionError(name)
    checks.append(name)


def canonical(operation, params, project=project_id):
    global ordinal
    ordinal += 1
    return {"api_version": "0.1", "request_id": f"P304-concurrent-{ordinal}",
            "project_id": project, "scope": {"kind": "workspace" if project is None else "project"},
            "operation": operation, "params": params}


def keep(route, request, response):
    requests.append({"route": route, "request": request, "response": response})
    (output / 'concurrency-requests.json').write_text(json.dumps(requests, ensure_ascii=False, indent=2), encoding='utf-8')
    return response


def data(reply):
    assert reply['outcome']['status'] == 'ok', reply
    return reply['outcome']['data']


def code(reply):
    return reply.get('outcome', {}).get('data', {}).get('code')


class StdioClient:
    def __init__(self):
        self.stderr = (output / 'concurrency-bridge.stderr.txt').open('wb')
        self.process = subprocess.Popen([bridge, '--host-pipe', endpoint, '--host-instance-id', host],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
            creationflags=subprocess.CREATE_NO_WINDOW)
        self.identifier = 0
        result = self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {},
            'clientInfo': {'name': 'superfactory-native-concurrent-proof', 'version': '1'}})
        record('actual_stdio_initialized', 'result' in result)
        self.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})

    def send(self, value):
        self.process.stdin.write(json.dumps(value, ensure_ascii=False).encode('utf-8') + b'\n')
        self.process.stdin.flush()

    def rpc(self, method, params):
        self.identifier += 1
        identity = self.identifier
        self.send({'jsonrpc': '2.0', 'id': identity, 'method': method, 'params': params})
        while True:
            line = self.process.stdout.readline()
            assert line, (method, self.process.poll())
            value = json.loads(line)
            if value.get('id') == identity:
                return value

    def call(self, operation, params, project=project_id):
        request = canonical(operation, params, project)
        wrapper = self.rpc('tools/call', {'name': 'superfactory_' + operation, 'arguments': request})
        assert 'result' in wrapper, wrapper
        result = wrapper['result']
        reply = result['structuredContent']
        if reply['outcome']['status'] == 'managed':
            resources = [item['resource'] for item in result['content'] if item['type'] == 'resource']
            assert len(resources) == 1 and resources[0]['mimeType'] == 'application/json', result
            reply = json.loads(resources[0]['text'])
        assert reply['request_id'] == request['request_id'], reply
        assert result['isError'] == (reply['outcome']['status'] == 'error'), result
        return keep('actual_external_stdio', request, reply)

    def close(self):
        self.process.stdin.close()
        assert self.process.wait(timeout=5) == 0
        self.stderr.close()


ui_index = 0


def ui(operation, params, project=project_id):
    global ui_index
    ui_index += 1
    request = canonical(operation, params, project)
    temporary = output / f'ui-request-{ui_index}.tmp'
    temporary.write_text(json.dumps(request), encoding='utf-8')
    temporary.replace(output / f'ui-request-{ui_index}.json')
    reply = output / f'ui-response-{ui_index}.json'
    end = time.monotonic() + 30
    while not reply.exists():
        assert time.monotonic() < end, ('Native UI command not received', request)
        time.sleep(.01)
    return keep('actual_main_window_command_controlled_input', request,
                json.loads(reply.read_text(encoding='utf-8')))


def project(client):
    return data(client.call('get_object', {'object_id': {'kind': 'project', 'id': project_id},
        'source': {'kind': 'draft'}, 'node_index': None}))['data']


def edit(device, value, revision):
    changed = copy.deepcopy(device)
    changed['pose']['translation']['x'] = value
    return {'base_revision': revision, 'commands': [{'kind': 'put_object',
        'data': {'kind': 'device', 'data': changed}}]}


def terminal(client, handle):
    end = time.monotonic() + 120
    while True:
        state = data(client.call('get_task', {'task_id': handle['id']}))
        if state['task']['state'] in ['completed', 'failed', 'cancelled']:
            assert state['task']['state'] == 'completed', state
            return state['result']['data']
        assert time.monotonic() < end, handle
        time.sleep(.01)


def run(client, revision):
    scenario = data(client.call('get_object', {'object_id': {'kind': 'scenario', 'id': 'LOCAL'},
        'source': {'kind': 'draft'}, 'node_index': None}))['data']
    handle = data(client.call('run_scenario', {'scenario_id': 'LOCAL', 'input_revision': revision,
        'observation_window': scenario['window'], 'intent': {'kind': 'project_scenario'}}))
    summary = terminal(client, handle)
    return data(client.call('get_run', {'run_id': summary['run_id'], 'query': {'kind': 'full_result'}}))


if __name__ == '__main__':
    client = None
    failure = None
    try:
        client = StdioClient()
        initial = project(client)
        revision = initial['revision']
        device = next(d for d in initial['devices'] if d['id'] == 'ASM-01')
        old_run = run(client, revision)
        first = data(ui('apply_commands', edit(device, 2, revision)))
        rejected = client.call('apply_commands', edit(device, 3, revision))
        after_ui = project(client)
        record('UI_first_external_stale_revision_conflict', code(rejected) == 'RevisionConflict')
        record('UI_first_only_first_value_committed', first['new_revision'] == revision + 1 and
            after_ui['revision'] == revision + 1 and after_ui['devices'][0]['pose']['translation']['x'] == 2)
        second = data(client.call('apply_commands', edit(device, 4, revision + 1)))
        rejected = ui('apply_commands', edit(device, 5, revision + 1))
        after_agent = project(client)
        record('external_first_UI_stale_revision_conflict', code(rejected) == 'RevisionConflict')
        record('external_first_only_first_value_committed', second['new_revision'] == revision + 2 and
            after_agent['revision'] == revision + 2 and after_agent['devices'][0]['pose']['translation']['x'] == 4)
        frozen = data(client.call('get_run', {'run_id': old_run['data']['id'], 'query': {'kind': 'full_result'}}))
        record('existing_actual_Run_unchanged_after_both_edit_orders', frozen == old_run)
        new_run = run(client, revision + 2)
        record('new_actual_Run_has_distinct_identity', new_run['data']['id'] != old_run['data']['id'] and
            new_run['data']['input_ref']['project_revision'] == revision + 2 and
            old_run['data']['input_ref']['project_revision'] == revision)
        selections = json.loads((output / 'actual-file-tokens.json').read_text(encoding='utf-8'))
        selection = next(x for x in selections if x['purpose'] == 'save_project')
        saved = terminal(client, data(client.call('save_project', {'revision': revision + 2, 'selected_directory_ref': selection})))
        record('actual_current_revision_saved', saved['saved_revision'] == revision + 2)
        created = data(ui('apply_commands', {'base_revision': None, 'commands': [{'kind': 'create_project',
            'data': {'title': 'P304 separate project', 'time_reference': initial['time_reference'],
                     'departure': 'discard_after_tasks_terminal'}}]}, None))
        context = data(ui('get_context', {'selection': []}, None))
        record('UI_switched_to_one_different_active_project', context['project']['id'] != project_id)
        old_scope = client.call('apply_commands', edit(device, 8, revision + 2))
        record('old_external_authority_does_not_edit_new_project', code(old_scope) == 'PermissionDenied')
        current = data(ui('get_context', {'selection': []}, None))
        record('rejected_old_project_request_preserves_new_context', current['project'] == context['project'])
        client.close()
        client = None
    except Exception as error:
        failure = repr(error)
        raise
    finally:
        (output / 'concurrency-result.json').write_text(json.dumps({'passed': len(checks), 'checks': checks,
            'failure': failure, 'actual_Native_Host': True, 'actual_stdio_external_client': True,
            'actual_Codex_client': False, 'controlled_UI_command_entry_not_human_gestures': True,
            'whole_NFR08_or_package_pass': False}, ensure_ascii=False, indent=2), encoding='utf-8')
        if client is not None:
            client.process.stdin.close()
            client.process.wait(timeout=5)
            client.stderr.close()
