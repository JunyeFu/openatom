from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q16';attempt=ROOT/(sys.argv[1]if len(sys.argv)>1 else '.work/native500-quality-20261025/b01q16/attempt-02')
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');catalog=read(HOME/'catalog.json')['equipment'];checks=[];cases=[]
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed']
assert all(j['state']=='completed' for j in run['jobs'])
for s in catalog:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[]):
  dev=key+'--'+condition;job=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev)
  assert job['operation_ref']==dict(id=s['definition_id'],version=s['version'])
  wait_start=load if s['rule']=='preserve' else total
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',wait_start,40000)}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;w=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert w['interval']==dict(start_ms=a,end_ms=b)and w['end_cause']=='resolved'
  else:assert not waits
  finish=total+({'missing':8000,'missing-auxiliary':8000,'phase-wait':5000,'interrupted':5000}.get(condition,0))
  if condition=='output-wait':finish=40000+(total-load if s['rule']=='preserve' else 0)
  assert job['finished_at_ms']==finish,(dev,finish,job['finished_at_ms'])
  assert sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==job['id'])==total
  raw=next(i for i in run['final_items']if i['id']=='paper-'+dev);out=[i for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out')];assert len(out)==1
  o=out[0];assert o['quality']=='released'and o['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
  assert o['attributes']['presentation_state']['value']==s['final']
  if s['rule']=='preserve':assert o['id']==raw['id'] and o['attributes']['bond_state']['value']=='pressed'and not o['composition_refs']
  else:
   aux=next(i for i in run['final_items']if i['id']=='auxiliary-'+dev);assert raw['lifecycle']==aux['lifecycle']=='transformed';assert set(o['composition_refs'])=={raw['id'],aux['id']}and o['id']not in[raw['id'],aux['id']]
   if key=='adhesive-dispenser':assert math.isclose(o['attributes']['adhesive_volume_m3']['value'],aux['attributes']['declared_dose_volume_m3']['value'],abs_tol=1e-15)and o['attributes']['area_m2']==raw['attributes']['area_m2']
   else:assert aux['attributes']['tape_length_m']['value']==.48
  rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'identity_quantity_quality_verified':True,'passed':True}
  # One actual Native state per applicable condition, plus ordinary delivery.
  at=load+round(auto*.5)if condition=='normal'else round((expected[1]+expected[2])/2)
  label={'normal':'processing midpoint','missing':'waiting for arrival','phase-wait':'holding before auto resource','interrupted':'paused by calendar','output-wait':'holding for occupied output','missing-auxiliary':'waiting for actual consumable'}[condition]
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   cases.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  capture(label,at)
  if condition=='normal':
   capture('delivered',finish+1)
   capture('simple processing',at,'simplified')
   if key=='carton-erector':capture('bottom tape application closeup',load+round(auto*.86),eye=dict(x=3.75,y=2.0,z=1.38),target=dict(x=3,y=3,z=1.04))
   if key=='adhesive-dispenser':capture('actual glue bead and nozzle closeup',load+round(auto*.65),eye=dict(x=3.3,y=2.38,z=1.13),target=dict(x=3.05,y=2.865,z=.902))
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host Run and fresh Host reopen; Native pending','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':cases});write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'})
print(f'Q16 real Host: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} device conditions; {len(cases)} Native cases planned.')
