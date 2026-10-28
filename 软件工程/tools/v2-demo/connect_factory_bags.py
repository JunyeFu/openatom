"""Apply the original draw.io bag design to the current actual factory inputs."""
import argparse
import copy
import math
from pathlib import Path
from prepare_full_factory import ROOT, DESIGN, read, write
from connect_full_factory_routes import quantity, pose
from connect_factory_families import replace


def prepare(source, media_source, output):
    assert not output.exists(), 'Retain previous attempts'
    output.mkdir(parents=True)
    p = read(source / 'actual-project.json'); s = copy.deepcopy(read(source / 'actual-scenario.json'))
    media = read(media_source / 'media-plan.json'); spec = read(DESIGN / 'bag-line-spec.json')
    version = spec['version']; window = spec['resources']['demonstration_window_ms']
    devices = {d['id']: copy.deepcopy(d) for d in p['devices']}
    locations = {d['id']: copy.deepcopy(d) for d in p['locations']}
    types = {d['id']: copy.deepcopy(d) for d in p['item_types']}
    connections = {d['data']['id']: copy.deepcopy(d) for d in p['connections']}
    cells = {r['steps'][0]['device_id']: r for r in p['routes'] if len(r['steps']) == 1}
    packages = read(source / 'actual-frozen-modules.json')
    exact = {(m['definition']['id'], m['definition']['version']): m for m in packages}
    def inline(device):
        reference = devices[device]['module_ref']
        return copy.deepcopy(exact[(reference['id'], reference['version'])]['definition']['payload']['data']['behavior']['inline'])
    selected = {'PF-' + name for name in ['paper-bag-tuber', 'paper-bag-bottomer', 'rope-handle-twister', 'bag-handle-attacher']}
    changed = {k: {} for k in ['item_type', 'device', 'location', 'connection', 'route', 'bom', 'product', 'calendar']}
    new_modules = copy.deepcopy(media['modules']); paths = []
    s.update(id='PF-BAG-FACTORY', title='纸包装整厂 · 五产品与纸袋提手协作', revision=1,
             project_revision=p['revision'], window={'start_ms': 0, 'end_ms': window}, scope=[])
    s['production_tasks'] = [t for t in s['production_tasks'] if not any(st['device_id'] in selected
        for st in next(r for r in p['routes'] if r['id'] == t['routing_ref']['id'])['steps'])]
    stores = {c['data']['destination_store'] for c in connections.values() if any(
        e['kind'] == 'port' and e['data']['instance_id'] in selected for e in [c['data']['source'], c['data']['destination']])}
    initial = {i['id']: i for i in s['initial_conditions']['items']}
    removed = {i['id'] for i in initial.values() if i['holder']['kind'] == 'location' and i['holder']['id'] in stores}
    while True:
        before = set(removed)
        for identity in list(removed): removed.update(initial[identity]['components'] + initial[identity]['contents'])
        if removed == before: break
    s['initial_conditions']['items'] = [i for i in initial.values() if i['id'] not in removed]
    for supply in s['supplies']:
        supply['window']['end_ms'] = window; supply['departure_windows'] = [{'start_ms': 0, 'end_ms': window}]
    for order in s['orders']: order['due_at_ms'] = window
    calendar = copy.deepcopy(next(c for c in p['calendars'] if c['id'] == 'PF-ALL'))
    calendar['intervals'] = [{'start_ms': 0, 'end_ms': window}]; changed['calendar'][calendar['id']] = calendar

    def module(name, title, category, data, dependencies=None):
        identity = 'sf.paperfactory.bag.' + name; directory = output / 'modules' / identity
        definition = {'id': identity, 'version': version, 'title': title, 'definition_state': 'runtime_candidate',
            'contribution': {'author': 'Super Factory bag-line design', 'license': 'CC0-1.0', 'assets': []},
            'parameters': [], 'attribute_schema': [], 'payload': {'category': category, 'data': data}}
        write(directory / 'module.json', {'schema_version': '1.0', 'data': {'definition': definition,
            'assets': [], 'files': [], 'dependencies': dependencies or [], 'rights': [], 'issues': []}})
        new_modules.append({'id': identity, 'version': version, 'directory': str(directory)})
        return {'id': identity, 'version': version}

    def item(name, title, numbers, tracking='individual'):
        state = media['materials'][name]['state']; attrs = {'presentation_state': {'kind': 'enum', 'value': state}}
        attrs.update({k: {'kind': 'number', 'value': v} for k, v in numbers.items()})
        schema = [{'name': 'presentation_state', 'value_type': 'enum', 'unit': None, 'shape': None, 'options': [state], 'basis': 'module_declared'}]
        schema.extend({'name': k, 'value_type': 'number', 'unit': None, 'shape': None, 'options': [], 'basis': 'module_declared'} for k in numbers)
        payload = {'unit': 'piece', 'tracking': tracking, 'spec_ref': None, 'attributes': attrs,
                   'attribute_schema': schema, 'presentation_or_media_ref': media['materials'][name]['presentation']}
        reference = module('items.' + name, title, 'items', payload, [media['materials'][name]['media_ref']])
        identity = 'PF-BAG-' + name
        value = {'id': identity, 'revision': 'r1', 'title': title, 'unit': 'piece', 'tracking': tracking,
                 'spec_ref': None, 'attributes': attrs, 'attribute_schema': schema, 'contents_layout': None, 'module_ref': reference}
        types[identity] = value; changed['item_type'][identity] = value
        return identity

    b = spec['bag']; h = spec['handles']
    body = {'bag_width_m': b['width_m'], 'bag_depth_m': b['depth_m'], 'bag_height_m': b['height_m'],
            'tube_length_m': b['tube_height_m'], 'paper_area_m2': b['paper_area_m2'], 'paper_thickness_m': b['paper_thickness_m'], 'paper_volume_m3': b['paper_volume_m3']}
    ids = {}
    for name, title in [('web', '530×240mm计量纸幅'), ('tube', '180×80×240mm开口袋筒'),
                        ('flat-tube', '侧褶展平袋筒'), ('closed-bag', '200×180mm糊底折平袋'), ('open-bag', '180×80×200mm开口纸袋')]:
        extra = {'web_length_m': b['web_length_m'], 'web_width_m': b['web_width_m']} if name == 'web' else {}
        ids[name] = item(name, title, dict(body, **extra))
    ids['strip'] = item('strip', '预裁纸条（每次三条）', {'strip_length_m': h['strip_length_m'], 'strip_width_m': h['strip_width_m'],
        'paper_thickness_m': h['strip_thickness_m'], 'visual_strips_in_lot': 3, 'paper_volume_m3': h['cord_paper_volume_m3'] / 3}, 'batch')
    ids['cord'] = item('cord', '300mm三股纸绳', {'cord_length_m': h['cord_length_m'], 'paper_strands': 3, 'paper_volume_m3': h['cord_paper_volume_m3']})
    ids['patch'] = item('patch', '预涂胶补强片（20×25×1mm）', {'length_m': .020, 'width_m': .025, 'paper_thickness_m': .001, 'paper_volume_m3': .020 * .025 * .001})
    ids['handle-pair'] = item('handle-pair', '一副双绳柄（两条绳、四片补强片）', {'handle_count': 2, 'cord_length_m': .3, 'patch_count': 4, 'paper_volume_m3': h['pair_paper_volume_m3']})
    ids['finished-bag'] = item('finished-bag', '180×80×200mm双提手纸袋', dict(body, handle_count=2, consumed_cords=2,
        patch_count=4, applied_glue_ml=2, paper_volume_m3=spec['final_paper_volume_m3']))

    mappings = {'PF-paper-bag-tuber': {'paper-bag-tuber-input': ids['web'], 'paper-bag-tuber-output': ids['tube']},
        'PF-paper-bag-bottomer': {'paper-bag-bottomer-input': ids['flat-tube'], 'paper-bag-bottomer-output': ids['closed-bag']},
        'PF-rope-handle-twister': {'rope-handle-twister-input': ids['strip'], 'rope-handle-twister-output': ids['cord']},
        'PF-bag-handle-attacher': {'bag-handle-attacher-input': ids['open-bag'], 'bag-handle-attacher-component': ids['handle-pair'], 'bag-handle-attacher-output': ids['finished-bag']}}
    for device_id, mapping in mappings.items():
        devices[device_id] = replace(devices[device_id], mapping)
        for identity, c in list(connections.items()):
            if any(e['kind'] == 'port' and e['data']['instance_id'] == device_id for e in [c['data']['source'], c['data']['destination']]):
                connections[identity] = replace(c, mapping); changed['connection'][identity] = connections[identity]
                location = c['data']['destination_store']; locations[location] = replace(locations[location], mapping); changed['location'][location] = locations[location]
        if device_id.removeprefix('PF-') in media['tooling']:
            devices[device_id]['presentation_override'] = media['tooling'][device_id.removeprefix('PF-')]
        changed['device'][device_id] = devices[device_id]

    manual = 'PF-manual-packing'; original_manual = inline(manual)
    def manual_operation(name, input_count=1, patch_count=0):
        op = copy.deepcopy(original_manual)
        def row(role, n, out=False): return {'role': role, 'port': role, 'quantity': quantity(n),
            'required_quality': [] if out else ['released'], 'declared_output_quality': 'released' if out else None}
        op.update(builtin='assemble' if patch_count else 'transform', identity_policy='assembly_with_composition' if patch_count else 'transform_with_origin',
            item_roles=['input', 'container', 'output'] if patch_count else ['input', 'output'],
            inputs=[row('input', input_count)] + ([row('container', patch_count)] if patch_count else []), outputs=[row('output', 1, True)],
            family=original_manual['family'], duration=None, material_flow=None, pack_identity=None, quality_and_routes=None,
            resource_roles=[], hold_on_output_wait=['operator'])
        for phase in op['phases']:
            phase['duration'] = {'value': spec['shared_manual_station']['manual_cycle_ms'][phase['id']], 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': None}
            phase['phase_resources'] = [{'role': 'operator', 'quantity': quantity(1, 'person'), 'qualifications': []}]
        return module('operation.' + name, {'flatten': '侧褶展平', 'open': '开袋与底角展开', 'handles': '两绳四片预制双提手'}[name], 'operations', op)
    operations = {'flatten': manual_operation('flatten'), 'open': manual_operation('open'), 'handles': manual_operation('handles', 2, 4)}
    attach = inline('PF-bag-handle-attacher')
    next(r for r in attach['inputs'] if r['role'] == 'component')['quantity'] = quantity(1)
    operations['attach'] = module('operation.attach', '一副双提手与纸袋粘接', 'operations', attach)
    devices[manual].setdefault('route_operation_refs', []).extend(operations[k] for k in ['flatten', 'open', 'handles'])
    devices['PF-bag-handle-attacher'].setdefault('route_operation_refs', []).append(operations['attach'])
    changed['device'][manual] = devices[manual]

    def port_stock(device, role, direction):
        port = devices[device]['role_bindings']['ports'][role]['port_id']
        item_type = devices[device]['role_bindings']['item_types'][role]
        for c in connections.values():
            e = c['data']['destination' if direction == 'input' else 'source']
            if c['data']['item_type'] == item_type and e['kind'] == 'port' and e['data']['instance_id'] == device and e['data']['port_id'] == port:
                return c['data']['destination_store']
        raise AssertionError((device, role))

    def stock(identity, title, slots, where):
        new = identity not in locations; loc = copy.deepcopy(locations.get(identity, next(iter(locations.values()))))
        loc.update(id=identity, title=title, hall_id='PF-H1', position=where, accepted_item_types=list(slots), capacity=quantity(sum(slots.values())))
        if new: loc.update(purpose='buffer', ordinary_issue_enabled=True)
        positions = []
        for item_type, count in slots.items():
            for _ in range(count):
                i = len(positions)
                columns, spacing = (4, .04) if identity.endswith('TABLE-patch') else (3, .4) if identity.endswith('TABLE-cord') else (3, .3) if identity.startswith('PF-BAG-TABLE-') else (10, .45)
                positions.append({'item_type_ref': {'id': item_type, 'revision': 'r1'}, 'anchor_role': None, 'offset': pose(i % columns * spacing, i // columns * spacing)})
        loc['item_layout'] = {'assignment': 'item_id_order', 'slots': positions}
        locations[identity] = loc; changed['location'][identity] = loc
        return identity

    def initial_item(identity, item_type, location, count=1):
        s['initial_conditions']['items'].append({'id': identity, 'item_type_ref': {'id': item_type, 'revision': 'r1'}, 'quantity': quantity(count),
            'holder': {'kind': 'location', 'id': location}, 'quality': 'released', 'components': [], 'contents': [], 'source': 'user_provided'})

    for device_id in sorted(selected):
        for role, item_type in devices[device_id]['role_bindings']['item_types'].items():
            direction = 'input' if role in ['input', 'component', 'auxiliary'] else 'output'
            location = port_stock(device_id, role, direction); count = 18 if item_type == ids['strip'] else 6 if item_type == ids['cord'] else 3
            stock(location, locations[location]['title'], {item_type: count}, locations[location]['position'])
            if role == 'auxiliary':
                for i in range(3): initial_item(f'PF-BAG-GLUE-{device_id}-{i}', item_type, location)
    for i in range(6): initial_item(f'PF-BAG-STRIPS-{i}', ids['strip'], port_stock('PF-rope-handle-twister', 'input', 'input'), 3)

    table_stocks = {}
    for i, (name, direction, role, count) in enumerate([('tube', 'input', 'input', 3), ('flat-tube', 'output', 'output', 3),
        ('closed-bag', 'input', 'input', 3), ('open-bag', 'output', 'output', 3), ('cord', 'input', 'input', 6),
        ('patch', 'input', 'container', 12), ('handle-pair', 'output', 'output', 3)]):
        location = stock('PF-BAG-TABLE-' + name, '人工台 · ' + types[ids[name]]['title'], {ids[name]: count}, pose(29.0 + i * 1.05, 71.7, .9))
        table_stocks[name] = location
        endpoint = {'kind': 'port', 'data': {'module_ref': devices[manual]['module_ref'], 'instance_id': manual, 'port_id': role}}
        connection = {'kind': 'material', 'data': {'id': 'PF-BAG-CONNECTION-' + name,
            'source': {'kind': 'location', 'data': location} if direction == 'input' else endpoint,
            'destination': endpoint if direction == 'input' else {'kind': 'location', 'data': location},
            'item_type': ids[name], 'unit': 'piece', 'quantity_rule': None, 'destination_store': location, 'capacity_rule': None}}
        connections[connection['data']['id']] = connection; changed['connection'][connection['data']['id']] = connection
        if direction == 'output': anchors = [('product_staging', pose(0, 0))]
        elif name == 'cord': anchors = [('product_0', pose(-.15, 0, .03)), ('product_1', pose(.15, 0, .03))]
        elif name == 'patch': anchors = [(f'product_{k}', pose(0, 0)) for k in range(2, 6)]
        else: anchors = [('product_0', pose(.1, .05, 0))]
        devices[manual]['item_layout']['slots'].extend({'item_type_ref': {'id': ids[name], 'revision': 'r1'}, 'anchor_role': anchor, 'offset': offset} for anchor, offset in anchors)
    for i in range(12): initial_item(f'PF-BAG-PATCH-{i:02d}', ids['patch'], table_stocks['patch'])
    receiving = stock('PF-BAG-RECEIVING', '纸袋计量纸幅 · 收货', {ids['web']: 3}, pose(26, 8, .9))
    for i in range(3): initial_item(f'PF-BAG-WEB-{i}', ids['web'], receiving)
    stock('PF-SHIPPING', '合格成品 · 五产品', {'PF-packed-case': 3, 'folder-gluer-output': 3, 'paper-core-grinder-output': 6,
        'label-peeler-output': 18, ids['finished-bag']: 3}, locations['PF-SHIPPING']['position'])

    def transfer(name, source_id, destination_id, item_type, count, load=1):
        a = locations[source_id]['position']['translation']; b = locations[destination_id]['position']['translation']
        road = lambda y: 16 if y < 42 else 40 if y < 66 else 64
        points = [dict(a), {'x': a['x'], 'y': road(a['y']), 'z': .9}, {'x': 52, 'y': road(a['y']), 'z': .9},
            {'x': 52, 'y': road(b['y']), 'z': .9}, {'x': b['x'], 'y': road(b['y']), 'z': .9}, dict(b)]
        points = [point for i, point in enumerate(points) if i == 0 or point != points[i - 1]]
        travel = math.ceil(sum(math.dist([a[k] for k in ['x', 'y', 'z']], [b[k] for k in ['x', 'y', 'z']]) for a, b in zip(points, points[1:])) / 1.2 * 1000)
        definition = copy.deepcopy(next(d['payload']['data'] for d in read(ROOT / 'packages/commands/tests/fixtures/production/p202-identity.json')['definitions'] if d['payload']['category'] == 'transport'))
        definition['batch_capacity'] = quantity(load); definition['loading_resources'] = [{'role': 'dispatch_gate', 'quantity': quantity(1, 'slot'), 'qualifications': []}]
        for phase, duration in [('loading', 1000), ('outbound', travel), ('unloading', 1000), ('returning', travel)]:
            definition['times'][phase] = {'value': duration, 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': None}
        reference = module('transport.' + name, '纸袋物料转运 · ' + name, 'transport', definition)
        supply = {'id': 'PF-BAG-TRANSFER-' + name, 'source': source_id, 'destination': destination_id,
            'item_type_ref': {'id': item_type, 'revision': 'r1'}, 'target_quantity': quantity(count), 'window': {'start_ms': 0, 'end_ms': window},
            'transport_ref': reference, 'resources': {'carrier': 'PF-CARRIERS', 'dispatch_gate': 'PF-TRANSIT-GATE'},
            'departure_windows': [{'start_ms': 0, 'end_ms': window}], 'allow_partial_load': load == 1,
            'presentation_path': {'hall_id': 'PF-H1', 'waypoints_m': points, 'cargo_layout': {'assignment': 'item_id_order',
                'slots': [{'item_type_ref': {'id': item_type, 'revision': 'r1'}, 'anchor_role': None, 'offset': pose((i - (load - 1) / 2) * .4, 0, .08)} for i in range(load)]}}}
        s['supplies'].append(supply); paths.append({'id': supply['id'], 'quantity': count, 'load': load, 'expected_trips': count // load, 'waypoints_m': points})
    transfer('web', receiving, port_stock('PF-paper-bag-tuber', 'input', 'input'), ids['web'], 3)
    transfer('tube-flat', port_stock('PF-paper-bag-tuber', 'output', 'output'), table_stocks['tube'], ids['tube'], 3)
    transfer('flat-bottom', table_stocks['flat-tube'], port_stock('PF-paper-bag-bottomer', 'input', 'input'), ids['flat-tube'], 3)
    transfer('bottom-open', port_stock('PF-paper-bag-bottomer', 'output', 'output'), table_stocks['closed-bag'], ids['closed-bag'], 3)
    transfer('open-attach', table_stocks['open-bag'], port_stock('PF-bag-handle-attacher', 'input', 'input'), ids['open-bag'], 3)
    transfer('ship', port_stock('PF-bag-handle-attacher', 'output', 'output'), 'PF-SHIPPING', ids['finished-bag'], 3)
    transfer('cord-handles', port_stock('PF-rope-handle-twister', 'output', 'output'), table_stocks['cord'], ids['cord'], 6, 2)
    transfer('handles-attach', table_stocks['handle-pair'], port_stock('PF-bag-handle-attacher', 'component', 'input'), ids['handle-pair'], 3)

    def step(name, device, inputs, predecessor, operation=None):
        result = copy.deepcopy(cells[device]['steps'][0]); result.update(id=name, predecessors=predecessor, item_bindings=inputs)
        if operation: result['operation_ref'] = operations[operation]
        return result
    body_steps = [step('bag-tube', 'PF-paper-bag-tuber', devices['PF-paper-bag-tuber']['role_bindings']['item_types'], []),
        step('bag-flatten', manual, {'input': ids['tube'], 'output': ids['flat-tube']}, ['bag-tube'], 'flatten'),
        step('bag-bottom', 'PF-paper-bag-bottomer', devices['PF-paper-bag-bottomer']['role_bindings']['item_types'], ['bag-flatten']),
        step('bag-open', manual, {'input': ids['closed-bag'], 'output': ids['open-bag']}, ['bag-bottom'], 'open'),
        step('bag-attach', 'PF-bag-handle-attacher', devices['PF-bag-handle-attacher']['role_bindings']['item_types'], ['bag-open'], 'attach')]
    rope_bindings = devices['PF-rope-handle-twister']['role_bindings']['item_types']
    handle_steps = [step('handle-rope-a', 'PF-rope-handle-twister', rope_bindings, []), step('handle-rope-b', 'PF-rope-handle-twister', rope_bindings, []),
        step('handle-pair', manual, {'input': ids['cord'], 'container': ids['patch'], 'output': ids['handle-pair']}, ['handle-rope-a', 'handle-rope-b'], 'handles')]
    for identity, steps, final, allocated in [('BAG', body_steps, ids['finished-bag'], True), ('HANDLES', handle_steps, ids['handle-pair'], False)]:
        route_id = 'PF-ROUTE-' + identity; bom_id = 'PF-BOM-' + identity; product_id = 'PF-PRODUCT-' + identity; order_id = 'PF-ORDER-' + identity
        changed['route'][route_id] = {'id': route_id, 'revision': 'r1', 'steps': steps}
        rows = []
        for st in steps:
            if st['operation_ref'] in operations.values():
                op = read(output / 'modules' / st['operation_ref']['id'] / 'module.json')['data']['definition']['payload']['data']
            else: op = inline(st['device_id'])
            rows.extend({'item_type_ref': {'id': st['item_bindings'][r['role']], 'revision': 'r1'}, 'quantity': r['quantity'], 'consume_at_operation': st['id']} for r in op['inputs'])
        changed['bom'][bom_id] = {'id': bom_id, 'revision': 'r1', 'rows': rows}
        changed['product'][product_id] = {'id': product_id, 'revision': 'r1', 'item_type_ref': {'id': final, 'revision': 'r1'},
            'bom_ref': {'id': bom_id, 'revision': 'r1'}, 'routing_ref': {'id': route_id, 'revision': 'r1'}, 'artifact_refs': []}
        if allocated: s['orders'].append({'id': order_id, 'product_ref': {'id': product_id, 'revision': 'r1'}, 'quantity': quantity(3),
            'release_at_ms': 0, 'due_at_ms': window, 'destination': 'PF-SHIPPING', 'acceptance_stage': 'unloaded_and_released', 'allow_partial': True, 'priority': 10})
        s['production_tasks'].append({'id': 'PF-TASK-' + identity, 'routing_ref': {'id': route_id, 'revision': 'r1'},
            'order_allocations': [{'order_id': order_id, 'quantity': quantity(3)}] if allocated else [], 'planned_quantity': quantity(3), 'release_at_ms': 0, 'priority': 10})
    checks = []
    for name in ['SHORT-PATCHES', 'MISSING-GLUE']:
        check = copy.deepcopy(s); check.update(id='PF-BAG-' + name, title='纸袋物料边界 · ' + name)
        check['initial_conditions']['items'] = [i for i in check['initial_conditions']['items'] if
            not (name == 'SHORT-PATCHES' and i['id'] in {f'PF-BAG-PATCH-{n:02d}' for n in range(8, 12)}) and
            not (name == 'MISSING-GLUE' and i['id'].startswith('PF-BAG-GLUE-PF-bag-handle-attacher-'))]
        checks.append(check)
    commands = [{'kind': 'put_object', 'data': {'kind': kind, 'data': row}} for kind, values in changed.items() for row in values.values()]
    commands.append({'kind': 'put_object', 'data': {'kind': 'scenario', 'data': s}})
    write(output / 'commands.json', {'base_revision': p['revision'], 'commands': commands, 'new_modules': new_modules,
        'scenario_id': s['id'], 'quality_checks': checks, 'expected': spec['expected']})
    write(output / 'bag-design-consumption.json', {'specification': spec, 'input_source': str(source), 'media_source': str(media_source), 'transport': paths,
        'all_device_modules_unchanged': all(d['module_ref'] == next(o['module_ref'] for o in p['devices'] if o['id'] == d['id']) for d in devices.values())})
    print('Staged actual bag factory:', len(new_modules), 'new exact modules;', len(paths), 'new supply legs;', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--media', type=Path, required=True); parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(); prepare(args.source, args.media, args.output)
