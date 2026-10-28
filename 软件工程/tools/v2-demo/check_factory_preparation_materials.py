"""Check actual new-branch quantities, identity, ancestry and material state."""
import argparse
import collections
import math
from pathlib import Path
from prepare_full_factory import read, write


def check(proof, source):
    project = read(proof / 'actual-frozen-project.json'); previous = read(source / 'actual-frozen-project.json')
    run = read(proof / 'actual-connected-run.json'); scenario = read(proof / 'actual-scenario.json')
    types = {t['id']: t for t in project['item_types']}; items = {i['id']: i for i in run['final_items']}
    amount = lambda q: q['value']['coefficient'] / 10 ** q['value']['decimal_places']
    assert len(project['devices']) == 92
    assert {d['id']: d['module_ref'] for d in project['devices']} == {d['id']: d['module_ref'] for d in previous['devices']}
    counts = collections.Counter(j['execution_ref']['data']['device_id'] for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device')
    expected = {'PF-paper-unwinder': 1, 'PF-paper-rewinder': 1, 'PF-rotary-sheet-cutter': 1,
                'PF-board-edge-trimmer': 3, 'PF-sheet-slitter': 3, 'PF-varnish-coater': 3, 'PF-uv-curer': 3}
    assert {k: counts[k] for k in expected} == expected
    def finished(item_type, count):
        rows = [i for i in items.values() if i['item_type_ref']['id'] == item_type and i['lifecycle'] == 'available']
        assert sum(amount(i['quantity']) for i in rows) == count
        assert all(i['holder'] == {'kind': 'location', 'id': 'PF-SHIPPING'} and i['quality'] == 'released' for i in rows)
        return rows
    def ancestors(identity):
        seen = set(); pending = [identity]
        while pending:
            current = pending.pop()
            if current in seen: continue
            seen.add(current); item = items[current]
            pending.extend(item['composition_refs'] + item['contents'])
            if item['split_from_ref']: pending.append(item['split_from_ref'])
        return seen
    roll_id = 'PF-PREP-SHEETS-raw-0'
    for device in ['PF-paper-unwinder', 'PF-paper-rewinder', 'PF-rotary-sheet-cutter']:
        job = next(j for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device' and j['execution_ref']['data']['device_id'] == device)
        assert job['input_refs'] == [roll_id], (device, job['input_refs'])
    sheets = finished('rotary-sheet-cutter-output', 60)
    assert all(roll_id in ancestors(i['id']) for i in sheets)
    a = types['paper-unwinder-input']['attributes']; b = types['rotary-sheet-cutter-output']['attributes']
    roll_volume = a['length_m']['value'] * a['width_m']['value'] * a['thickness_m']['value']
    sheet_volume = sum(amount(i['quantity']) for i in sheets) * b['length_m']['value'] * b['width_m']['value'] * b['thickness_m']['value']
    assert math.isclose(roll_volume, sheet_volume, abs_tol=1e-14)
    boards = finished('sheet-slitter-output', 6)
    assert all(len([n for n in ancestors(i['id']) if n.startswith('PF-PREP-BOARDS-raw-')]) == 1 for i in boards)
    scraps = {name: [i for i in items.values() if i['item_type_ref']['id'] == name] for name in ['board-edge-trimmer-side_scrap', 'board-edge-trimmer-end_scrap']}
    assert all(sum(amount(i['quantity']) for i in rows) == 6 for rows in scraps.values())
    board_input = types['board-edge-trimmer-input']['attributes']['area_m2']['value'] * 3
    board_output = types['sheet-slitter-output']['attributes']['area_m2']['value'] * 6
    scrap_area = sum(types[name]['attributes']['area_m2']['value'] * sum(amount(i['quantity']) for i in rows) for name, rows in scraps.items())
    assert math.isclose(board_input, board_output + scrap_area, abs_tol=1e-14)
    cured = finished('varnish-coater-input', 3)
    assert {i['id'] for i in cured} == {f'PF-PREP-CURED-raw-{i}' for i in range(3)}
    assert all(i['attributes']['presentation_state']['value'] == 'varnished_cured' for i in cured)
    for device in ['PF-varnish-coater', 'PF-uv-curer']:
        input_ids = {n for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device' and j['execution_ref']['data']['device_id'] == device for n in j['input_refs'] if n.startswith('PF-PREP-CURED-raw-')}
        assert input_ids == {i['id'] for i in cured}
    doses = [i for i in items.values() if i['id'].startswith('PF-PREP-COATING-')]
    assert len(doses) == 3 and all(i['lifecycle'] == 'transformed' for i in doses)
    used_doses = [n for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device'
                  and j['execution_ref']['data']['device_id'] == 'PF-varnish-coater' for n in j['input_refs'] if n.startswith('PF-PREP-COATING-')]
    assert len(used_doses) == 3 and set(used_doses) == {i['id'] for i in doses}
    negative = read(proof / 'actual-PF-PREP-MISSING-COATING-run.json')
    assert amount(next(o for o in negative['orders'] if o['order_id'] == 'PF-ORDER-CURED')['accepted']) == 0
    assert not any(i['attributes'].get('presentation_state', {}).get('value') == 'varnished_cured' for i in negative['final_items'] if i['id'].startswith('PF-PREP-CURED-raw-'))
    trips = [t for t in run['transport'] if t['supply_ref'].startswith('PF-PREP-TRANSFER-')]
    assert len(trips) == 27 and all(t['phase'] == 'returned' for t in trips)
    sheet_trips = [t for t in trips if t['supply_ref'] == 'PF-PREP-TRANSFER-SHEETS-ship']
    assert len(sheet_trips) == 6 and all(len(t['cargo']) == 10 for t in sheet_trips)
    receipt = {'status': 'actual_preparation_material_balances_identity_and_shortage_verified', 'run_id': run['id'],
        'accepted_device_modules_unchanged': True, 'manufacturing_by_new_role': expected,
        'paper_roll_identity_preserved': roll_id, 'sheet_count': 60, 'paper_volume_m3': roll_volume,
        'half_boards': 6, 'side_scraps': 6, 'end_scraps': 6, 'input_board_area_m2': board_input, 'finished_and_scrap_area_m2': board_output + scrap_area,
        'cured_boards': 3, 'coating_doses': 3, 'coating_volume_m3': .000012, 'new_transport_trips': 27,
        'missing_coating_accepted': 0, 'Native_visual_verified': False, 'whole_factory_accepted': False}
    write(proof / 'preparation-materials-receipt.json', receipt)
    print('Actual preparation material balances and preserve-operation identities verified:', proof)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('proof', type=Path); parser.add_argument('source', type=Path)
    args = parser.parse_args(); check(args.proof, args.source)
