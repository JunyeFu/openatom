"""Check actual completed case movements, nested contents and shortage behavior."""
import argparse
from pathlib import Path
from prepare_full_factory import DESIGN, read, write


def check(proof, source):
    spec = read(DESIGN / 'logistics-line-spec.json')
    run = read(proof / 'actual-connected-run.json')
    negative = read(proof / 'actual-PF-LOGISTICS-MISSING-CONTENTS-run.json')
    original = read(source / 'actual-scenario.json')
    before = {d['id']: d for d in read(source / 'actual-project.json')['devices']}
    after = read(proof / 'actual-project.json')['devices']
    assert len(before) == len(after) == 92
    assert all(d['module_ref'] == before[d['id']]['module_ref'] for d in after)
    contents = {i['id'] for i in original['initial_conditions']['items'] if i['item_type_ref']['id'] == 'manual-packing-input'}
    cases = [i for i in run['final_items'] if i['item_type_ref']['id'] == spec['case']['type']]
    assert len(cases) == 3
    identities = {i['id'] for i in cases}; final = {i['id']: i for i in run['final_items']}
    assert len(contents) == 18 and {j for i in cases for j in i['contents']} == contents
    for case in cases:
        assert case['holder'] == {'kind': 'location', 'id': 'PF-SHIPPING'}
        assert case['lifecycle'] == 'available' and case['quality'] == 'released' and len(case['contents']) == 6
        assert all(final[i]['holder'] == {'kind': 'container', 'id': case['id']} for i in case['contents'])
        for key, value in {'case_length_m': .32, 'case_width_m': .24, 'case_height_m': .24,
                           'seal_state': 'taped', 'label_state': 'labelled', 'band_state': 'strapped'}.items():
            assert case['attributes'][key]['value'] == value
    stages = []
    for name in spec['devices']:
        jobs = [j for j in run['jobs'] if j['execution_ref']['kind'] == 'project_device' and j['execution_ref']['data']['device_id'] == 'PF-'+name]
        assert len(jobs) == 3 and all(j['state'] == 'completed' and len(j['input_refs']) == 1 for j in jobs)
        assert {j['input_refs'][0] for j in jobs} == identities
        stages.append({j['input_refs'][0]: j for j in jobs})
    legs = []
    for index in range(5):
        trips = [t for t in run['transport'] if t['supply_ref'] == 'PF-LOGISTICS-TRANSFER-'+str(index)]
        assert len(trips) == 3 and all(t['phase'] == 'returned' and len(t['cargo']) == 1 for t in trips)
        assert {t['cargo'][0] for t in trips} == identities
        legs.append({t['cargo'][0]: t for t in trips})
    for identity in identities:
        for i, jobs in enumerate(stages):
            assert legs[i][identity]['received_ms'] <= jobs[identity]['started_at_ms']
            assert jobs[identity]['finished_at_ms'] <= legs[i+1][identity]['departed_ms']
    for result, case_count in [(run, 3), (negative, 2)]:
        expected = dict(spec['expected']['accepted_orders'], **{'PF-ORDER-CASE': case_count})
        assert {o['order_id']: o['accepted']['value']['coefficient'] for o in result['orders']} == expected
        assert all(o['outstanding']['value']['coefficient'] == (3-case_count if o['order_id'] == 'PF-ORDER-CASE' else 0) for o in result['orders'])
    assert len([t for t in negative['transport'] if t['supply_ref'].startswith('PF-LOGISTICS-TRANSFER-')]) == 10
    receipt = {'status': 'actual_cases_contents_transport_order_and_shortage_verified', 'case_ids': sorted(identities),
               'original_contents': sorted(contents), 'completed_logistics_jobs': 12, 'returned_logistics_trips': 15,
               'missing_six_contents_accepts_cases': 2, 'case_geometry_m': [.32, .24, .24],
               'Native_verified': False, 'pallet_assembly_and_wrapping_implemented': False, 'whole_factory_accepted': False}
    write(proof / 'logistics-materials-receipt.json', receipt)
    print('Three actual cases and eighteen contents preserved through four stations; shortage cannot create the third case.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('proof', type=Path); parser.add_argument('source', type=Path)
    args = parser.parse_args(); check(args.proof, args.source)
