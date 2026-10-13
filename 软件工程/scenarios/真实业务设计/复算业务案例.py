"""六个设计变体的有限算术复算，不是生产引擎或通用调度求解器。"""
from pathlib import Path
import json

B=Path(__file__).parent

def job(kind,item,start,end,resources):
    return dict(kind=kind,item=item,start_ms=start*1000,end_ms=end*1000,resources=resources)

def no_overlap(jobs):
    groups={}
    for j in jobs:
        assert j['start_ms']<j['end_ms']
        for res in j['resources']:groups.setdefault(res,[]).append(j)
    for res,rows in groups.items():
        rows.sort(key=lambda j:(j['start_ms'],j['end_ms']))
        assert all(x['end_ms']<=y['start_ms'] for x,y in zip(rows,rows[1:])),res

def multi_order(first):
    sequence=['A','A','B','B'] if first=='A' else ['B','B','A','A']
    starts=[0,30,100,130] if first=='A' else [15,45,100,130]
    setups=[('A_to_B',60,75)] if first=='A' else [('A_to_B',0,15),('B_to_A',75,100)]
    jobs=[job('setup',name,a,b,['W-A','Assembly']) for name,a,b in setups]
    products=[];counter={'A':0,'B':0}
    for i,(variant,s) in enumerate(zip(sequence,starts)):
        counter[variant]+=1;item=f'{variant}{counter[variant]}'
        jobs.extend([job('assembly',item,s,s+30,['W-A','Assembly']),
                     job('inspection',item,s+30,s+40,['W-B','Inspection']),
                     job('packing',item,s+40,s+50,['W-B','Packing'])])
        products.append(dict(id=item,order=variant,pcb=f'pcb{i+1}',assembly_start_ms=s*1000,
                             packaged_at_ms=(s+50)*1000,quality='released'))
    shipments=[]
    for order in ['A','B']:
        items=[p for p in products if p['order']==order]
        ready=max(p['packaged_at_ms'] for p in items)//1000
        window=next(w for w in [120,180] if w>=ready)
        accepted=window+25
        shipments.append(dict(order=order,items=[p['id'] for p in items],quantity=2,
                              depart_ms=window*1000,accepted_ms=accepted*1000,
                              cart_available_ms=(window+45)*1000))
        jobs.extend([job('transport',order,window,window+20,['T1']),
                     job('unload',order,window+20,window+25,['T1']),
                     job('return',order,window+25,window+45,['T1'])])
    no_overlap(jobs)
    # PCB仅2片初始、2片100秒到达；领用与归属不因订单改变而复制。
    consumption=sorted(p['assembly_start_ms'] for p in products)
    for t in consumption:
        used=sum(x<=t for x in consumption);available=2+(2 if t>=100000 else 0)
        assert used<=available
    due={'A':160000,'B':200000}
    outcomes=[dict(order=s['order'],demand=2,accepted=2,completion_ms=s['accepted_ms'],
                   due_ms=due[s['order']],tardiness_ms=max(0,s['accepted_ms']-due[s['order']])) for s in shipments]
    return dict(id='multi_'+first.lower()+'_first',family='multi_order',
                changes={'dispatch_first':first},jobs=jobs,products=products,shipments=shipments,
                expected_orders=outcomes,expected_last_acceptance_ms=205000,
                expected_worker_a_occupied_ms=sum(j['end_ms']-j['start_ms'] for j in jobs if 'W-A' in j['resources']),
                actual_result=None)

def calendar_detail():
    rows=[job('load','part1',0,5,['W-C','CNC']),job('auto','part1',5,35,['CNC']),
          job('wait_for_worker','part1',35,40,['CNC']),job('unload','part1',40,45,['W-C','CNC'])]
    no_overlap(rows)
    for row in rows:
        if 'W-C' in row['resources']:
            assert any(a*1000<=row['start_ms']<row['end_ms']<=b*1000 for a,b in [(0,20),(40,60)])
    return dict(id='phase_calendar',family='calendar',jobs=rows,
                expected_completion_ms=45000,expected_process_ms=40000,
                expected_worker_occupied_ms=10000,expected_wait_ms=5000,actual_result=None)

def quality_case():
    rows=[]
    for i in range(1,4):
        s=(i-1)*20;rows += [job('assembly',f'assembly{i}',s,s+20,['W-A','Assembly']),
                             job('inspection',f'assembly{i}',s+20,s+30,['W-Q','Inspection'])]
    rows += [job('packing','assembly1',30,40,['W-P','Packing']),
             job('rework','assembly2',60,75,['W-A','Rework']),
             job('reinspection','assembly2',75,85,['W-Q','Inspection']),
             job('packing','assembly3',90,100,['W-P','Packing'])]
    no_overlap(rows)
    events=[dict(at_ms=30000,item='assembly1',decision='PASS',destination='packing'),
            dict(at_ms=50000,item='assembly2',decision='FAIL',destination='rework'),
            dict(at_ms=70000,item='assembly3',decision='UNKNOWN',destination='pending_store',release_inspection_resources=True),
            dict(at_ms=85000,item='assembly2',decision='FAIL_after_one_rework',destination='isolation_store'),
            dict(at_ms=90000,item='assembly3',decision='manual_release_preset',destination='packing'),
            dict(at_ms=100000,item='assembly2',decision='scrap_preset',destination='scrap_record')]
    # 检查员70秒释放，75秒可复检；待判和隔离各容量1。
    assert rows[5]['end_ms']==70000
    return dict(id='quality_pending_and_scrap',family='quality',jobs=rows,events=events,
                expected_inventory=dict(packages=2,empty_cartons=1,pending=0,isolated=0,scrapped_assemblies=1),
                expected_order=dict(demand=3,accepted_at_factory_release=2,outstanding=1,completion_ms=None),
                input_consumption=dict(shell=3,pcb=3,fastener=12,carton=2),
                expected_quality_at_ms_80=dict(pending=['assembly3'],reinspection=['assembly2']),
                actual_result=None)

def coarse_detail():
    windows=[(0,20),(40,60)]
    assert all(b-a<40 for a,b in windows)
    detail=calendar_detail()
    return [dict(id='coarse_full_worker',family='refinement',declared_rule='single40s_job_requires_worker_fulltime',
                 expected_completion_ms=None,reason='no continuous40s worker interval',jobs=[],actual_result=None),
            dict(id='refined_phases',family='refinement',declared_rule='load5_auto30_unload5',
                 expected_completion_ms=45000,jobs=detail['jobs'],
                 reason='behavior assumption changed; not an appearance-only edit',actual_result=None)]

if __name__=='__main__':
    cases=[multi_order('A'),multi_order('B'),calendar_detail(),quality_case(),*coarse_detail()]
    assert [cases[i]['expected_worker_a_occupied_ms'] for i in (0,1)]==[135000,160000]
    assert max(o['tardiness_ms'] for o in cases[0]['expected_orders'])==5000
    assert max(o['tardiness_ms'] for o in cases[1]['expected_orders'])==45000
    for c in cases:assert c['actual_result'] is None
    output=dict(document_kind='manual_business_expectations_not_engine_results',
                time_unit='millisecond',source='self-built explicit scenario arithmetic',cases=cases,
                actual_result=None)
    (B/'expected-business.design.json').write_text(json.dumps(output,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('6 design variants consistent: allocation, resources, calendar, delivery, pending/scrap and refinement. No runtime results.')
