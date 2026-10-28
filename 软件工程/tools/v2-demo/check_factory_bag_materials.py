"""Independent actual-Run bag ancestry, paper balance and shared-resource check."""
import argparse
import collections
import math
from pathlib import Path
from prepare_full_factory import read, write


def check(proof, source):
    project = read(proof / 'actual-frozen-project.json'); scenario = read(proof / 'actual-scenario.json')
    run = read(proof / 'actual-connected-run.json'); previous = read(source / 'actual-frozen-project.json')
    types = {t['id']: t for t in project['item_types']}; items = {i['id']: i for i in run['final_items']}
    seeds = {i['id']: i for i in scenario['initial_conditions']['items']}
    def amount(q): return q['value']['coefficient'] / 10 ** q['value']['decimal_places']
    def ancestors(identity):
        seen = set(); pending = [identity]
        while pending:
            current = pending.pop()
            if current in seen: continue
            seen.add(current); item = items[current]
            pending.extend(item['composition_refs'] + item['contents'])
            if item['split_from_ref']: pending.append(item['split_from_ref'])
        return seen
    assert len(project['devices']) == 92
    previous_modules = {d['id']: d['module_ref'] for d in previous['devices']}
    assert {d['id']: d['module_ref'] for d in project['devices']} == previous_modules
    for device_id in ['PF-paper-bag-tuber', 'PF-paper-bag-bottomer']:
        d = next(d for d in project['devices'] if d['id'] == device_id)
        assert d['presentation_override']['source']['data']['id'] == 'sf.paperfactory.bag.tooling.' + device_id.removeprefix('PF-')
    assert not any(i['item_type_ref']['id'] in ['PF-BAG-open-bag', 'PF-BAG-handle-pair', 'PF-BAG-finished-bag'] for i in seeds.values())
    finished = [i for i in items.values() if i['item_type_ref']['id'] == 'PF-BAG-finished-bag']
    assert len(finished) == 3 and all(i['lifecycle'] == 'available' and i['quality'] == 'released' and i['holder'] == {'kind': 'location', 'id': 'PF-SHIPPING'} for i in finished)
    details = []; all_webs = set(); all_strips = set(); all_patches = set()
    for bag in finished:
        lineage = ancestors(bag['id'])
        webs = sorted(n for n in lineage if n.startswith('PF-BAG-WEB-'))
        strips = sorted(n for n in lineage if n.startswith('PF-BAG-STRIPS-'))
        patches = sorted(n for n in lineage if n.startswith('PF-BAG-PATCH-'))
        doses = sorted(n for n in lineage if n.startswith('PF-BAG-GLUE-'))
        assert len(webs) == 1 and len(strips) == 2 and len(patches) == 4 and len(doses) == 3
        assert sum(amount(seeds[n]['quantity']) for n in strips) == 6
        paper = sum(amount(seeds[n]['quantity']) * types[seeds[n]['item_type_ref']['id']]['attributes']['paper_volume_m3']['value'] for n in webs + strips + patches)
        glue = sum(amount(seeds[n]['quantity']) * types[seeds[n]['item_type_ref']['id']]['attributes']['glue_ml']['value'] for n in doses)
        assert math.isclose(paper, bag['attributes']['paper_volume_m3']['value'], rel_tol=0, abs_tol=1e-14)
        assert glue == 4
        assert bag['attributes']['handle_count']['value'] == 2 and bag['attributes']['consumed_cords']['value'] == 2
        assert len([n for n in lineage if items[n]['item_type_ref']['id'] == 'PF-BAG-cord']) == 2
        assert len([n for n in lineage if items[n]['item_type_ref']['id'] == 'PF-BAG-handle-pair']) == 1
        assert not all_webs.intersection(webs) and not all_strips.intersection(strips) and not all_patches.intersection(patches)
        all_webs.update(webs); all_strips.update(strips); all_patches.update(patches)
        details.append({'bag': bag['id'], 'webs': webs, 'three_strip_lots': strips, 'patches': patches, 'paper_volume_m3': paper, 'metered_glue_ml': glue})
    counts = collections.Counter(j['execution_ref']['data']['device_id'] for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device')
    expected = {'PF-paper-bag-tuber': 3, 'PF-paper-bag-bottomer': 3, 'PF-rope-handle-twister': 6, 'PF-bag-handle-attacher': 3, 'PF-manual-packing': 12}
    assert {d: counts[d] for d in expected} == expected
    resources = {}
    for resource_id in ['machine-manual-packing--normal', 'operator-manual-packing--normal']:
        resource = next(r for r in project['resources'] if r['id'] == resource_id); assert amount(resource['capacity']) == 1
        intervals = sorted((i['start_ms'], i['end_ms'], c['job_id']) for c in run['resource_claims'] if c['resource_id'] == resource_id for i in c['intervals'])
        for previous_claim, current_claim in zip(intervals, intervals[1:]):
            assert current_claim[0] >= previous_claim[1], (resource_id, previous_claim, current_claim)
        jobs = {j for _, _, j in intervals}
        assert len(jobs) == 12
        resources[resource_id] = {'capacity': 1, 'actual_jobs': len(jobs), 'claim_intervals': len(intervals), 'overlap': False}
    assert len(run['transport']) == 78 and all(t['phase'] == 'returned' for t in run['transport'])
    cord_trips = [t for t in run['transport'] if t['supply_ref'] == 'PF-BAG-TRANSFER-cord-handles']
    assert len(cord_trips) == 3 and all(len(t['cargo']) == 2 for t in cord_trips)
    assert len({n for t in cord_trips for n in t['cargo']}) == 6
    boundary = []
    for name, accepted in [('SHORT-PATCHES', 2), ('MISSING-GLUE', 0)]:
        checked = read(proof / f'actual-PF-BAG-{name}-run.json')
        actual = next(o for o in checked['orders'] if o['order_id'] == 'PF-ORDER-BAG')
        assert amount(actual['accepted']) == accepted
        boundary.append({'scenario': 'PF-BAG-' + name, 'accepted_bags': accepted})
    receipt = {'status': 'actual_bag_material_ancestry_paper_balance_shared_station_and_boundaries_verified',
        'run_id': run['id'], 'factory_devices': 92, 'accepted_device_module_refs_unchanged': True,
        'bag_count': 3, 'source_webs': len(all_webs), 'source_strips': 18, 'source_patches': len(all_patches),
        'details': details, 'manufacturing_by_new_role_and_shared_station': expected,
        'shared_resources': resources, 'cord_transport': {'trips': 3, 'cords_per_trip': 2, 'unique_cords': 6},
        'boundary_checks': boundary, 'Native_visual_verified': False, 'whole_factory_accepted': False}
    write(proof / 'bag-materials-receipt.json', receipt); print('Actual bag material/resource balances verified:', proof)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('proof', type=Path); parser.add_argument('source', type=Path)
    args = parser.parse_args(); check(args.proof, args.source)
