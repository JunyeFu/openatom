"""Assemble an input-only P0-01 archive from the frozen typed fixtures.

This declares a design, not a production Run or accepted professional result.
The original fixture files and original no-phase operation are left unchanged.
"""
from copy import deepcopy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'tools/type-contract-proof/fixtures'
OUT = ROOT / '.work/whole-packages/p001-20261017/complete-input'


def fixture(name):
    return json.loads((FIXTURES / f'{name}.json').read_text(encoding='utf-8'))


def write(relative, value):
    target = OUT / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


manifest = fixture('manifest')
for document in manifest['data']['documents']:
    path = document['file_ref']['path']
    write(path, fixture(path[:-5]))

project = fixture('project')
p = project['data']
p['revision'] = 2
p['title'] = 'P0-01 complete typed design — self-built acceptance input'
device_package = fixture('module-devices')
definition = device_package['data']['definition']
definition['id'] = 'p001.complete.devices'
definition['version'] = '1.0.0'
definition['title'] = 'P0-01 typed boundary device; original no-phase behavior'
definition['parameters'].append({'id': 'declared_cycle_ms', 'label': 'Declared no-phase cycle', 'value_type': 'duration',
    'unit': 'millisecond', 'binding': '/payload/data/behavior/inline/duration', 'minimum': None, 'maximum': None,
    'integer': False, 'options': [], 'basis': 'user_provided'})
module_ref = {'id': definition['id'], 'version': definition['version']}
ports = definition['payload']['data']['ports']
ports.append({**deepcopy(ports[0]), 'id': 'in_aux'})
for direction in ['input', 'output']:
    ports.append({'id': f'signal_{direction}', 'direction': direction,
                  'payload': {'kind': 'signal', 'data': {'value_type': 'boolean', 'enum_values': [], 'effect_target': None}}})
    ports.append({'id': f'artifact_{direction}', 'direction': direction,
                  'payload': {'kind': 'artifact', 'data': {'artifact_type': 'paper_packaging', 'schema_version': '1.0',
                    'unit': 'millimeter', 'coordinate_frame': 'right_handed_z_up', 'supported_scope': ['envelope_mm']}}})
p['module_refs'] = [module_ref if value['id'] == 'demo.devices' else value for value in p['module_refs']]
entry = deepcopy(next(value for value in manifest['data']['documents'] if value['role'] == 'module_definition' and value['module_ref']['id'] == 'demo.devices'))
entry['module_ref'] = module_ref
entry['object_ref']['id'] = module_ref['id']
entry['file_ref']['path'] = 'complete/module-current.json'
manifest['data']['documents'].append(entry)
manifest['data']['module_refs'].append(module_ref)
write(entry['file_ref']['path'], device_package)

hall = deepcopy(p['halls'][0])
hall.update(id='H2', title='second declared hall', input_locations=['S3'], output_locations=['S4'])
hall['origin']['translation']['x'] = 20.0
p['halls'].append(hall)
for old, new, purpose in [('S1', 'S3', 'input'), ('S2', 'S4', 'output')]:
    location = deepcopy(next(value for value in p['locations'] if value['id'] == old))
    location.update(id=new, title=new, hall_id='H2', purpose=purpose)
    p['locations'].append(location)
device = deepcopy(p['devices'][0])
p['devices'] = []
for index in range(1, 5):
    value = deepcopy(device)
    value.update(id=f'D{index}', title=f'declared device {index}', hall_id='H1' if index <= 2 else 'H2',
                 line_id='L-BRANCH' if index <= 2 else 'L-MERGE', module_ref=module_ref)
    value['pose']['translation'].update(x=1.0 if index % 2 else 5.0, y=2.0)
    p['devices'].append(value)
pose = deepcopy(device['pose'])
pose['translation'].update(x=0.0, y=0.0, z=0.0)
p['lines'] = [{'id': name, 'title': name, 'hall_id': hall_id, 'members': members,
               'transform': deepcopy(pose), 'region_polygon_m': [], 'boundary_ports': []}
              for name, hall_id, members in [('L-BRANCH', 'H1', ['D1', 'D2']), ('L-MERGE', 'H2', ['D3', 'D4'])]]
p['routes'][0]['steps'][0]['operation_ref'] = deepcopy(module_ref)


def port(device_id, port_id):
    return {'module_ref': module_ref, 'instance_id': device_id, 'port_id': port_id}


p['connections'] = []
for index, (source, destination, destination_port, store) in enumerate([
    ('D1', 'D3', 'in', 'S3'), ('D2', 'D3', 'in_aux', 'S3'), ('D3', 'D4', 'in', 'S4')]):
    p['connections'].append({'kind': 'material', 'data': {'id': f'P0-M{index}',
        'source': {'kind': 'port', 'data': port(source, 'out')}, 'destination': {'kind': 'port', 'data': port(destination, destination_port)},
        'item_type': 'T1', 'unit': 'piece', 'destination_store': store, 'quantity_rule': None, 'capacity_rule': None}})
p['connections'].extend([
    {'kind': 'signal', 'data': {'id': 'P0-SIGNAL', 'source': port('D1', 'signal_output'), 'destination': port('D4', 'signal_input'), 'value_type': 'boolean', 'effect_target': None}},
    {'kind': 'artifact', 'data': {'id': 'P0-ARTIFACT', 'source': port('D1', 'artifact_output'), 'destination': port('D4', 'artifact_input'), 'binding_ref': 'P0-DECLARED-BINDING'}},
])
p['sources'] = [{'id': 'P0-SOURCE', 'title': 'Self-built P0-01 input declaration', 'locator': 'tools/p0-01-host/prepare-complete-input.py',
                 'read_scope': 'typed persistence acceptance only; no real factory measurement', 'read_date': '2026-10-17', 'basis': 'user_provided'}]
p['parameter_sources'] = [{'target': {'kind': 'device', 'id': 'D1'}, 'field_path': '/configuration/declared_cycle_ms/value',
                           'value': {'kind': 'duration', 'value': {'value': 1000, 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': 'P0-SOURCE'}},
                           'basis': 'user_provided', 'source_ref': 'P0-SOURCE'}]
p['devices'][0]['configuration']['declared_cycle_ms'] = deepcopy(p['parameter_sources'][0]['value'])
p['generation_context'] = {'goal': {'industry': 'self-built fixture', 'product_family': 'discrete',
    'product_specification_ref': {'id': 'PRODUCT1', 'revision': 'r1'}, 'accepted_quantity_per_day': {'value': {'coefficient': 24, 'decimal_places': 0}, 'unit': 'piece'},
    'acceptance_scope': {'product_ref': {'id': 'PRODUCT1', 'revision': 'r1'}, 'unit': 'piece', 'destination': 'S4', 'stage': 'unloaded_and_released'},
    'production_calendar_ref': 'CAL1', 'observation_window': {'start_ms': 0, 'end_ms': 86400000},
    'site_envelope': {'regions': [], 'length_m': 30.0, 'width_m': 16.0, 'height_m': 5.0, 'usable_polygon_m': [], 'clearance_m': 0.5},
    'existing_equipment': [{'kind': 'device', 'id': 'D1'}], 'constraints': []},
    'template_ref': {'id': 'demo.templates', 'version': '0.1.0'}, 'parameter_sources': deepcopy(p['parameter_sources']),
    'draft_check': None, 'initial_run_id': None, 'revised_run_ids': [], 'changes': [], 'stop_reason': None}
# An empty-scope scheduled declaration tests typed references without inventing
# a human approval or turning the original Undetermined fixture into Pass.
p['artifact_refs'] = ['A1']
p['acknowledgments'] = [{'id': 'P0-DECLARED-ACK', 'artifact_ref': 'A1', 'accepted_subject': {'id': 'PRODUCT1', 'revision': 'r1'},
    'accepted_scope': [], 'decision_source': {'kind': 'scheduled_preset', 'data': 'P0-INPUT-ONLY'}, 'conditions': [], 'recorded_at_iso8601': None}]
p['bindings'] = [{'id': 'P0-DECLARED-BINDING', 'consumer_ref': {'kind': 'device', 'id': 'D4'}, 'operation_ref': {'id': 'demo.operations', 'version': '0.1.0'},
    'artifact_ref': 'A1', 'expected_subject_revision': 'r1', 'required_payload': [], 'requested_scope': [], 'acceptance_ref': 'P0-DECLARED-ACK', 'mode': 'declared_process'}]
for name, role, kind, object_kind, object_id, revision in [
    ('packaging-job', 'professional_input', 'packaging_job', 'packaging_job', 'PACKJOB1', 1),
    ('packaging-result', 'professional_result', 'packaging_result', 'artifact', 'A1', None),
    ('artifact', 'artifact_metadata', 'artifact_record', 'artifact', 'A1', None),
]:
    write(f'{name}.json', fixture(name))
    manifest['data']['documents'].append({'role': role, 'object_ref': {'kind': object_kind, 'id': object_id}, 'revision': revision,
        'module_ref': None, 'file_ref': {'path': f'{name}.json', 'kind': kind}, 'required': True})
scenario = fixture('scenario')
scenario['data']['project_revision'] = 2
write('scenario.json', scenario)
template_package = fixture('module-templates')
template_definition = template_package['data']['definition']
template_definition.update(id='p001.complete.templates', version='1.0.0', title='P0-01 complete declared combination')
template_ref = {'id': template_definition['id'], 'version': template_definition['version']}
p['module_refs'] = [template_ref if value['id'] == 'demo.templates' else value for value in p['module_refs']]
p['generation_context']['template_ref'] = template_ref
template_definition['payload']['data'] = {'project': 'complete/template.json', 'generation_profile': 'complete/generation-profile.json'}
template_package['data']['files'] = [{'path': 'complete/template.json', 'kind': 'template'}, {'path': 'complete/generation-profile.json', 'kind': 'generation_profile'}]
template_package['data']['dependencies'] = [value for value in p['module_refs'] if value != template_ref]
entry = deepcopy(next(value for value in manifest['data']['documents'] if value['role'] == 'module_definition' and value['module_ref']['id'] == 'demo.templates'))
entry.update(module_ref=template_ref, object_ref={'kind': 'module', 'id': template_ref['id']})
entry['file_ref']['path'] = 'complete/module-template.json'
manifest['data']['documents'].append(entry)
manifest['data']['module_refs'].append(template_ref)
write(entry['file_ref']['path'], template_package)
profile = fixture('generation-profile')
profile['data']['site_applicability'] = deepcopy(p['generation_context']['goal']['site_envelope'])
profile['data']['parameter_sources'] = deepcopy(p['parameter_sources'])
categories = ['items', 'devices', 'resources', 'tasks', 'operations', 'transport', 'templates', 'professional', 'media']
profile['data']['nine_category_roles'] = [{'role': category, 'quantity': {'value': {'coefficient': 1, 'decimal_places': 0}, 'unit': 'piece'}} for category in categories]
profile['data']['role_bindings']['targets'] = {category: {'kind': 'module', 'id': ref['id']} for category, ref in zip(categories, p['module_refs'])}
subset = fixture('template')
subset['data']['layout_scope'] = 'region'
for key in ['halls', 'lines', 'devices', 'connections', 'item_types', 'locations']:
    subset['data'][key] = deepcopy(p[key])
subset['data']['module_refs'] = deepcopy(template_package['data']['dependencies'])
for name, role, kind in [('template', 'template_data', 'template'), ('generation-profile', 'generation_profile', 'generation_profile')]:
    write(f'complete/{name}.json', subset if name == 'template' else profile)
    manifest['data']['documents'].append({'role': role, 'object_ref': {'kind': 'module', 'id': f'p001-{name}'}, 'revision': None,
        'module_ref': None, 'file_ref': {'path': f'complete/{name}.json', 'kind': kind}, 'required': True})
active = next(value for value in manifest['data']['documents'] if value['role'] == 'active_project')
active['file_ref']['path'] = 'payload/project-state.json'
active['revision'] = 2
manifest['data'].update(project_revision=2, title=p['title'])
write(active['file_ref']['path'], project)
write('project.json', manifest)
write('input-expectations.json', {'input_only': True, 'actual_result': None, 'halls': 2, 'devices': 4, 'lines': 2,
    'material_connections': 3, 'signal_connections': 1, 'artifact_connections': 1, 'current_exact_categories': 9,
    'coordinate_unit': 'meter', 'professional_port_unit': 'millimeter', 'professional_outcome': 'undetermined',
    'professional_accepted_scope': [], 'no_human_approval_or_production_Run_declared': True,
    'expected_explicit_draft_issue': 'professional_inputs.nonhuman_acknowledgment',
    'declared_category_roles_and_exact_selected_module_targets': 9,
    'original_fixtures_and_original_no_phase_behavior_unchanged': True})
print(OUT)
