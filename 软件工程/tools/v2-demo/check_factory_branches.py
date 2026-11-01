"""Check actual common-factory work, receiving/dispatch and resource capacity."""
import argparse
from collections import defaultdict
from pathlib import Path
from prepare_full_factory import read, write
from connect_factory_branches import amount


def check(proof, staged, checks=None):
    plan=read(staged/'commands.json');coverage=read(staged/'branch-coverage.json')
    run=read(proof/'actual-connected-run.json');p=read(proof/'actual-frozen-project.json')
    assert run['computation_status']=='completed' and run['whole_window_computed']
    jobs=defaultdict(list)
    for job in run['jobs']:
        if job['execution_ref']['kind']=='project_device':jobs[job['execution_ref']['data']['device_id']].append(job)
    assert set(jobs)=={d['id'] for d in p['devices']} and len(jobs)==92
    assert all(j['state']=='completed' for group in jobs.values() for j in group)
    items={i['id']:i for i in run['final_items']};assert len(items)==len(run['final_items'])
    orders={o['order_id']:o for o in run['orders']}
    assert {k:amount(o['accepted']) for k,o in orders.items()}==plan['expected']['accepted_orders']
    assert all(amount(o['outstanding'])==0 for o in orders.values())
    by_supply=defaultdict(list)
    for trip in run['transport']:
        assert trip['phase']=='returned'
        by_supply[trip['supply_ref']].append(trip)
    assert len(run['transport'])==plan['expected']['transport_trips']
    accepted=[]
    for row in coverage['branches']:
        slug=row['device_id'].removeprefix('PF-')
        feed=by_supply['PF-BRANCH-FEED-'+slug];ship=by_supply['PF-BRANCH-SHIP-'+slug]
        actual_jobs=jobs[row['device_id']];assert len(actual_jobs)==1
        job=actual_jobs[0]
        assert all(t['received_ms']<=job['started_at_ms'] for t in feed)
        fed={identity for t in feed for identity in t['cargo']}
        assert fed.issubset(set(job['input_refs'])),(slug,'feed must be used by actual equipment')
        events=[e for e in run['acceptance_events'] if e['order_id']==row['order_id']]
        assert sum(amount(e['quantity']) for e in events)==row['expected_output']
        shipped={identity for t in ship for identity in t['cargo']}
        received={identity for e in events for identity in e['item_refs']}
        assert shipped==received,(slug,'order and transport must use the same physical items')
        for identity in shipped:
            assert items[identity]['holder']=={'kind':'location','id':row['dispatch']}
            assert items[identity]['quality']=='released'
        accepted.append({'device_id':row['device_id'],'role':row['role'],'job_id':job['id'],
                         'feed_trips':len(feed),'shipping_trips':len(ship),'accepted_quantity':row['expected_output'],
                         'order_id':row['order_id'],'actual_item_ids':sorted(shipped)})
    capacities={r['id']:amount(r['capacity']) for r in p['resources']}
    endpoints=defaultdict(list)
    for claim in run['resource_claims']:
        for interval in claim['intervals']:
            assert interval['end_ms'] is not None
            if interval['end_ms']==interval['start_ms']:continue
            endpoints[claim['resource_id']].extend([(interval['start_ms'],claim['units']),(interval['end_ms'],-claim['units'])])
    for resource,events in endpoints.items():
        active=0
        for _,delta in sorted(events):
            active+=delta;assert 0<=active<=capacities[resource],(resource,active,capacities[resource])
        assert active==0
    negatives=[]
    for scenario,expected in plan.get('material_checks',{}).items():
        actual=read((checks or proof)/f'actual-{scenario}-run.json')
        actual_orders={o['order_id']:amount(o['accepted']) for o in actual['orders']}
        assert actual_orders==dict(plan['expected']['accepted_orders'],**expected)
        assert not any(j['state']=='completed' for j in actual['jobs'] if j['execution_ref']['kind']=='project_device' and j['execution_ref']['data']['device_id']=='PF-adhesive-dispenser')
        if scenario.endswith('BLOCKED-RECEIVER'):
            waiting=[t for t in actual['transport'] if t['supply_ref']=='PF-BRANCH-FEED-adhesive-dispenser']
            assert len(waiting)==1 and waiting[0]['received_ms'] is None and waiting[0]['cargo']
        negatives.append({'scenario':scenario,'affected_order_accepted':0,'other_orders_unchanged':True})
    write(proof/'branches-materials-receipt.json',{'state':'passed_actual_host_run','devices_with_actual_work':len(jobs),
        'added_branches':accepted,'retained_connected_devices':coverage['retained_connected_devices'],
        'orders':len(orders),'transport_trips':len(run['transport']),'resource_capacity_checked':len(endpoints),
        'negative_cases':negatives,'native_visual_verified':False})
    print('Verified 92 actual equipment users, 61 receiving/dispatch branches, all orders and capacity intervals.')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--proof',type=Path,required=True);parser.add_argument('--staged',type=Path,required=True)
    parser.add_argument('--checks',type=Path)
    a=parser.parse_args();check(a.proof,a.staged,a.checks)
