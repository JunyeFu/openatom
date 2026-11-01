from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q19';attempt=ROOT/(sys.argv[1]if len(sys.argv)>1 else '.work/native500-quality-20261025/b01q19/attempt-03')
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];captures=[]
assert receipt['fresh_Host_save_reopen']and run['whole_window_computed'];assert all(j['state']=='completed'for j in run['jobs'])
for s in catalog:
 key=s['id'];pack=s['rule']=='pack';total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[])+s.get('acceptance_conditions',[]):
  dev=key+'--'+condition;job=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev);assert job['operation_ref']==dict(id=s['definition_id'],version=s['version']);wait_start=total if pack else load
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'missing-container':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',wait_start,40000)}[condition];waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;w=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert w['interval']==dict(start_ms=a,end_ms=b)and w['end_cause']=='resolved'
  else:assert not waits
  finish=total+{'missing':8000,'missing-auxiliary':8000,'missing-container':8000,'phase-wait':5000,'interrupted':5000}.get(condition,0)
  if condition=='output-wait':finish=40000+(0 if pack else total-load)
  assert job['finished_at_ms']==finish,(dev,finish,job['finished_at_ms']);assert sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==job['id'])==total
  out=next(i for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out'));assert out['quality']=='released'and out['lifecycle']=='available'and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and out['attributes']['presentation_state']['value']==s['final']
  content_ids=['product-'+str(i)+'-'+dev if pack else'unit-'+str(i)+'-'+dev for i in range(6)];assert set(out['contents'])==set(content_ids)
  for uid in content_ids:
   child=next(i for i in run['final_items']if i['id']==uid);assert child['holder']==dict(kind='container',id=out['id'])and child['quantity']==out['quantity']and child['quality']=='released'and child['lifecycle']=='available'
  if pack:
   material=next(i for i in run['final_items']if i['id']=='container-'+dev);assert material['lifecycle']=='transformed'and out['id']!=material['id']and out['composition_refs']==[material['id']]
  else:
   assert out['id']=='paper-'+dev and not out['composition_refs'];aux=next(i for i in run['final_items']if i['id']=='auxiliary-'+dev);assert aux['lifecycle']=='transformed';field='band_state'if key=='strapper'else'shrink_state';assert out['attributes'][field]['value']==('strapped'if key=='strapper'else'shrunk')
   if key=='strapper':assert aux['attributes']['strap_count']['value']==2 and aux['attributes']['strap_total_length_m']['value']==1.96
   else:assert aux['attributes']['hood_area_m2']['value']==.48
  rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'carrier_id':out['id'],'content_ids':content_ids,'identity_quantity_quality_and_six_contents_verified':True,'destination':dev+'-out','passed':True}
  at=load+round(auto*.5)if condition=='normal'else round((expected[1]+expected[2])/2)
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  capture('actual '+condition,at)
  if condition=='normal':
   capture('delivered',finish+1);capture('simple processing',at,'simplified');capture('finished package closeup',finish+1,eye=dict(x=4.52,y=2.85,z=2.4)if pack else dict(x=4.95,y=2.4,z=1.65),target=dict(x=4.32,y=3,z=1.02))
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host/fresh Host reopen; preserved package or six originals into a sourced new container','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])});write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'});print(f'Q19: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} conditions, {len(captures)} Native states planned.')
