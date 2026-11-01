from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q20';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));checks=[]
catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b01q20/attempt-03/behavior-checks.json');names=['b01q20-ordinary-03','b01q20-ordinary-04'];proofs=[read(BASE/(n+'-proof.json'))for n in names]
for name,p in zip(names,proofs):
 assert p['passed']and p['exact_Run_after_UI_open']and p['viewing_preserves_Run']and read(BASE/(name+'-close.json'))['normal_close']
 for spec in behavior['checks']:
  key=spec['id'];s=catalog[key];pack=s['rule']=='pack'
  for condition,value in spec['conditions'].items():
   counter=key=='carton-counter-stacker';rows=[r for r in p['samples']if r['device']==key and r['condition']==condition];assert len(rows)==(3 if value['wait']else 2);dev=key+'--'+condition;carrier=value['carrier_id'];carriers=value['carrier_ids'];children=value['content_ids'];items=[r['data']for r in rows[-1]['items']if r['kind']=='item'];outs=[i for i in items if i['id']in carriers];assert len(outs)==len(carriers)
   for out in outs:assert out['holder']==dict(kind='location',id=value['destination'])and out['quality']=='released'and out['lifecycle']=='available'and out['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0))and out['attributes']['presentation_state']['value']==s['final']
   out=outs[0];assert set(out['contents'])==set(children)
   if pack:assert next(i for i in items if i['id']=='container-'+dev)['lifecycle']=='transformed'and out['composition_refs']==['container-'+dev]
   elif counter:assert all(not i['contents']and not i['composition_refs']for i in outs)and set(carriers)=={'carton-'+str(i)+'-'+dev for i in range(6)}
   else:assert out['id']=='paper-'+dev and next(i for i in items if i['id']=='auxiliary-'+dev)['lifecycle']=='transformed'and not out['composition_refs']
   for sample in rows:
    current={r['data']['id']:r['data']for r in sample['items']if r['kind']=='item'};parent=current.get(carrier)
    if parent:
     assert set(parent['contents'])==set(children)
     for uid in children:assert current[uid]['holder']==dict(kind='container',id=carrier)and current[uid]['quality']=='released'and current[uid]['quantity']==out['quantity']and current[uid]['lifecycle']=='available'
    for uid in children:
     if uid in current:assert current[uid]['quantity']==out['quantity']
   for sample in rows[:-1]:
    j=sample['job'][0]['data'];current={r['data']['id']:r['data']for r in sample['items']if r['kind']=='item'}
    if condition in ['missing','missing-auxiliary','missing-container']:assert j['started_at_ms']is None
    elif condition in ['phase-wait','interrupted']:
     held=children+['container-'+dev]if pack else carriers if counter else['paper-'+dev,'auxiliary-'+dev];assert j['state']=='waiting_resource';assert all(current[uid]['holder']==dict(kind='job',id=j['id'])for uid in held)
    elif condition=='output-wait':
     assert j['state']=='waiting_output'and current[carrier]['holder']==dict(kind='job',id=j['id'])
     if pack or counter:assert set(j['pending_outputs'])==set(carriers)
     else:assert j['effective_progress_ms']==0 and current[carrier]['attributes']['presentation_state']['value']==s['initial']and current['auxiliary-'+dev]['holder']==dict(kind='job',id=j['id'])
   if pack:
    for sample in rows:
     j=sample['job'][0]['data']
     if j['started_at_ms']is not None and j['state']!='completed':assert 'operator-'+dev in j['job_resources']
   if condition=='interrupted':a,b=[r['job'][0]['data']for r in rows[:2]];assert a['effective_progress_ms']==b['effective_progress_ms']>0 and a['phase_ref']==b['phase_ref']=='auto'
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'passed':True,'holders_verified':True,'six_original_contents_verified':True,'recovery_verified':True})
media=[];saved=BASE/'b01q20/native-saved-r2/saved'
for folder in [HOME/'installable',HOME/'items']:
 for model in folder.rglob('*.glb'):
  d=read(model.parent/'module.json')['data']['definition'];target=saved/'bundled'/d['id'].encode().hex()/d['version'].encode().hex()/model.name;assert target.read_bytes()==model.read_bytes();media.append({'module_ref':{'id':d['id'],'version':d['version']},'asset':model.name,'exact_current_media_preserved':True})
(BASE/'b01q20/Native-persistence-checks.json').write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'processes':[p['identity']for p in proofs],'current_media_checks':media,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q20 two ordinary processes: {len(checks)} condition checks, {len(media)} exact assets and six original products verified.')
