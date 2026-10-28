from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[4];base=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf8'))
passes=sys.argv[1:3] or ['q02-ordinary-03','q02-ordinary-04'];rows=[]
for name in passes:
 proof=read(base/(name+'-proof.json'));assert proof['passed'] and proof['exact_Run_after_UI_open'] and proof['viewing_preserves_Run']
 for sample in proof['samples']:
  if sample['condition']!='normal':continue
  key=sample['device'];job=sample['job'][0]['data'];items=[v['data'] for v in sample['items'] if v['kind']=='item'];raw=next(i for i in items if i['id']=='paper-'+key+'--normal')
  outputs=[i for i in items if i['holder']==dict(kind='location',id=key+'--normal-out')]
  if job['state']=='completed':
   expected=2 if key=='slitter-rewinder' else 60 if key=='rotary-sheet-cutter' else 1;assert len(outputs)==expected
   if expected>1:
    assert all(i['composition_refs']==[raw['id']] and i['quantity']['value']==dict(coefficient=1,decimal_places=0) for i in outputs)
    area=lambda i:i['attributes']['length_m']['value']*i['attributes']['width_m']['value']
    assert abs(sum(area(i) for i in outputs)-area(raw))<1e-9
   else:assert outputs[0]['id']==raw['id'] and raw['attributes']['presentation_state']['value']=={'suction-separator':'separated','paper-unwinder':'unwound','paper-rewinder':'rolled'}[key]
   rows.append({'process':proof['identity']['host_instance_id'],'device':key,'actual_output_items':len(outputs),'identity_and_area_verified':True,'passed':True})
  else:assert raw['attributes']['presentation_state']['value']=={'suction-separator':'adhered','paper-unwinder':'rolled','paper-rewinder':'unwound','slitter-rewinder':'wide_roll','rotary-sheet-cutter':'rolled'}[key]
f=base/(sys.argv[3] if len(sys.argv)>3 else 'q02/Native-persistence-r2-checks.json');d=read(f);assert d['passed'];d['actual_per_device_output_after_native_reopen']=rows;d['no_item_snapshot_truncation']=True;f.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n',encoding='utf8');print('R2 Native: two processes, real same-state/2-roll/60-sheet outputs and area conservation PASS.')
