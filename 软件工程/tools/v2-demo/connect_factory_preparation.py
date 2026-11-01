"""Consume the original preparation design through public Host edit commands."""
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
    spec = read(DESIGN / 'preparation-line-spec.json'); media = read(media_source / 'media-plan.json')
    packages = read(source / 'actual-frozen-modules.json')
    exact = {(m['definition']['id'], m['definition']['version']): m for m in packages}
    devices = {d['id']: copy.deepcopy(d) for d in p['devices']}
    locations = {d['id']: copy.deepcopy(d) for d in p['locations']}
    types = {d['id']: copy.deepcopy(d) for d in p['item_types']}
    connections = {d['data']['id']: copy.deepcopy(d) for d in p['connections']}
    cells = {r['steps'][0]['device_id']: r for r in p['routes'] if len(r['steps']) == 1}
    selected = {'PF-' + name for b in spec['branches'] for name in b['devices']}
    changed = {k: {} for k in ['item_type', 'device', 'location', 'connection', 'route', 'bom', 'product']}
    modules = copy.deepcopy(media['modules']); paths = []; aliases = []
    window = spec['resources']['window_ms']

    def operation(device):
        r = devices[device]['module_ref']
        return exact[(r['id'], r['version'])]['definition']['payload']['data']['behavior']['inline']

    def module(name, title, category, data, dependencies=None):
        identity = 'sf.paperfactory.preparation.' + name; directory = output / 'modules' / identity
        definition = {'id': identity, 'version': spec['version'], 'title': title, 'definition_state': 'runtime_candidate',
                      'contribution': {'author': 'Super Factory preparation composition', 'license': 'CC0-1.0', 'assets': []},
                      'parameters': [], 'attribute_schema': [], 'payload': {'category': category, 'data': data}}
        write(directory / 'module.json', {'schema_version': '1.0', 'data': {'definition': definition,
              'assets': [], 'files': [], 'dependencies': dependencies or [], 'rights': [], 'issues': []}})
        ref = {'id': identity, 'version': spec['version']}; modules.append(dict(ref, directory=str(directory)))
        return ref

    s.update(id='PF-PREPARATION-FACTORY', title='纸包装整厂 · 八产品与原纸表面处理', revision=1, scope=[])
    routes = {r['id']: r for r in p['routes']}
    s['production_tasks'] = [t for t in s['production_tasks'] if not any(st['device_id'] in selected for st in routes[t['routing_ref']['id']]['steps'])]
    stores = {c['data']['destination_store'] for c in connections.values() if any(
        e['kind'] == 'port' and e['data']['instance_id'] in selected for e in [c['data']['source'], c['data']['destination']])}
    initial = {i['id']: i for i in s['initial_conditions']['items']}
    removed = {i['id'] for i in initial.values() if i['holder']['kind'] == 'location' and i['holder']['id'] in stores}
    while True:
        old = set(removed)
        for identity in list(removed): removed.update(initial[identity]['components'] + initial[identity]['contents'])
        if old == removed: break
    s['initial_conditions']['items'] = [i for i in initial.values() if i['id'] not in removed]
    mappings = {
        'PF-paper-rewinder': {'paper-rewinder-input': 'paper-unwinder-input'},
        'PF-rotary-sheet-cutter': {'rotary-sheet-cutter-input': 'paper-unwinder-input'},
        'PF-sheet-slitter': {'sheet-slitter-input': 'board-edge-trimmer-output'},
        'PF-uv-curer': {'uv-curer-input': 'varnish-coater-input'},
    }
    for device, mapping in mappings.items():
        for old, new in mapping.items():
            a, b = types[old], types[new]
            physical = lambda t: {k: v for k, v in t['attributes'].items() if k != 'presentation_state'}
            assert (a['unit'], a['tracking'], physical(a)) == (b['unit'], b['tracking'], physical(b)), (old, new)
            aliases.append({'device': device, 'old_type': old, 'shared_type': new, 'equal_physical_attributes': physical(a)})
        devices[device] = replace(devices[device], mapping)
        for identity, connection in list(connections.items()):
            if any(e['kind'] == 'port' and e['data']['instance_id'] == device for e in [connection['data']['source'], connection['data']['destination']]):
                connections[identity] = replace(connection, mapping); changed['connection'][identity] = connections[identity]
                stock_id = connection['data']['destination_store']; locations[stock_id] = replace(locations[stock_id], mapping)
    for device in selected: changed['device'][device] = devices[device]
    for identity, model in media['materials'].items():
        t = types[identity]
        payload = {k: copy.deepcopy(t[k]) for k in ['unit', 'tracking', 'spec_ref', 'attributes', 'attribute_schema']}
        payload['presentation_or_media_ref'] = model['presentation']
        t['module_ref'] = module('items.' + identity, t['title'], 'items', payload, [model['media_ref']])
        changed['item_type'][identity] = t
    # Shared material names describe the stock, rather than one previous owner.
    for identity, title in {'paper-unwinder-input': '30m×500mm原纸',
                            'rotary-sheet-cutter-output': '500×500mm裁张原纸',
                            'board-edge-trimmer-output': '500×400mm净纸板',
                            'sheet-slitter-output': '500×200mm分切纸板',
                            'varnish-coater-input': '500×400mm上光纸板',
                            'PF-b01-q20-load-carton': '载运纸箱',
                            'PF-b01-q19-content-unit': '装箱内件'}.items():
        types[identity]['title'] = title
        changed['item_type'][identity] = types[identity]

    def port_stock(device, role, direction):
        port = devices[device]['role_bindings']['ports'][role]['port_id']
        return next(c['data']['destination_store'] for c in connections.values() if
                    (e := c['data']['destination' if direction == 'input' else 'source'])['kind'] == 'port'
                    and e['data']['instance_id'] == device and e['data']['port_id'] == port)

    def stock(identity, title, slots, position):
        new = identity not in locations
        loc = copy.deepcopy(locations.get(identity, next(iter(locations.values()))))
        loc.update(id=identity, title=title, hall_id='PF-H1', position=position,
                   accepted_item_types=list(slots), capacity=quantity(sum(slots.values())))
        if new: loc.update(purpose='buffer', ordinary_issue_enabled=True)
        layout = []
        for type_index, (item, count) in enumerate(slots.items()):
            if identity == 'PF-SHIPPING':
                plan = spec['storage_layout']['shipping']; per_stack, pitch = plan['stacks'][item]
                ox = plan['origin_xy_m'][0] + type_index % plan['type_columns'] * plan['type_spacing_xy_m'][0]
                oy = plan['origin_xy_m'][1] + type_index // plan['type_columns'] * plan['type_spacing_xy_m'][1]
                columns = plan['stack_columns']; sx, sy = plan['stack_spacing_xy_m']
            else:
                plan = spec['storage_layout']['process_buffers']; thickness = types[item]['attributes'].get('thickness_m')
                per_stack = plan['flat_paper_per_stack'] if thickness else 1
                pitch = thickness['value'] + plan['flat_paper_display_gap_m'] if thickness else 0
                ox, oy = (type_index - (len(slots) - 1) / 2) * plan['type_spacing_m'], 0
                columns = plan['stack_columns']; sx = sy = plan['stack_spacing_m']
            stacks = math.ceil(count / per_stack); columns = min(columns, stacks); rows = math.ceil(stacks / columns)
            for i in range(count):
                stack = i // per_stack
                layout.append({'item_type_ref': {'id': item, 'revision': 'r1'}, 'anchor_role': None,
                               'offset': pose(ox + (stack % columns - (columns - 1) / 2) * sx,
                                              oy + (stack // columns - (rows - 1) / 2) * sy, i % per_stack * pitch)})
        loc['item_layout'] = {'assignment': 'item_id_order', 'slots': layout}
        locations[identity] = loc; changed['location'][identity] = loc
        return identity

    def initial_item(identity, item, location):
        s['initial_conditions']['items'].append({'id': identity, 'item_type_ref': {'id': item, 'revision': 'r1'}, 'quantity': quantity(),
            'holder': {'kind': 'location', 'id': location}, 'quality': 'released', 'components': [], 'contents': [], 'source': 'user_provided'})

    stock_slots = {}
    for device in selected:
        cycles = next(b['cycles'] for b in spec['branches'] if device.removeprefix('PF-') in b['devices'])
        op = operation(device)
        for role, item in devices[device]['role_bindings']['item_types'].items():
            direction = 'input' if role in ['input', 'component', 'auxiliary'] else 'output'
            location = port_stock(device, role, direction)
            row = next(r for r in op['inputs' if direction == 'input' else 'outputs'] if r['role'] == role)
            count = cycles * row['quantity']['value']['coefficient'] // 10 ** row['quantity']['value']['decimal_places']
            stock_slots.setdefault(location, {})[item] = count
            if role == 'auxiliary':
                for i in range(3): initial_item(f'PF-PREP-COATING-{i}', item, location)
    for location, slots in stock_slots.items():
        stock(location, locations[location]['title'], slots, locations[location]['position'])

    def transfer(name, source_id, destination_id, item, count, load=1):
        a = locations[source_id]['position']['translation']; b = locations[destination_id]['position']['translation']
        road = lambda y: 16 if y < 42 else 40 if y < 66 else 64
        points = [dict(a), {'x': a['x'], 'y': road(a['y']), 'z': .9}, {'x': 52, 'y': road(a['y']), 'z': .9},
                  {'x': 52, 'y': road(b['y']), 'z': .9}, {'x': b['x'], 'y': road(b['y']), 'z': .9}, dict(b)]
        points = [v for i, v in enumerate(points) if i == 0 or v != points[i - 1]]
        travel = math.ceil(sum(math.dist([a[k] for k in ['x', 'y', 'z']], [b[k] for k in ['x', 'y', 'z']]) for a, b in zip(points, points[1:])) / 1.2 * 1000)
        definition = copy.deepcopy(next(d['payload']['data'] for d in read(ROOT / 'packages/commands/tests/fixtures/production/p202-identity.json')['definitions'] if d['payload']['category'] == 'transport'))
        definition['batch_capacity'] = quantity(load)
        definition['loading_resources'] = [{'role': 'dispatch_gate', 'quantity': quantity(1, 'slot'), 'qualifications': []}]
        for phase, duration in [('loading', 1000), ('outbound', travel), ('unloading', 1000), ('returning', travel)]:
            definition['times'][phase] = {'value': duration, 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': None}
        ref = module('transport.' + name, '原纸与表面处理转运 · ' + name, 'transport', definition)
        supply = {'id': 'PF-PREP-TRANSFER-' + name, 'source': source_id, 'destination': destination_id,
            'item_type_ref': {'id': item, 'revision': 'r1'}, 'target_quantity': quantity(count), 'window': {'start_ms': 0, 'end_ms': window},
            'transport_ref': ref, 'resources': {'carrier': 'PF-CARRIERS', 'dispatch_gate': 'PF-TRANSIT-GATE'},
            'departure_windows': [{'start_ms': 0, 'end_ms': window}], 'allow_partial_load': load == 1,
            'presentation_path': {'hall_id': 'PF-H1', 'waypoints_m': points, 'cargo_layout': {'assignment': 'item_id_order',
                'slots': [{'item_type_ref': {'id': item, 'revision': 'r1'}, 'anchor_role': None, 'offset': pose(0, 0, .08 + i * .001)} for i in range(load)]}}}
        s['supplies'].append(supply); paths.append({'id': supply['id'], 'count': count, 'load': load, 'trips': count // load, 'waypoints_m': points})

    shipping = 'PF-SHIPPING'
    ship_slots = {t: sum(1 for slot in locations[shipping]['item_layout']['slots'] if slot['item_type_ref']['id'] == t) for t in locations[shipping]['accepted_item_types']}
    for index, branch in enumerate(spec['branches']):
        identity = branch['id']; names = branch['devices']; cycles = branch['cycles']; count = cycles * branch['yield_per_cycle']
        first, last = 'PF-' + names[0], 'PF-' + names[-1]
        raw = devices[first]['role_bindings']['item_types']['input']; final = devices[last]['role_bindings']['item_types']['output']
        receiving = stock('PF-PREP-RECEIVING-' + identity, '来料 · ' + branch['title'], {raw: cycles}, pose(30 + index * 3, 8, .9))
        for i in range(cycles): initial_item(f'PF-PREP-{identity}-raw-{i}', raw, receiving)
        transfer(identity + '-feed', receiving, port_stock(first, 'input', 'input'), raw, cycles)
        steps = []; bom = []
        for i, name in enumerate(names):
            device = 'PF-' + name; st = copy.deepcopy(cells[device]['steps'][0])
            st.update(id=f'{identity}-{i}', predecessors=[f'{identity}-{i-1}'] if i else [], item_bindings=devices[device]['role_bindings']['item_types'])
            steps.append(st)
            bom.extend({'item_type_ref': {'id': st['item_bindings'][r['role']], 'revision': 'r1'}, 'quantity': r['quantity'], 'consume_at_operation': st['id']} for r in operation(device)['inputs'])
            if i:
                previous = 'PF-' + names[i-1]; item = devices[previous]['role_bindings']['item_types']['output']
                assert item == devices[device]['role_bindings']['item_types']['input']
                transfer(identity + f'-leg-{i}', port_stock(previous, 'output', 'output'), port_stock(device, 'input', 'input'), item, cycles)
        route, product, bom_id, order = ['PF-' + prefix + '-' + identity for prefix in ['ROUTE', 'PRODUCT', 'BOM', 'ORDER']]
        changed['route'][route] = {'id': route, 'revision': 'r1', 'steps': steps}
        changed['bom'][bom_id] = {'id': bom_id, 'revision': 'r1', 'rows': bom}
        changed['product'][product] = {'id': product, 'revision': 'r1', 'item_type_ref': {'id': final, 'revision': 'r1'},
            'bom_ref': {'id': bom_id, 'revision': 'r1'}, 'routing_ref': {'id': route, 'revision': 'r1'}, 'artifact_refs': []}
        s['orders'].append({'id': order, 'product_ref': {'id': product, 'revision': 'r1'}, 'quantity': quantity(count), 'release_at_ms': 0,
            'due_at_ms': window, 'destination': shipping, 'acceptance_stage': 'unloaded_and_released', 'allow_partial': True, 'priority': 10})
        s['production_tasks'].append({'id': 'PF-TASK-' + identity, 'routing_ref': {'id': route, 'revision': 'r1'},
            'order_allocations': [{'order_id': order, 'quantity': quantity(count)}], 'planned_quantity': quantity(count), 'release_at_ms': 0, 'priority': 10})
        ship_slots[final] = count
        transfer(identity + '-ship', port_stock(last, 'output', 'output'), shipping, final, count, branch['load'])
    stock(shipping, '合格成品 · 八产品', ship_slots, locations[shipping]['position'])
    check = copy.deepcopy(s); check.update(id='PF-PREP-MISSING-COATING', title='上光物料边界 · 缺光油')
    check['initial_conditions']['items'] = [i for i in check['initial_conditions']['items'] if not i['id'].startswith('PF-PREP-COATING-')]
    commands = [{'kind': 'put_object', 'data': {'kind': kind, 'data': row}} for kind, values in changed.items() for row in values.values()]
    commands.append({'kind': 'put_object', 'data': {'kind': 'scenario', 'data': s}})
    write(output / 'commands.json', {'base_revision': p['revision'], 'commands': commands, 'new_modules': modules,
        'scenario_id': s['id'], 'quality_checks': [check], 'material_checks': {check['id']: {'PF-ORDER-CURED': 0}}, 'expected': spec['expected']})
    write(output / 'design-consumption.json', {'specification': spec, 'source': str(source), 'media_source': str(media_source),
        'compatible_aliases': aliases, 'new_transport': paths, 'accepted_device_modules_unchanged': True})
    print('Staged three additional exact branches:', len(modules), 'modules;', len(paths), 'transport legs;', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ['source', 'media', 'output']: parser.add_argument('--' + arg, type=Path, required=True)
    args = parser.parse_args(); prepare(args.source, args.media, args.output)
