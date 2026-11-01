"""Extend the saved factory with exact-compatible tube and label production.

Three wound tubes become six ground cores; three printed webs become eighteen
labels. The original two routes, source scenarios and Runs remain untouched.
"""
import argparse
import copy
import math
import shutil
from pathlib import Path

from prepare_full_factory import ROOT, read, write
from connect_full_factory_routes import quantity, pose

WINDOW = 3600000  # Explicit one-hour demonstration shift, including vehicle returns.
TUBES = ['paper-tube-winder', 'paper-tube-cutoff', 'paper-core-grinder']
LABELS = ['digital-label-printer', 'label-die-cutter', 'label-web-inspector', 'label-peeler']


def replace(value, mapping):
    if isinstance(value, str): return mapping.get(value, value)
    if isinstance(value, list): return [replace(v, mapping) for v in value]
    if isinstance(value, dict): return {k: replace(v, mapping) for k, v in value.items()}
    return value


def prepare(source, output):
    assert not output.exists(), 'Retain each previous attempt'
    output.mkdir(parents=True)
    p = read(source / 'actual-project.json')
    s = copy.deepcopy(read(source / 'actual-scenario.json'))
    saved = source / 'project'
    packages = read(source / 'actual-frozen-modules.json')
    by_ref = {(m['definition']['id'], m['definition']['version']): m for m in packages}
    def package(ref): return by_ref[(ref['id'], ref['version'])]
    devices = {v['id']: copy.deepcopy(v) for v in p['devices']}
    locations = {v['id']: copy.deepcopy(v) for v in p['locations']}
    types = {v['id']: copy.deepcopy(v) for v in p['item_types']}
    connections = {v['data']['id']: copy.deepcopy(v) for v in p['connections']}
    cells = {r['steps'][0]['device_id']: r for r in p['routes'] if len(r['steps']) == 1}
    selected = {'PF-' + name for name in TUBES + LABELS}
    changed = {kind: {} for kind in ['item_type', 'device', 'location', 'connection', 'route', 'bom', 'product', 'calendar']}
    new_modules = []
    paths = []
    s.update(id='PF-MULTI-FAMILY', title='纸包装整厂 · 纸套、装箱、纸管与标签', revision=1,
             project_revision=p['revision'], window={'start_ms': 0, 'end_ms': WINDOW}, scope=[])
    s['production_tasks'] = [t for t in s['production_tasks'] if not any(
        st['device_id'] in selected for st in next(r for r in p['routes'] if r['id'] == t['routing_ref']['id'])['steps'])]
    for supply in s['supplies']:
        supply['window']['end_ms'] = WINDOW
        supply['departure_windows'] = [{'start_ms': 0, 'end_ms': WINDOW}]
    for order in s['orders']: order['due_at_ms'] = WINDOW
    selected_stocks = {c['data']['destination_store'] for c in connections.values() if any(
        e['kind'] == 'port' and e['data']['instance_id'] in selected for e in [c['data']['source'], c['data']['destination']])}
    initial = {i['id']: i for i in s['initial_conditions']['items']}
    removed = {i['id'] for i in initial.values() if i['holder']['kind'] == 'location' and i['holder']['id'] in selected_stocks}
    while True:
        before = set(removed)
        for identity in list(removed): removed.update(initial[identity]['contents'] + initial[identity]['components'])
        if removed == before: break
    s['initial_conditions']['items'] = [i for i in initial.values() if i['id'] not in removed]
    inspector = devices['PF-label-web-inspector']['module_ref']
    s['quality_presets'] = [q for q in s['quality_presets'] if q['inspection_module'] != inspector]
    calendar = copy.deepcopy(next(c for c in p['calendars'] if c['id'] == 'PF-ALL'))
    calendar['intervals'] = [{'start_ms': 0, 'end_ms': WINDOW}]
    changed['calendar'][calendar['id']] = calendar

    def module(identity, title, category, payload, assets=None, rights=None, dependencies=None):
        directory = output / 'modules' / identity
        definition = {'id': identity, 'version': '1.0.0', 'title': title, 'definition_state': 'runtime_candidate',
                      'contribution': {'author': 'Super Factory factory composition', 'license': 'CC0-1.0', 'assets': rights or []},
                      'parameters': [], 'attribute_schema': [], 'payload': {'category': category, 'data': payload}}
        write(directory / 'module.json', {'schema_version': '1.0', 'data': {'definition': definition,
              'assets': assets or [], 'files': [{'path': a['path'], 'kind': 'asset'} for a in assets or []],
              'dependencies': dependencies or [], 'rights': rights or [], 'issues': []}})
        new_modules.append({'id': identity, 'version': '1.0.0', 'directory': str(directory)})
        return {'id': identity, 'version': '1.0.0'}

    # Only presentation-state aliases differ in these exact physical stocks.
    aliases = {
        'PF-paper-tube-winder': {'paper-tube-winder-output': 'paper-tube-cutoff-input'},
        'PF-paper-tube-cutoff': {'paper-tube-cutoff-output': 'paper-core-grinder-input'},
        'PF-digital-label-printer': {'digital-label-printer-output': 'label-die-cutter-input'},
        'PF-label-die-cutter': {'label-die-cutter-output': 'label-web-inspector-input'},
        'PF-label-peeler': {'label-peeler-input': 'label-web-inspector-input'},
    }
    specifications = []
    for device_id, mapping in aliases.items():
        for old, new in mapping.items():
            a, b = types[old], types[new]
            assert (a['unit'], a['tracking']) == (b['unit'], b['tracking'])
            attrs = lambda t: {k: v for k, v in t['attributes'].items() if k != 'presentation_state'}
            assert attrs(a) == attrs(b), (old, new, attrs(a), attrs(b))
            specifications.append({'source_type': old, 'destination_type': new, 'physical_attributes_equal': True,
                                   'state_aliases': [a['attributes']['presentation_state']['value'], b['attributes']['presentation_state']['value']]})
        devices[device_id] = replace(devices[device_id], mapping)
        for identity, c in list(connections.items()):
            if any(e['kind'] == 'port' and e['data']['instance_id'] == device_id for e in [c['data']['source'], c['data']['destination']]):
                connections[identity] = replace(c, mapping)
                changed['connection'][identity] = connections[identity]
                stock_id = c['data']['destination_store']
                locations[stock_id] = replace(locations[stock_id], mapping)
                changed['location'][stock_id] = locations[stock_id]
    for identity in selected: changed['device'][identity] = devices[identity]

    # A physical web is unchanged by inspection. Its static cut-web view is used
    # at the scanner, and only the peeler drives the web's separation animation.
    item_owners = {}
    for device_id in sorted(selected):
        for type_id in devices[device_id]['role_bindings']['item_types'].values():
            item_owners[type_id] = (type_id, device_id)
    item_owners.update({
        'paper-tube-cutoff-input': ('paper-tube-cutoff-input', 'PF-paper-tube-cutoff'),
        'paper-core-grinder-input': ('paper-core-grinder-input', 'PF-paper-core-grinder'),
        'label-die-cutter-input': ('label-die-cutter-input', 'PF-label-die-cutter'),
        'label-web-inspector-input': ('label-peeler-input', 'PF-label-peeler'),
    })
    for type_id, (media_type, device_id) in sorted(item_owners.items()):
        item = copy.deepcopy(package(types[media_type]['module_ref']))
        wrapper = item['definition']['payload']['data']['presentation_or_media_ref']
        old_media_ref = wrapper['source']['data']
        old_media = package(old_media_ref)
        media = copy.deepcopy(old_media['definition']['payload']['data'])
        assert media['source']['kind'] == 'asset' and len(old_media['assets']) == 1
        old_asset = media['source']['data']
        media_id = 'sf.paperfactory.families.media.' + type_id
        asset = {'id': media_id + '.glb', 'path': 'material.glb', 'kind': 'glb'}
        renames = {old_asset['id']: asset['id'], old_asset['path']: asset['path']}
        media = replace(media, renames)
        operation = cells[device_id]['steps'][0]['operation_ref']
        for clip in media['timeline']:
            if ':' in clip['phase_ref']:
                state, phase = clip['phase_ref'].split(':', 1)
                clip['phase_ref'] = f"{state}:{operation['id']}:{operation['version']}:{phase}"
        rights = replace(copy.deepcopy(old_media['rights']), renames)
        media_ref = module(media_id, types[type_id]['title'] + ' · 工序动画', 'media', media, [asset], rights)
        shutil.copyfile(saved / old_asset['path'], output / 'modules' / media_id / asset['path'])
        def refs(value):
            if value == old_media_ref: return media_ref
            if isinstance(value, dict): return {k: refs(v) for k, v in value.items()}
            if isinstance(value, list): return [refs(v) for v in value]
            return value
        payload = {k: copy.deepcopy(types[type_id][k]) for k in ['unit', 'tracking', 'spec_ref', 'attributes', 'attribute_schema']}
        payload['presentation_or_media_ref'] = refs(wrapper)
        item_ref = module('sf.paperfactory.families.items.' + type_id, types[type_id]['title'], 'items', payload, dependencies=[media_ref])
        types[type_id]['module_ref'] = item_ref
        changed['item_type'][type_id] = types[type_id]

    def port_stock(device_id, role, direction):
        port_id = devices[device_id]['role_bindings']['ports'][role]['port_id']
        for c in connections.values():
            e = c['data']['destination' if direction == 'input' else 'source']
            if e['kind'] == 'port' and e['data']['instance_id'] == device_id and e['data']['port_id'] == port_id:
                return c['data']['destination_store']
        raise AssertionError((device_id, role, direction))

    def stock(identity, title, slots, where):
        new = identity not in locations
        loc = copy.deepcopy(locations.get(identity, next(iter(locations.values()))))
        loc.update(id=identity, title=title, hall_id='PF-H1', position=where,
                   accepted_item_types=list(slots), capacity=quantity(sum(slots.values())))
        if new: loc.update(purpose='buffer', ordinary_issue_enabled=True)
        layout = []
        for type_id, count in slots.items():
            for _ in range(count):
                index = len(layout)
                layout.append({'item_type_ref': {'id': type_id, 'revision': types[type_id]['revision']},
                               'anchor_role': None, 'offset': pose(index % 10 * .45, index // 10 * .45)})
        loc['item_layout'] = {'assignment': 'item_id_order', 'slots': layout}
        locations[identity] = loc; changed['location'][identity] = loc
        return identity

    def initial_item(identity, type_id, location):
        s['initial_conditions']['items'].append({'id': identity, 'item_type_ref': {'id': type_id, 'revision': types[type_id]['revision']},
            'quantity': quantity(), 'holder': {'kind': 'location', 'id': location}, 'quality': 'released',
            'components': [], 'contents': [], 'source': 'user_provided'})

    for device_id in sorted(selected):
        for role, type_id in devices[device_id]['role_bindings']['item_types'].items():
            direction = 'input' if role in ['input', 'component', 'auxiliary'] else 'output'
            stock_id = port_stock(device_id, role, direction)
            count = 18 if (device_id, role) == ('PF-label-peeler', 'output') else 6 if device_id == 'PF-paper-core-grinder' or (device_id, role) == ('PF-paper-tube-cutoff', 'output') else 3
            stock(stock_id, locations[stock_id]['title'], {type_id: count}, locations[stock_id]['position'])
    for name in ['paper-tube-winder', 'digital-label-printer']:
        device_id = 'PF-' + name
        for role in devices[device_id]['role_bindings']['item_types']:
            if role not in ['component', 'auxiliary']: continue
            type_id = devices[device_id]['role_bindings']['item_types'][role]
            for i in range(3): initial_item(f'PF-FAMILY-{name}-{role}-{i}', type_id, port_stock(device_id, role, 'input'))

    shipping = 'PF-SHIPPING'
    stock(shipping, '合格成品 · 纸套、纸箱、纸管与标签', {'PF-packed-case': 3, 'folder-gluer-output': 3,
          'paper-core-grinder-output': 6, 'label-peeler-output': 18}, locations[shipping]['position'])
    locations[shipping]['purpose'] = 'output'

    def transfer(identity, source_id, destination_id, type_id, count, load_size=1):
        a = locations[source_id]['position']['translation']; b = locations[destination_id]['position']['translation']
        road = lambda y: 16 if y < 42 else 40 if y < 66 else 64
        points = [dict(a), {'x': a['x'], 'y': road(a['y']), 'z': .9}, {'x': 52, 'y': road(a['y']), 'z': .9},
                  {'x': 52, 'y': road(b['y']), 'z': .9}, {'x': b['x'], 'y': road(b['y']), 'z': .9}, dict(b)]
        points = [v for i, v in enumerate(points) if i == 0 or v != points[i - 1]]
        distance = sum(math.dist([a[k] for k in ['x', 'y', 'z']], [b[k] for k in ['x', 'y', 'z']]) for a, b in zip(points, points[1:]))
        travel = math.ceil(distance / 1.2 * 1000)
        transport = copy.deepcopy(next(d['payload']['data'] for d in read(ROOT / 'packages/commands/tests/fixtures/production/p202-identity.json')['definitions'] if d['payload']['category'] == 'transport'))
        transport['batch_capacity'] = quantity(load_size)
        transport['loading_resources'] = [{'role': 'dispatch_gate', 'quantity': quantity(1, 'slot'), 'qualifications': []}]
        for key, time in [('loading', 1000), ('outbound', travel), ('unloading', 1000), ('returning', travel)]:
            transport['times'][key] = {'value': time, 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': None}
        ref = module('sf.paperfactory.families.transport.' + identity, '厂内转运 · ' + identity, 'transport', transport)
        supply = {'id': identity, 'source': source_id, 'destination': destination_id,
                  'item_type_ref': {'id': type_id, 'revision': types[type_id]['revision']}, 'target_quantity': quantity(count),
                  'window': {'start_ms': 0, 'end_ms': WINDOW}, 'transport_ref': ref,
                  'resources': {'carrier': 'PF-CARRIERS', 'dispatch_gate': 'PF-TRANSIT-GATE'},
                  'departure_windows': [{'start_ms': 0, 'end_ms': WINDOW}], 'allow_partial_load': load_size == 1}
        offsets = [pose(0, 0, .08)] if load_size == 1 else [pose(-.2, 0, .08), pose(.2, 0, .08)] if load_size == 2 else [pose((i % 3 - 1) * .1, (i // 3 - .5) * .1, .08) for i in range(6)]
        supply['presentation_path'] = {'hall_id': 'PF-H1', 'waypoints_m': points, 'cargo_layout': {'assignment': 'item_id_order',
              'slots': [{'item_type_ref': supply['item_type_ref'], 'anchor_role': None, 'offset': offset} for offset in offsets]}}
        s['supplies'].append(supply)
        paths.append({'supply_id': identity, 'count': count, 'load_size': load_size, 'trips': count // load_size,
                      'distance_m': distance, 'outbound_ms': travel, 'waypoints_m': points})

    for product, name, position in [('TUBE', 'paper-tube-winder', pose(20, 8, .9)), ('LABEL', 'digital-label-printer', pose(23, 8, .9))]:
        type_id = devices['PF-' + name]['role_bindings']['item_types']['input']
        source_id = stock('PF-FAMILY-RECEIVING-' + product, '收货备料 · ' + product, {type_id: 3}, position)
        for i in range(3): initial_item(f'PF-FAMILY-{product}-raw-{i}', type_id, source_id)
        transfer('PF-FAMILY-FEED-' + product, source_id, port_stock('PF-' + name, 'input', 'input'), type_id, 3)

    def route(identity, title, definitions, final_type, planned, bom_rows):
        steps = []
        for step_id, name, predecessors in definitions:
            step = copy.deepcopy(cells['PF-' + name]['steps'][0])
            step.update(id=step_id, predecessors=predecessors, item_bindings=devices['PF-' + name]['role_bindings']['item_types'])
            steps.append(step)
        route_id = 'PF-ROUTE-' + identity; bom_id = 'PF-BOM-' + identity; product_id = 'PF-PRODUCT-' + identity; order_id = 'PF-ORDER-' + identity
        changed['route'][route_id] = {'id': route_id, 'revision': 'r1', 'steps': steps}
        changed['bom'][bom_id] = {'id': bom_id, 'revision': 'r1', 'rows': [{'item_type_ref': {'id': t, 'revision': types[t]['revision']},
                 'quantity': quantity(n), 'consume_at_operation': step} for t, n, step in bom_rows]}
        changed['product'][product_id] = {'id': product_id, 'revision': 'r1', 'item_type_ref': {'id': final_type, 'revision': types[final_type]['revision']},
                'bom_ref': {'id': bom_id, 'revision': 'r1'}, 'routing_ref': {'id': route_id, 'revision': 'r1'}, 'artifact_refs': []}
        s['orders'].append({'id': order_id, 'product_ref': {'id': product_id, 'revision': 'r1'}, 'quantity': quantity(planned), 'release_at_ms': 0,
             'due_at_ms': WINDOW, 'destination': shipping, 'acceptance_stage': 'unloaded_and_released', 'allow_partial': True, 'priority': 10})
        s['production_tasks'].append({'id': 'PF-TASK-' + identity, 'routing_ref': {'id': route_id, 'revision': 'r1'},
             'order_allocations': [{'order_id': order_id, 'quantity': quantity(planned)}], 'planned_quantity': quantity(planned), 'release_at_ms': 0, 'priority': 10})

    route('TUBE', '100mm精磨纸管', [('tube-wind', TUBES[0], []), ('tube-cut', TUBES[1], ['tube-wind']),
          ('tube-grind-a', TUBES[2], ['tube-cut']), ('tube-grind-b', TUBES[2], ['tube-cut'])], 'paper-core-grinder-output', 6,
          [('paper-tube-winder-input', 1, 'tube-wind'), ('paper-tube-winder-component', 1, 'tube-wind'), ('paper-tube-winder-auxiliary', 1, 'tube-wind'),
           ('paper-tube-cutoff-input', 1, 'tube-cut'), ('paper-core-grinder-input', 1, 'tube-grind-a'), ('paper-core-grinder-input', 1, 'tube-grind-b')])
    route('LABEL', '80×60mm印刷标签', [('label-print', LABELS[0], []), ('label-cut', LABELS[1], ['label-print']),
          ('label-inspect', LABELS[2], ['label-cut']), ('label-peel', LABELS[3], ['label-inspect'])], 'label-peeler-output', 18,
          [('digital-label-printer-input', 1, 'label-print'), ('digital-label-printer-auxiliary', 1, 'label-print'),
           ('label-die-cutter-input', 1, 'label-cut'), ('label-web-inspector-input', 1, 'label-inspect'), ('label-web-inspector-input', 1, 'label-peel')])
    for identity, names, counts in [('TUBE', TUBES, [3, 6]), ('LABEL', LABELS, [3, 3, 3])]:
        for a, b, count in zip(names, names[1:], counts):
            type_id = devices['PF-' + a]['role_bindings']['item_types']['output']
            assert type_id == devices['PF-' + b]['role_bindings']['item_types']['input']
            transfer('PF-FAMILY-' + a + '-' + b, port_stock('PF-' + a, 'output', 'output'), port_stock('PF-' + b, 'input', 'input'), type_id, count,
                     2 if (a, b) == ('paper-tube-cutoff', 'paper-core-grinder') else 1)
        last = 'PF-' + names[-1]; type_id = devices[last]['role_bindings']['item_types']['output']
        transfer('PF-FAMILY-SHIP-' + identity, port_stock(last, 'output', 'output'), shipping, type_id, 6 if identity == 'TUBE' else 18, 2 if identity == 'TUBE' else 6)
    for i in range(3):
        s['quality_presets'].append({'id': f'PF-FAMILY-LABEL-PASS-{i}', 'item_or_ancestor': f'PF-FAMILY-LABEL-raw-{i}',
             'match_kind': 'ancestor', 'inspection_module': inspector, 'results': [{'kind': 'string', 'value': 'PASS'}]})
    checks = []
    for suffix, result in [('FAIL', 'FAIL'), ('PENDING', None)]:
        check = copy.deepcopy(s); check.update(id='PF-LABEL-QC-' + suffix, title='标签检验 · ' + suffix)
        first = next(q for q in check['quality_presets'] if q['id'] == 'PF-FAMILY-LABEL-PASS-0')
        first['results'] = [] if result is None else [{'kind': 'string', 'value': result}]
        checks.append(check)
    commands = [{'kind': 'put_object', 'data': {'kind': kind, 'data': row}} for kind, rows in changed.items() for row in rows.values()]
    commands.append({'kind': 'put_object', 'data': {'kind': 'scenario', 'data': s}})
    write(output / 'commands.json', {'base_revision': p['revision'], 'commands': commands, 'new_modules': new_modules,
         'scenario_id': s['id'], 'quality_checks': checks, 'expected': {'manufacturing_jobs': 125, 'transport_trips': 54,
         'accepted_orders': {'PF-ORDER-CASE': 3, 'PF-ORDER-SLEEVE': 3, 'PF-ORDER-TUBE': 6, 'PF-ORDER-LABEL': 18},
         'connected_devices': 15, 'independent_cells': 77, 'tube_cut_dust_lots': 3, 'tube_grind_dust_lots': 6, 'label_backings': 3}})
    write(output / 'physical-specifications.json', {'compatible_aliases': specifications, 'new_transport': paths,
         'tube_balance': '3 wound = 6 ground + 3 cutting dust lots + 6 grinding dust lots',
         'label_balance': '3 webs = 18 labels + 3 spent liner/matrix sheets',
         'remaining_bag_design': 'Tuber/bottomer/handle-attacher physical specifications differ; do not resize stock by renaming.'})
    print(f'Staged two additional product families: {len(new_modules)} modules, {len(paths)} new supply legs, {output}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(); prepare(args.source, args.output)
