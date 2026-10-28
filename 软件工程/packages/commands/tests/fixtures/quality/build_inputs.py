"""Lower the frozen original P1-05 business case to existing public DTOs."""
import copy
import json
from pathlib import Path

HERE = Path(__file__).parent
PRODUCTION = HERE.parent / 'production'
clone = copy.deepcopy
q = lambda n, u='piece': {'value': {'coefficient': n, 'decimal_places': 0}, 'unit': u}
ref = lambda s: {'id': s, 'revision': 'r1'}
mref = lambda s: {'id': s, 'version': '0.1.0'}
stable = lambda tag, *parts: tag + ':' + ''.join(f'{len(p)}:{p}' for p in parts)

d = json.loads((PRODUCTION / 'p202-ix01-baseline.json').read_text(encoding='utf8'))
p, s = d['project'], d['scenario']
s.update(id='QUALITY-ORIGINAL', title='Original quality pending and scrap', window={'start_ms': 0, 'end_ms': 120000})
p['devices'] = [x for x in p['devices'] if x['id'] != 'CNC01']
p['resources'] = [x for x in p['resources'] if x['id'] not in ['CAP-CNC01', 'W-C', 'T1']]
worker = next(x for x in p['resources'] if x['id'] == 'W-B')
worker.update(id='W-Q', title='W-Q', home_location='inspect-in')
packer = clone(worker)
packer.update(id='W-P', title='W-P', home_location='passed-in')
p['resources'].append(packer)
for dev in p['devices']:
    if dev['id'] == 'Inspection01':
        dev['role_bindings']['resources']['worker'] = 'W-Q'
        dev['role_bindings']['targets']['pending'] = {'kind': 'location', 'id': 'pending-store'}
    if dev['id'] == 'Packing01':
        dev['role_bindings']['resources']['worker'] = 'W-P'
p['locations'] = [x for x in p['locations'] if x['id'] not in ['receiving', 'cnc-out', 'shell-buffer']]
shell = clone(p['locations'][0])
shell.update(id='shell-store', title='shell-store', accepted_item_types=['shell'])
p['locations'].append(shell)
pending = clone(next(x for x in p['locations'] if x['id'] == 'quarantine'))
pending.update(id='pending-store', title='pending-store', purpose='pending', ordinary_issue_enabled=False)
p['locations'].append(pending)
for loc in p['locations']:
    loc['capacity'] = q(12 if loc['id'] == 'fastener-store' else 1 if loc['id'] in ['quarantine', 'pending-store'] else 3)
for calendar in p['calendars']:
    calendar['intervals'] = [{'start_ms': 0, 'end_ms': 120000}]
p['halls'][0].update(input_locations=['shell-store', 'pcb-store', 'fastener-store', 'carton-store'], output_locations=['finished'])
p['connections'] = [x for x in p['connections'] if not any(e['kind'] == 'port' and e['data']['instance_id'] == 'CNC01' for e in [x['data']['source'], x['data']['destination']])]
for c in p['connections']:
    a = c['data']
    if a['destination']['kind'] == 'port' and a['destination']['data']['instance_id'] == 'Assembly01' and a['destination']['data']['port_id'] == 'shell':
        a.update(source={'kind': 'location', 'data': 'shell-store'}, destination_store='shell-store')
for route in p['routes']:
    route['steps'] = [x for x in route['steps'] if x['id'] != 'step-shell']
    for step in route['steps']:
        step['predecessors'] = [x for x in step['predecessors'] if x != 'step-shell']
        if step['id'] == 'step-inspection': step['resource_bindings']['worker'] = 'W-Q'
        if step['id'] == 'step-pack': step['resource_bindings']['worker'] = 'W-P'
p['boms'][0]['rows'] = [x for x in p['boms'][0]['rows'] if x['consume_at_operation'] != 'step-shell']
for definition in d['definitions']:
    data = definition['payload']['data']
    if definition['id'] in ['w5.op.assembly', 'w5.op.rework', 'w5.op.pack']:
        data['duration']['value'] = {'w5.op.assembly': 20000, 'w5.op.rework': 15000, 'w5.op.pack': 10000}[definition['id']]
    if definition['id'] == 'w5.op.inspect':
        data['quality_and_routes'].update(pending_policy='pending_store', pending_location_role='pending', max_completed_reworks=1)
sample = clone(s['initial_conditions']['items'][0])
s['initial_conditions']['items'] = []
for item, loc, count in [('shell', 'shell-store', 3), ('pcb', 'pcb-store', 3), ('fastener', 'fastener-store', 12), ('carton', 'carton-store', 3)]:
    for i in range(1, count + 1):
        x = clone(sample)
        x.update(id=f'{item}{i:02}', item_type_ref=ref(item), holder={'kind': 'location', 'id': loc})
        s['initial_conditions']['items'].append(x)
s['orders'] = []
s['arrivals'] = []
s['supplies'] = []
s['production_tasks'][0].update(order_allocations=[], planned_quantity=q(3))
s['quality_presets'] = [
    {'id': f'QC-{i}', 'item_or_ancestor': f'shell{i:02}', 'match_kind': 'ancestor', 'inspection_module': mref('w5.op.inspect'), 'results': results}
    for i, results in [(1, [{'kind': 'string', 'value': 'PASS'}]), (2, [{'kind': 'string', 'value': 'FAIL'}] * 2), (3, [None])]
]
def assembly_item(cycle):
    return stable('output', stable('job', 'TASK-LOCAL', str(cycle), 'step-assembly'), 'assembled', '0')
def decision(key, cycle, at, disposition):
    return {'id': key, 'item_ref': assembly_item(cycle), 'inspection_ref': mref('w5.op.inspect'), 'source_kind': 'preset', 'observed_result': None, 'disposition': disposition, 'effective_at_ms': at, 'decision_source': {'kind': 'scheduled_preset', 'data': f'QC-{cycle+1}'}}
s['initial_conditions']['declared_quality_decisions'] = [decision('manual-release-3', 2, 90000, 'release'), decision('explicit-scrap-2', 1, 100000, 'scrap')]
goal = json.loads((PRODUCTION / 'p1-legacy-goal.json').read_text(encoding='utf8'))['scenario']['acceptance_goal']
goal.update(industry='original quality business case', product_family='package', product_specification_ref=ref('PACKAGE'), accepted_quantity_per_day=q(3), observation_window=clone(s['window']), existing_equipment=[{'kind': 'device', 'id': x['id']} for x in p['devices']])
goal['acceptance_scope'].update(product_ref=ref('PACKAGE'), destination='finished')
s['acceptance_goal'] = goal
p['scenario_refs'] = [s['id']]
d['input_ref']['scenario_id'] = s['id']
d['source_mapping'] = {'source': 'scenarios/真实业务设计/business-inputs.design.json#quality_pending_and_scrap', 'fixed_expectations': 'original-expectations.json', 'decisions': 'Existing generated physical item IDs are selected from the published deterministic identity construction before running; no initial assemblies or replacement tasks.', 'demand': 'Original cumulative demand 3 uses the saved no-order acceptance goal; no OrderLine or GenerationContext invented.', 'actual_result': None}
(HERE / 'quality-original.json').write_text(json.dumps(d, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
