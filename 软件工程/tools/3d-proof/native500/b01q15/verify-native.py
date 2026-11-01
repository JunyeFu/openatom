from pathlib import Path
import json,sys,math
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';home=ROOT/'modules/media/native-equipment-500/reviews/B01-Q15';read=lambda p:json.loads(p.read_text(encoding='utf8'));checks=[];passes=sys.argv[1:3]or['b01q15-ordinary-01','b01q15-ordinary-02']
for name in passes:
 p=read(base/(name+'-proof.json'));assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and len(p['samples'])==17
 for condition in ['normal','missing','phase-wait','interrupted','output-wait','missing-auxiliary']:
  rows=[s for s in p['samples']if s['device']=='tray-former'and s['condition']==condition];assert len(rows)==(2 if condition=='normal'else 3);dev='tray-former--'+condition;job=rows[-1]['job'][0]['data'];assert job['state']=='completed';items=[r['data']for r in rows[-1]['items']if r['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+dev);aux=next(i for i in items if i['id']=='auxiliary-'+dev);out=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];assert len(out)==1;out=out[0]
  assert raw['lifecycle']==aux['lifecycle']=='transformed'and set(out['composition_refs'])=={raw['id'],aux['id']}and out['id']not in[raw['id'],aux['id']]and out['quality']=='released'and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
  assert out['attributes']['presentation_state']['value']=='four_corner_glued_open_tray'and math.isclose(out['attributes']['developed_area_m2']['value'],raw['attributes']['area_m2']['value'],abs_tol=1e-12)and math.isclose(out['attributes']['adhesive_volume_m3']['value'],aux['attributes']['declared_dose_volume_m3']['value'],abs_tol=1e-15)
  for sample in rows[:-1]:
   j=sample['job'][0]['data'];held=[r['data']for r in sample['items']if r['kind']=='item'and r['data']['holder']==dict(kind='job',id=j['id'])and r['data']['lifecycle']in['available','in_process']]
   if condition in['missing','missing-auxiliary']:assert j['started_at_ms']is None and not held
   elif condition in['phase-wait','interrupted']:assert j['state']=='waiting_resource'and {i['id']for i in held}=={raw['id'],aux['id']}
   elif condition=='output-wait':assert j['state']=='waiting_output'and len(held)==1 and {i['id']for i in held}==set(j['pending_outputs'])and held[0]['id']==out['id']
  if condition=='interrupted':a,b=[s['job'][0]['data']for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto'and a['effective_progress_ms']==b['effective_progress_ms']>0
  checks.append({'process':p['identity']['host_instance_id'],'condition':condition,'two_source_new_open_tray_and_actual_holders':True})
a,b=[read(base/(n+'-proof.json'))for n in passes];assert a['identity']['host_instance_id']!=b['identity']['host_instance_id']and len(checks)==12
for n in passes:assert read(base/(n+'-close.json'))['normal_close']
saved=base/(sys.argv[3] if len(sys.argv)>3 else 'b01q15/native-saved-r1/saved');out=base/(sys.argv[4] if len(sys.argv)>4 else 'b01q15/Native-persistence-r1-checks.json')
assert (saved/'project.json').is_file()
media=[]
for folder in [home/'installable',home/'items']:
 for model in folder.rglob('*.glb'):
  definition=read(model.parent/'module.json')['data']['definition'];target=saved/'bundled'/definition['id'].encode().hex()/definition['version'].encode().hex()/model.name
  assert target.read_bytes()==model.read_bytes(),target
  media.append({'module_ref':{'id':definition['id'],'version':definition['version']},'asset':model.name,'bytes':model.stat().st_size,'exact_current_media_preserved':True})
assert len(media)==6

out.write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'samples_per_process':17,'processes':[a['identity'],b['identity']],'actual_media_save':True,'current_media_checks':media,'formal_NFR_timing':False},indent=2)+'\n',encoding='utf8');print('B01-Q15:two ordinary Native processes,12 condition checks,17samples/process,new two-source open tray, waits,pause and save/reopen PASS.')
