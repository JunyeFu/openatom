"""C01—04隔离语义/候选载体核验；不执行真实Host，不写公共生成物。"""
from copy import deepcopy
from pathlib import Path
import json
import sys

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
DOC = ROOT / 'docs/04_开发与验收/特征细化'
sys.path.insert(0, str(ROOT / 'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator
from DRF03_旧清单适配试做 import duration


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def candidate(category):
    return read(BASE / 'adapter-output' / category / 'module.json')['data']


def upgrade(inp):
    current = {'revision': 1, 'project_module_refs': ['0.1.0'], 'A': {'version': '0.1.0', 'override': 40}, 'B': {'version': '0.1.0'}, 'stock': 4}
    old_run = deepcopy(current)
    staged = deepcopy(current)
    staged['project_module_refs'].append('0.2.0')
    staged['A']['version'] = '0.2.0'
    if inp['fail'] == 'revision':
        result = 'RevisionConflict'
    elif inp['fail'] == 'unit':
        result = 'InvalidInput'
    else:
        assert inp['fail'] is None
        staged['revision'] += 1
        current = staged
        result = 'committed_finite_model'
    if result != 'committed_finite_model':
        assert current == old_run and current['project_module_refs'] == ['0.1.0']
    return {'result': result, 'A': current['A']['version'], 'B': current['B']['version'], 'old_run_A': old_run['A']['version'],
            'override': current['A']['override'], 'project_revision': current['revision'], 'stock': current['stock']}


def derive(inp):
    # Finite hypothetical context, explicitly not a registered artifact/provider.
    selection = inp.get('selection', True)
    if not selection:
        return 'missing_job_selection'
    desired_revision = inp.get('desired_revision', 1)
    if inp.get('current_subject_revision', 'r1') != 'r1':
        return 'subject_mismatch'
    if desired_revision != 1:
        return 'job_revision_mismatch'
    if inp.get('artifact_input_thickness', 1) != 1:
        return 'immutable_input_mismatch'
    profile = read(DOC / 'DRF03_C02_专业绑定推导.json')
    purposes = ['forming', 'opening', 'linear_extraction', 'net_space', 'manufacturing']
    requested = inp.get('requested_scope', purposes)
    if not set(purposes) <= set(requested):
        return 'scope_cannot_reduce_complete_profile'
    assert len(profile['payload_derivation']) == 6
    if not inp.get('has_glue', True):
        return 'required_payload_missing'
    if not inp.get('ack_host_recorded', True):
        return 'ack_not_host_recorded'
    return 'eligible_selected_profile_only'


def selected_sample(inp, defs):
    if inp.get('old_named_pose'):
        sample = {'id': 'pose-1', 'node_transforms': []}
    else:
        job = read(ROOT / 'tools/packaging-algorithm-proof/output/local-contact-job.json')['data']
        panels = [{'panel_id': panel['id'], 'pose': deepcopy(next(p for p in job['parts'] if p['id'] == panel['part_id'])['placement'])} for panel in job['panels']]
        sample = {'step_id': inp.get('step_id', job['steps'][0]['id']), 'progress': inp.get('progress', 0), 'panel_poses': panels}
        if inp.get('reverse_panels'):
            panels.reverse()
        if inp.get('missing_panel'):
            panels.pop()
        if inp.get('duplicate_panel'):
            panels.append(deepcopy(panels[0]))
        if 'pose_unit' in inp:
            panels[0]['pose']['unit'] = inp['pose_unit']
    fragment = read(DOC / 'DRF03_C03_过程样本Schema片段.json')
    # After owner codegen, consume the real canonical carrier rather than shadowing it.
    active_defs = defs if 'PackagingProcessSample' in defs else {**defs, **fragment['$defs']}
    schema = {'$schema': fragment['$schema'], '$defs': active_defs, '$ref': fragment['$ref']}
    if list(Draft202012Validator(schema).iter_errors(sample)):
        return 'invalid_shape'
    # Canonical shape admits FiniteNumber; range/foreign keys are separate semantic checks.
    if not 0 <= sample['progress'] <= 1:
        return 'invalid_shape'
    job = read(ROOT / 'tools/packaging-algorithm-proof/output/local-contact-job.json')['data']
    if sample['step_id'] not in {step['id'] for step in job['steps']}:
        return 'unknown_step'
    ids = [entry['panel_id'] for entry in sample['panel_poses']]
    if len(ids) != len(set(ids)):
        return 'duplicate_panel'
    if set(ids) != {panel['id'] for panel in job['panels']}:
        return 'panel_set_mismatch'
    if any(entry['pose']['unit'] != 'millimeter' or entry['pose']['coordinate_frame'] != 'right_handed_z_up'
           or entry['pose']['scale'] != {'x': 1, 'y': 1, 'z': 1} for entry in sample['panel_poses']):
        return 'pose_frame_or_unit_mismatch'
    return 'valid_selected_sample'


def check(case, defs):
    kind, inp = case['kind'], case['input']
    if kind == 'upgrade':
        return upgrade(inp)
    if kind == 'task_lowering':
        task = candidate('tasks')['definition']['payload']['data']
        return {'work_kind': 'preparation', 'duration_ms': task['duration']['value'], 'target': 'D1',
                'resource_role': task['resource_roles'][0]['role'], 'material_delta': int(task['completion_effect']['produces_material'])}
    if kind == 'transport_lowering':
        times = candidate('transport')['definition']['payload']['data']['times']
        at_accept = sum(times[key]['value'] for key in ['loading', 'outbound', 'unloading'])
        return {'accept_at_ms': at_accept, 'carrier_free_at_ms': at_accept + times['returning']['value'], 'cargo_ids': ['C1']}
    if kind == 'derive':
        return derive(inp)
    if kind == 'sample':
        return selected_sample(inp, defs)
    if kind == 'adapter_identity':
        registry = read(ROOT / 'packages/module-kit/category-contracts.design.json')
        names = [c['id'] for c in registry['categories']]
        return {'candidates': len(names), 'identities_preserved': all(candidate(c['id'])['definition']['id'] == read(ROOT/'modules'/c['example_manifest'])['id'] for c in registry['categories']),
                'all_design': all(candidate(name)['definition']['definition_state'] == 'design' for name in names)}
    if kind == 'adapter_transport':
        data = candidate('transport')['definition']['payload']['data']
        return {'times_ms': [data['times'][key]['value'] for key in ['loading', 'outbound', 'unloading', 'returning']],
                'batch_coefficient': data['batch_capacity']['value']['coefficient'], 'batch_unit': data['batch_capacity']['unit']}
    if kind == 'adapter_operation':
        data = candidate('operations')['definition']['payload']['data']
        return {'inputs': [x['quantity']['value']['coefficient'] for x in data['inputs']], 'outputs': [x['quantity']['value']['coefficient'] for x in data['outputs']],
                'duration_ms': data['duration']['value'], 'identity': data['identity_policy']}
    if kind == 'adapter_template':
        raw = read(BASE/'adapter-output/templates/sources/project.design.json')
        original = read(ROOT/'modules/templates/parallel-electromechanical/project.design.json')
        blocked = any(issue['code'] == 'template_inner_file_not_canonical' for issue in candidate('templates')['issues'])
        return {'canonical_inner_ready': not blocked, 'stores_preserved': len(raw['stores']), 'connections_preserved': len(raw['connections']), 'raw_equal': raw == original}
    if kind == 'adapter_duration':
        try:
            duration({'value': inp['seconds'], 'unit': 'second'}, 'template_declared_example')
        except ValueError as exc:
            return str(exc)
        raise AssertionError('Unrepresentable duration was silently rounded.')
    if kind == 'adapter_professional':
        data = candidate('professional')
        payload = data['definition']['payload']['data']
        return {'artifact_type': payload['output_artifact_type'], 'has_available_modes': 'available_modes' in payload,
                'has_ack': 'acknowledgments' in data, 'blocked': any('execution' in issue['blocks'] for issue in data['issues'])}
    if kind == 'adapter_parameters':
        device = candidate('devices')['definition']
        cycle = next(p for p in device['parameters'] if p['id'] == 'cycle_seconds')
        return {'cycle_unit': cycle['unit'], 'cycle_binding': cycle['binding'], 'cycle_value_ms': device['payload']['data']['behavior']['inline']['duration']['value'],
                'transport_parameter_count': len(candidate('transport')['definition']['parameters'])}
    if kind == 'adapter_media':
        media = candidate('media')['definition']['payload']['data']
        source_nodes = [node for node in media['nodes'] if not node['name'].startswith('anchor:')]
        cylinder = next(node for node in source_nodes if node['geometry']['kind'] == 'cylinder')['geometry']['data']
        return {'source_nodes': len(source_nodes), 'cylinder_radius': cylinder['radius'], 'cylinder_height': cylinder['height'], 'named_poses': len(media['poses']), 'production_effect': media['production_effect']}
    raise AssertionError(kind)


def request_shapes():
    template = read(ROOT/'tools/type-contract-proof/api-fixtures/apply_commands.request.json')
    project = read(ROOT/'tools/type-contract-proof/fixtures/project.json')['data']
    scenario = read(ROOT/'tools/type-contract-proof/fixtures/scenario.json')['data']
    device = deepcopy(project['devices'][0])
    device['module_ref']['version'] = '0.2.0'
    commands = [
        {'kind': 'adopt_module', 'data': {'module_ref': device['module_ref'], 'targets': [{'kind': 'device', 'id': device['id']}],
                                       'bindings': device['role_bindings'], 'new_instances': [], 'explicit_seed': None}},
        {'kind': 'put_object', 'data': {'kind': 'device', 'data': device}},
    ]
    task_ref = {'id': candidate('tasks')['definition']['id'], 'version': candidate('tasks')['definition']['version']}
    route = deepcopy(project['routes'][0])
    route['steps'][0]['operation_ref'] = task_ref
    route['steps'][0]['resource_bindings'] = {'setter': project['resources'][0]['id']}
    task_device = deepcopy(project['devices'][0])
    task_device['role_bindings']['targets']['board_station'] = {'kind': 'device', 'id': task_device['id']}
    task_commands = [
        {'kind': 'adopt_module', 'data': {'module_ref': task_ref, 'targets': [{'kind': 'route', 'id': route['id']}], 'bindings': task_device['role_bindings'], 'new_instances': [], 'explicit_seed': None}},
        {'kind': 'put_object', 'data': {'kind': 'route', 'data': route}},
        {'kind': 'put_object', 'data': {'kind': 'device', 'data': task_device}},
    ]
    supply = {'id': 'TRIAL-SUPPLY', 'source': 'TRIAL-SOURCE', 'destination': 'TRIAL-DEST', 'item_type_ref': {'id': 'T1', 'revision': 'r1'},
              'target_quantity': {'value': {'coefficient': 1, 'decimal_places': 0}, 'unit': 'piece'}, 'window': {'start_ms': 0, 'end_ms': 80000},
              'transport_ref': {'id': candidate('transport')['definition']['id'], 'version': candidate('transport')['definition']['version']},
              'resources': {'carrier': 'TRIAL-CARRIER'}, 'departure_windows': [], 'allow_partial_load': False}
    scenario['supplies'] = [supply]
    transport_commands = [{'kind': 'put_object', 'data': {'kind': 'scenario', 'data': scenario}}]
    schema = read(ROOT/'tools/type-contract-proof/output/api-schemas/apply_commands.request.schema.json')
    validator = Draft202012Validator(schema)
    names = []
    for name, data in [('device-upgrade', commands), ('task-route', task_commands), ('transport-scenario', transport_commands)]:
        request = deepcopy(template)
        request['request_id'] = 'DRF03-' + name
        request['params']['commands'] = data
        validator.validate(request)
        (BASE / ('DRF03_' + name + '.request.json')).write_text(json.dumps(request, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
        names.append(name)
    invalid = deepcopy(template)
    invalid['params']['commands'] = [{'kind': 'put_object', 'data': {'kind': 'supply', 'data': supply}}]
    assert list(validator.iter_errors(invalid))
    return {'positive_request_shapes': names, 'direct_PutObject_Supply_rejected': True,
            'scope': 'shape only; trial object IDs/resource compatibility do not prove Host admission or execution'}


def main():
    defs = read(ROOT/'tools/type-contract-proof/output/schemas/all-types.schema.json')['$defs']
    expected = read(BASE/'DRF03_第二轮独立预期.json')
    results = []
    for case in expected['cases']:
        actual = check(case, defs)
        assert case['actual_product_result'] is None
        results.append({'id': case['id'], 'expected': case['expected'], 'actual_isolated_reference': actual, 'matched': actual == case['expected'], 'actual_product_result': None})
    shapes = request_shapes()
    carrier = defs['PackagingResult']['properties']['process_poses']['items']['$ref'].rsplit('/', 1)[-1]
    canonical_result_check = None
    if carrier == 'PackagingProcessSample':
        job = read(ROOT/'tools/packaging-algorithm-proof/output/local-contact-job.json')['data']
        fixture = read(ROOT/'tools/type-contract-proof/fixtures/packaging-result.json')
        assert fixture['data']['evidence_origin'] == 'type_fixture'
        fixture['data']['job_ref'] = {'id': job['id'], 'revision': job['revision']}
        fixture['data']['subject_ref'] = deepcopy(job['subject_ref'])
        panels = [{'panel_id': panel['id'], 'pose': deepcopy(next(part for part in job['parts'] if part['id'] == panel['part_id'])['placement'])} for panel in job['panels']]
        fixture['data']['process_poses'] = [{'step_id': job['steps'][0]['id'], 'progress': 0, 'panel_poses': panels}]
        canonical_schema = read(ROOT/'tools/type-contract-proof/output/schemas/packaging-result.schema.json')
        validator = Draft202012Validator(canonical_schema)
        validator.validate(fixture)
        legacy = deepcopy(fixture)
        legacy['data']['process_poses'] = [{'id': 'legacy-pose', 'node_transforms': []}]
        assert list(validator.iter_errors(legacy))
        path = BASE/'DRF03_C03_正式结果载体夹具.json'
        path.write_text(json.dumps(fixture, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
        canonical_result_check = {'actual_current_result_schema_positive': True, 'old_NamedPose_negative_rejected': True,
                                  'fixture_evidence_origin': 'type_fixture', 'geometric_or_runtime_result_created': False}
    result = {'document_kind': 'DRF03_second_round_actual_isolated_trials_not_product', 'cases': results, 'case_count': len(results),
              'all_independent_expectations_matched': all(case['matched'] for case in results), 'current_public_process_carrier': carrier,
              'selected_C03_schema_used': carrier != 'PackagingProcessSample', 'canonical_C03_merged': carrier == 'PackagingProcessSample', 'request_shapes': shapes,
              'canonical_C03_result_checks': canonical_result_check,
              'legacy_adapter_result_ref': 'scenarios/九类模块采用/DRF03_适配试做结果.json', 'template_inner_converted': False,
              'previous_22_cases_changed': False, 'S06_04_expectations_changed': False, 'actual_Host_or_Engine': None,
              'actual_product_result': None, 'shared_files_written': False}
    (BASE/'DRF03_第二轮核验结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({key: result[key] for key in ['case_count', 'all_independent_expectations_matched', 'current_public_process_carrier', 'canonical_C03_merged', 'template_inner_converted', 'actual_Host_or_Engine']}, ensure_ascii=False))
    if not result['all_independent_expectations_matched']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
