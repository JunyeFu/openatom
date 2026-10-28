"""Verify actual pallet composition, film consumption, transport and shortages."""
import argparse
from pathlib import Path
from prepare_full_factory import DESIGN, read, write


def check(proof, source):
    spec=read(DESIGN/'pallet-line-spec.json');run=read(proof/'actual-connected-run.json')
    before=read(source/'actual-project.json');after=read(proof/'actual-project.json')
    assert {x['id']:x['module_ref'] for x in before['devices']}=={x['id']:x['module_ref'] for x in after['devices']}
    assert len(after['devices'])==92
    final={x['id']:x for x in run['final_items']}
    pallets=[x for x in final.values() if x['item_type_ref']['id']=='PF-PALLET-loaded']
    assert len(pallets)==1; pallet=pallets[0]
    assert pallet['holder']=={'kind':'location','id':'PF-SHIPPING'} and pallet['lifecycle']=='available'
    assert pallet['attributes']['wrap_state']['value']=='wrapped'
    assert pallet['attributes']['declared_case_contents']['value']==3
    assert pallet['composition_refs']==['PF-PALLET-BASE-01']
    cases={x['id']:x for x in final.values() if x['item_type_ref']['id']=='PF-packed-case'}
    assert len(cases)==3 and set(pallet['contents'])==set(cases)
    original=read(source/'actual-scenario.json')
    inner={x['id'] for x in original['initial_conditions']['items'] if x['item_type_ref']['id']=='manual-packing-input'}
    assert len(inner)==18 and {i for c in cases.values() for i in c['contents']}==inner
    for c in cases.values():
        assert c['holder']=={'kind':'container','id':pallet['id']} and len(c['contents'])==6
        for k,v in {'case_length_m':.32,'case_width_m':.24,'case_height_m':.24,'seal_state':'taped','label_state':'labelled','band_state':'strapped'}.items():assert c['attributes'][k]['value']==v
        assert all(final[i]['holder']=={'kind':'container','id':c['id']} for i in c['contents'])
    jobs=[x for x in run['jobs'] if x['task_ref']=='PF-TASK-PALLET']
    load=next(x for x in jobs if x['execution_ref']['data']['route_step_id']=='pallet-load')
    wrap=next(x for x in jobs if x['execution_ref']['data']['route_step_id']=='pallet-wrap')
    film='auxiliary-pallet-stretch-wrapper--normal'
    assert set(load['input_refs'])==set(cases)|{'PF-PALLET-BASE-01'}
    assert set(wrap['input_refs'])=={film,pallet['id']}
    assert final[film]['lifecycle']=='transformed' and final[film]['holder']=={'kind':'job','id':wrap['id']}
    assert final[film]['attributes']['film_length_m']['value']==9
    manufacturing=[x for x in run['jobs'] if x['execution_ref']['kind']=='project_device']
    assert len(manufacturing)==spec['expected']['manufacturing_jobs']
    assert all(x['state']=='completed' for x in run['jobs'])
    manual=sorted((x for x in manufacturing if x['execution_ref']['data']['device_id']=='PF-manual-packing'),key=lambda x:x['started_at_ms'])
    assert all(a['finished_at_ms']<=b['started_at_ms'] for a,b in zip(manual,manual[1:]))
    assert len(run['transport'])==spec['expected']['transport_trips'] and all(t['phase']=='returned' for t in run['transport'])
    trips=[[t for t in run['transport'] if t['supply_ref']=='PF-PALLET-TRANSFER-'+str(i)] for i in range(3)]
    assert [len(x) for x in trips]==[3,1,1]
    assert {t['cargo'][0] for t in trips[0]}==set(cases)
    assert all(t['received_ms']<=load['started_at_ms'] for t in trips[0])
    assert trips[1][0]['cargo']==trips[2][0]['cargo']==[pallet['id']]
    assert load['finished_at_ms']<=trips[1][0]['departed_ms']<=trips[1][0]['received_ms']<=wrap['started_at_ms']
    assert wrap['finished_at_ms']<=trips[2][0]['departed_ms']
    expected=spec['expected']['accepted_orders']
    assert {o['order_id']:o['accepted']['value']['coefficient'] for o in run['orders']}==expected
    negatives={}
    for name in ['SHORT-CASES','MISSING-FILM','MISSING-PALLET']:
        negative=read(proof/('actual-PF-PALLET-'+name+'-run.json'))
        assert {o['order_id']:o['accepted']['value']['coefficient'] for o in negative['orders']}==dict(expected,**{'PF-ORDER-PALLET':0})
        negative_pallets=[x for x in negative['final_items'] if x['item_type_ref']['id']=='PF-PALLET-loaded']
        if name=='MISSING-FILM':
            assert len(negative_pallets)==1 and negative_pallets[0]['attributes']['wrap_state']['value']=='unwrapped'
            assert set(negative_pallets[0]['contents'])==set(cases)
        else:assert not negative_pallets
        assert not any(t['supply_ref']=='PF-PALLET-TRANSFER-2' for t in negative['transport'])
        negatives[name]={'accepted_pallets':0,'other_orders_unchanged':True,'loaded_pallets':len(negative_pallets)}
    receipt={'state':'actual_pallet_composition_film_transport_and_shortages_verified','pallet_id':pallet['id'],'case_ids':sorted(cases),
        'inner_ids':sorted(inner),'film_m_consumed':9,'manufacturing_jobs':len(manufacturing),'transport_trips':len(run['transport']),
        'accepted_orders':expected,'manual_station_jobs_serial':True,'negative_cases':negatives,'Native_verified':False,'whole_factory_accepted':False}
    write(proof/'pallet-materials-receipt.json',receipt)
    print('Actual three cases and eighteen inner identities packed, wrapped with9m film and shipped; all three shortages block pallet acceptance.')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('proof',type=Path);parser.add_argument('source',type=Path)
    args=parser.parse_args();check(args.proof,args.source)
