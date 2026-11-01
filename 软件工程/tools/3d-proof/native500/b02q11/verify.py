from pathlib import Path
import json

ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q11';attempt=ROOT/'.work/native500-quality-20261025/b02q11/attempt-02'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');checks=[];captures=[]
q=lambda n:dict(unit='piece',value=dict(coefficient=n,decimal_places=0));number=lambda item,key:item['attributes'][key]['value']
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed'] and all(j['state']=='completed' for j in run['jobs'])
for s in read(HOME/'catalog.json')['equipment']:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[])+s['acceptance_conditions']:
  dev=key+'--'+condition;job=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev);assert job['operation_ref']==dict(id=s['definition_id'],version=s['version'])
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-component':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',total,40000),'waste-wait':('output_capacity',total,40000)}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;wait=next(w for w in waits if any(r['kind']==kind for r in w['reasons']));assert wait['interval']==dict(start_ms=a,end_ms=b)and wait['end_cause']=='resolved',(dev,wait)
  else:assert not waits
  finish=40000 if condition in ['output-wait','waste-wait']else total+(8000 if condition.startswith('missing')else 5000 if condition in ['phase-wait','interrupted']else 0)
  assert job['finished_at_ms']==finish and sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==job['id'])==total
  source_roles=['input']+(['component']if s['rule']=='assemble'else[])+(['auxiliary']if s['auxiliary']else[]);sources=[('paper-'if role=='input'else role+'-')+dev for role in source_roles];by_id={i['id']:i for i in run['final_items']};raw=by_id[sources[0]]
  for role,uid in zip(source_roles,sources):assert by_id[uid]['lifecycle']=='transformed'and by_id[uid]['quantity']==q(s['aux_quantity']if role=='auxiliary'else s['input_quantity']if role=='input'else s['component_quantity']if role=='component'else 1)
  outputs=[i for i in run['final_items']if i['item_type_ref']['id']==key+'-output'and raw['id']in i['composition_refs']];waste=[i for i in run['final_items']if i['item_type_ref']['id']==key+'-waste'and raw['id']in i['composition_refs']];assert len(outputs)==1 and len(waste)==len(s['extra_outputs'])
  out=outputs[0]
  for item in outputs+waste:
   assert item['holder']==dict(kind='location',id=dev+('-waste'if item in waste else'-out'))and item['lifecycle']=='available'and item['quality']=='released'and item['quantity']==q(1)and not item['contents']
   state='two_partition_slot_offcuts'if item in waste else s['final']
   assert set(item['composition_refs'])==set(sources)and number(item,'presentation_state')==state
  if s['kind']=='handle':assert number(out,'handle_count')==number(out,'consumed_handles')==2 and number(out,'applied_glue_ml')==number(by_id['auxiliary-'+dev],'glue_ml')==2 and len(sources)==3
  elif s['kind']=='rope':assert number(out,'paper_strands')==number(out,'consumed_strips')==3 and number(out,'cord_length_m')==.30
  elif s['kind']=='slot':assert number(out,'slot_count')==2 and number(waste[0],'offcut_count')==2 and abs(number(raw,'paper_area_m2')-number(out,'paper_area_m2')-number(waste[0],'removed_area_m2'))<1e-10
  elif s['kind']=='divider':assert number(out,'total_strips')==4 and number(out,'longitudinal_strips')==number(out,'cross_strips')==2
  elif s['kind']=='baler':assert number(out,'paper_mass_kg')==4*number(raw,'mass_per_charge_kg')==1 and number(out,'consumed_charges')==4 and number(out,'strap_count')==2
  else:assert number(out,'board_count')==10 and number(out,'pallet_count')==1 and number(out,'total_height_m')==.11
  rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'input_id':raw['id'],'output_ids':[out['id']],'waste_ids':[i['id']for i in waste],'source_ids':sources,'destination':dev+'-out','identity_quantities_origins_quality_and_declared_spec_verified':True,'passed':True}
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  at=load+round(auto*.55)if condition=='normal'else round((expected[1]+expected[2])/2);capture('actual '+condition,at)
  # One actual Native frame per distinct condition. Output-wait already shows
  # the finished item; LOD and product detail remain in component captures.
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'real Host/new Host reopen; independent handles, three-strip cord, slotted/divider boards, strapped paper bale and whole-stack pallet assembly','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures})
write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'})
print(f'Q11: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} conditions, {len(captures)} Native states planned.')
