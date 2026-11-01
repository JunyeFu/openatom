from pathlib import Path
import json,math
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf8'));checks=[]
for name in ['b01q11-ordinary-01','b01q11-ordinary-02']:
 p=read(base/(name+'-proof.json'));assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and len(p['samples'])==20
 for condition in ['normal','missing','missing-component','missing-auxiliary','phase-wait','interrupted','output-wait']:
  rows=[s for s in p['samples']if s['device']=='flute-laminator'and s['condition']==condition];assert len(rows)==(2 if condition=='normal'else 3);dev='flute-laminator--'+condition;ids={'paper-'+dev,'component-'+dev,'auxiliary-'+dev};last=rows[-1];job=last['job'][0]['data'];assert job['state']=='completed'and set(job['input_refs'])==ids;items=[r['data']for r in last['items']if r['kind']=='item'];outputs=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];assert len(outputs)==1;out=outputs[0];assert out['id']not in ids and set(out['composition_refs'])==ids and out['contents']==[]and out['attributes']['presentation_state']['value']=='wet_laminated_board'and math.isclose(out['attributes']['finished_total_thickness_m']['value'],.0047,abs_tol=1e-12)
  for id in ids:assert next(i for i in items if i['id']==id)['lifecycle']=='transformed'
  for sample in rows[:-1]:
   j=sample['job'][0]['data'];held=[r['data']for r in sample['items']if r['kind']=='item'and r['data']['holder']==dict(kind='job',id=j['id'])and r['data']['lifecycle']in['available','in_process']]
   if condition.startswith('missing'):assert j['started_at_ms']is None and not held
   elif condition in['phase-wait','interrupted']:assert j['state']=='waiting_resource'and {i['id']for i in held}==ids
   elif condition=='output-wait':assert j['state']=='waiting_output'and len(held)==1 and held[0]['id']==out['id']and j['pending_outputs']==[out['id']]and set(held[0]['composition_refs'])==ids
  if condition=='interrupted':a,b=[s['job'][0]['data']for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto'and a['effective_progress_ms']==b['effective_progress_ms']>0
  checks.append({'process':p['identity']['host_instance_id'],'condition':condition,'three_real_sources_one_new_wet_composite':True,'holders_and_waits':True})
a,b=[read(base/(n+'-proof.json'))for n in ['b01q11-ordinary-01','b01q11-ordinary-02']];assert a['identity']['host_instance_id']!=b['identity']['host_instance_id']and len(checks)==14
for n in ['b01q11-ordinary-01','b01q11-ordinary-02']:assert read(base/(n+'-close.json'))['normal_close']
assert (base/'b01q11/native-saved-r1/saved/project.json').is_file();(base/'b01q11/Native-persistence-r1-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'samples_per_process':20,'processes':[a['identity'],b['identity']],'actual_media_save':True,'formal_NFR_timing':False},indent=2)+'\n',encoding='utf8');print('B01-Q11: two distinct ordinary Native processes,20 samples each, three-source identity/holders/waits and menu save/new process reopen PASS.')
