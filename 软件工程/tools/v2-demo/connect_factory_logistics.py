"""Connect actual finished cases to four existing logistics devices through public edits."""
import argparse
import copy
import math
from pathlib import Path
from prepare_full_factory import ROOT, DESIGN, read, write
from connect_full_factory_routes import quantity, pose
from connect_factory_families import replace


def prepare(source, media_source, output):
    assert not output.exists()
    output.mkdir(parents=True)
    p = read(source / 'actual-project.json'); s = copy.deepcopy(read(source / 'actual-scenario.json'))
    spec = read(DESIGN / 'logistics-line-spec.json'); media = read(media_source / 'media-plan.json')
    devices = {d['id']: copy.deepcopy(d) for d in p['devices']}
    locations = {d['id']: copy.deepcopy(d) for d in p['locations']}
    connections = {d['data']['id']: copy.deepcopy(d) for d in p['connections']}
    routes = {r['id']: copy.deepcopy(r) for r in p['routes']}
    cells = {r['steps'][0]['device_id']: r for r in p['routes'] if len(r['steps']) == 1}
    names = ['PF-'+n for n in spec['devices']]; selected = set(names); item = spec['case']['type']
    modules = copy.deepcopy(media['modules']); changed = {k: {} for k in ['device', 'location', 'connection', 'route', 'bom']}
    s.update(id='PF-LOGISTICS-FACTORY', title='纸包装整厂 · 八产品与成品物流', revision=1, scope=[])
    s['production_tasks'] = [t for t in s['production_tasks'] if not any(st['device_id'] in selected for st in routes[t['routing_ref']['id']]['steps'])]

    def port_stock(device, role, direction):
        port = devices[device]['role_bindings']['ports'][role]['port_id']
        return next(c['data']['destination_store'] for c in connections.values() if
                    (e := c['data']['destination' if direction == 'input' else 'source'])['kind'] == 'port'
                    and e['data']['instance_id'] == device and e['data']['port_id'] == port)

    stores = {port_stock(d, role, role) for d in names for role in ['input', 'output']}
    initial = {i['id']: i for i in s['initial_conditions']['items']}
    removed = {i['id'] for i in initial.values() if i['holder']['kind'] == 'location' and i['holder']['id'] in stores}
    while True:
        old = set(removed)
        for identity in list(removed): removed.update(initial[identity]['components'] + initial[identity]['contents'])
        if old == removed: break
    s['initial_conditions']['items'] = [i for i in initial.values() if i['id'] not in removed]
    for device_id in names:
        device = devices[device_id]; mapping = {v: item for v in device['role_bindings']['item_types'].values()}
        device = replace(device, mapping); devices[device_id] = device
        if device_id == 'PF-vacuum-palletizer': device['presentation_override'] = media['presentation']
        changed['device'][device_id] = device
        for connection_id, connection in list(connections.items()):
            if any(e['kind'] == 'port' and e['data']['instance_id'] == device_id for e in [connection['data']['source'], connection['data']['destination']]):
                connections[connection_id] = replace(connection, mapping); changed['connection'][connection_id] = connections[connection_id]
    for location_id in stores:
        location = locations[location_id]; location['accepted_item_types'] = [item]; location['capacity'] = quantity(3)
        location['item_layout'] = {'assignment': 'item_id_order', 'slots': [
            {'item_type_ref': {'id': item, 'revision': 'r1'}, 'anchor_role': None, 'offset': pose((i-1)*.5, 0, 0)} for i in range(3)]}
        changed['location'][location_id] = location
    case_route = routes['PF-ROUTE-CASE']; previous_device = case_route['steps'][-1]['device_id']
    product = next(v for v in p['products'] if v['routing_ref']['id'] == case_route['id'])
    bom = copy.deepcopy(next(v for v in p['boms'] if v['id'] == product['bom_ref']['id']))
    for i, device_id in enumerate(names):
        step = copy.deepcopy(cells[device_id]['steps'][0])
        step.update(id='CASE-logistics-'+str(i), predecessors=[case_route['steps'][-1]['id']], item_bindings=devices[device_id]['role_bindings']['item_types'])
        case_route['steps'].append(step)
        bom['rows'].append({'item_type_ref': {'id': item, 'revision': 'r1'}, 'quantity': quantity(), 'consume_at_operation': step['id']})
    changed['route'][case_route['id']] = case_route; changed['bom'][bom['id']] = bom
    original_ship = next(x for x in s['supplies'] if x['id'] == 'PF-SHIP-CASE')
    assert original_ship['source'] == port_stock(previous_device, 'output', 'output')
    s['supplies'] = [x for x in s['supplies'] if x['id'] != original_ship['id']]
    legs = [(original_ship['source'], port_stock(names[0], 'input', 'input'))]
    legs += [(port_stock(a, 'output', 'output'), port_stock(b, 'input', 'input')) for a, b in zip(names, names[1:])]
    legs += [(port_stock(names[-1], 'output', 'output'), original_ship['destination'])]
    paths = []
    for i, (a_id, b_id) in enumerate(legs):
        a = locations[a_id]['position']['translation']; b = locations[b_id]['position']['translation']
        road = lambda y: 16 if y < 42 else 40 if y < 66 else 64
        points = [dict(a), {'x': a['x'], 'y': road(a['y']), 'z': .9}, {'x': 52, 'y': road(a['y']), 'z': .9},
                  {'x': 52, 'y': road(b['y']), 'z': .9}, {'x': b['x'], 'y': road(b['y']), 'z': .9}, dict(b)]
        points = [v for j, v in enumerate(points) if j == 0 or v != points[j-1]]
        travel = math.ceil(sum(math.dist([a[k] for k in ['x', 'y', 'z']], [b[k] for k in ['x', 'y', 'z']]) for a, b in zip(points, points[1:])) / 1.2 * 1000)
        definition = copy.deepcopy(next(d['payload']['data'] for d in read(ROOT / 'packages/commands/tests/fixtures/production/p202-identity.json')['definitions'] if d['payload']['category'] == 'transport'))
        definition['batch_capacity'] = quantity()
        definition['loading_resources'] = [{'role': 'dispatch_gate', 'quantity': quantity(1, 'slot'), 'qualifications': []}]
        for phase, duration in [('loading', 1000), ('outbound', travel), ('unloading', 1000), ('returning', travel)]:
            definition['times'][phase] = {'value': duration, 'unit': 'millisecond', 'basis': 'user_provided', 'source_ref': None}
        identity = 'sf.paperfactory.logistics.transport.'+str(i); reference = {'id': identity, 'version': spec['version']}
        directory = output / 'modules' / identity
        write(directory / 'module.json', {'schema_version': '1.0', 'data': {'definition': dict(reference, title='成品箱转运 · '+str(i+1), definition_state='runtime_candidate',
            contribution={'author': 'Super Factory logistics composition', 'license': 'CC0-1.0', 'assets': []}, parameters=[], attribute_schema=[], payload={'category': 'transport', 'data': definition}),
            'assets': [], 'files': [], 'dependencies': [], 'rights': [], 'issues': []}})
        modules.append(dict(reference, directory=str(directory)))
        supply = copy.deepcopy(original_ship); supply.update(id='PF-LOGISTICS-TRANSFER-'+str(i), source=a_id, destination=b_id, transport_ref=reference)
        supply['presentation_path']['waypoints_m'] = points; s['supplies'].append(supply); paths.append({'id': supply['id'], 'source': a_id, 'destination': b_id, 'waypoints_m': points, 'trips': 3})
    check = copy.deepcopy(s); check.update(id='PF-LOGISTICS-MISSING-CONTENTS', title='成品物流边界 · 少六件装箱内件')
    raw = [i for i in check['initial_conditions']['items'] if i['item_type_ref']['id'] == 'manual-packing-input']
    assert len(raw) == 18
    missing = {i['id'] for i in raw[:6]}; check['initial_conditions']['items'] = [i for i in check['initial_conditions']['items'] if i['id'] not in missing]
    commands = [{'kind': 'put_object', 'data': {'kind': kind, 'data': value}} for kind, values in changed.items() for value in values.values()]
    commands.append({'kind': 'put_object', 'data': {'kind': 'scenario', 'data': s}})
    write(output / 'commands.json', {'base_revision': p['revision'], 'commands': commands, 'new_modules': modules,
        'scenario_id': s['id'], 'quality_checks': [check], 'material_checks': {check['id']: {'PF-ORDER-CASE': 2}}, 'expected': spec['expected']})
    write(output / 'design-consumption.json', {'specification': spec, 'source': str(source), 'new_transport': paths,
         'isolated_items_removed': sorted(removed), 'canonical_device_modules_unchanged': True, 'height_fixture': media['height_calibration']})
    print('Staged four actual case-logistics devices and five real transfer legs:', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ['source', 'media', 'output']: parser.add_argument('--'+arg, type=Path, required=True)
    args = parser.parse_args(); prepare(args.source, args.media, args.output)
