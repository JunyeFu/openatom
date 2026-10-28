from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q18';attempt=ROOT/(sys.argv[1]if len(sys.argv)>1 else '.work/native500-quality-20261025/b01q18/attempt-06')
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];captures=[]
assert receipt['fresh_Host_save_reopen']and run['whole_window_computed'];assert all(j['state']=='completed'for j in run['jobs'])
for s in catalog:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[])+s.get('acceptance_conditions',[]):
  dev=key+'--'+condition;jobs=sorted([j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev],key=lambda j:j['finished_at_ms']);job=jobs[0];last=jobs[-1]
  assert all(j['operation_ref']==dict(id=s['definition_id'],version=s['version'])for j in jobs)
  wait_start=load if s['rule']=='preserve'else total
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',wait_start,40000),'pending-recovery':('quality',12000,25000),'reject-wait':('output_capacity',12000,40000),'rejected':None,'rework':None}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;w=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert w['interval']==dict(start_ms=a,end_ms=b)and w['end_cause']=='resolved',(dev,w)
  else:assert not waits
  finish=total+{'missing':8000,'missing-auxiliary':8000,'phase-wait':5000,'interrupted':5000}.get(condition,0)
  if condition=='output-wait':finish=40000+(total-load if s['rule']=='preserve'else 0)
  if condition=='pending-recovery':finish=25000
  if condition in ['rejected','rework']:finish=28000
  if condition=='reject-wait':finish=56000
  assert last['finished_at_ms']==finish,(dev,finish,last['finished_at_ms'])
  for j in jobs:assert sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==j['id'])==total
  if condition in ['rework','rejected','reject-wait']:
   assert len(jobs)==2;rw=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev+'-rework');assert rw['started_at_ms']==(40000 if condition=='reject-wait'else 12000)and rw['finished_at_ms']==(44000 if condition=='reject-wait'else 16000)
  raw=next(i for i in run['final_items']if i['id']=='paper-'+dev);destination=dev+('-isolation'if condition=='rejected'else'-out');assert raw['holder']==dict(kind='location',id=destination)
  assert raw['quality']==('rejected'if condition=='rejected'else'released')and raw['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and raw['lifecycle']=='available'
  assert raw['attributes']['presentation_state']['value']==s['final'];assert not raw['composition_refs'];assert raw['contents']==['unit-'+str(i)+'-'+dev for i in range(6)]
  for uid in raw['contents']:
   child=next(i for i in run['final_items']if i['id']==uid);assert child['holder']==dict(kind='container',id=raw['id'])and child['quality']=='released'and child['lifecycle']=='available'and child['quantity']==raw['quantity']
  if s['auxiliary']:
   aux=next(i for i in run['final_items']if i['id']=='auxiliary-'+dev);assert aux['lifecycle']=='transformed';field='seal_state'if key=='tape-case-sealer'else'label_state';assert raw['attributes'][field]['value']==('taped'if field=='seal_state'else'labelled')
   if field=='label_state':assert raw['attributes']['label_code']['value']=='SF-DEMO-01'
  rows[condition]={'job_id':job['id'],'inspection_job_ids':[j['id']for j in jobs],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'identity_quantity_quality_and_six_contents_verified':True,'destination':destination,'passed':True}
  at=load+round(auto*.5)if condition=='normal'else round((expected[1]+expected[2])/2)if expected else 20000
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  capture('actual '+condition,at)
  if condition=='normal':
   capture('delivered',finish+1);capture('simple processing',at,'simplified');capture('finished package closeup',finish+1,eye=dict(x=4.95,y=2.4,z=1.65),target=dict(x=4.32,y=3,z=1.02))
  if condition in ['rejected','rework','pending-recovery','reject-wait']:capture('resolved destination',finish+1)
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host and fresh Host reopen; same package and six real children; Native pending','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])});write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'});print(f'Q18: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} conditions, {len(captures)} Native states planned.')
