"""Map the frozen NFR-B60 load into real Project/Scenario/module inputs."""
import copy
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'tests/planner/p201-whole/fixtures/b60/cut-r2'
OUT.mkdir(parents=True, exist_ok=True)
profile = json.loads((ROOT / 'scenarios/非功能基准/benchmark.design.json').read_text(encoding='utf-8-sig'))
base = json.loads((ROOT / 'tests/planner/p201-whole/fixtures/b60/balanced.input.json').read_text(encoding='utf-8-sig'))

def write(name, value):
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

def pose(x=0, y=0, z=0, rotation=None):
    return {'translation': dict(x=x, y=y, z=z), 'rotation': rotation or dict(x=0, y=0, z=0, w=1),
            'scale': dict(x=1, y=1, z=1), 'unit': 'meter', 'coordinate_frame': 'right_handed_z_up'}

def quat(axis, angle):
    q = dict(x=0, y=0, z=0, w=math.cos(angle / 2))
    q[axis] = math.sin(angle / 2)
    return q

def quantity(value, unit='piece'):
    return {'value': {'coefficient': value, 'decimal_places': 0}, 'unit': unit}

def module(identifier, category, data, title):
    return {'id': identifier, 'version': '1.0.0', 'title': title, 'definition_state': 'runtime_candidate',
            'contribution': {'author': None, 'license': None, 'assets': []}, 'parameters': [], 'attribute_schema': [],
            'payload': {'category': category, 'data': data}}

def ref(definition):
    return {key: definition[key] for key in ('id', 'version')}

nodes = []
materials = {
    'steel': ([.46, .5, .53, 1], .8, .3), 'paint': ([.75, .79, .8, 1], .25, .45),
    'rubber': ([.06, .075, .08, 1], .05, .85), 'blue': ([.08, .3, .5, 1], .3, .35),
    'screen': ([.02, .09, .12, 1], .1, .25), 'glass': ([.63, .83, .88, .22], .05, .12),
    'red': ([.75, .07, .05, 1], .1, .35), 'yellow': ([.95, .65, .06, 1], .1, .35),
    'green': ([.12, .65, .3, 1], .1, .35),
}

def node(name, xyz, geometry, material='steel', rotation=None, parent=None):
    color, metal, rough = materials[material]
    index = len(nodes)
    nodes.append({'id': index, 'name': name, 'parent_index': parent, 'transform': pose(*xyz, rotation),
                  'geometry': geometry, 'material': {'color_rgba': color, 'metallic': metal, 'roughness': rough, 'texture': None}})
    return index

def box(name, xyz, size, material='steel', rotation=None, parent=None):
    return node(name, xyz, {'kind': 'box', 'data': dict(zip(('x', 'y', 'z'), size))}, material, rotation, parent)

def cylinder(name, xyz, radius, height, material='steel', rotation=None, parent=None):
    return node(name, xyz, {'kind': 'cylinder', 'data': {'radius': radius, 'height': height}}, material, rotation, parent)

# Full bottom, four adjustable feet, structural frame and a separate service tray.
box('welded_base_bottom', (0, 0, .16), (1.9, 1.9, .14))
box('removable_bottom_service_tray', (0, -.08, .27), (1.48, 1.58, .06), 'paint')
for x in (-.79, .79):
    for y in (-.79, .79):
        cylinder(f'adjustable_foot_{x}_{y}', (x, y, .04), .11, .08, 'rubber')
        cylinder(f'foot_thread_{x}_{y}', (x, y, .12), .035, .16)
        box(f'corner_column_{x}_{y}', (x, y, .67), (.12, .12, .92))
        cylinder(f'base_fixing_bolt_{x}_{y}', (x, y, .255), .025, .025)
for x in (-.79, .79):
    box(f'upper_length_rail_{x}', (x, 0, 1.04), (.12, 1.7, .13))
for y in (-.79, .79):
    box(f'upper_cross_rail_{y}', (0, y, 1.04), (1.7, .12, .13))
box('rear_lower_service_cover', (0, .9, .72), (1.74, .08, .84), 'paint')
for x in (-.52, .52):
    box(f'side_service_door_{x}', (x, -.89, .65), (.94, .06, .69), 'paint')
    cylinder(f'service_door_handle_{x}', (x + .25, -.94, .68), .018, .16)
for z in (.4, .84):
    for x in (-.94, .94):
        cylinder(f'door_hinge_{x}_{z}', (x, -.9, z), .022, .09)

# Real rollers, shafts, bearings, edge rails, drive motor and coupling guard.
for index in range(10):
    y = -.72 + index * .16
    cylinder(f'roller_{index}', (0, y, 1.12), .045, 1.38, 'steel', quat('y', math.pi / 2))
    for x in (-.73, .73):
        box(f'roller_bearing_{index}_{x}', (x, y, 1.12), (.09, .09, .09), 'blue')
for x in (-.7, .7):
    box(f'product_edge_rail_{x}', (x, 0, 1.24), (.04, 1.75, .16), 'paint')
cylinder('drive_motor_body', (.88, .1, .82), .095, .3, 'blue')
box('drive_motor_mount', (.86, .1, .63), (.22, .32, .08))
box('transmission_guard', (.91, .1, 1.02), (.1, .48, .28), 'paint')
box('cable_trunk', (-.9, .25, .7), (.07, 1.15, .09), 'rubber')
for index in range(5):
    box(f'rear_vent_louver_{index}', (0, .92, 1.6 + index * .08), (.74, .06, .035), 'steel', quat('x', -.35))

# Rigid upper gantry, enclosed back and separate transparent safety doors.
for x in (-.86, .86):
    box(f'gantry_vertical_{x}', (x, .52, 1.85), (.13, .13, 1.4))
box('gantry_crossbeam', (0, .52, 2.52), (1.85, .2, .18), 'blue')
box('rear_upper_left_panel', (-.65, .91, 1.92), (.42, .07, 1.28), 'paint')
box('rear_upper_right_panel', (.65, .91, 1.92), (.42, .07, 1.28), 'paint')
box('rear_upper_header', (0, .91, 2.5), (1.72, .07, .16), 'paint')
for x in (-.43, .43):
    box(f'clear_safety_door_{x}', (x, -.84, 1.97), (.82, .025, 1.0), 'glass')
    box(f'safety_door_edge_{x}', (x, -.85, 2.48), (.86, .04, .045), 'steel')
    box(f'safety_door_handle_{x}', (x * .24, -.89, 1.85), (.035, .07, .2), 'rubber')
box('control_console', (-.73, -.95, 1.65), (.38, .1, .46), 'paint')
box('control_display', (-.73, -.995, 1.72), (.27, .01, .2), 'screen')
cylinder('emergency_stop', (-.82, -.96, 1.45), .032, .035, 'red', quat('x', math.pi / 2))
cylinder('reset_button', (-.65, -.96, 1.45), .019, .03, 'green', quat('x', math.pi / 2))
for index, color in enumerate(('red', 'yellow', 'green')):
    cylinder(f'stack_light_{color}', (.72, .68, 2.79 + index * .075), .045, .065, color)
cylinder('stack_light_stem', (.72, .68, 2.66), .022, .18)
cylinder('stack_light_cap', (.72, .68, 2.975), .045, .05, 'rubber')
for x in (-.36, .36):
    cylinder(f'linear_guide_{x}', (x, .5, 1.95), .028, .9)
    box(f'guide_bearing_{x}', (x, .5, 2.06), (.12, .13, .18), 'blue')
ram = box('cycle_ram_carriage', (0, .5, 2.05), (.62, .24, .14), 'blue')
cylinder('ram_tool_shaft', (0, 0, -.22), .052, .4, 'steel', parent=ram)
box('ram_tool_head', (0, 0, -.45), (.36, .25, .08), 'rubber', parent=ram)
part_anchor = box('workpiece_fixture_anchor', (0, 0, 1.19), (.28, .28, .05))
for y in (-.71, .71):
    box(f'photo_sensor_body_{y}', (-.66, y, 1.38), (.08, .09, .12), 'blue')
    box(f'photo_sensor_window_{y}', (-.61, y, 1.38), (.01, .04, .055), 'screen')

presentation = {'kind': 'native_scene', 'source': {'kind': 'native_scene', 'data': 'p201.b60.fixed-station'},
    'unit': 'meter', 'coordinate_frame': 'right_handed_z_up', 'scale': dict(x=1, y=1, z=1), 'nodes': nodes,
    'poses': [{'id': 'cycle_home', 'node_transforms': []}],
    'actions': [{'id': 'actual_cycle_ram', 'node_index': ram, 'transition': 'linear', 'keys': [
        {'at_progress': progress, 'pose': pose(0, .5, z)} for progress, z in [(0, 2.05), (.3, 1.805), (.65, 1.805), (1, 2.05)]]}],
    'timeline': [{'id': 'actual_cycle', 'phase_ref': 'cycle', 'start_progress': 0, 'end_progress': 1,
                  'source': {'kind': 'native_pose', 'data': 'cycle_home'}, 'transition': 'hold'}],
    'tags': [], 'anchor_roles': {'workpiece': {'kind': 'scene_node', 'data': part_anchor}},
    'port_anchors': {port: {'kind': 'scene_node', 'data': next(index for index, node in enumerate(nodes) if node['name'] == roller)} for port, roller in [('input', 'roller_0'), ('output', 'roller_9')]}, 'lod': {'detailed': None, 'simplified': {'kind': 'media_ref', 'data': {'id': 'p201.b60.simple', 'version': '1.0.1'}}, 'region': 'line_whole'}, 'production_effect': 'none'}
simple = copy.deepcopy(presentation)
selected = [i for i, n in enumerate(nodes) if n['name'] in ('roller_0', 'roller_9') or n['name'].startswith(('welded_base', 'corner_column', 'upper_length', 'upper_cross', 'gantry_', 'workpiece_fixture', 'cycle_ram', 'ram_tool', 'stack_light'))]
index_map = {old: new for new, old in enumerate(selected)}
simple['nodes'] = [dict(copy.deepcopy(nodes[old]), id=index_map[old], parent_index=index_map.get(nodes[old]['parent_index'])) for old in selected]
simple['actions'][0]['node_index'] = index_map[ram]
simple['anchor_roles']['workpiece']['data'] = index_map[part_anchor]
simple['port_anchors'] = {key: {'kind': 'scene_node', 'data': index_map[value['data']]} for key, value in presentation['port_anchors'].items()}
simple['source']['data'] = 'p201.b60.simple'
simple['lod']['simplified'] = copy.deepcopy(simple['source'])
simple_module = module('p201.b60.simple', 'media', simple, 'B60固定站 · 明确简化档')
simple_module['version'] = '1.0.1'
operation = {'builtin': 'transform', 'item_roles': ['workpiece'],
    'inputs': [{'role': 'workpiece', 'quantity': quantity(1), 'port': 'input', 'required_quality': ['released'], 'declared_output_quality': None}],
    'outputs': [{'role': 'workpiece', 'quantity': quantity(1), 'port': 'output', 'required_quality': [], 'declared_output_quality': 'released'}],
    'resource_roles': [], 'duration': None, 'phases': [{'id': 'cycle', 'duration': {'value': 2000, 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': 'NFR-B60'},
        'phase_resources': [], 'interruption_policy': 'non_preemptive', 'work_kind': 'processing'}],
    'hold_on_output_wait': [], 'quality_and_routes': None, 'identity_policy': 'preserve', 'family': None, 'pack_identity': None}
station = module('p201.b60.station', 'devices', {'placement': 'fixed_during_simulation',
    'ports': [{'id': name, 'direction': direction, 'payload': {'kind': 'material', 'data': {'item_role': 'workpiece', 'unit': 'piece', 'quantity_rule': quantity(1)}}} for name, direction in [('input', 'input'), ('output', 'output')]],
    'behavior': {'inline': operation, 'operation_ref': None, 'selected': 'inline'}, 'configuration': {}, 'presentation': presentation, 'operations': []}, 'B60两秒固定保身份工位')
station['version'] = '1.0.1'
station['parameters'] = [{'id': 'processing_sim_seconds', 'value_type': 'integer', 'binding': '/payload/data/behavior/inline/phases/0/duration/value', 'label': '处理工时', 'unit': 'millisecond', 'minimum': 1, 'maximum': None, 'integer': True, 'options': [], 'basis': 'user_provided'}]
capacity = module('p201.b60.capacity', 'resources', {'kind': 'fixture', 'role': 'execution_capacity', 'capacity': quantity(1, 'slot'), 'qualifications': [], 'default_calendar_ms': [{'start_ms': 0, 'end_ms': 1200000}], 'presentation': None}, 'B60单件容量')
item_media = copy.deepcopy(presentation)
item_media.update(source={'kind': 'native_scene', 'data': 'p201.b60.workpiece'}, nodes=[dict(copy.deepcopy(nodes[part_anchor]), id=0, parent_index=None, name='benchmark_workpiece', transform=pose(), geometry={'kind': 'box', 'data': dict(x=.18, y=.18, z=.1)})], poses=[], actions=[], timeline=[], anchor_roles={}, lod={'detailed': None, 'simplified': None, 'region': 'device_icons'})
item_media['lod']['simplified'] = copy.deepcopy(item_media['source'])
item = module('p201.b60.workpiece', 'items', {'presentation_or_media_ref': item_media, 'unit': 'piece', 'tracking': 'individual', 'spec_ref': None, 'attributes': {}, 'attribute_schema': []}, 'B60真实追踪试件')
item['version'] = '1.0.1'
definitions = [station, simple_module, capacity, item]
for definition in definitions:
    write(definition['id'] + '.module.json', {'schema_version': '1.0', 'data': {'definition': definition, 'assets': [], 'files': [], 'dependencies': [ref(simple_module)] if definition is station else [], 'rights': [], 'issues': []}})

for variant in profile['variants']:
    name = variant['id']
    project = copy.deepcopy(base['project'])
    for key in ('setup_policies', 'products', 'boms', 'acknowledgments', 'bindings', 'artifact_refs', 'template_adoptions', 'sources', 'issues', 'parameter_sources'):
        project[key] = []
    project.update(id='NFR-B60-' + name, revision=2, title='NFR-B60 · ' + name, generation_context=None,
                   halls=[], lines=[], devices=[], resources=[], routes=[], locations=[], connections=[], module_refs=list(map(ref, definitions)), scenario_refs=['B60-' + name])
    project['sources'] = [{'id': 'NFR-B60', 'title': '已冻结自建B60负载', 'locator': 'scenarios/非功能基准/benchmark.design.json', 'read_scope': '负载、坐标、单位、时间与独立固定预期', 'read_date': '2026-10-17', 'basis': 'user_provided'}]
    project['views'].update(normal_canvas='line_canvas', selected_hall='H1', selected_objects=[], lod='detailed')
    project['calendars'] = [{'id': 'B60-ALL', 'title': 'B60明确全时段日历', 'intervals': [{'start_ms': 0, 'end_ms': 1200000}], 'source': 'user_provided'}]
    project['item_types'] = [{'id': 'benchmark-item-v1', 'revision': 'r1', 'title': 'B60试件', 'module_ref': ref(item), 'unit': 'piece', 'tracking': 'individual', 'spec_ref': None, 'attributes': {}, 'attribute_schema': [], 'contents_layout': None}]
    for index, hall in enumerate(profile['halls']):
        project['halls'].append({'id': hall['id'], 'title': hall['id'], 'width_m': hall['width_m'], 'depth_m': hall['depth_m'], 'height_m': hall['height_m'], 'origin': pose(index * 70), 'input_locations': [], 'output_locations': []})
    scenario = copy.deepcopy(base['scenario'])
    scenario.update(id='B60-' + name, title='NFR-B60实际1200秒 · ' + name, project_revision=2, revision=2, window={'start_ms': 0, 'end_ms': 1200000}, orders=[], arrivals=[], supplies=[], parameter_overrides=[], scope=[], issues=[], cost_inputs=[], acceptance_goal=None, quality_presets=[], production_tasks=[])
    scenario['initial_conditions'] = {'items': [], 'initial_setup_families': {}, 'declared_quality_decisions': [], 'material_allocations': []}
    for line in profile['lines']:
        hall_id = line[name + '_hall']
        line_number = int(line['id'][1:])
        target_hall = next(value for value in project['halls'] if value['id'] == hall_id)
        target_hall['input_locations'].append(line['source_queue']); target_hall['output_locations'].append(line['accepted_sink'])
        row_y = 5 + ((line_number - 1) % 2 if name == 'balanced' else line_number - 1) * 12
        project['lines'].append({'id': line['id'], 'title': line['id'], 'hall_id': hall_id, 'transform': pose(), 'members': line['device_ids'], 'boundary_ports': [{'instance_id': line['device_ids'][0], 'module_ref': ref(station), 'port_id': 'input'}, {'instance_id': line['device_ids'][-1], 'module_ref': ref(station), 'port_id': 'output'}], 'region_polygon_m': [{'x': 2, 'y': row_y-2}, {'x': 33, 'y': row_y-2}, {'x': 33, 'y': row_y+2}, {'x': 2, 'y': row_y+2}]})
        buffers = [line['source_queue']] + [f"{line['id']}-BUFFER-{index:02}" for index in range(1, 10)] + [line['accepted_sink']]
        for index, identifier in enumerate(buffers):
            project['locations'].append({'id': identifier, 'title': identifier, 'hall_id': hall_id, 'position': pose(2 if index == 0 else 33 if index == 10 else 4 + (index-.5)*3, row_y), 'capacity': quantity(line['initial_tracked_item_count']), 'accepted_item_types': ['benchmark-item-v1'], 'purpose': 'input' if index == 0 else 'output' if index == 10 else 'buffer', 'ordinary_issue_enabled': True, 'item_layout': None})
        steps = []
        for index, identifier in enumerate(line['device_ids']):
            source = next(value for value in profile['devices'] if value['id'] == identifier)
            a, b, c = source[name + '_position_m']
            # The legacy tuple is [width, height, depth]; Project uses [X,Y,Z].
            project['devices'].append({'id': identifier, 'title': identifier, 'hall_id': hall_id, 'line_id': line['id'], 'module_ref': ref(station), 'pose': pose(a, c, b, quat('z', -math.pi / 2)), 'footprint_m': [{'x': -1, 'y': -1}, {'x': 1, 'y': -1}, {'x': 1, 'y': 1}, {'x': -1, 'y': 1}], 'height_m': 3, 'configuration': {'processing_sim_seconds': {'kind': 'integer', 'value': 2000}}, 'role_bindings': {'item_types': {'workpiece': 'benchmark-item-v1'}, 'resources': {'execution_capacity': identifier + '-CAP'}, 'targets': {}, 'ports': {}}, 'presentation_override': None, 'operation_binding': None, 'route_operation_refs': [], 'item_layout': {'assignment': 'item_id_order', 'slots': [{'item_type_ref': {'id': 'benchmark-item-v1', 'revision': 'r1'}, 'anchor_role': 'workpiece', 'offset': pose(0, 0, .075)}]}})
            project['parameter_sources'].append({'target': {'kind': 'device', 'id': identifier}, 'field_path': '/configuration/processing_sim_seconds/value', 'value': {'kind': 'integer', 'value': 2000}, 'basis': 'user_provided', 'source_ref': 'NFR-B60'})
            project['resources'].append({'id': identifier + '-CAP', 'title': identifier + ' 单件容量', 'definition_ref': ref(capacity), 'kind': 'fixture', 'role': 'execution_capacity', 'capacity': quantity(1, 'slot'), 'qualifications': [], 'calendar_id': 'B60-ALL', 'home_location': None, 'presentation': None})
            steps.append({'id': identifier + '-step', 'operation_ref': ref(station), 'device_id': identifier, 'predecessors': [] if index == 0 else [line['device_ids'][index-1] + '-step'], 'item_bindings': {'workpiece': 'benchmark-item-v1'}, 'resource_bindings': {}})
            for direction, src, dst, store in [('input', {'kind': 'location', 'data': buffers[index]}, {'kind': 'port', 'data': {'module_ref': ref(station), 'instance_id': identifier, 'port_id': 'input'}}, buffers[index]), ('output', {'kind': 'port', 'data': {'module_ref': ref(station), 'instance_id': identifier, 'port_id': 'output'}}, {'kind': 'location', 'data': buffers[index+1]}, buffers[index+1])]:
                project['connections'].append({'kind': 'material', 'data': {'id': identifier + '-' + direction, 'source': src, 'destination': dst, 'item_type': 'benchmark-item-v1', 'unit': 'piece', 'quantity_rule': None, 'capacity_rule': None, 'destination_store': store}})
        project['routes'].append({'id': line['id'] + '-ROUTE', 'revision': 'r1', 'steps': steps})
        project['boms'].append({'id': line['id'] + '-BOM', 'revision': 'r1', 'rows': [{'item_type_ref': {'id': 'benchmark-item-v1', 'revision': 'r1'}, 'quantity': quantity(1), 'consume_at_operation': steps[0]['id']}]})
        project['products'].append({'id': line['id'] + '-PRODUCT', 'revision': 'r1', 'item_type_ref': {'id': 'benchmark-item-v1', 'revision': 'r1'}, 'bom_ref': {'id': line['id'] + '-BOM', 'revision': 'r1'}, 'routing_ref': {'id': line['id'] + '-ROUTE', 'revision': 'r1'}, 'artifact_refs': []})
        scenario['production_tasks'].append({'id': line['id'] + '-TASK', 'routing_ref': {'id': line['id'] + '-ROUTE', 'revision': 'r1'}, 'order_allocations': [], 'planned_quantity': quantity(500), 'release_at_ms': 0, 'priority': 0})
        for index in range(1, 501):
            scenario['initial_conditions']['items'].append({'id': f"{line['id']}-ITEM-{index:04}", 'item_type_ref': {'id': 'benchmark-item-v1', 'revision': 'r1'}, 'quantity': quantity(1), 'holder': {'kind': 'location', 'id': line['source_queue']}, 'quality': 'released', 'components': [], 'contents': [], 'source': 'user_provided'})
    scopes = [{'product_ref': {'id': line['id'] + '-PRODUCT', 'revision': 'r1'}, 'unit': 'piece', 'destination': line['accepted_sink'], 'stage': 'factory_released'} for line in profile['lines']]
    scenario['acceptance_goal'] = {'industry': '自建性能基准', 'product_family': '同类保身份试件', 'product_specification_ref': None, 'accepted_quantity_per_day': None,
        'acceptance_scope': scopes[0], 'additional_acceptance_scopes': scopes[1:], 'production_calendar_ref': 'B60-ALL', 'observation_window': scenario['window'],
        'site_envelope': {'regions': [], 'length_m': None, 'width_m': None, 'height_m': None, 'usable_polygon_m': [], 'clearance_m': None},
        'existing_equipment': [{'kind': 'device', 'id': device['id']} for device in project['devices']], 'constraints': []}
    packages = [{'definition': definition, 'assets': [], 'files': [], 'dependencies': [ref(simple_module)] if definition is station else [], 'rights': [], 'issues': []} for definition in definitions]
    write(name + '.input.json', {'project': project, 'scenario': scenario, 'definitions': definitions, 'packages': packages})
write('fixture-summary.json', {'scope': 'prepared_real_Project_Scenario_inputs_not_runtime_or_quality_acceptance', 'profile': 'NFR-B60', 'variants': ['balanced', 'dense'], 'halls': 3, 'lines': 6, 'devices': 60, 'initial_identities': 3000, 'detailed_nodes': len(nodes), 'simplified_nodes': len(simple['nodes']), 'envelope_m': [2, 2, 3], 'representation': 'At most one exact-ID item slot per station; queue inventories retain actual IDs and holder records without invented independent positions.', 'expected_first_accept_s': 20, 'expected_last_accept_s': 1018, 'expected_accepted_at_600_s': 1746, 'expected_accepted_at_1200_s': 3000, 'buffer_capacity_mapping': 'Every store declares the closed-line universe of 500 identities as a nonbinding capacity; preserve-only routes and no arrivals/new stock make it behaviorally equivalent to an unbounded buffer for this exact frozen input.', 'actual_execution': None, 'quality_acceptance': None, 'Native': None})
print(json.dumps({'output': str(OUT), 'detailed_nodes': len(nodes), 'simplified_nodes': len(simple['nodes']), 'actual_execution': None}, ensure_ascii=False))
