"""Check same-identity robotic handling and prepare actual Native states."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q22';attempt=ROOT/'.work/native500-quality-20261025/b01q22/attempt-01'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_bytes((json.dumps(v,ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');checks=[];captures=[]
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed'] and all(j['state']=='completed' for j in run['jobs'])
metadata={s['id']:s for s in read(HOME/'items/inventory.json')['metadata']}
for s in read(HOME/'catalog.json')['equipment']:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']:
  dev=key+'--'+condition;job=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev);assert job['operation_ref']==dict(id=s['definition_id'],version=s['version'])
  expected={'normal':None,'missing':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',load,40000)}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;wait=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert wait['interval']==dict(start_ms=a,end_ms=b)and wait['end_cause']=='resolved'
  else:assert not waits
  finish=40000+total-load if condition=='output-wait'else total+(8000 if condition=='missing'else 5000 if condition in ['phase-wait','interrupted']else 0)
  assert job['finished_at_ms']==finish and sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==job['id'])==total
  uid='paper-'+dev;raw=next(i for i in run['final_items']if i['id']==uid)
  assert raw['holder']==dict(kind='location',id=dev+'-out')and raw['lifecycle']=='available'and raw['quality']=='released'
  assert raw['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and not raw['composition_refs']and not raw['contents']and raw['split_from_ref']is None
  assert raw['attributes']==metadata[key+'-input']['attributes']and raw['item_type_ref']==dict(id=key+'-input',revision='r1')
  assert [i['id']for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out')]==[uid]
  transfers=[e['payload']['data']for e in run['events']if e['payload']['kind']=='item_transferred'and e['payload']['data']['item_ref']==uid];assert transfers[0]['to_holder']==dict(kind='job',id=job['id'])and transfers[-1]['to_holder']==dict(kind='location',id=dev+'-out')
  rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'input_id':uid,'output_ids':[uid],'waste_ids':[],'source_ids':[uid],'destination':dev+'-out','preserves_identity':True,'expected_attributes':raw['attributes'],'expected_item_type':raw['item_type_ref'],'identity_quantities_origins_quality_and_declared_spec_verified':True,'passed':True}
  at=finish+1 if condition=='normal'else round((expected[1]+expected[2])/2)
  view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':'detailed','camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':dict(x=6.8,y=7.3,z=3.8),'target':dict(x=3,y=3.2,z=1),'zoom':1}}
  captures.append({'name':dev+' / actual '+condition,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'actual Host same-ID robotic gripping, planar arm and gantry handling, vacuum placement on a pallet slot; fresh Host reopen','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':20,'actual_jobs':len(run['jobs'])})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures})
write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':20,'jobs':len(run['jobs']),'Native':'pending'})
print(f'Q22: {len(run["jobs"])} jobs, 20 conditions; identity/attributes/quantity/quality preserved and real output reservation verified.')
