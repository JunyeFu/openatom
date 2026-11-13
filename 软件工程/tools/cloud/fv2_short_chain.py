"""Bounded Agent -> actual MCP/Host production observation, stop and save.

This entry never creates a Host, fabricates receipts, edits a database, replays an
unknown request, or treats an old saved execution as a new production session.
"""
import argparse
import copy
import json
import math
from pathlib import Path
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from integrations.agent.stdio_client import StdioClient
from integrations.agent.workflow import ExecutionWorkflow, execution_response, unwrap


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def discover(client):
    catalog = unwrap(client.request(None, 'describe_capabilities', {}))
    tools = client.rpc('tools/list', {})
    if tools.get('nextCursor'):
        raise ValueError('Unexpected paginated Host tools; no partial catalog assumed complete')
    advertised = {tool['name'] for tool in tools['tools']}
    catalog = copy.deepcopy(catalog)
    catalog['implemented_operations'] = [row for row in catalog['implemented_operations']
        if 'superfactory_' + row['operation'] in advertised]
    return catalog, tools


def finish(flow, result, seconds):
    deadline = time.monotonic() + seconds
    while result['status'] == 'pending' and time.monotonic() < deadline:
        time.sleep(.25)
        result = flow.advance()
    if result['status'] != 'completed':
        raise RuntimeError('Original Task did not complete: ' + result['status'])
    return result['data']


def control(flow, identity, action):
    flow.snapshot(identity, {'kind': 'latest'})
    result = flow.control(action, confirmed=True)
    if result['status'] != 'acknowledged':
        raise RuntimeError('Control requires a new decision; action was not retried: ' + result['status'])
    return result


def observe(flow, identity, seconds, record, *, until_settled=False):
    deadline = time.monotonic() + seconds
    while True:
        status = flow.exit_status(identity)
        record(status)
        if until_settled and status['terminal_and_settled'] and status.get('publication_synchronized', False):
            return status
        if time.monotonic() >= deadline:
            if until_settled:
                raise RuntimeError('Stop observation budget exhausted; actual obligations/publication remain in status.json')
            return status
        time.sleep(.25)


def await_execution_authority(flow, identity, seconds, maintenance_device, record):
    """Wait for explicit Native approval; this function never submits controls."""
    deadline = time.monotonic() + seconds
    while True:
        status = flow.exit_status(identity)
        record(status)
        actual = status['actual']
        if actual['delivery']['read_origin'] != 'host' or actual['access']['ownership'] != 'current_host_owner':
            raise ValueError('Execution authority requires the original current Host; archived sources stay read-only')
        if status['lifecycle_state'] != 'prepared':
            raise ValueError('Original execution is no longer prepared; no run action submitted')
        if time.monotonic() >= deadline:
            raise RuntimeError('Native execution authority was not supplied within the selected wait budget; no run submitted')
        access = actual['access']
        if {'run', 'stop_dispatch'} <= set(access['allowed_session_controls']) and (
                maintenance_device is None or maintenance_device in access['device_acceptance_control_ids']):
            return status
        time.sleep(.25)


def save(client, project_id, selection_file, seconds):
    # The desktop Agent panel copies an actual token for this currently open
    # connection. The filesystem here transports that token, never a raw path.
    deadline = time.monotonic() + seconds
    while not selection_file.is_file() and time.monotonic() < deadline:
        time.sleep(.25)
    if not selection_file.is_file():
        raise RuntimeError('Current-connection Native save directory token was not supplied')
    selected = read(selection_file)
    if set(selected) != {'selection_id', 'purpose'} or selected['purpose'] != 'save_project' or not selected['selection_id']:
        raise ValueError('Only the actual current-connection save_project token is accepted')
    context = client.request(project_id, 'get_context', {'selection': []})['context']
    response = client.request(project_id, 'save_project', {'revision': context['project_revision'],
                              'selected_directory_ref': selected})
    handle = unwrap(response)
    deadline = time.monotonic() + seconds
    while True:
        state = unwrap(client.request(project_id, 'get_task', {'task_id': handle['id']}))
        if state['originating_operation'] != 'save_project' or state.get('originating_request_id') not in (None, response['request_id']) or any(
                state['task'][key] != handle[key] for key in ('id', 'scope', 'kind', 'input_revision')):
            raise ValueError('Save Task differs from original admission')
        if state['task']['state'] == 'completed':
            if state['result']['kind'] != 'save' or state['result']['data']['still_dirty']:
                raise RuntimeError('Save Task completed but does not cover all current content; no save resubmitted')
            return state
        if state['task']['state'] in ('failed', 'cancelled') or time.monotonic() >= deadline:
            raise RuntimeError('Original save Task is ' + state['task']['state'] + '; no save resubmitted')
        time.sleep(.25)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=('inspect', 'prepare', 'execute', 'resume-prepared', 'observe', 'readback', 'recover'))
    parser.add_argument('--bridge', type=Path, required=True)
    parser.add_argument('--host-pipe', required=True)
    parser.add_argument('--host-instance-id', required=True)
    parser.add_argument('--project-id', required=True)
    parser.add_argument('--output', type=Path, required=True, help='New directory for actual requests/results')
    parser.add_argument('--scenario-id')
    parser.add_argument('--scenario-revision', type=int)
    parser.add_argument('--policy', type=Path, help='Actual explicitly chosen ExecutionPolicy, unchanged')
    parser.add_argument('--confirm-prepare-run-stop', action='store_true')
    parser.add_argument('--confirm-prepare', action='store_true', help='prepare only: explicitly authorize H01 without run or stop')
    parser.add_argument('--confirm-run-stop', action='store_true', help='resume-prepared only: authorize run and stop, without new preparation')
    parser.add_argument('--await-terminal', action='store_true', help='observe only: wait for the Native operator to stop and settle; sends no controls')
    parser.add_argument('--await-execution-authority', action='store_true', help='execute/resume-prepared: wait for explicit Native action/device grants; never grants authority itself')
    parser.add_argument('--run-seconds', type=float, default=10)
    parser.add_argument('--wait-seconds', type=float, default=60)
    parser.add_argument('--maintenance-device', help='Explicitly withdraw then restore this device acceptance')
    parser.add_argument('--save-selection-file', type=Path, help='New file containing the token copied from the current Native Agent panel')
    parser.add_argument('--identity', type=Path, help='observe/readback/resume-prepared: original exact ExecutionIdentity')
    parser.add_argument('--expected-inputs', type=Path, help='Compare the prior complete frozen inputs')
    parser.add_argument('--pending-state', type=Path, help='recover only: prior status.json with the original pending request')
    parser.add_argument('--run-id', action='append', default=[], help='Old Run IDs to read before/after, never execute')
    args = parser.parse_args()
    if not all(math.isfinite(value) and value > 0 for value in (args.run_seconds, args.wait_seconds)):
        parser.error('Observation and wait budgets must be positive')
    if args.phase == 'execute' and (not args.confirm_prepare_run_stop or args.policy is None or args.identity):
        parser.error('execute requires an explicit policy and prepare/run/stop authorization; old identity is forbidden')
    if args.phase == 'prepare' and (not args.confirm_prepare or args.policy is None or args.identity):
        parser.error('prepare requires an explicit policy and H01 authorization; old identity is forbidden')
    if args.phase in ('readback', 'observe') and args.identity is None:
        parser.error('readback/observe require the original identity')
    if args.phase == 'resume-prepared' and (args.identity is None or not args.confirm_run_stop or args.policy is not None):
        parser.error('resume-prepared requires the original identity and explicit run/stop authorization; policy cannot be replaced')
    if args.await_execution_authority and args.phase not in ('execute', 'resume-prepared'):
        parser.error('--await-execution-authority is only valid for execute/resume-prepared')
    if args.phase in ('inspect', 'prepare', 'execute') and (args.scenario_id is None or args.scenario_revision is None):
        parser.error('inspect/execute require the actual Scenario ID and revision')
    if args.phase == 'recover' and args.pending_state is None:
        parser.error('recover requires the original pending status.json')
    if args.save_selection_file and args.save_selection_file.exists():
        parser.error('save token file must be new, issued after authorizing this connection')
    args.output.mkdir(parents=True, exist_ok=False)
    state = {'phase': args.phase, 'completed': False, 'pending': None}
    flow = None
    with (args.output / 'bridge.stderr.txt').open('wb') as stderr, (args.output / 'calls.jsonl').open('w', encoding='utf-8') as calls:
        try:
            with StdioClient(args.bridge, args.host_pipe, args.host_instance_id, timeout=args.wait_seconds, stderr=stderr) as client:
                raw_send = client.send
                def send(request):
                    calls.write(json.dumps({'request': request}, ensure_ascii=False) + '\n'); calls.flush()
                    reply = raw_send(request)
                    calls.write(json.dumps({'response': execution_response(reply)}, ensure_ascii=False) + '\n'); calls.flush()
                    return reply
                client.send = send
                context = client.request(args.project_id, 'get_context', {'selection': []})
                catalog, tools = discover(client)
                if context['context']['host_instance_id'] != args.host_instance_id or context['context']['project_id'] != args.project_id:
                    raise ValueError('Actual Host/project does not match selected connection')
                flow = ExecutionWorkflow(send, context['context'], catalog)
                state['context'] = context['context']
                write(args.output / 'context.json', context)
                write(args.output / 'catalog.json', catalog)
                write(args.output / 'tools.json', tools)
                print(json.dumps({'authorized_context': context['context']}, ensure_ascii=False), flush=True)
                if args.phase == 'recover':
                    previous = read(args.pending_state)
                    if previous['context']['host_instance_id'] != context['context']['host_instance_id'] or previous['context']['project_id'] != args.project_id:
                        raise ValueError('Original request belongs to another Host or project; it cannot be recovered here')
                    if not previous.get('pending'):
                        raise ValueError('No original pending execution Task in the supplied status')
                    flow.pending = copy.deepcopy(previous['pending'])
                    recovered = flow.advance() if flow.pending['handle'] is not None else flow.recover()
                    if recovered['status'] == 'pending':
                        finish(flow, recovered, args.wait_seconds)
                        recovered = {'status': 'completed', 'note': 'Original Task observed; no production request submitted'}
                    state['recovery'] = recovered
                    state['completed'] = recovered['status'] in ('completed', 'failed', 'cancelled')
                    return 0 if state['completed'] else 1
                def old_runs():
                    return {identity: unwrap(client.request(args.project_id, 'get_run',
                        {'run_id': identity, 'query': {'kind': 'full_result'}})) for identity in args.run_id}
                original_runs = old_runs()
                write(args.output / 'old-runs-before.json', original_runs)
                if args.phase in ('readback', 'resume-prepared', 'observe'):
                    identity = read(args.identity)
                else:
                    selection = {'kind': 'configuration', 'base_revision': context['context']['project_revision'],
                                 'scenario_id': args.scenario_id, 'scenario_revision': args.scenario_revision}
                    configuration = finish(flow, flow.cloud(selection), args.wait_seconds)
                    write(args.output / 'configuration.json', configuration)
                    if args.phase == 'inspect':
                        state['completed'] = True
                        return 0
                    prepared = finish(flow, flow.prepare(read(args.policy), confirmed=True), args.wait_seconds)
                    identity = prepared['identity']
                    write(args.output / 'prepared.json', prepared)
                    catalog, tools = discover(client)
                    flow.refresh_catalog(catalog)
                    write(args.output / 'catalog-after-preparation.json', catalog)
                    write(args.output / 'tools-after-preparation.json', tools)
                write(args.output / 'identity.json', identity)
                frozen = flow.view({'kind': 'inputs', 'identity': identity})
                write(args.output / 'frozen-inputs.json', frozen)
                if args.expected_inputs and frozen != read(args.expected_inputs):
                    raise ValueError('New-process frozen input readback differs')
                def record(status):
                    state['last_observation'] = status
                    write(args.output / 'status.json', state)
                if args.phase == 'resume-prepared':
                    actual = flow.snapshot(identity, {'kind': 'latest'})
                    if identity['host_context_session_id'] != context['context']['host_instance_id'] or actual['snapshot']['lifecycle']['state'] != 'prepared':
                        raise ValueError('Only this Host original prepared session may continue; no run action submitted')
                if args.phase in ('execute', 'resume-prepared'):
                    if args.await_execution_authority:
                        print(json.dumps({'awaiting_native_execution_authority': identity,
                                          'required_session_controls': ['run', 'stop_dispatch'],
                                          'required_device': args.maintenance_device}, ensure_ascii=False), flush=True)
                        await_execution_authority(flow, identity, args.wait_seconds, args.maintenance_device, record)
                        catalog, tools = discover(client)
                        flow.refresh_catalog(catalog)
                        write(args.output / 'catalog-after-authority.json', catalog)
                        write(args.output / 'tools-after-authority.json', tools)
                    if args.maintenance_device:
                        control(flow, identity, {'kind': 'set_device_acceptance', 'device_id': args.maintenance_device, 'accept_new_work': False})
                    control(flow, identity, {'kind': 'run'})
                    observe(flow, identity, args.run_seconds, record)
                    if args.maintenance_device:
                        control(flow, identity, {'kind': 'set_device_acceptance', 'device_id': args.maintenance_device, 'accept_new_work': True})
                        observe(flow, identity, args.run_seconds, record)
                    if not state['last_observation']['terminal_and_settled']:
                        control(flow, identity, {'kind': 'stop_dispatch'})
                    observe(flow, identity, args.wait_seconds, record, until_settled=True)
                elif args.phase == 'observe':
                    observe(flow, identity, args.wait_seconds if args.await_terminal else args.run_seconds,
                            record, until_settled=args.await_terminal)
                else:
                    record(flow.exit_status(identity))
                if args.save_selection_file:
                    state['save'] = save(client, args.project_id, args.save_selection_file, args.wait_seconds)
                if original_runs != old_runs():
                    raise ValueError('An original frozen Run changed')
                state['old_runs_unchanged'] = list(original_runs)
                state['completed'] = True
                return 0
        except Exception as error:
            state['error'] = {'type': type(error).__name__, 'message': str(error)}
            print(json.dumps(state['error'], ensure_ascii=False), file=sys.stderr)
            return 1
        finally:
            state['pending'] = copy.deepcopy(flow.pending) if flow else None
            write(args.output / 'status.json', state)


if __name__ == '__main__':
    raise SystemExit(main())
