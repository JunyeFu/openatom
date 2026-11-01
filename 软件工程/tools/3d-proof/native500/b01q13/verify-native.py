from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';home=ROOT/'modules/media/native-equipment-500/reviews/B01-Q13';read=lambda p:json.loads(p.read_text(encoding='utf8'));checks=[];passes=sys.argv[1:3]or['b01q13-ordinary-05','b01q13-ordinary-06']
for name in passes:
 p=read(base/(name+'-proof.json'));assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and len(p['samples'])==17
 for condition in ['normal','missing','phase-wait','interrupted','output-wait','missing-auxiliary']:
  rows=[s for s in p['samples']if s['device']=='folder-gluer'and s['condition']==condition];assert len(rows)==(2 if condition=='normal'else 3);dev='folder-gluer--'+condition;job=rows[-1]['job'][0]['data'];assert job['state']=='completed';items=[r['data']for r in rows[-1]['items']if r['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+dev);aux=next(i for i in items if i['id']=='auxiliary-'+dev);out=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];assert len(out)==1;out=out[0]
  assert raw['lifecycle']==aux['lifecycle']=='transformed'and set(out['composition_refs'])=={raw['id'],aux['id']}and out['id']not in[raw['id'],aux['id']]and out['quality']=='released'and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
  assert out['attributes']['presentation_state']['value']=='glued_flat_sleeve'and math.isclose(out['attributes']['developed_area_m2']['value'],raw['attributes']['area_m2']['value'],abs_tol=1e-12)and math.isclose(out['attributes']['adhesive_volume_m3']['value'],aux['attributes']['declared_dose_volume_m3']['value'],abs_tol=1e-15)
  for sample in rows[:-1]:
   j=sample['job'][0]['data'];held=[r['data']for r in sample['items']if r['kind']=='item'and r['data']['holder']==dict(kind='job',id=j['id'])and r['data']['lifecycle']in['available','in_process']]
   if condition in['missing','missing-auxiliary']:assert j['started_at_ms']is None and not held
   elif condition in['phase-wait','interrupted']:assert j['state']=='waiting_resource'and {i['id']for i in held}=={raw['id'],aux['id']}
   elif condition=='output-wait':assert j['state']=='waiting_output'and len(held)==1 and {i['id']for i in held}==set(j['pending_outputs'])and held[0]['id']==out['id']
  if condition=='interrupted':a,b=[s['job'][0]['data']for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto'and a['effective_progress_ms']==b['effective_progress_ms']>0
  checks.append({'process':p['identity']['host_instance_id'],'condition':condition,'two_source_new_sleeve_and_actual_holders':True})
a,b=[read(base/(n+'-proof.json'))for n in passes];assert a['identity']['host_instance_id']!=b['identity']['host_instance_id']and len(checks)==12
for n in passes:assert read(base/(n+'-close.json'))['normal_close']
assert (base/'b01q13/native-saved-r3/saved/project.json').is_file()
(base/'b01q13/Native-persistence-r3-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'samples_per_process':17,'processes':[a['identity'],b['identity']],'actual_media_save':True,'formal_NFR_timing':False},indent=2)+'\n',encoding='utf8');print('B01-Q13:two ordinary Native processes,12 condition checks,17samples/process,new two-source sleeve, waits,pause and save/reopen PASS.')
