"""Check completed B60 raw histories against the original declared queues/route.
No render position, animation, or visible Native performance is certified here.
"""
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];OUT=ROOT/'.work/delivery/nfr-release'
def read(p):return json.loads(p.read_text(encoding='utf-8'))
design=read(ROOT/'scenarios/非功能基准/benchmark.design.json');records=[]
for variant in ['balanced','dense']:
    source=read(ROOT/f'tests/planner/p201-whole/fixtures/b60/cut-r2/{variant}.input.json')
    initial={i['id']:i for i in source['scenario']['initial_conditions']['items']}
    previous=None
    for round in [1,2,3]:
        raw=read(OUT/f'b60-{variant}-r{round}.json');run=raw['run'];jobs={j['id']:j for j in run['jobs']};moments={}
        for frame in raw['observations']:
            at=frame['at_ms'];items={i['id']:i for i in frame['items']}
            assert items.keys()==initial.keys() and len(items)==3000
            for identity,item in items.items():assert item['quantity']==initial[identity]['quantity']
            if at in moments:assert moments[at]==items,'same-time reverse read changed actual records'
            moments[at]=items
        assert set(moments)=={0,600000,1200000}
        if previous is not None:assert previous==moments,'same frozen input changed historical values'
        previous=moments
        spot=[]
        for line in design['lines']:
            ids=sorted(i for i in initial if initial[i]['holder']=={'kind':'location','id':line['source_queue']})
            assert len(ids)==500
            for index in [0,249,499]:
                identity=ids[index]
                for at in [0,600000,1200000]:
                    holder=moments[at][identity]['holder']
                    if at==0 and index==0:
                        assert holder['kind']=='job'
                        execution=jobs[holder['id']]['execution_ref'];assert execution['kind']=='project_device'
                        assert execution['data']['device_id']==line['device_ids'][0]
                        position={'kind':'execution_device','id':execution['data']['device_id']}
                    else:
                        expected=line['accepted_sink'] if at==1200000 or at==600000 and index!=499 else line['source_queue']
                        assert holder=={'kind':'location','id':expected};position=holder
                    spot.append({'line':line['id'],'index':index+1,'item':identity,'at_ms':at,'actual_holder_or_execution':position})
        records.append({'variant':variant,'round':round,'all_3000_values_and_quantities_checked':True,
            'reverse_read_exact':True,'three_same_input_histories_exact':True,'first_middle_last_holder_checks':spot})
result={'samples':records,'actual_RuntimeItem_and_JobExecutionRef_only':True,'rendered_coordinates_or_Native_acceptance':False}
(OUT/'history-validation.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({'passed_samples':len(records),'all_identities_per_sample':3000,'first_middle_last_checks':sum(len(r['first_middle_last_holder_checks']) for r in records),'Native_UI':False}))
