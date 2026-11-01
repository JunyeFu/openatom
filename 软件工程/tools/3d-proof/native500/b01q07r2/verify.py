"""Q03 actual Host business verification and exact Native capture plan."""
from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q07-R2';attempt=ROOT/(sys.argv[1] if len(sys.argv)>1 else '.work/native500-quality-20261025/b01q07r2/attempt-01')
read=lambda p:json.loads(p.read_text(encoding='utf-8'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];cases=[];sample_times=set()
assert len(run['jobs'])==13 and all(j['state']=='completed' for j in run['jobs']);assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed']
def capture(spec,condition,label,at,lod='detailed'):
 device=spec['id']+'--'+condition
 view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+device,'object_ref':{'kind':'line','id':'line-'+device},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':dict(x=6.8,y=7.3,z=3.8),'target':dict(x=3,y=3.25,z=.95),'zoom':1}}
 cases.append({'name':device+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+device},'preview':None,'delivery':'capture_only'}});sample_times.add(at)
for spec in catalog:
 key=spec['id'];op=read(HOME/'devices'/key/'module.json')['data']['definition']['payload']['data']['behavior']['inline'];cycle=20000;load=4000;auto=12000;cut=8800;preserve=spec['rule']=='preserve';extra=[o for o in op['outputs'] if o['role']!='output'];row={'id':key,'version':spec['version'],'conditions':{}}
 conditions=['normal','missing','phase-wait','interrupted','output-wait']+(['scrap-wait','full-scrap-wait'] if extra else [])+(['missing-ink'] if spec['auxiliary'] else [])
 for condition in conditions:
  dev=key+'--'+condition;job=next(j for j in run['jobs'] if j['execution_ref']['data']['device_id']==dev)
  assert job['operation_ref']=={'id':spec['definition_id'],'version':spec['version']}
  intervals=[p for p in run['phase_work_intervals'] if p['job_id']==job['id']];assert sum(p['interval']['end_ms']-p['interval']['start_ms'] for p in intervals)==cycle
  waits=[w for w in run['waits'] if w['object_ref']['id']==job['id']]
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-ink':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',cut,cut+5000),'output-wait':('output_capacity',load if preserve else cycle,30000),'scrap-wait':('output_capacity',cycle,30000),'full-scrap-wait':('output_capacity',cycle,36000)}[condition]
  if expected:
   reason,start,end=expected;wait=next(w for w in waits if any(r['kind']==reason for r in w['reasons']));assert wait['interval']==dict(start_ms=start,end_ms=end) and wait['end_cause']=='resolved',(dev,wait,expected)
   if condition=='missing-ink':assert any(r['blocking_object']['id']==dev+'-ink' for r in wait['reasons'])
   if condition in ['scrap-wait','full-scrap-wait']:assert any(r['blocking_object']['id']==dev+'-scrap' for r in wait['reasons'])
  else:assert not waits
  finished={'normal':20000,'missing':28000,'missing-ink':28000,'phase-wait':25000,'interrupted':25000,'output-wait':46000 if preserve else 30000,'scrap-wait':30000,'full-scrap-wait':36000}[condition];assert job['finished_at_ms']==finished
  raw_id='paper-'+dev;raw=next(i for i in run['final_items'] if i['id']==raw_id)
  transfers=[e for e in run['events'] if e['payload']['kind']=='item_transferred' and e['payload']['data']['item_ref']==raw_id]
  assert transfers[0]['payload']['data']['to_holder']==dict(kind='job',id=job['id']) and transfers[0]['at_ms']==(8000 if condition in ['missing','missing-ink'] else 0)
  outputs=[i for i in run['final_items'] if i['holder']==dict(kind='location',id=dev+'-out')];assert len(outputs)==1
  if preserve:
   assert outputs[0]['id']==raw_id and raw['attributes']['presentation_state']==dict(kind='enum',value='creased')
  else:
   assert raw['lifecycle']=='transformed';scraps=[i for i in run['final_items'] if i['holder']==dict(kind='location',id=dev+'-scrap')];assert len(scraps)==sum(e['quantity']['value']['coefficient'] for e in extra)
   for e in extra:assert len([i for i in scraps if i['item_type_ref']['id']==key+'-'+e['role']])==e['quantity']['value']['coefficient']
   origin=[raw_id]+(['ink-'+dev] if spec['auxiliary'] else [])
   for out in outputs+scraps:
    assert out['quality']=='released' and set(out['composition_refs'])==set(origin);assert out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
   area=lambda i:i['attributes']['area_m2']['value'];assert math.isclose(sum(map(area,outputs+scraps)),area(raw),rel_tol=0,abs_tol=1e-12)
   if spec['auxiliary']:
    ink=next(i for i in run['final_items'] if i['id']=='ink-'+dev);assert ink['lifecycle']=='transformed' and ink['attributes']['volume_m3']['value']==.000002
  row['conditions'][condition]={'job_id':job['id'],'wait':expected,'started_ms':job['started_at_ms'],'finished_ms':finished,'effective_work_ms':cycle,'passed':True}
 capture(spec,'normal','held at processing midpoint',10000);capture(spec,'normal','held simple LOD',10000,'simplified');capture(spec,'normal','loading',2000);capture(spec,'normal','unloading',18000);capture(spec,'normal','delivered',20001)
 capture(spec,'missing','waiting for arrival',4000);capture(spec,'phase-wait','holding before phase resource',5000);capture(spec,'interrupted','paused A',9800);capture(spec,'interrupted','paused B',12800);capture(spec,'interrupted','resumed',15000)
 capture(spec,'output-wait','holding while output blocked',5000 if preserve else 21000);capture(spec,'output-wait','delivered after buffer clearance',46001 if preserve else 30001)
 if extra:capture(spec,'scrap-wait','holding while scrap bin blocked',21000);capture(spec,'scrap-wait','delivered after scrap bin clearance',30001)
 if spec['auxiliary']:capture(spec,'missing-ink','waiting for actual ink',4000);capture(spec,'missing-ink','recovered after actual ink arrival',18000)
 capture(spec,'full-scrap-wait','all four scraps held while full',21000)
 capture(spec,'full-scrap-wait','remaining scraps held after partial clearance',33000)
 capture(spec,'full-scrap-wait','delivered after final scrap clearance',36001)
 checks.append(row)
assert len(cases)==17
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host Run; Native visual acceptance still pending','run_id':run['id'],'input_ref':run['input_ref'],'checks':checks,'fresh_Host_save_reopen':True,'output_identity_quantity_area_conservation':True,'actual_device_conditions':7,'actual_jobs':13})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':cases});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'jobs':13,'device_conditions':7,'actual_blank_and_corner_identity_quantity_area':True,'fresh_Host_save_reopen':True,'Native':'pending'})
print('B01-Q07-R2 Host:13 jobs,7 conditions, blank and four corners identity/quantity/area and fresh Host reopen PASS;17 Native views prepared.')
