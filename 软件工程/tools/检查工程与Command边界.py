"""Verify the actual product-generated schemas with frozen finite inputs.

No Host handlers, authorization decisions or product acceptance are exercised.
"""
from copy import deepcopy
import argparse
from pathlib import Path
import json
import sys

ROOT = Path(__file__).resolve().parent.parent
PROOF = ROOT / 'tools/type-contract-proof'
sys.path.insert(0, str(PROOF / 'python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', default='.work/p0-01-persistence-history/schema-validation-result.json')
    args = parser.parse_args()
    generated = ROOT / 'packages/commands/generated'
    schemas = {p.name.removesuffix('.schema.json'): read(p)
               for p in (generated / 'schemas').glob('*.schema.json')}
    # All eighteen frozen wire operations have derived request/response schemas.
    # Their existence does not advertise an implemented Host handler.
    assert len(schemas) == 4 + 18 * 2
    for schema in schemas.values():
        Draft202012Validator.check_schema(schema)
    cases = []

    def check(case_id, schema_name, value, expected):
        accepted = Draft202012Validator(schemas[schema_name]).is_valid(value)
        assert accepted is expected, (case_id, schema_name, accepted)
        cases.append({'id': case_id, 'schema': schema_name, 'accepted': accepted,
                      'expected_accepted': expected})

    project = read(PROOF / 'fixtures/project.json')
    check('project', 'project', project, True)
    check('manifest', 'manifest', read(PROOF / 'fixtures/manifest.json'), True)
    check('scenario', 'scenario', read(PROOF / 'fixtures/scenario.json')['data'], True)
    legacy_scenario = read(PROOF / 'fixtures/scenario.json')['data']
    legacy_scenario['acceptance_goal'] = None
    check('scenario-legacy-null-goal', 'scenario', legacy_scenario, True)
    manual_goal = read(PROOF / 'fixtures/scenario.json')['data']
    manual_goal['acceptance_goal'] = read(PROOF / 'api-fixtures/check_factory_draft.request.json')['params']['generation_goal']
    check('scenario-explicit-manual-goal', 'scenario', manual_goal, True)
    missing_unit = deepcopy(manual_goal)
    del missing_unit['acceptance_goal']['acceptance_scope']['unit']
    check('scenario-manual-goal-requires-acceptance-unit', 'scenario', missing_unit, False)
    for category in ['items', 'devices', 'resources', 'tasks', 'operations',
                     'transport', 'templates', 'professional', 'media']:
        check('module-' + category, 'module', read(PROOF / f'fixtures/module-{category}.json'), True)
    for op in ['describe_capabilities', 'get_context', 'get_object', 'apply_commands',
               'save_project', 'import_package', 'get_task', 'cancel_task',
               'render_view', 'run_scenario', 'get_run', 'compare_runs',
               'compute_packaging', 'adopt_artifact', 'export_package',
               'export_evaluation', 'query_module_library', 'check_factory_draft']:
        for direction in ['request', 'response']:
            name = f'{op}.{direction}'
            check(name, name, read(PROOF / f'api-fixtures/{name}.json'), True)
    for key in ['id', 'revision', 'views', 'module_refs', 'time_reference']:
        value = deepcopy(project)
        del value['data'][key]
        check('missing-project-' + key, 'project', value, False)
    for invalid in ['r1', -1, 1.5, 9007199254740992]:
        value = deepcopy(project)
        value['data']['revision'] = invalid
        check('bad-project-revision-' + str(invalid), 'project', value, False)
    value = deepcopy(project)
    value['data']['runtime_inventory'] = []
    check('undeclared-inventory', 'project', value, False)
    request = read(PROOF / 'api-fixtures/apply_commands.request.json')
    for field in ['project_id', 'operation', 'params']:
        value = deepcopy(request)
        del value[field]
        check('missing-request-' + field, 'apply_commands.request', value, False)
    value = deepcopy(request)
    value['caller'] = {'origin': 'interactive_ui'}
    check('caller-authority', 'apply_commands.request', value, False)
    value = deepcopy(request)
    value['params']['commands'][0]['data']['edit_grants'] = []
    check('nested-authority', 'apply_commands.request', value, False)
    for op, field in [('save_project', 'selected_directory_ref'),
                      ('import_package', 'selected_file_ref')]:
        value = read(PROOF / f'api-fixtures/{op}.request.json')
        value['params'][field]['path'] = 'C:/untrusted/target'
        check(op + '-reject-wire-path', op + '.request', value, False)
        value = read(PROOF / f'api-fixtures/{op}.request.json')
        del value['params'][field]['selection_id']
        check(op + '-requires-host-token', op + '.request', value, False)
    value = read(PROOF / 'api-fixtures/save_project.request.json')
    value['params']['revision'] = -1
    check('save-requires-valid-revision', 'save_project.request', value, False)
    value = deepcopy(project)
    value['data']['devices'][0]['route_operation_refs'] = [
        {'id': 'demo.operations', 'version': '0.1.0'}]
    check('device-explicit-route-operation', 'project', value, True)
    del value['data']['devices'][0]['route_operation_refs'][0]['version']
    check('device-route-operation-requires-exact-version', 'project', value, False)
    value = read(PROOF / 'fixtures/module-transport.json')
    transport = value['data']['definition']['payload']['data']
    transport['loading_resources'] = deepcopy(transport['resources'])
    transport['unloading_resources'] = deepcopy(transport['resources'])
    check('transport-explicit-phase-resources', 'module', value, True)
    del transport['unloading_resources'][0]['quantity']['unit']
    check('transport-phase-resource-requires-unit', 'module', value, False)
    value = deepcopy(request)
    command = {'template_ref': {'id': 'demo.templates', 'version': '0.1.0'},
               'subset': read(PROOF / 'fixtures/template.json')['data'],
               'mapped_bindings': {'item_types': {}, 'resources': {}, 'targets': {}, 'ports': {}},
               'proposed_objects': [], 'explicit_new_scenario_seed': None}
    value['params']['commands'] = [{'kind': 'instantiate_template', 'data': command}]
    check('template-legacy-no-preparation', 'apply_commands.request', value, True)
    command['preparation'] = None
    check('template-null-preparation', 'apply_commands.request', value, True)
    command['preparation'] = {
        'source_project': 'example-project.json', 'namespace': 'schema-example',
        'resources': {'new-worker': None, 'existing-worker': 'W1'},
        'reuse_items': {'source-item': 'T1'}, 'include_example_product': True,
        'site_envelope': None}
    check('template-explicit-source-and-resource-choices', 'apply_commands.request', value, True)
    command['preparation']['source_records'] = []
    check('template-no-caller-source-record-channel', 'apply_commands.request', value, False)
    del command['preparation']['source_records']
    command['preparation']['source_project'] = '../outside-project.json'
    check('template-source-project-is-relative-contained-path', 'apply_commands.request', value, False)
    quality_scenario = read(PROOF / 'fixtures/scenario.json')['data']
    decision = {
        'id': 'explicit-recheck', 'item_ref': 'actual-item-id',
        'inspection_ref': {'id': 'exact.inspection', 'version': '1.0.0'},
        'source_kind': 'preset', 'observed_result': None,
        'disposition': 'reinspect', 'effective_at_ms': 80000,
        'decision_source': {'kind': 'scheduled_preset', 'data': 'declared-recheck'}}
    quality_scenario['initial_conditions']['declared_quality_decisions'] = [decision]
    check('scenario-explicit-reinspect-decision', 'scenario', quality_scenario, True)
    decision['disposition'] = 'automatic_pass'
    check('scenario-no-invented-quality-disposition', 'scenario', quality_scenario, False)
    exact = read(PROOF / 'api-fixtures/get_object.request.json')
    exact['params']['object_id'] = {'kind': 'module', 'id': 'demo.devices'}
    exact['params']['source'] = {'kind': 'module_version', 'data': {
        'module_ref': {'id': 'demo.devices', 'version': '0.1.0'}}}
    check('module-read-explicit-exact-version', 'get_object.request', exact, True)
    del exact['params']['source']['data']['module_ref']['version']
    check('module-read-version-required', 'get_object.request', exact, False)
    frozen = read(PROOF / 'api-fixtures/get_object.request.json')
    frozen['params']['object_id'] = {'kind': 'module', 'id': 'demo.devices'}
    frozen['params']['source'] = {'kind': 'run_module_version', 'data': {
        'run_id': 'actual-run', 'at_ms': 0,
        'module_ref': {'id': 'demo.devices', 'version': '0.1.0'}}}
    check('run-module-read-explicit-historical-version', 'get_object.request', frozen, True)
    invalid_frozen = deepcopy(frozen)
    del invalid_frozen['params']['source']['data']['module_ref']['version']
    check('run-module-requires-exact-version', 'get_object.request', invalid_frozen, False)
    invalid_frozen = deepcopy(frozen)
    del invalid_frozen['params']['source']['data']['run_id']
    check('run-module-requires-run-identity', 'get_object.request', invalid_frozen, False)
    invalid_frozen = deepcopy(frozen)
    invalid_frozen['params']['source']['data']['at_ms'] = -1
    check('run-module-rejects-negative-observation', 'get_object.request', invalid_frozen, False)
    frozen['params']['object_id'] = {'kind': 'scenario', 'id': 'frozen-scenario'}
    frozen['params']['source'] = {'kind': 'run_snapshot', 'data': {'run_id': 'actual-run', 'at_ms': 0}}
    check('run-scenario-uses-existing-frozen-snapshot', 'get_object.request', frozen, True)
    preview = read(PROOF / 'api-fixtures/get_object.request.json')
    preview['params']['object_id'] = {'kind': 'module', 'id': 'demo.templates'}
    preview['params']['source'] = {'kind': 'template_preview', 'data': {
        'module_ref': {'id': 'demo.templates', 'version': '0.1.0'},
        'base_revision': 1, 'choices': None, 'seed': None}}
    check('template-preview-readonly-metadata-source', 'get_object.request', preview, True)
    invalid_preview = deepcopy(preview)
    del invalid_preview['params']['source']['data']['base_revision']
    check('template-preview-requires-captured-project-revision', 'get_object.request', invalid_preview, False)
    preview['params']['source']['data']['choices'] = {
        'source_project': 'example-project.json', 'namespace': 'explicit-line',
        'resources': {'worker': None}, 'reuse_items': {},
        'include_example_product': True, 'site_envelope': None}
    preview['params']['source']['data']['seed'] = {
        'scenario_id': 'explicit-example', 'supply': True, 'demand': True,
        'initial_quality': {'input-location': 'released'},
        'adopt_declared_inspection_results': False}
    check('template-preview-explicit-choices-and-seed', 'get_object.request', preview, True)
    invalid_preview = deepcopy(preview)
    invalid_preview['params']['source']['data']['seed']['initial_quality']['input-location'] = 'automatic_pass'
    check('template-preview-no-invented-initial-quality', 'get_object.request', invalid_preview, False)
    invalid_preview = deepcopy(preview)
    invalid_preview['params']['source']['data']['choices']['source_records'] = []
    check('template-preview-no-caller-source-authority', 'get_object.request', invalid_preview, False)
    invalid_preview = deepcopy(preview)
    del invalid_preview['params']['source']['data']['module_ref']['version']
    check('template-preview-requires-exact-source-version', 'get_object.request', invalid_preview, False)
    settings_request = read(PROOF / 'api-fixtures/apply_commands.request.json')
    settings_request['project_id'] = project['data']['id']
    settings_request['scope'] = {'kind': 'project'}
    settings_request['params'] = {'base_revision': project['data']['revision'], 'commands': [
        {'kind': 'set_project_settings', 'data': {
            'title': project['data']['title'], 'time_reference': project['data']['time_reference'],
            'generation_context': None, 'sources': []}}]}
    check('settings-legacy-omitted-setup-policies', 'apply_commands.request', settings_request, True)
    settings_request['params']['commands'][0]['data']['setup_policies'] = None
    check('settings-null-preserves-setup-policies', 'apply_commands.request', settings_request, True)
    settings_request['params']['commands'][0]['data']['setup_policies'] = []
    check('settings-explicit-clear-setup-policies', 'apply_commands.request', settings_request, True)
    settings_request['params']['commands'][0]['data']['setup_policies'] = 'automatic'
    check('settings-rejects-untyped-setup-policy', 'apply_commands.request', settings_request, False)
    settings_request['params']['commands'][0]['data']['setup_policies'] = [{}]
    check('settings-requires-canonical-setup-policy-fields', 'apply_commands.request', settings_request, False)
    context = read(PROOF / 'api-fixtures/get_context.response.json')
    check('context-legacy-empty-runs-omitted', 'get_context.response', context, True)
    context['outcome']['data']['run_summaries'] = []
    check('context-explicit-empty-run-list', 'get_context.response', context, True)
    context['outcome']['data']['run_summaries'] = [{}]
    check('context-no-fabricated-empty-run-summary', 'get_context.response', context, False)
    composition = read(PROOF / 'api-fixtures/apply_commands.request.json')
    composition['project_id'] = project['data']['id']
    composition['scope'] = {'kind': 'definition_draft', 'data': 'actual-definition-draft'}
    composition['params'] = {'base_revision': 1, 'commands': [{
        'kind': 'put_template_composition', 'data': {
            'project_revision': project['data']['revision'],
            'definition': read(PROOF / 'fixtures/module-templates.json')['data']['definition'],
            'subset': read(PROOF / 'fixtures/template.json')['data']}}]}
    check('template-composition-canonical-definition-and-subset', 'apply_commands.request', composition, True)
    invalid_composition = deepcopy(composition)
    del invalid_composition['params']['commands'][0]['data']['project_revision']
    check('template-composition-requires-project-revision', 'apply_commands.request', invalid_composition, False)
    invalid_composition = deepcopy(composition)
    invalid_composition['params']['commands'][0]['data']['project_revision'] = 1.5
    check('template-composition-project-revision-is-safe-integer', 'apply_commands.request', invalid_composition, False)
    invalid_composition = deepcopy(composition)
    invalid_composition['params']['commands'][0]['data']['source_path'] = 'C:/untrusted/project.json'
    check('template-composition-no-wire-filesystem-path', 'apply_commands.request', invalid_composition, False)
    invalid_composition = deepcopy(composition)
    invalid_composition['params']['commands'][0]['data']['definition']['payload']['data']['project'] = 'C:/untrusted/template.json'
    check('template-composition-rejects-absolute-definition-file', 'apply_commands.request', invalid_composition, False)
    invalid_composition = deepcopy(composition)
    del invalid_composition['params']['commands'][0]['data']['subset']
    check('template-composition-requires-actual-typed-subset', 'apply_commands.request', invalid_composition, False)
    captured_composition = deepcopy(composition)
    captured_composition['params']['commands'][0]['data']['source_scenario'] = {'scenario_id': 'source-scenario', 'revision': 7}
    check('template-composition-captured-scenario-reference', 'apply_commands.request', captured_composition, True)
    captured_composition['params']['commands'][0]['data']['source_scenario']['revision'] = 7.5
    check('template-composition-source-revision-is-safe-integer', 'apply_commands.request', captured_composition, False)
    captured_composition['params']['commands'][0]['data']['source_scenario'] = {'revision': 7}
    check('template-composition-source-reference-requires-id', 'apply_commands.request', captured_composition, False)
    definition = composition['params']['commands'][0]['data']['definition']
    instantiation = deepcopy(composition)
    instantiation['scope'] = {'kind': 'project'}
    instantiation['params']['commands'] = [{'kind': 'instantiate_template', 'data': {
        'template_ref': {'id': definition['id'], 'version': definition['version']},
        'subset': composition['params']['commands'][0]['data']['subset'],
        'mapped_bindings': {'item_types': {}, 'resources': {}, 'targets': {}, 'ports': {}},
        'proposed_objects': [], 'explicit_new_scenario_seed': None}}]
    check('template-instantiation-legacy-setup-omitted', 'apply_commands.request', instantiation, True)
    instantiation['params']['commands'][0]['data']['project_setup_policies'] = []
    check('template-instantiation-explicit-policy-array', 'apply_commands.request', instantiation, True)
    instantiation['params']['commands'][0]['data']['project_setup_policies'] = [{
        'device_id': 'mapped-device', 'initial_family': None, 'changeovers': [], 'first_article': None}]
    check('template-instantiation-canonical-policy-proposal', 'apply_commands.request', instantiation, True)
    instantiation['params']['commands'][0]['data']['project_setup_policies'] = 'grant-all'
    check('template-instantiation-rejects-untyped-policy-proposal', 'apply_commands.request', instantiation, False)
    instantiation['params']['commands'][0]['data']['project_setup_policies'] = [{'changeovers': []}]
    check('template-instantiation-policy-requires-device', 'apply_commands.request', instantiation, False)
    result = {'status': 'passed_actual_product_schema_finite_cases',
              'schemas': len(schemas), 'cases': cases, 'case_count': len(cases),
              'actual_Host_handlers_executed': False, 'whole_task_done': False}
    out = ROOT / args.output
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'status': result['status'], 'schemas': len(schemas), 'cases': len(cases)}))


if __name__ == '__main__':
    main()
