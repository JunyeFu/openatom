"""Finite factory workflow over an already authorized canonical MCP caller.

The caller owns MCP transport and file selection. All checks, edits, Runs and
reports come from the connected Host; this module has no simulation or inventory.
"""
import copy
import time
import uuid


class HostError(RuntimeError):
    def __init__(self, response):
        self.response = response
        super().__init__(str(response['outcome']['data']))


def unwrap(response):
    if response['outcome']['status'] != 'ok':
        raise HostError(response)
    return response['outcome']['data']


def complete(call, operation, params):
    handle = unwrap(call(operation, params))
    return wait_task(call, operation, handle)


def wait_task(call, operation, handle):
    while True:
        state = unwrap(call('get_task', {'task_id': handle['id']}))
        if state['originating_operation'] != operation or any(state['task'][key] != handle[key] for key in ('id', 'scope', 'kind', 'input_revision')):
            raise ValueError('Task response does not match the admitted operation')
        if state['task']['state'] in ('completed', 'failed', 'cancelled'):
            return state
        time.sleep(.005)


def object_at_draft(call, kind, identity):
    return unwrap(call('get_object', {'object_id': {'kind': kind, 'id': identity},
        'source': {'kind': 'draft'}, 'node_index': None}))['data']


def template_for_goal(call, goal):
    """Only actual applicable candidates may be selected autonomously.

    An explicitly chosen private example can still be previewed as a labeled
    model through the separate low-level functions; it is not an industry-fit
    claim and never changes the catalog's original applicability/missing fields.
    """
    library = unwrap(call('query_module_library', {'category_filters': ['templates'],
        'industry': goal['industry'], 'product_family': goal['product_family'],
        'site_envelope': goal['site_envelope'], 'source_scope': ['bundled', 'project_packages']}))
    applicable = [candidate for candidate in library['candidates'] if candidate['applicable']]
    return {'status': 'available' if applicable else 'missing_conditions',
        'applicable_candidates': applicable, 'actual_library': library}


def instantiate(call, project_id, reference, choices, seed):
    """Preview uses current Host bytes; adopting the preview remains explicit."""
    project = object_at_draft(call, 'project', project_id)
    params = {'object_id': {'kind': 'module', 'id': reference['id']}, 'node_index': None,
        'source': {'kind': 'template_preview', 'data': {'module_ref': reference,
            'base_revision': project['revision'], 'choices': choices, 'seed': seed}}}
    preview = unwrap(call('get_object', params))['data']
    return preview


def adopt_template(call, preview):
    if preview['command'] is None:
        raise ValueError('Resolve template choices before explicit adoption')
    return unwrap(call('apply_commands', {'base_revision': preview['base_revision'],
        'commands': [{'kind': 'instantiate_template', 'data': preview['command']}]}))


def run_factory(call, project_id, scenario_id, goal, select_directory, propose, tools, render_request=None):
    """Round zero directly delivers; only confirmed effective edits use rounds 1..3.

    propose receives actual diagnostics and returns full editable domain objects,
    or an empty list with its inability reason. The Host checks every changed leaf
    against the current grant. No conflict, rejection or lost reply is replayed.
    """
    result = {'committed_repair_rounds': 0, 'run_ids': [], 'checks': [], 'changes': [],
        'comparisons': [], 'renders': [], 'status': None, 'goal': copy.deepcopy(goal)}
    frozen = {}
    while True:
        project = object_at_draft(call, 'project', project_id)
        scenario = object_at_draft(call, 'scenario', scenario_id)
        if scenario['acceptance_goal'] != goal:
            raise ValueError('The confirmed goal must match the actual Scenario input')
        check = unwrap(call('check_factory_draft', {'revision': project['revision'],
            'generation_goal': goal, 'check_scope': {'objects': [{'kind': 'project', 'id': project_id}],
                'check_references': True, 'check_business': True, 'check_static_layout': True}}))
        result['checks'].append(check)
        runnable = bool(check['checks']) and all(row['outcome'] == 'pass' for row in check['checks'])
        if render_request is not None:
            if 'render_view' not in tools:
                result['renders'].append({'executed': False, 'reason': 'render_view absent from actual connected tools/list'})
                result['status'] = 'render_unavailable'; break
            request = copy.deepcopy(render_request)
            if request['view']['run_id'] is not None or request['view']['view_time_ms'] is not None:
                result['renders'].append({'executed': False, 'reason': 'Factory draft feedback must select the current draft, not an existing Run'})
                result['status'] = 'render_source_mismatch'; break
            request['revision'] = project['revision']
            request['view']['project_revision'] = project['revision']
            try:
                rendered = complete(call, 'render_view', request)
            except HostError as failure:
                result['renders'].append({'executed': False, 'response': failure.response})
                result['status'] = 'render_unsupported' if failure.response['outcome']['data']['code'] == 'UnsupportedCapability' else 'render_rejected'
                result['terminal_rejection'] = failure.response; break
            result['renders'].append(rendered)
            if rendered['task']['state'] != 'completed':
                result['status'] = 'render_' + rendered['task']['state']; result['terminal_task'] = rendered; break
        else:
            result['renders'].append({'executed': False, 'reason': 'render_view absent from actual connected tools/list' if 'render_view' not in tools else 'No product view/camera selected; no rendered-image evidence supplied'})
        current = None
        if runnable:
            try:
                task = complete(call, 'run_scenario', {'scenario_id': scenario_id,
                    'input_revision': project['revision'], 'observation_window': scenario['window'],
                    'intent': {'kind': 'project_scenario'}})
            except HostError as failure:
                result['status'] = 'run_rejected'; result['terminal_rejection'] = failure.response; break
            if task['task']['state'] != 'completed':
                result['status'] = 'task_' + task['task']['state']; result['terminal_task'] = task
                break
            run_id = task['result']['data']['run_id']
            current = unwrap(call('get_run', {'run_id': run_id, 'query': {'kind': 'full_result'}}))['data']
            result['run_ids'].append(run_id); frozen[run_id] = copy.deepcopy(current)
            if current['computed_until_ms'] is None or current['computed_until_ms'] < scenario['window']['end_ms']:
                result['status'] = 'missing_conditions'; result['reason'] = 'Actual result does not cover the complete observation window'; break
            if len(result['run_ids']) > 1:
                result['comparisons'].append(unwrap(call('compare_runs', {'run_ids': result['run_ids'][-2:],
                    'metric_scope': {'keys': [], 'objects': [], 'acceptance': []}})))
            if current['computation_status'] == 'completed' and current['computed_until_ms'] >= scenario['window']['end_ms'] and current['goal_met'] is True:
                result['status'] = 'initial_target_met' if result['committed_repair_rounds'] == 0 else 'repaired_target_met'
                break
        if any(row['outcome'] == 'undetermined' for row in check['checks']):
            result['status'] = 'missing_conditions'; break
        if result['committed_repair_rounds'] == 3:
            result['status'] = 'finite_budget_exhausted'; break
        proposal = propose({'project': project, 'scenario': scenario, 'check': check,
            'run': current, 'render': result['renders'][-1], 'committed_repair_rounds': result['committed_repair_rounds']})
        objects = proposal['objects']; effective = []
        for candidate in objects:
            old = object_at_draft(call, candidate['kind'], candidate['data']['id'])
            if old != candidate['data']:
                effective.append({'before': old, 'after': copy.deepcopy(candidate['data']), 'kind': candidate['kind']})
        if not effective:
            result['status'] = 'unrepairable_shortfall'; result['reason'] = proposal['reason']; break
        commands = [{'kind': 'put_object', 'data': {'kind': change['kind'], 'data': change['after']}} for change in effective]
        response = call('apply_commands', {'base_revision': project['revision'], 'commands': commands})
        if response['outcome']['status'] != 'ok':
            result['status'] = 'edit_rejected'; result['rejected_proposal'] = {'response': response, 'changes': effective}; break
        commit = response['outcome']['data']
        result['committed_repair_rounds'] += 1
        result['changes'].append({'commit': commit, 'changes': effective, 'reason': proposal['reason']})
    result['render_complete'] = render_request is not None and bool(result['renders']) and all(item.get('task', {}).get('state') == 'completed' for item in result['renders'])
    for run_id, original in frozen.items():
        actual = unwrap(call('get_run', {'run_id': run_id, 'query': {'kind': 'full_result'}}))['data']
        if actual != original:
            raise ValueError('A prior frozen Run changed')
    project = object_at_draft(call, 'project', project_id)
    result['frozen_runs'] = frozen
    selected = select_directory('save_project', 'editable-project')
    result['save'] = complete(call, 'save_project', {'revision': project['revision'], 'selected_directory_ref': selected['token']})
    result['saved_directory'] = selected['path']
    result['delivery_complete'] = result['save']['task']['state'] == 'completed'
    if not result['delivery_complete']:
        return result
    if result['run_ids']:
        selected = select_directory('export_evaluation', 'evaluation')
        result['report'] = complete(call, 'export_evaluation', {'revision': project['revision'],
            'report_mode': 'simulation_evaluation', 'scenario_refs': [], 'run_refs': result['run_ids'],
            'metric_scope': {'keys': [], 'objects': [], 'acceptance': []}, 'formats': ['pdf', 'html', 'csv', 'json'],
            'selected_destination_ref': selected['token']})
        result['report_directory'] = selected['path']
        result['delivery_complete'] = result['report']['task']['state'] == 'completed'
    return result


EXECUTION_CONTRACT = 'sf.execution/2.0-draft11'
_EXECUTION_TASKS = {
    'start_execution_session': ('execution_preparation', 'execution_session'),
    'read_cloud_production': ('cloud_read', 'cloud_read'),
    'export_execution_report': ('exporting', 'execution_report'),
    'export_cloud_data': ('exporting', 'cloud_data_export'),
}
_GATEWAY_ONLY = frozenset(('apply_device_receipt', 'apply_device_observation',
                         'get_execution_cloud_work', 'acknowledge_execution_cloud_work'))


def execution_response(reply):
    """Read a real MCP result, including its original managed Response.

    A direct canonical Response is also accepted for the authenticated IPC caller.
    No result URI is fetched or simulated here.
    """
    if 'structuredContent' not in reply:
        return reply
    outer = reply['structuredContent']
    if outer['outcome']['status'] != 'managed':
        return outer
    import json
    originals = [part['resource']['text'] for part in reply.get('content', [])
                 if part.get('type') == 'resource'
                 and part['resource'].get('mimeType') == 'application/json'
                 and part['resource'].get('uri') == 'superfactory-result://' + outer['outcome']['data']['result_id']]
    if len(originals) != 1:
        raise ValueError('Managed result requires its actual original Response text')
    original = json.loads(originals[0])
    if original['outcome']['status'] == 'managed' or any(original[k] != outer[k] for k in ('api_version', 'request_id', 'context')):
        raise ValueError('Managed Response correlation differs')
    return original


class ExecutionWorkflow:
    """Explicit, finite H01-H11 consumer; never a scheduler or inventory runtime.

    send receives a complete canonical ApiRequest and calls the corresponding
    actual superfactory_<operation>. context/catalog must be read from that same
    authorized Host. advance() does a bounded number of Task reads (default one),
    not background polling. Keep pending across transport loss; never replay it.
    """
    def __init__(self, send, context, catalog, select_directory=None, pending=None):
        self.send = send
        self.context = copy.deepcopy(context)
        self.operations = {row['operation'] for row in catalog['implemented_operations']
                           if {'kind': 'project'} in row['allowed_scopes']} - _GATEWAY_ONLY
        self.select_directory = select_directory
        self.pending = copy.deepcopy(pending)
        self.configuration = None
        self.last_snapshot = None
        self.query = None

    def _request(self, operation, params):
        if operation not in self.operations:
            raise ValueError('Operation unavailable in actual authorized project catalog: ' + operation)
        return {'api_version': '0.1', 'request_id': 'agent-execution-' + str(uuid.uuid4()),
                'project_id': self.context['project_id'], 'scope': {'kind': 'project'},
                'operation': operation, 'params': copy.deepcopy(params)}

    def _exchange(self, request):
        reply = execution_response(self.send(copy.deepcopy(request)))
        if reply['api_version'] != request['api_version'] or reply['request_id'] != request['request_id']:
            raise ValueError('Response request identity differs')
        if reply['context'] is not None:
            for key in ('host_instance_id', 'session_id', 'project_id'):
                if reply['context'][key] != self.context[key]:
                    raise ValueError('Actual Host/caller/project context changed: ' + key)
        elif reply['outcome']['status'] != 'error':
            raise ValueError('Successful response has no actual Host context')
        return unwrap(reply)

    def _read(self, operation, params):
        return self._exchange(self._request(operation, params))

    def _admit(self, operation, params):
        if self.pending is not None:
            raise ValueError('Resolve or observe the original pending request; no duplicate submission')
        request = self._request(operation, params)
        self.pending = {'request': request, 'handle': None, 'status': 'registration_unknown'}
        try:
            handle = self._exchange(request)
        except HostError:
            self.pending = None  # An actual correlated rejection is known, not a lost admission.
            raise
        self._handle(handle, request)
        self.pending.update(handle=handle, status='admitted')
        return self.advance()

    def _handle(self, handle, request):
        operation = request['operation']
        if handle['kind'] != _EXECUTION_TASKS[operation][0]:
            raise ValueError('Task kind differs from original operation')
        if handle['scope'] != {'kind': 'edit', 'data': {'kind': 'project', 'data': self.context['project_id']}}:
            raise ValueError('Task belongs to another Project scope')
        params = request['params']
        revision = params.get('base_revision', params.get('revision'))
        if revision is not None and handle['input_revision'] != revision:
            raise ValueError('Task frozen revision differs from original request')

    def advance(self, max_reads=1):
        """Observe only the admitted original Task; budget exhaustion is pending."""
        if self.pending is None:
            raise ValueError('No original Task to observe')
        saved = self.pending
        if saved['handle'] is None:
            return {'status': 'registration_unknown', 'pending': copy.deepcopy(saved)}
        request, handle = saved['request'], saved['handle']
        for _ in range(max_reads):
            state = self._read('get_task', {'task_id': handle['id']})
            if state['originating_operation'] != request['operation'] or state['originating_request_id'] != request['request_id']:
                raise ValueError('Task operation/original request differs')
            if any(state['task'][key] != handle[key] for key in ('id', 'scope', 'kind', 'input_revision')):
                raise ValueError('Task original handle changed')
            if state['task']['state'] not in ('completed', 'failed', 'cancelled'):
                continue
            if state['task']['state'] != 'completed':
                self.pending = None
                return {'status': state['task']['state'], 'task_state': state}
            if state['result'] is None or state['result']['kind'] != _EXECUTION_TASKS[request['operation']][1]:
                raise ValueError('Completed Task output differs from original operation')
            data = state['result']['data']
            self._result(request, data)
            self.pending = None
            return {'status': 'completed', 'task_state': state, 'data': data}
        return {'status': 'pending', 'pending': copy.deepcopy(saved)}

    def recover(self):
        """H09 original-request lookup; not_found remains unresolved, never resends."""
        if self.pending is None:
            raise ValueError('No original request to recover')
        request = self.pending['request']
        response = self._read('read_execution_view', {'execution_contract_version': EXECUTION_CONTRACT,
            'selection': {'kind': 'request', 'originating_request_id': request['request_id']}})
        if response['kind'] != 'request':
            raise ValueError('Recovery returned another view')
        recovery = response['data']
        if recovery['originating_request_id'] != request['request_id'] or recovery['host_context_session_id'] != self.context['host_instance_id']:
            raise ValueError('Recovery belongs to another original request/Host')
        if recovery['state'] == 'not_found':
            return {'status': 'registration_unknown', 'pending': copy.deepcopy(self.pending), 'recovery': recovery}
        if request['operation'] not in _EXECUTION_TASKS or recovery['operation'] != request['operation']:
            raise ValueError('Recovery returned another originating operation')
        self._handle(recovery['task'], request)
        self.pending.update(handle=recovery['task'], status='admitted')
        return self.advance()

    def cloud(self, selection):
        return self._admit('read_cloud_production', {'execution_contract_version': EXECUTION_CONTRACT, 'selection': selection})

    def prepare(self, policy, *, confirmed=False):
        if not confirmed:
            raise ValueError('Preparing a new execution requires explicit user authorization')
        if self.configuration is None:
            raise ValueError('Read actual H08 configuration/issues first')
        config = self.configuration
        return self._admit('start_execution_session', {'execution_contract_version': EXECUTION_CONTRACT,
            'base_revision': config['project_revision'], 'scenario_id': config['scenario_id'],
            'scenario_revision': config['scenario_revision'], 'candidate_bindings': config['candidate_bindings'],
            'policy': policy})

    def snapshot(self, identity, selection):
        envelope = self._read('get_execution_snapshot', {'execution_contract_version': EXECUTION_CONTRACT,
            'identity': identity, 'selection': selection})
        snapshot = envelope['snapshot']
        if snapshot['identity'] != identity or (selection['kind'] == 'revision' and snapshot['execution_revision'] != selection['execution_revision']):
            raise ValueError('Snapshot differs from original identity/exact R')
        self.last_snapshot = {'envelope': copy.deepcopy(envelope), 'selection': copy.deepcopy(selection)}
        return envelope

    def control(self, action, *, confirmed=False):
        if not confirmed or self.last_snapshot is None:
            raise ValueError('Explicit action and an actual latest Host snapshot are required')
        if self.pending is not None:
            raise ValueError('Unresolved original request; control is not resubmitted')
        current = self.last_snapshot
        envelope, selection = current['envelope'], current['selection']
        access = envelope['access']
        if selection['kind'] != 'latest' or envelope['delivery']['read_origin'] != 'host' or access['ownership'] != 'current_host_owner':
            raise ValueError('Control requires actual latest Host-owner access')
        if action['kind'] == 'set_device_acceptance':
            allowed = action['device_id'] in access['device_acceptance_control_ids']
        else:
            allowed = action['kind'] in access['allowed_session_controls']
        if not allowed:
            raise ValueError('This action is absent from actual execution access')
        original = envelope['snapshot']
        request = self._request('control_execution_session', {'execution_contract_version': EXECUTION_CONTRACT,
            'identity': original['identity'], 'expected_execution_revision': original['execution_revision'], 'action': action})
        self.pending = {'request': request, 'handle': None, 'status': 'registration_unknown'}
        try:
            ack = self._exchange(request)
        except HostError as error:
            self.pending = None
            if error.response['outcome']['data']['code'] == 'RevisionConflict':
                refreshed = self.snapshot(original['identity'], {'kind': 'latest'})
                return {'status': 'revision_conflict', 'response': error.response, 'refreshed': refreshed}
            raise
        if ack['identity'] != original['identity'] or ack['action'] != action:
            raise ValueError('Acknowledgement differs from original action/identity')
        self.pending = None
        self.last_snapshot = None  # Read the actual new R before any further action.
        return {'status': 'acknowledged', 'data': ack}

    @staticmethod
    def _view(selection, result):
        if result['kind'] != selection['kind']:
            raise ValueError('Execution view selection/result differs')
        data = result['data']
        if selection['kind'] == 'inputs' and data['identity'] != selection['identity']:
            raise ValueError('Frozen inputs belong to another execution')
        if selection['kind'] in ('observations', 'history'):
            query = selection['query']
            keys = ('identity', 'execution_revision', 'at_host_ms') if selection['kind'] == 'observations' else ('identity', 'execution_revision', 'host_from_ms', 'host_until_ms')
            if any(data[key] != query[key] for key in keys):
                raise ValueError('Page differs from exact identity/R/T')
        return data

    def view(self, selection):
        result = self._read('read_execution_view', {'execution_contract_version': EXECUTION_CONTRACT, 'selection': selection})
        self._view(selection, result)
        return result

    def view_pages(self, selection, max_pages=20):
        """Full fixed-filter pages within a caller budget; partial stays partial."""
        selection = copy.deepcopy(selection)
        pages, frontier, cursors, last_sequence, last_time = [], None, set(), None, None
        for _ in range(max_pages):
            data = self.view(selection)['data']
            if frontier is None:
                frontier = data['observation_frontier']
            if data['observation_frontier'] != frontier:
                raise ValueError('Page frontier changed at fixed R/T')
            if selection['kind'] == 'history':
                for record in data['records']:
                    sequence, at = record['record_sequence'], record['at_host_ms']
                    if record['execution_revision'] > selection['query']['execution_revision'] or not selection['query']['host_from_ms'] <= at <= selection['query']['host_until_ms']:
                        raise ValueError('History record lies outside selected R/time')
                    if last_sequence is not None and (sequence <= last_sequence or (not selection['query']['objects'] and sequence != last_sequence + 1)):
                        raise ValueError('Original history sequence has a gap/reversal; not sorted or repaired')
                    if last_time is not None and at < last_time:
                        raise ValueError('Original history time reversed')
                    last_sequence, last_time = sequence, at
            pages.append(data)
            cursor = data['next_cursor']
            if cursor is None:
                return {'complete': True, 'pages': pages, 'next_selection': None}
            if cursor in cursors:
                raise ValueError('Page cursor repeated; not declaring complete')
            cursors.add(cursor)
            selection['query']['page']['cursor'] = cursor
        return {'complete': False, 'pages': pages, 'next_selection': selection}

    def _result(self, request, data):
        operation, params = request['operation'], request['params']
        if operation == 'start_execution_session':
            if data['state'] != 'prepared' or data['identity']['project_id'] != self.context['project_id'] or data['identity']['host_context_session_id'] != self.context['host_instance_id']:
                raise ValueError('Preparation output is not this Host/project prepared execution')
        elif operation == 'read_cloud_production':
            selection = params['selection']
            expected = 'query_result' if selection['kind'] == 'data_query' else selection['kind']
            if data['kind'] != expected:
                raise ValueError('Cloud result kind differs from original selection')
            value = data['data']
            if expected == 'configuration':
                if any(value[key] != selection[key] for key in ('scenario_id', 'scenario_revision')) or value['project_revision'] != selection['base_revision']:
                    raise ValueError('Configuration is not the selected frozen draft revisions')
                self.configuration = copy.deepcopy(value)
            elif expected in ('inputs', 'observations', 'history'):
                self._view(selection, data)
            elif expected == 'snapshot':
                if value['snapshot']['identity'] != selection['identity'] or (selection['selection']['kind'] == 'revision' and value['snapshot']['execution_revision'] != selection['selection']['execution_revision']):
                    raise ValueError('Cloud snapshot identity/exact R differs')
                self.last_snapshot = {'envelope': copy.deepcopy(value), 'selection': copy.deepcopy(selection['selection'])}
            elif expected == 'query_result':
                self._query_result(selection['query'], value)
        elif operation == 'export_cloud_data':
            if any(data[key] != params[key] for key in ('query', 'selection')) or data['destination'] != params['selected_destination_ref']:
                raise ValueError('Export result changed frozen filter/selection/destination')
        elif operation == 'export_execution_report':
            if data['destination'] != params['selected_destination_ref'] or data['prediction'] != params['prediction'] or len(data['sources']) != len(params['sources']):
                raise ValueError('Report destination/source/prediction differs')
            for original, actual in zip(params['sources'], data['sources']):
                if any(original[key] != actual[key] for key in ('read_origin', 'identity', 'execution_revision')):
                    raise ValueError('Report source identity/R/origin differs')

    def query_first_page(self, query):
        if self.pending is not None:
            raise ValueError('Resolve the original pending request before selecting another query')
        if query['page']['cursor'] is not None:
            raise ValueError('First query must start at the original first page')
        if query['dataset'] == 'execution' and query['selection']['kind'] != 'revision':
            raise ValueError('Select exact execution R before querying')
        self.query = {'original': copy.deepcopy(query), 'stamp': None, 'next_cursor': None, 'pages': 0, 'complete': False}
        return self.cloud({'kind': 'data_query', 'query': query})

    def _query_result(self, query, result):
        if result['dataset'] != query['dataset']:
            raise ValueError('Dataset differs from original query')
        if query['dataset'] == 'execution':
            if result['selected_epoch'] != query['identity']['execution_epoch_id'] or result['selected_execution_revision'] != query['selection']['execution_revision']:
                raise ValueError('Dataset epoch/exact R differs')
            stamp = {key: result[key] for key in ('dataset', 'selected_epoch', 'selected_execution_revision')}
        else:
            if result['dataset_id'] != query['dataset_id'] or (query['device_ids'] and sorted(result['selected_device_ids']) != sorted(query['device_ids'])):
                raise ValueError('Telemetry dataset/selected devices differs')
            stamp = {key: result[key] for key in ('dataset', 'dataset_id', 'selected_visible_frontier', 'selected_device_ids')}
        if self.query is not None:
            expected = copy.deepcopy(self.query['original'])
            expected['page']['cursor'] = self.query['next_cursor']
            if query != expected or (self.query['stamp'] is not None and stamp != self.query['stamp']):
                raise ValueError('Fixed query filters/frontier changed')
            cursor = result['next_cursor']
            if cursor is not None and cursor == query['page']['cursor']:
                raise ValueError('Dataset cursor did not advance')
            self.query.update(stamp=stamp, next_cursor=cursor, pages=self.query['pages'] + 1, complete=cursor is None)

    def query_next_page(self):
        if self.query is None or not self.query['pages'] or self.query['complete']:
            raise ValueError('No admitted next page')
        query = copy.deepcopy(self.query['original'])
        query['page']['cursor'] = self.query['next_cursor']
        return self.cloud({'kind': 'data_query', 'query': query})

    def _destination(self, purpose):
        if self.pending is not None:
            raise ValueError('Resolve the original pending request before selecting another destination')
        if self.select_directory is None:
            raise ValueError('Actual Native Agent directory selection is unavailable')
        selected = self.select_directory(purpose)
        if selected is None or selected.get('purpose') != purpose or not selected.get('selection_id') or set(selected) != {'selection_id', 'purpose'}:
            raise ValueError('Directory selection missing or wrong purpose; no export submitted')
        return copy.deepcopy(selected)  # The actual Host verifies ownership; never mint a token/path.

    def report(self, formats=('pdf', 'html', 'csv', 'json'), prediction=None, *, confirmed=False):
        if not confirmed or self.last_snapshot is None:
            raise ValueError('Select an actual fixed source and explicitly authorize export')
        self._request('export_execution_report', {})  # Availability before opening a selector.
        envelope = self.last_snapshot['envelope']
        snapshot = envelope['snapshot']
        return self._admit('export_execution_report', {'execution_contract_version': EXECUTION_CONTRACT,
            'revision': self.context['project_revision'], 'sources': [{'read_origin': envelope['delivery']['read_origin'],
                'identity': snapshot['identity'], 'execution_revision': snapshot['execution_revision']}],
            'formats': list(formats), 'selected_destination_ref': self._destination('export_execution_report'), 'prediction': prediction})

    def export_data(self, *, confirmed=False):
        if not confirmed or self.query is None or not self.query['pages']:
            raise ValueError('Read the first fixed-filter page and explicitly authorize complete export')
        self._request('export_cloud_data', {})
        query, stamp = copy.deepcopy(self.query['original']), self.query['stamp']
        query['page'] = {'limit': 200, 'cursor': None}
        if query['dataset'] == 'telemetry':
            query['device_ids'] = stamp['selected_device_ids']
            selection = {'dataset': 'telemetry', 'cloud_project_id': query['cloud_project_id'],
                'dataset_id': stamp['dataset_id'], 'selected_visible_frontier': stamp['selected_visible_frontier'],
                'selected_device_ids': stamp['selected_device_ids']}
        else:
            selection = {'dataset': 'execution', 'identity': query['identity'], 'execution_revision': stamp['selected_execution_revision']}
        return self._admit('export_cloud_data', {'execution_contract_version': EXECUTION_CONTRACT,
            'revision': self.context['project_revision'], 'query': query, 'selection': selection,
            'selected_destination_ref': self._destination('export_cloud_data')})

    def exit_status(self, identity):
        """Only stop observing: read unresolved obligations without stop/cancel."""
        actual = self.snapshot(identity, {'kind': 'latest'})
        snapshot = actual['snapshot']
        unresolved = {'commands': snapshot['lifecycle']['unresolved_command_ids'],
            'pending_outputs': snapshot['business_facts']['pending_output_refs'],
            'reservations': snapshot['pending_reservations']}
        settled = not any(unresolved.values())
        return {'observing_stopped': True, 'production_stopped_by_this_call': False,
            'lifecycle_state': snapshot['lifecycle']['state'], 'goal_at_snapshot': snapshot['goal_at_snapshot'],
            'terminal_and_settled': snapshot['lifecycle']['state'] in ('completed', 'stopped', 'failed') and settled,
            'unsettled_obligations': unresolved, 'actual': actual,
            'pending_original_request': copy.deepcopy(self.pending)}
