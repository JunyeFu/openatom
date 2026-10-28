"""Verify Q04 against actual Host Run facts and prepare actual Native views."""
from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q02';attempt=ROOT/(sys.argv[1] if len(sys.argv)>1 else '.work/native500-quality-20261025/b01q02/attempt-03')
read=lambda p:json.loads(p.read_text(encoding='utf-8'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];cases=[]
assert len(run['jobs'])==8 and all(j['state']=='completed' for j in run['jobs']);assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed']
def capture(spec,condition,label,at,lod='detailed'):
 dev=spec['id']+'--'+condition;view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':dict(x=6.8,y=7.3,z=3.8),'target':dict(x=3,y=3.2,z=1.0),'zoom':1}}
 cases.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
for spec in catalog:
 key=spec['id'];op=read(HOME/'devices'/key/'module.json')['data']['definition']['payload']['data']['behavior']['inline'];load,auto,unload=[p['duration']['value'] for p in op['phases']];cycle=load+auto+unload;cut=load+int(auto*.4);preserve=spec['rule']=='preserve';extras=[o for o in op['outputs'] if o['role']!='output'];row={'id':key,'version':spec['version'],'conditions':{}}
 conditions=['normal','missing','phase-wait','interrupted','output-wait']+(['backing-wait'] if extras else [])+(['missing-auxiliary','missing-red'] if spec['auxiliary'] else [])
 for condition in conditions:
  dev=key+'--'+condition;job=next(j for j in run['jobs'] if j['execution_ref']['data']['device_id']==dev);assert job['operation_ref']==dict(id=spec['definition_id'],version=spec['version']);intervals=[p for p in run['phase_work_intervals'] if p['job_id']==job['id']];assert sum(p['interval']['end_ms']-p['interval']['start_ms'] for p in intervals)==cycle
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'missing-red':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',cut,cut+5000),'output-wait':('output_capacity',load if preserve else cycle,30000),'backing-wait':('output_capacity',cycle,30000)}[condition]
  waits=[w for w in run['waits'] if w['object_ref']['id']==job['id']]
  if expected:
   kind,start,end=expected;w=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert w['interval']==dict(start_ms=start,end_ms=end) and w['end_cause']=='resolved',(dev,w,expected)
   if condition=='missing-auxiliary':assert any(r['blocking_object']['id']==dev+'-auxiliary' for r in w['reasons'])
   if condition=='backing-wait':assert any(r['blocking_object']['id']==dev+'-backing' for r in w['reasons'])
  else:assert not waits
  finished=cycle+(8000 if condition in ['missing','missing-auxiliary','missing-red'] else 5000 if condition in ['phase-wait','interrupted'] else 0)
  if condition=='output-wait':finished=30000+cycle-load if preserve else 30000
  if condition=='backing-wait':finished=30000
  assert job['finished_at_ms']==finished
  raw_id='paper-'+dev;raw=next(i for i in run['final_items'] if i['id']==raw_id);out=[i for i in run['final_items'] if i['holder']==dict(kind='location',id=dev+'-out')];assert len(out)==1 and out[0]['quality']=='released';assert out[0]['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
  events=[e for e in run['events'] if e['payload']['kind']=='item_transferred' and e['payload']['data']['item_ref']==raw_id];assert events[0]['payload']['data']['to_holder']==dict(kind='job',id=job['id']) and events[0]['at_ms']==(8000 if condition in ['missing','missing-auxiliary','missing-red'] else 0)
  assert math.isclose(out[0]['attributes']['area_m2']['value'],raw['attributes']['area_m2']['value'],abs_tol=1e-12)
  if preserve:
   assert out[0]['id']==raw_id;assert raw['attributes']['presentation_state']['value']=='two_color_printed'
   for role,color in [('auxiliary','blue'),('auxiliary_red','red')]:
    aux=next(i for i in run['final_items'] if i['id']==role+'-'+dev);assert aux['lifecycle']=='transformed' and aux['id'] in job['input_refs'];assert aux['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0));assert aux['attributes']['declared_dose_volume_m3']==raw['attributes']['declared_'+color+'_layer_volume_m3'];assert next(t for t in receipt['project']['item_types'] if t['id']==key+'-'+role)['tracking']=='batch'
   if condition=='missing-red':assert any(r['blocking_object']['id']==dev+'-auxiliary_red' for w in waits for r in w['reasons'])
  row['conditions'][condition]={'job_id':job['id'],'wait':expected,'started_ms':job['started_at_ms'],'finished_ms':finished,'effective_work_ms':cycle,'passed':True}
 capture(spec,'normal','held at processing midpoint',load+auto//2);capture(spec,'normal','held simple LOD',load+auto//2,'simplified');capture(spec,'normal','loading',load//2);capture(spec,'normal','unloading',cycle-unload//2);capture(spec,'normal','delivered',cycle+1)
 capture(spec,'missing','waiting for arrival',4000);capture(spec,'phase-wait','holding before phase resource',load+1000);capture(spec,'interrupted','paused A',cut+1000);capture(spec,'interrupted','paused B',cut+4000);capture(spec,'interrupted','resumed',cut+5000+auto//10)
 capture(spec,'output-wait','holding while output blocked',(load if preserve else cycle)+1000);capture(spec,'output-wait','delivered after buffer clearance',(30000+cycle-load if preserve else 30000)+1)
 if extras:capture(spec,'backing-wait','holding while spent carrier buffer blocked',cycle+1000);capture(spec,'backing-wait','delivered after spent carrier clearance',30001)
 if spec['auxiliary']:capture(spec,'missing-auxiliary','waiting for actual consumable',4000);capture(spec,'missing-auxiliary','recovered after actual consumable arrival',8000+load+auto//2)
 capture(spec,'missing-red','waiting for actual red ink',4000);capture(spec,'missing-red','recovered after red ink arrival',8000+load+auto//2)
 checks.append(row)
assert len(cases)==16
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host Run; Native visual acceptance pending','run_id':run['id'],'input_ref':run['input_ref'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':7,'actual_jobs':8,'substrate_identity_and_consumable_or_carrier_balance':True})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':cases});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'jobs':8,'device_conditions':7,'substrate_identity_consumable_carrier':True,'fresh_Host_save_reopen':True,'Native':'pending'})
print('B01-Q02 actual Host:8 jobs,7 conditions, same paper identity and two separate consumed ink doses and fresh Host reopen PASS;16 Native views prepared.')
