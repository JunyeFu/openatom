"""Independent controlled MCP client of the real native Host and bridge processes."""
import json
import queue
import subprocess
import sys
import threading
import time
from pathlib import Path

BRIDGE, ENDPOINT, HOST, OUT = sys.argv[1:]
OUT = Path(OUT)
TRANSCRIPT = (OUT / 'transcript.jsonl').open('w', encoding='utf8')
RESULTS = []
PROJECT_SETTINGS = {}

def assert_(condition):
    assert condition

def record(name, action):
    try:
        action()
        RESULTS.append({'name': name, 'passed': True})
    except Exception as error:
        RESULTS.append({'name': name, 'passed': False, 'failure': repr(error)})

class MCP:
    def __init__(self):
        self.p = subprocess.Popen([BRIDGE, '--host-pipe', ENDPOINT, '--host-instance-id', HOST], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=subprocess.CREATE_NO_WINDOW)
        self.lines = queue.Queue()
        self.next_id = 0
        threading.Thread(target=self.read, daemon=True).start()
        self.call('initialize', {'protocolVersion': '2024-11-05', 'capabilities': {}, 'clientInfo': {'name': 'independent-w5-controlled-client', 'version': '1'}})
        self.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
    def read(self):
        for line in self.p.stdout:
            self.lines.put(json.loads(line))
        self.lines.put(None)
    def send(self, value):
        TRANSCRIPT.write(json.dumps({'sent': value}, ensure_ascii=False) + '\n'); TRANSCRIPT.flush()
        self.p.stdin.write((json.dumps(value, ensure_ascii=False) + '\n').encode('utf8')); self.p.stdin.flush()
    def call(self, method, params):
        self.next_id += 1
        self.send({'jsonrpc': '2.0', 'id': self.next_id, 'method': method, 'params': params})
        while True:
            response = self.lines.get(timeout=8)
            assert response is not None, f'bridge closed: {self.p.poll()}'
            TRANSCRIPT.write(json.dumps({'received': response}, ensure_ascii=False) + '\n'); TRANSCRIPT.flush()
            if response.get('id') == self.next_id:
                assert 'error' not in response, response
                return response['result']
    def tool(self, operation, params, project='P1', extra=None):
        request = {'api_version': '0.1', 'request_id': f'independent-{id(self)}-{self.next_id+1}', 'project_id': project, 'scope': {'kind': 'project' if project else 'workspace'}, 'operation': operation, 'params': params}
        if extra: request.update(extra)
        result = self.call('tools/call', {'name': 'superfactory_' + operation, 'arguments': request})
        outer = result['structuredContent']; original = outer
        if outer['outcome']['status'] == 'managed':
            resources = [c['resource'] for c in result['content'] if c['type'] == 'resource']
            assert len(resources) == 1
            original = json.loads(resources[0]['text'])
            assert len(resources[0]['text'].encode('utf8')) == outer['outcome']['data']['byte_length']
            assert original['context'] == outer['context']
        assert original['request_id'] == request['request_id']
        assert bool(result.get('isError', False)) == (original['outcome']['status'] == 'error')
        return result, original
    def stop(self):
        self.p.kill(); self.p.wait(timeout=3)

def data(reply):
    assert reply['outcome']['status'] == 'ok', reply
    return reply['outcome']['data']
def denied(reply):
    assert reply['outcome']['status'] == 'error', reply
    assert reply['outcome']['data']['code'] == 'PermissionDenied', reply
def names(client): return {t['name'] for t in client.call('tools/list', {})['tools']}
def context(client): return data(client.tool('get_context', {'selection': []})[1])
def obj(client): return data(client.tool('get_object', {'object_id': {'kind': 'project', 'id': 'P1'}, 'source': {'kind': 'draft'}, 'node_index': None})[1])['data']
def apply(client, revision, commands): return client.tool('apply_commands', {'base_revision': revision, 'commands': commands})[1]
def setting(title, reference=None): return {'kind': 'set_project_settings', 'data': {**PROJECT_SETTINGS, 'title': title, 'time_reference': reference if reference is not None else PROJECT_SETTINGS['time_reference']}}
def wait_task(client, task):
    for _ in range(100):
        state = data(client.tool('get_task', {'task_id': task['id']})[1])
        if state['task']['state'] in ['completed', 'failed', 'cancelled']: return state
        time.sleep(.01)
    raise AssertionError('actual task did not complete')

try:
    readonly = MCP()
    record('dynamic_readonly_tools_exclude_edits_and_unimplemented_renderer', lambda: (
        assert_('superfactory_apply_commands' not in names(readonly)),
        assert_('superfactory_render_view' not in names(readonly)),
        assert_('superfactory_export_evaluation' not in names(readonly))))
    large, original = readonly.tool('get_object', {'object_id': {'kind': 'project', 'id': 'P1'}, 'source': {'kind': 'draft'}, 'node_index': None})
    PROJECT_SETTINGS = {key: data(original)['data'][key] for key in ['time_reference', 'generation_context', 'sources']}
    record('actual_managed_result_preserves_complete_original_identity_and_1200000_title', lambda: (
        assert_(large['structuredContent']['outcome']['status'] == 'managed'),
        assert_(len(data(original)['data']['title']) == 1200000),
        assert_(original['context']['host_instance_id'] == HOST)))
    def uri_check():
        resource = next(c['resource'] for c in large['content'] if c['type'] == 'resource')
        assert resource['mimeType'] == 'application/json'
        assert resource['uri'] == 'superfactory-result://' + large['structuredContent']['outcome']['data']['result_id'], resource['uri']
    record('managed_embedded_resource_uses_frozen_superfactory_result_uri', uri_check)
    record('readonly_direct_edit_preserves_original_permission_error', lambda: denied(apply(readonly, 1, [setting('unauthorized')])) )
    readonly.stop()
    writable = MCP()
    record('writable_dynamic_tools_follow_actual_native_grant', lambda: assert_('superfactory_apply_commands' in names(writable)))
    def atomic():
        before = context(writable)['project']['revision']
        denied(apply(writable, before, [setting('must-not-commit'), setting('must-not-commit', {'label': 'not-granted', 'origin_iso8601': None, 'timezone': None})]))
        assert context(writable)['project']['revision'] == before
        assert len(obj(writable)['title']) == 1200000
        data(apply(writable, before, [setting('Edited by independent native MCP')]))
        item = {'id': 'independent-item', 'revision': 'r1', 'title': 'explicitly authorized', 'unit': 'piece', 'tracking': 'individual', 'spec_ref': None, 'attributes': {}, 'attribute_schema': []}
        data(apply(writable, before+1, [{'kind': 'put_object', 'data': {'kind': 'item_type', 'data': item}}]))
        second = dict(item, id='over-create-budget')
        denied(apply(writable, before+2, [{'kind': 'put_object', 'data': {'kind': 'item_type', 'data': second}}]))
        assert context(writable)['project']['revision'] == before+2
    record('native_atomic_field_authority_and_cumulative_create_budget', atomic)
    revision = context(writable)['project']['revision']
    run_task = data(writable.tool('run_scenario', {'scenario_id': 'QUALITY-ORIGINAL', 'input_revision': revision, 'observation_window': {'start_ms': 0, 'end_ms': 120000}, 'intent': {'kind': 'project_scenario'}})[1])
    def real_run():
        state = wait_task(writable, run_task)
        assert state['originating_operation'] == 'run_scenario' and state['task']['state'] == 'completed', state
        run_id = state['result']['data']['run_id']
        run = data(writable.tool('get_run', {'run_id': run_id, 'query': {'kind': 'full_result'}})[1])['data']
        assert run['evidence_origin'] == 'actual_software' and run['goal_met'] is False
        assert sum(e['quantity']['value']['coefficient'] for e in run['acceptance_events']) == 2
    record('actual_bridge_task_reaches_same_engine_two_packages_without_admission_success_claim', real_run)
    for _ in range(100):
        if (OUT/'selection.json').exists(): break
        time.sleep(.01)
    selection = json.loads((OUT/'selection.json').read_text(encoding='utf8'))
    save = data(writable.tool('save_project', {'revision': revision, 'selected_directory_ref': selection['selection']})[1])
    def actual_save():
        state = wait_task(writable, save)
        assert state['originating_operation'] == 'save_project' and state['task']['state'] == 'completed', state
        assert state['result']['kind'] == 'save'
        assert (OUT/'saved'/'project.json').is_file()
    record('native_selected_directory_actual_IO_task_completion', actual_save)
    def completed_wins():
        control = data(writable.tool('cancel_task', {'task_id': save['id'], 'action': 'cancel'})[1])
        assert control['terminal_won'] is True and control['action_accepted'] is False, control
        assert wait_task(writable, save)['task']['state'] == 'completed'
    record('actual_completed_io_commit_wins_later_business_cancellation', completed_wins)
    (OUT/'revoke.json').write_text(json.dumps({'session_id': selection['session_id']}), encoding='utf8')
    for _ in range(100):
        if (OUT/'revoked.json').exists(): break
        time.sleep(.01)
    def revoked():
        try:
            writable.tool('get_context', {'selection': []})
        except (AssertionError, BrokenPipeError):
            pass
        else:
            raise AssertionError('revoked pipe still returned a usable Host response')
        writable.p.wait(timeout=4)
        assert writable.p.returncode == 76, writable.p.returncode
    record('actual_main_window_revocation_closes_old_bridge_transport', revoked)
    fresh = MCP()
    record('fresh_native_session_does_not_inherit_old_selected_file_token', lambda: denied(fresh.tool('save_project', {'revision': revision, 'selected_directory_ref': selection['selection']})[1]))
    record('fresh_native_session_does_not_inherit_old_task_authority', lambda: denied(fresh.tool('get_task', {'task_id': save['id']})[1]))
    fresh.stop()
    def wrong_host():
        p = subprocess.run([BRIDGE, '--host-pipe', ENDPOINT, '--host-instance-id', 'wrong-native-host'], input=b'', capture_output=True, timeout=4, creationflags=subprocess.CREATE_NO_WINDOW)
        assert p.returncode == 2, p.returncode
    record('wrong_host_instance_is_rejected_before_any_native_grant', wrong_host)
except Exception as error:
    RESULTS.append({'name': 'remaining_process_sequence', 'passed': False, 'failure': repr(error)})
finally:
    for candidate in ['readonly', 'writable', 'fresh']:
        client = globals().get(candidate)
        if client and client.p.poll() is None: client.stop()
    (OUT/'client-result.json').write_text(json.dumps({'actual_groups': RESULTS, 'real_codex_business_verified': False, 'whole_task_done': False}, ensure_ascii=False, indent=2), encoding='utf8')
    print(json.dumps(RESULTS, ensure_ascii=False))
    TRANSCRIPT.close()
sys.exit(0 if RESULTS and all(r['passed'] for r in RESULTS) else 1)
