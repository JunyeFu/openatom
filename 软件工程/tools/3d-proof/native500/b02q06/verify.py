from pathlib import Path
import json,math
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q06';attempt=ROOT/'.work/native500-quality-20261025/b02q06/attempt-02'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];captures=[]
assert receipt['fresh_Host_save_reopen']and run['whole_window_computed']and all(j['state']=='completed'for j in run['jobs'])
for s in catalog:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={};winder=s['rule']=='assemble';cutoff=key=='paper-tube-cutoff'
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[])+s.get('acceptance_conditions',[]):
  dev=key+'--'+condition;job=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev);assert job['operation_ref']==dict(id=s['definition_id'],version=s['version'])
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-component':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',total,40000),'waste-wait':('output_capacity',total,40000)}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;w=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert w['interval']==dict(start_ms=a,end_ms=b)and w['end_cause']=='resolved',(dev,w)
  else:assert not waits
  finish=40000 if condition in ['output-wait','waste-wait']else total+(8000 if condition in ['missing','missing-component','missing-auxiliary']else 5000 if condition in ['phase-wait','interrupted']else 0)
  assert job['finished_at_ms']==finish and sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==job['id'])==total
  raw=next(i for i in run['final_items']if i['id']=='paper-'+dev);sources=[raw['id']]+(['component-'+dev,'auxiliary-'+dev]if winder else[]);outputs=[i for i in run['final_items']if i['item_type_ref']['id']==key+'-output'and raw['id']in i['composition_refs']];waste=[i for i in run['final_items']if i['item_type_ref']['id']==key+'-waste'and raw['id']in i['composition_refs']]
  assert len(outputs)==(2 if cutoff else 1)and len(waste)==int(not winder)
  for uid in sources:assert next(i for i in run['final_items']if i['id']==uid)['lifecycle']=='transformed'
  for out in outputs+waste:
   assert out['holder']==dict(kind='location',id=dev+('-waste'if out in waste else'-out'))and out['lifecycle']=='available'and out['quality']=='released'and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and not out['contents']
   assert set(out['composition_refs'])==set(sources)and out['attributes']['presentation_state']['value']==('collected_paper_dust'if out in waste else s['final'])
  value=lambda i,k:i['attributes'][k]['value']
  if winder:
   plies=[next(i for i in run['final_items']if i['id']==uid)for uid in sources[:2]];incoming=sum(value(i,'strip_length_m')*value(i,'strip_width_m')*value(i,'paper_thickness_m')for i in plies)
   assert abs(incoming-value(outputs[0],'paper_volume_m3'))<1e-12 and value(next(i for i in run['final_items']if i['id']==sources[2]),'glue_ml')==1
  else:assert abs(value(raw,'paper_volume_m3')-sum(value(i,'paper_volume_m3')for i in outputs)-value(waste[0],'removed_volume_m3'))<1e-12
  rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'input_id':raw['id'],'output_ids':[i['id']for i in outputs],'waste_ids':[i['id']for i in waste],'source_ids':sources,'destination':dev+'-out','identity_quantity_quality_origins_and_declared_material_balance_verified':True,'passed':True}
  at=load+round(auto*.5)if condition=='normal'else round((expected[1]+expected[2])/2)
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  capture('actual '+condition,at)
  if condition=='normal':capture('delivered',finish+1);capture('simple processing',at,'simplified');capture('finished package closeup',finish+1,eye=dict(x=5.10,y=2.75,z=1.12),target=dict(x=4.32,y=3,z=.95))
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'real Host and fresh Host reopen; two kraft plies plus glue to one hollow tube, two cut pieces and separately collected waste','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])});write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'});print(f'Q06: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} conditions, {len(captures)} Native states planned.')
