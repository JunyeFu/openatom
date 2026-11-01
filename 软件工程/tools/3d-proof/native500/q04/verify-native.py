from pathlib import Path
import json,math,sys
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';home=ROOT/'modules/media/native-equipment-500/reviews/B02-Q04';read=lambda p:json.loads(p.read_text(encoding='utf-8'));checks=[]
passes=sys.argv[1:3] or ['q04-ordinary-01','q04-ordinary-02'];catalog={e['id']:e for e in read(home/'catalog.json')['equipment']}
for name in passes:
 p=read(base/(name+'-proof.json'));assert p['passed'] and p['exact_Run_after_UI_open'] and p['viewing_preserves_Run'] and len(p['samples'])==85
 for key,spec in catalog.items():
  conditions=['normal','missing','phase-wait','interrupted','output-wait']+(['backing-wait'] if key=='hot-foil-stamper' else [])+(['missing-auxiliary'] if spec['auxiliary'] else [])
  for condition in conditions:
   rows=[s for s in p['samples'] if s['device']==key and s['condition']==condition];assert len(rows)==(2 if condition=='normal' else 3);dev=key+'--'+condition;job=rows[-1]['job'][0]['data'];assert job['state']=='completed'
   items=[r['data'] for r in rows[-1]['items'] if r['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+dev);out=[i for i in items if i['holder']==dict(kind='location',id=dev+'-out')];assert len(out)==1 and out[0]['quantity']==dict(unit='piece',value=dict(coefficient=1,decimal_places=0));assert math.isclose(out[0]['attributes']['area_m2']['value'],raw['attributes']['area_m2']['value'],abs_tol=1e-12)
   if spec['rule']=='preserve':
    assert out[0]['id']==raw['id'];assert raw['attributes']['presentation_state']['value']=={'blind-embosser':'embossed','spot-uv-printer':'uv_cured','screen-print-press':'screen_printed','pad-print-press':'pad_printed'}[key]
    if spec['auxiliary']:
     aux=next(i for i in items if i['id']=='auxiliary-'+dev);assert aux['lifecycle']=='transformed' and aux['id'] in job['input_refs'];assert aux['attributes']['declared_dose_volume_m3']==raw['attributes']['declared_finish_layer_volume_m3']
   else:
    aux=next(i for i in items if i['id']=='auxiliary-'+dev);backing=[i for i in items if i['holder']==dict(kind='location',id=dev+'-backing')];assert raw['lifecycle']==aux['lifecycle']=='transformed' and len(backing)==1
    assert backing[0]['attributes']['carrier_volume_m3']==aux['attributes']['carrier_volume_m3'];assert out[0]['attributes']['applied_layer_volume_m3']==aux['attributes']['transfer_volume_m3'];assert all(set(i['composition_refs'])=={raw['id'],aux['id']} for i in out+backing)
   for sample in rows[:-1]:
    j=sample['job'][0]['data'];held=[r['data'] for r in sample['items'] if r['kind']=='item' and r['data']['holder']==dict(kind='job',id=j['id']) and r['data']['lifecycle'] in ['available','in_process']]
    if condition in ['missing','missing-auxiliary']:assert j['started_at_ms'] is None and not held
    elif condition in ['phase-wait','interrupted','output-wait','backing-wait']:
     expected=1 if spec['rule']!='preserve' and condition in ['output-wait','backing-wait'] else 2 if spec['auxiliary'] else 1
     assert j['state'] in ['waiting_resource','waiting_output'] and len(held)==expected,(dev,len(held),expected)
     if spec['rule']!='preserve' and condition in ['output-wait','backing-wait']:assert {i['id'] for i in held}==set(j['pending_outputs'])
   if condition=='interrupted':a,b=[s['job'][0]['data'] for s in rows[:2]];assert a['phase_ref']==b['phase_ref']=='auto' and a['effective_progress_ms']==b['effective_progress_ms']>0
   checks.append({'process':p['identity']['host_instance_id'],'device':key,'condition':condition,'primary_identity_consumed_dose_or_carrier_and_holders':True})
a,b=[read(base/(n+'-proof.json')) for n in passes];assert a['identity']['host_instance_id']!=b['identity']['host_instance_id'] and len(checks)==60
for name in passes:assert read(base/(name+'-close.json'))['normal_close']
saved=sys.argv[3] if len(sys.argv)>3 else 'q04/native-saved-r1';assert (base/saved/'saved/project.json').is_file()
(base/(sys.argv[4] if len(sys.argv)>4 else 'q04/Native-persistence-r1-checks.json')).write_text(json.dumps({'passed':True,'ordinary_UI_open_save_new_process_reopen':True,'checks':checks,'samples_per_process':85,'processes':[a['identity'],b['identity']],'actual_media_save':True,'formal_NFR_timing':False},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Q04 Native persistence:two distinct ordinary processes,30 conditions each, exact Run, substrate identity, doses/PET, holders and normal close PASS.')
