"""S03-02有限业务规则复核；不运行产品，不改写冻结预期。"""
import copy
import json
from pathlib import Path
import runpy

B = Path(__file__).parent
S = B.parents[1]


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def verify():
    contract = read(S / 'packages/domain/business-objects.design.json')
    data = read(B / 'business-rule-boundaries.design.json')
    cases = {row['id']: row for row in data['cases']}
    assert data['actual_result'] is None and contract['actual_result'] is None
    assert 'reserved' not in contract['state_axes']['lifecycle']
    assert 'hard_reserved' in contract['state_axes']['allocation']
    stock = contract['policies']['stock_eligibility']

    row = cases['B01']
    before = copy.deepcopy(row['items'])
    eligible = [item for item in row['items'] if item['lifecycle'] == stock['ordinary_lifecycle']
                and item['quality'] in stock['quality_if_inspection_required' if item['inspection_required'] else 'quality_if_no_inspection_required']]
    free = [item['id'] for item in eligible if item['reserved_for'] is None]
    owned = [item['id'] for item in eligible if item['reserved_for'] in [None, row['request']['task']]]
    acquired = owned[:row['request']['quantity']] if len(owned) >= row['request']['quantity'] and row['request']['resource_available'] else []
    result = dict(on_hand=len(row['items']), free_ids=free, eligible_for_T=owned, newly_acquired=acquired)
    assert result == row['expected'] and row['items'] == before
    assert stock['overlapping_restrictions'] == 'exclude_union_once'
    results = [dict(id='B01', status='passed_finite_calculation', calculated=result)]

    row = cases['B02']
    policy = contract['policies']['order_acceptance']
    assert policy['allow_partial_false'] == 'remaining_demand_full_kit_and_release_then_accept_once'
    assert policy['physical_receipt_separate'] and policy['transport_partial_load_separate']
    result = {'allow_partial_true': [min(row['demand'], q) for q in row['received_and_released']],
              'allow_partial_false': [row['demand'] if q >= row['demand'] else 0 for q in row['received_and_released']]}
    assert result == row['expected']
    results.append(dict(id='B02', status='passed_finite_calculation', calculated=result))

    row = cases['B03']
    released = [x['item'] for x in row['allocations'] if x['demand'] == row['cancel_demand'] and x['mode'] != 'committed']
    kept = [x for x in row['allocations'] if x['item'] not in released]
    result = dict(released_items=released, preserved_items=[x['item'] for x in kept],
                  preserved_committed_holder=next(x['holder'] for x in kept if x['mode'] == 'committed'))
    assert result == row['expected']
    assert contract['policies']['cancellation_semantics']['other_demands'] == 'unchanged'
    results.append(dict(id='B03', status='passed_finite_calculation', calculated=result))

    row = cases['B04']
    result = [dict(holder='pending_store' if capacity else 'inspection_job',
                   held_resources=[] if capacity else row['station_resources'], quality=row['quality'])
              for capacity in row['receiving_free_capacity_sequence']]
    assert result == row['expected']
    assert contract['policies']['quality_states']['missing_or_UNKNOWN'] == 'pending'
    results.append(dict(id='B04', status='passed_finite_calculation', calculated=result))

    row = cases['B05']
    directions = contract['policies']['dispatch_directions']
    assert directions == dict(explicit_priority='descending', due_at_if_present='ascending_null_last', ready_at='ascending', stable_id='lexicographic_ascending')
    result = [job['id'] for job in sorted(row['jobs'], key=lambda job: (-job['priority'], job['due'] is None,
              job['due'] if job['due'] is not None else 0, job['ready'], job['id']))]
    assert result == row['expected']
    results.append(dict(id='B05', status='passed_finite_calculation', calculated=result))

    row = cases['B06']
    holder, commits, holders = 'job', 0, []
    for capacity in row['receiving_free_capacity_sequence']:
        if holder == 'job' and capacity:
            holder, commits = 'store', commits + 1
        holders.append(holder)
    result = dict(holders=holders, commit_count=commits, accepted_quantity=commits)
    assert result == row['expected']
    results.append(dict(id='B06', status='passed_finite_calculation', calculated=result))

    # 原六例在内存复算并与原冻结JSON逐项比较，不运行其写文件入口。
    original = runpy.run_path(str(B / '复算业务案例.py'))
    calculated = [original['multi_order']('A'), original['multi_order']('B'), original['calendar_detail'](),
                  original['quality_case'](), *original['coarse_detail']()]
    frozen = read(B / 'expected-business.design.json')
    assert calculated == frozen['cases'] and frozen['actual_result'] is None
    assert all(row['actual_result'] is None for row in calculated)
    results += [dict(id=row['id'], status='passed_existing_frozen_expectation') for row in calculated]
    return {'document_kind': 'finite_business_rule_verification_not_product_runtime', 'status': 'passed',
            'contract_ref': 'packages/domain/business-objects.design.json', 'cases': results,
            'existing_expectations_rewritten': False, 'actual_runtime': None}


if __name__ == '__main__':
    result = verify()
    path = B / 'business-rule-verification.json'
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(f"{len(result['cases'])} finite business checks passed; frozen expectations unchanged; no product runtime.")
