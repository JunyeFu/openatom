"""Explicit canonical test input: original 5/30/5 phase work, one declared override.

Creates a new private definition identity instead of changing a published version.
The real existing Engine remains the only duration/resource consumer.
"""
import json
from pathlib import Path

root = Path.cwd()
value = json.loads((root / 'packages/commands/tests/fixtures/production/p1-phase-calendar.json').read_text())
device = value['project']['devices'][0]
original_ref = device['module_ref'].copy()
definition = next(d for d in value['definitions'] if d['id'] == original_ref['id'] and d['version'] == original_ref['version'])
operation = next(d['payload']['data'] for d in value['definitions'] if d['payload']['category'] == 'operations')
new_ref = {'id': 'p205.fixture.inline-cnc', 'version': '0.1.0'}
definition['id'] = new_ref['id']
definition['title'] = 'P205 explicit 5/30/5 declared inline phase fixture'
definition['payload']['data']['behavior'] = {'selected': 'inline', 'inline': operation, 'operation_ref': None}
definition['parameters'] = [{'id': 'unload_ms', 'value_type': 'integer', 'binding': '/payload/data/behavior/inline/phases/2/duration/value', 'label': '下料有效工时（ms）', 'unit': 'millisecond', 'minimum': 0, 'maximum': 60000, 'integer': True, 'options': [], 'basis': 'user_provided'}]

def remap(node):
    if isinstance(node, dict):
        if node == original_ref:
            node.update(new_ref)
        for child in node.values():
            remap(child)
    elif isinstance(node, list):
        for child in node:
            remap(child)

remap(value)
for route in value['project']['routes']:
    for step in route['steps']:
        if step['device_id'] == device['id']:
            step['operation_ref'] = new_ref.copy()
device['configuration']['unload_ms'] = {'kind': 'integer', 'value': 5000}
device['operation_binding'] = None
device['route_operation_refs'] = [new_ref.copy()]
value['project']['parameter_sources'] = [{'target': {'kind': 'device', 'id': device['id']}, 'field_path': '/configuration/unload_ms/value', 'value': {'kind': 'integer', 'value': 5000}, 'basis': 'user_provided', 'source_ref': None}]
value['project']['title'] = 'P205 explicit declared phase input from original 5/30/5 example'
(root / 'scenarios/真实业务设计/runtime-ui/declared-phase.input.json').write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
