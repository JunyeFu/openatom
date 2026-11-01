from pathlib import Path
import json,math
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q16';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));checks=[]
catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b01q16/attempt-03/behavior-checks.json')
proofs=[read(BASE/(name+'-proof.json'))for name in ['b01q16-ordinary-03','b01q16-ordinary-04']]
for name,p in zip(['b01q16-ordinary-03','b01q16-ordinary-04'],proofs):
 assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and read(BASE/(name+'-close.json'))['normal_close']
 for spec in behavior['checks']:
  key=spec['id'];s=catalog[key]
  for condition in spec['conditions']:
   rows=[r for r in p['samples']if r['device']==key and r['condition']==condition];assert len(rows)==(2 if condition=='normal'else 3);dev=key+'--'+condition;job=rows[-1]['job'][0]['data'];assert job['state']=='completed'
   items=[r['data']for r in rows[-1]['items']if r['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+dev);outs=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];assert len(outs)==1;out=outs[0]
   assert out['attributes']['presentation_state']['value']==s['final']and out['quality']=='released'and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))
   if s['rule']=='preserve':assert out['id']==raw['id']and out['attributes']['bond_state']['value']=='pressed'
   else:
    aux=next(i for i in items if i['id']=='auxiliary-'+dev);assert raw['lifecycle']==aux['lifecycle']=='transformed'and set(out['composition_refs'])=={raw['id'],aux['id']}
    if key=='adhesive-dispenser':assert math.isclose(out['attributes']['adhesive_volume_m3']['value'],aux['attributes']['declared_dose_volume_m3']['value'],abs_tol=1e-15)
   for sample in rows[:-1]:
    j=sample['job'][0]['data'];held=[r['data']for r in sample['items']if r['kind']=='item'and r['data']['holder']==dict(kind='job',id=j['id'])and r['data']['lifecycle']in['available','in_process']]
    if condition in ['missing','missing-auxiliary']:assert j['started_at_ms']is None and not held
    elif condition in ['phase-wait','interrupted']:assert j['state']in['waiting_resource','waiting_output']and {i['id']for i in held}=={raw['id']}|({'auxiliary-'+dev}if s['auxiliary']else set())
    elif condition=='output-wait':
     assert j['state']=='waiting_output'and len(held)==1
     if s['rule']=='preserve':assert held[0]['id']==raw['id']and j['effective_progress_ms']==0
     else:assert held[0]['id']==out['id']and set(j['pending_outputs'])=={out['id']}
   if condition=='interrupted':a,b=[r['job'][0]['data']for r in rows[:2]];assert a['effective_progress_ms']==b['effective_progress_ms']>0 and a['phase_ref']==b['phase_ref']=='auto'
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'passed':True,'holders_verified':True,'identity_quantity_quality_and_recovery':True})
assert proofs[0]['identity']['host_instance_id']!=proofs[1]['identity']['host_instance_id']
saved=BASE/'b01q16/native-saved-r2/saved';media=[]
for folder in [HOME/'installable',HOME/'items']:
 for model in folder.rglob('*.glb'):
  d=read(model.parent/'module.json')['data']['definition'];target=saved/'bundled'/d['id'].encode().hex()/d['version'].encode().hex()/model.name;assert target.read_bytes()==model.read_bytes(),target;media.append({'module_ref':{'id':d['id'],'version':d['version']},'asset':model.name,'exact_current_media_preserved':True})
(BASE/'b01q16/Native-persistence-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'processes':[p['identity']for p in proofs],'current_media_checks':media,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(f'Q16 two ordinary processes, {len(checks)} holder/recovery checks and {len(media)} exact current media preserved.')
