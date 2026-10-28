"""Check actual family Runs against independent piece/material expectations."""
import argparse
import collections
import math
from pathlib import Path

from prepare_full_factory import read, write


def check(proof):
    run = read(proof / 'actual-connected-run.json')
    project = read(proof / 'actual-frozen-project.json')
    scenario = read(proof / 'actual-scenario.json')
    types = {i['id']: i for i in project['item_types']}
    items = {i['id']: i for i in run['final_items']}
    def number(q): return q['value']['coefficient'] / 10 ** q['value']['decimal_places']
    def attr(i, name): return i['attributes'][name]['value']
    def inventory(type_id):
        return [i for i in items.values() if i['item_type_ref']['id'] == type_id and i['lifecycle'] == 'available']
    def ancestors(identity):
        result = set()
        pending = [identity]
        while pending:
            current = pending.pop()
            if current in result: continue
            result.add(current)
            item = items[current]
            pending.extend(item['composition_refs'] + item['contents'])
            if item['split_from_ref']: pending.append(item['split_from_ref'])
        return result
    def roots(type_id, family):
        counts = collections.Counter()
        for item in inventory(type_id):
            found = [r for r in ancestors(item['id']) if r.startswith('PF-FAMILY-' + family + '-raw-')]
            assert len(found) == 1, (item['id'], found)
            counts[found[0]] += number(item['quantity'])
        return dict(counts)
    expected_inventory = {'paper-core-grinder-output': 6, 'paper-tube-cutoff-waste': 3,
                          'paper-core-grinder-waste': 6, 'label-peeler-output': 18, 'label-peeler-backing': 3}
    actual_inventory = {t: sum(number(i['quantity']) for i in inventory(t)) for t in expected_inventory}
    assert actual_inventory == expected_inventory, actual_inventory
    for type_id in ['paper-core-grinder-output', 'label-peeler-output']:
        assert all(i['holder'] == {'kind': 'location', 'id': 'PF-SHIPPING'} and i['quality'] == 'released' for i in inventory(type_id))
    tube_roots = roots('paper-core-grinder-output', 'TUBE')
    label_roots = roots('label-peeler-output', 'LABEL')
    assert tube_roots == {f'PF-FAMILY-TUBE-raw-{i}': 2 for i in range(3)}, tube_roots
    assert label_roots == {f'PF-FAMILY-LABEL-raw-{i}': 6 for i in range(3)}, label_roots
    assert len(run['transport']) == 54
    tray_loads = {}
    for supply_id, count in [('PF-FAMILY-SHIP-LABEL', 6), ('PF-FAMILY-SHIP-TUBE', 2),
                              ('PF-FAMILY-paper-tube-cutoff-paper-core-grinder', 2)]:
        trips = [t for t in run['transport'] if t['supply_ref'] == supply_id]
        assert len(trips) == 3 and all(t['phase'] == 'returned' and len(t['cargo']) == count for t in trips)
        cargo = [identity for t in trips for identity in t['cargo']]
        assert len(cargo) == len(set(cargo)), supply_id
        assert all(i in items for i in cargo)
        supply = next(s for s in scenario['supplies'] if s['id'] == supply_id)
        slots = supply['presentation_path']['cargo_layout']['slots']
        assert len(slots) == count and len({tuple(slot['offset']['translation'].values()) for slot in slots}) == count
        tray_loads[supply_id] = {'trips': len(trips), 'items_per_trip': count, 'unique_cargo': len(set(cargo))}
    paper_input = 3 * sum(types[t]['attributes']['paper_area_m2']['value'] * types[t]['attributes']['paper_thickness_m']['value']
                          for t in ['paper-tube-winder-input', 'paper-tube-winder-component'])
    paper_output = sum(number(i['quantity']) * attr(i, 'paper_volume_m3') for i in inventory('paper-core-grinder-output'))
    paper_dust = sum(number(i['quantity']) * attr(i, 'removed_volume_m3') for t in ['paper-tube-cutoff-waste', 'paper-core-grinder-waste'] for i in inventory(t))
    assert math.isclose(paper_input, paper_output + paper_dust, rel_tol=0, abs_tol=1e-14), (paper_input, paper_output, paper_dust)
    face_input = 3 * types['digital-label-printer-input']['attributes']['facestock_area_m2']['value']
    face_labels = sum(number(i['quantity']) * attr(i, 'facestock_area_m2') for i in inventory('label-peeler-output'))
    face_matrix = sum(number(i['quantity']) * attr(i, 'residual_matrix_area_m2') for i in inventory('label-peeler-backing'))
    assert math.isclose(face_input, face_labels + face_matrix, rel_tol=0, abs_tol=1e-12)
    liner_input = 3 * types['digital-label-printer-input']['attributes']['liner_area_m2']['value']
    liner_output = sum(number(i['quantity']) * attr(i, 'liner_area_m2') for i in inventory('label-peeler-backing'))
    assert math.isclose(liner_input, liner_output, rel_tol=0, abs_tol=1e-12)
    normal_counts = collections.Counter(j['execution_ref']['data'].get('device_id') for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device')
    expected_cycles = {'paper-tube-winder': 3, 'paper-tube-cutoff': 3, 'paper-core-grinder': 6,
                       'digital-label-printer': 3, 'label-die-cutter': 3, 'label-web-inspector': 3, 'label-peeler': 3}
    assert {n: normal_counts['PF-' + n] for n in expected_cycles} == expected_cycles
    quality = []
    for suffix, accepted, peels in [('FAIL', 12, 2), ('PENDING', 0, 0)]:
        checked = read(proof / f'actual-PF-LABEL-QC-{suffix}-run.json')
        order = next(o for o in checked['orders'] if o['order_id'] == 'PF-ORDER-LABEL')
        assert number(order['accepted']) == accepted
        jobs = [j for j in checked['jobs'] if j['execution_ref']['kind'] == 'project_device' and j['execution_ref']['data']['device_id'] == 'PF-label-peeler' and j['state'] == 'completed']
        assert len(jobs) == peels
        quality.append({'scenario': 'PF-LABEL-QC-' + suffix, 'accepted_labels': accepted, 'completed_peels': len(jobs)})
    # Every declared stock state must be covered by the actual exact media.
    modules = {(m['definition']['id'], m['definition']['version']): m for m in read(proof / 'actual-frozen-modules.json')}
    scoped_states = []
    for type_id, item_type in types.items():
        if not item_type['module_ref']['id'].startswith('sf.paperfactory.families.items.'): continue
        module = modules[(item_type['module_ref']['id'], item_type['module_ref']['version'])]
        media_ref = module['definition']['payload']['data']['presentation_or_media_ref']['source']['data']
        media = modules[(media_ref['id'], media_ref['version'])]['definition']['payload']['data']
        state = item_type['attributes']['presentation_state']['value']
        assert state in [c['phase_ref'] for c in media['timeline']], (type_id, state)
        for clip in media['timeline']:
            if ':' in clip['phase_ref']:
                assert ':sf.native500.device.' in clip['phase_ref'] and ':1.1.0:' in clip['phase_ref']
        scoped_states.append(type_id)
    receipt = {'status': 'actual_run_piece_identity_paper_volume_label_area_and_quality_verified',
               'run_id': run['id'], 'scenario_id': scenario['id'], 'inventory': actual_inventory,
               'tube_final_units_per_source': tube_roots, 'label_final_units_per_source': label_roots,
               'tube_paper_m3': {'input': paper_input, 'finished': paper_output, 'collected_dust': paper_dust},
               'label_face_m2': {'input': face_input, 'finished': face_labels, 'waste_matrix': face_matrix},
               'liner_m2': {'input': liner_input, 'spent': liner_output}, 'manufacturing_cycles': expected_cycles,
               'tray_loads': tray_loads,
               'quality_checks': quality, 'scoped_item_media_states': scoped_states,
               'visual_verified': False, 'whole_factory_accepted': False}
    write(proof / 'family-materials-receipt.json', receipt)
    print('Verified actual material and ancestry balances:', proof)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('proof', type=Path)
    check(parser.parse_args().proof)
