from pathlib import Path
import json

ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q08';attempt=ROOT/'.work/native500-quality-20261025/b02q08/attempt-03'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
run=read(attempt/'PAPER-QA.run.json');receipt=read(attempt/'host-receipt.json');checks=[];captures=[]
q=dict(unit='piece',value=dict(coefficient=1,decimal_places=0));number=lambda item,key:item['attributes'][key]['value']
assert receipt['fresh_Host_save_reopen'] and run['whole_window_computed'] and all(j['state']=='completed' for j in run['jobs'])
for s in read(HOME/'catalog.json')['equipment']:
 key=s['id'];total=s['cycle_seconds']*1000;load=round(total*.2);auto=round(total*.6);rows={}
 for condition in ['normal','missing','phase-wait','interrupted','output-wait']+(['missing-auxiliary']if s['auxiliary']else[]):
  dev=key+'--'+condition;job=next(j for j in run['jobs']if j['execution_ref']['data']['device_id']==dev)
  assert job['operation_ref']==dict(id=s['definition_id'],version=s['version'])
  expected={'normal':None,'missing':('missing_material',0,8000),'missing-auxiliary':('missing_material',0,8000),'phase-wait':('calendar',load,load+5000),'interrupted':('calendar',load+int(auto*.4),load+int(auto*.4)+5000),'output-wait':('output_capacity',total,40000)}[condition]
  waits=[w for w in run['waits']if w['object_ref']['id']==job['id']]
  if expected:
   kind,a,b=expected;wait=next(w for w in waits if any(r['kind']==kind for r in w['reasons']))
   assert wait['interval']==dict(start_ms=a,end_ms=b) and wait['end_cause']=='resolved',(dev,wait)
  else:assert not waits
  finish=40000 if condition=='output-wait'else total+(8000 if condition.startswith('missing')else 5000 if condition in ['phase-wait','interrupted']else 0)
  assert job['finished_at_ms']==finish and sum(p['interval']['end_ms']-p['interval']['start_ms']for p in run['phase_work_intervals']if p['job_id']==job['id'])==total
  sources=['paper-'+dev]+(['auxiliary-'+dev]if s['auxiliary']else[]);by_id={i['id']:i for i in run['final_items']};raw=by_id[sources[0]]
  assert all(by_id[uid]['lifecycle']=='transformed'and by_id[uid]['quantity']==q for uid in sources)
  outputs=[i for i in run['final_items']if i['item_type_ref']['id']==key+'-output'and raw['id']in i['composition_refs']];assert len(outputs)==1
  out=outputs[0];assert out['holder']==dict(kind='location',id=dev+'-out')and out['lifecycle']=='available'and out['quality']=='released'and out['quantity']==q and not out['contents']
  assert set(out['composition_refs'])==set(sources)and number(out,'presentation_state')==s['final']
  assert abs(number(out,'nominal_length_m')-(s['L']-s['flap']if s['kind']=='bottom'else s['L']))<1e-12 and number(out,'nominal_width_m')==s['W']
  if key=='paper-bag-tuber':assert number(raw,'metered_web_length_m')==.610 and number(raw,'web_width_m')==.300 and number(raw,'presentation_state')=='metered_rolled_paper_web'
  if s['auxiliary']:assert number(by_id['auxiliary-'+dev],'glue_ml')==s['glue_ml']
  if s['kind']=='heatseal':assert number(raw,'preapplied_heat_seal_layer')==1 and number(raw,'sealed_edges')==0 and number(out,'sealed_edges')==3
  if s['kind']=='lid':assert number(out,'single_lid_count')==1 and number(out,'nominal_height_m')==.035
  rows[condition]={'job_id':job['id'],'wait':expected,'finished_ms':finish,'effective_work_ms':total,'input_id':raw['id'],'output_ids':[out['id']],'waste_ids':[],'source_ids':sources,'destination':dev+'-out','identity_quantity_origins_quality_and_declared_spec_verified':True,'passed':True}
  def capture(label,at,lod='detailed',eye=None,target=None):
   view={'kind':'line_canvas','canvas':'line_canvas','hall_id':'hall-'+dev,'object_ref':{'kind':'line','id':'line-'+dev},'project_revision':3,'run_id':run['id'],'view_time_ms':at,'lod':lod,'camera':{'coordinate_frame':'right_handed_z_up','unit':'meter','eye':eye or dict(x=6.8,y=7.3,z=3.8),'target':target or dict(x=3,y=3.2,z=1),'zoom':1}}
   captures.append({'name':dev+' / '+label,'scope':{'kind':'project'},'params':{'revision':3,'view':view,'object_id':{'kind':'line','id':'line-'+dev},'preview':None,'delivery':'capture_only'}})
  at=load+round(auto*.55)if condition=='normal'else round((expected[1]+expected[2])/2);capture('actual '+condition,at)
  if condition=='normal':
   capture('simple processing',at,'simplified')
   capture('finished package closeup',finish+1,eye=dict(x=5.08,y=3.04,z=.9+s['H']/2+.025)if s['kind']=='wrap'else dict(x=5.02,y=2.55,z=1.14),target=dict(x=4.32,y=3,z=.9+s['H']/2)if s['kind']=='wrap'else dict(x=4.32,y=3,z=.93))
 checks.append({'id':key,'version':s['version'],'conditions':rows})
write(attempt/'behavior-checks.json',{'passed':True,'scope':'real Host/new Host reopen; six pre-cut folding/sealing operations with explicit glue or preapplied thermoplastic layer; one new identity per operation','run_id':run['id'],'checks':checks,'fresh_Host_save_reopen':True,'actual_device_conditions':sum(len(c['conditions'])for c in checks),'actual_jobs':len(run['jobs'])})
write(attempt/'native-plan.json',{'preparations':[],'allow_view_control':False,'cases':captures})
write(HOME/'host-checks.json',{'passed':True,'attempt':attempt.relative_to(ROOT).as_posix(),'device_conditions':sum(len(c['conditions'])for c in checks),'jobs':len(run['jobs']),'Native':'pending'})
print(f'Q08: {len(run["jobs"])} completed Jobs, {sum(len(c["conditions"])for c in checks)} conditions, {len(captures)} Native states planned.')
