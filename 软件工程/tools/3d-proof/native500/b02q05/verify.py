from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q05';attempt=ROOT/'.work/native500-quality-20261025/b02q05/attempt-03'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];captures=[]
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed']
assert all(j['state']=='completed'for j in run['jobs'])
for s in catalog:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={};inspect=s['rule']=='inspect';peeler=key=='label-peeler'
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[])+s.get('acceptance_conditions',[]):
  dev=key+'--'+condition;jobs=sorted([j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev],key=lambda j:j['finished_at_ms']);job=jobs[0];last=jobs[-1]
  assert all(j['operation_ref']==dict(id=s['definition_id'],version=s['version'])for j in jobs)
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',total,40000),'backing-wait':('output_capacity',total,40000),'pending-recovery':('quality',24000,36000),'reject-wait':('output_capacity',24000,40000),'rejected':None,'rework':None}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;w=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert w['interval']==dict(start_ms=a,end_ms=b)and w['end_cause']=='resolved',(dev,w)
  else:assert not waits
  finish=total+{'missing':8000,'missing-auxiliary':8000,'phase-wait':5000,'interrupted':5000}.get(condition,0)
  if condition in ['output-wait','backing-wait']:finish=40000
  if condition=='pending-recovery':finish=36000
  if condition in ['rejected','rework']:finish=52000
  if condition=='reject-wait':finish=68000
  assert last['finished_at_ms']==finish,(dev,finish,last['finished_at_ms'])
  for j in jobs:assert sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==j['id'])==total
  if condition in ['rework','rejected','reject-wait']:
   assert len(jobs)==2;rw=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev+'-rework');assert rw['started_at_ms']==(40000 if condition=='reject-wait'else 24000)and rw['finished_at_ms']==(44000 if condition=='reject-wait'else 28000)
  raw=next(i for i in run['final_items']if i['id']=='paper-'+dev);destination=dev+('-isolation'if condition=='rejected'else'-out');sources=[raw['id']]+(['auxiliary-'+dev]if s['auxiliary']else[])
  outputs=[raw]if inspect else[i for i in run['final_items']if i['item_type_ref']['id']==key+'-output'and raw['id']in i['composition_refs']]
  backing=[i for i in run['final_items']if i['item_type_ref']['id']==key+'-backing'and raw['id']in i['composition_refs']]if peeler else[]
  assert len(outputs)==(6 if peeler else 1)and len(backing)==int(peeler)
  assert raw['lifecycle']==('available'if inspect else'transformed')
  for out in outputs+backing:
   role='-backing'if out in backing else'';dest=dev+'-backing'if role else destination
   assert out['holder']==dict(kind='location',id=dest)and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and out['lifecycle']=='available'
   assert out['quality']==('rejected'if condition=='rejected'else'released')and not out['contents']
   assert out['attributes']['presentation_state']['value']==('spent_liner_matrix'if role else s['final'])
   assert set(out['composition_refs'])==(set()if inspect else set(sources))
  if inspect:assert outputs[0]['id']==raw['id']and raw['completed_reworks']==int(condition in ['rework','rejected','reject-wait'])
  if s['auxiliary']:assert next(i for i in run['final_items']if i['id']=='auxiliary-'+dev)['lifecycle']=='transformed'
  if peeler:
   labels_area=sum(i['attributes']['facestock_area_m2']['value']for i in outputs);assert abs(labels_area+.0192-.048)<1e-12 and backing[0]['attributes']['liner_area_m2']['value']==.048
  rows[condition]={'job_id':job['id'],'inspection_job_ids':[j['id']for j in jobs],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'input_id':raw['id'],'output_ids':[i['id']for i in outputs],'backing_ids':[i['id']for i in backing],'source_ids':sources,'destination':destination,'identity_quantity_quality_and_origins_verified':True,'passed':True}
  at=load+round(auto*.5)if condition=='normal'else round((expected[1]+expected[2])/2)if expected else finish+1 if condition=='rejected'else 40000
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  capture('actual '+condition,at)
  if condition=='normal':capture('delivered',finish+1);capture('simple processing',at,'simplified');capture('finished package closeup',finish+1,eye=dict(x=4.95,y=2.4,z=1.65),target=dict(x=4.32,y=3,z=.92))
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'real Host and fresh Host reopen; six labels are one web until actual separation creates six identities and one backing','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])});write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'});print(f'Q05: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} conditions, {len(captures)} Native states planned.')
