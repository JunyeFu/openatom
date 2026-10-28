"""Verify this declared paper QA against actual Run facts; prepare exact Native views."""
from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[4]
attempt=ROOT/(sys.argv[1] if len(sys.argv)>1 else '.work/native500-quality-20261025/q02/attempt-04')
read=lambda p:json.loads(p.read_text(encoding='utf-8'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json')
catalog=read(ROOT/(sys.argv[2] if len(sys.argv)>2 else 'modules/media/native-equipment-500/reviews/B02-Q02')/'catalog.json')['equipment']
assert len(run['jobs'])==30 and all(j['state']=='completed' for j in run['jobs'])
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed']
cases=[];checks=[];sample_times=set()
def capture(spec,condition,label,at,lod='detailed'):
 device=spec['id']+'--'+condition;target=dict(x=3,y=3,z=1.05)
 view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+device,'object_ref':{'kind':'line','id':'line-'+device},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,
       'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':dict(x=6.4,y=6.5,z=3.4),'target':target,'zoom':1}}
 cases.append({'name':device+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+device},'preview':None,'delivery':'capture_only'}})
 sample_times.add(at)
for spec in catalog:
 key=spec['id'];cycle=spec['cycle_seconds']*1000;load=int(cycle*.2);auto=int(cycle*.6);cut=load+int(auto*.4)
 output_wait_start=load if spec['rule']=='preserve' else cycle
 output_recovered=30000+cycle-load if spec['rule']=='preserve' else 30000
 row={'id':key,'version':spec['version'],'conditions':{}}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']:
  dev=key+'--'+condition;job=next(j for j in run['jobs'] if j['execution_ref']['data']['device_id']==dev)
  assert job['operation_ref']=={'id':spec['definition_id'],'version':spec['version']}
  intervals=[p for p in run['phase_work_intervals'] if p['job_id']==job['id']]
  assert sum(p['interval']['end_ms']-p['interval']['start_ms'] for p in intervals)==cycle
  waits=[w for w in run['waits'] if w['object_ref']['id']==job['id']]
  expected={'normal':None,'missing':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',cut,cut+5000),'output-wait':('output_capacity',output_wait_start,30000)}[condition]
  if expected:
   reason,start,end=expected;wait=next(w for w in waits if any(r['kind']==reason for r in w['reasons']))
   assert wait['interval']==dict(start_ms=start,end_ms=end) and wait['end_cause']=='resolved',(dev,wait,expected)
  else:assert not waits
  assert job['finished_at_ms']=={'normal':cycle,'missing':cycle+8000,'phase-wait':cycle+5000,'interrupted':cycle+5000,'output-wait':output_recovered}[condition]
  transferred=[e for e in run['events'] if e['payload']['kind']=='item_transferred' and e['payload']['data']['item_ref']=='paper-'+dev]
  assert transferred[0]['payload']['data']['to_holder']==dict(kind='job',id=job['id'])
  raw_id='paper-'+dev
  raw=next(i for i in run['final_items'] if i['id']==raw_id)
  assert transferred[0]['at_ms']==(8000 if condition=='missing' else 0)
  if spec['rule']=='preserve':
   assert raw['holder']==dict(kind='location',id=dev+'-out') and raw['item_type_ref']['id']==key+'-input'
   assert raw['attributes']['presentation_state']==dict(kind='enum',value={'suction-separator':'separated','paper-unwinder':'unwound','paper-rewinder':'rolled'}[key])
   expected_transfer=(load+auto if key=='suction-separator' else cycle)+{'normal':0,'missing':8000,'phase-wait':5000,'interrupted':5000,'output-wait':30000-load}[condition]
   assert transferred[-1]['at_ms']==expected_transfer,(dev,transferred[-1],expected_transfer)
   assert transferred[-1]['payload']['data']['to_holder']==dict(kind='location',id=dev+'-out')
   assert raw['quantity']['value']==dict(coefficient=1,decimal_places=0)
  else:
   outputs=[i for i in run['final_items'] if i['holder']==dict(kind='location',id=dev+'-out')]
   assert len(outputs)==(2 if key=='slitter-rewinder' else 60),(dev,len(outputs))
   assert len({i['id'] for i in outputs})==len(outputs)
   assert raw['lifecycle']=='transformed'
   for output in outputs:
    assert output['item_type_ref']['id']==key+'-output' and output['quality']=='released'
    assert output['composition_refs']==[raw_id]
    assert output['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
   area=lambda i:i['attributes']['length_m']['value']*i['attributes']['width_m']['value']
   assert abs(sum(area(i) for i in outputs)-area(raw))<1e-9
   assert all(i['attributes']['thickness_m']==raw['attributes']['thickness_m'] for i in outputs)
  row['conditions'][condition]={'job_id':job['id'],'wait':expected,'started_ms':job['started_at_ms'],'finished_ms':job['finished_at_ms'],'effective_work_ms':cycle,'passed':True}
 capture(spec,'normal','held at processing midpoint',load+auto//2)
 capture(spec,'normal','held simple LOD',load+auto//2,'simplified')
 capture(spec,'normal','loading',load//2)
 capture(spec,'normal','unloading',cycle-load//2)
 capture(spec,'normal','delivered',cycle+1)
 capture(spec,'missing','waiting for arrival',4000)
 capture(spec,'phase-wait','holding before phase resource',load+1000)
 capture(spec,'interrupted','paused A',cut+1000)
 capture(spec,'interrupted','paused B',cut+4000)
 capture(spec,'interrupted','resumed',cut+5000+max(1,auto//10))
 capture(spec,'output-wait','holding while output blocked',output_wait_start+1000)
 capture(spec,'output-wait','delivered after actual buffer clearance',output_recovered+1)
 checks.append(row)
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host Run, not Native image acceptance','run_id':run['id'],'input_ref':run['input_ref'],'checks':checks,'fresh_Host_save_reopen':True,'output_identity_quantity_area_conservation':True})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':cases})
write(attempt/'sample-times.json',sorted(sample_times))
print(f'30 real jobs completed; 25 conditions verified; {len(cases)} exact Native views prepared.')
