from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q01';native=ROOT/'.work/native500-quality-20261025/b01q01-render-01';attempt=ROOT/'.work/native500-quality-20261025/b01q01/attempt-04';read=lambda p:json.loads(p.read_text(encoding='utf-8'))
report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');specs=read(HOME/'catalog.json')['equipment'];assert report['passed'] and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==24;rows=[]
for c in frames:
 dev,label=c['name'].split(' / ');s=next(s for s in specs if dev.startswith(s['id']+'--'));condition=dev.split('--')[1];visible={(o['kind'],o['id']) for o in c['visible_objects']};assert c['passed'] and ('device',dev) in visible and c['scene']['view']['run_id']==run['id'];expected=[];absent=[]
 if label=='waiting for arrival':absent=['paper-'+dev];expected=['auxiliary-'+dev] if s['auxiliary'] else []
 elif label=='waiting for actual consumable':expected=['paper-'+dev];absent=['auxiliary-'+dev]
 elif 'delivered' in label or (s['rule']!='preserve' and condition in ['output-wait','backing-wait']):
  expected=[i['id'] for i in run['final_items'] if i['holder'] in [dict(kind='location',id=dev+'-out'),dict(kind='location',id=dev+'-backing')] and i['item_type_ref']['id']!='blocker'];assert len(expected)==(2 if s['id']=='hot-foil-stamper' else 1)
  if s['rule']!='preserve':absent=['paper-'+dev,'auxiliary-'+dev]
  elif s['auxiliary']:absent=['auxiliary-'+dev]
 else:
  expected=['paper-'+dev]
  if s['auxiliary']:
   if s['rule']=='preserve' and label=='unloading':absent=['auxiliary-'+dev]
   else:expected.append('auxiliary-'+dev)
 for item in expected:assert ('item',item) in visible,(c['name'],item)
 for item in absent:assert ('item',item) not in visible,(c['name'],'unexpected item '+item)
 assert not any((issue.get('object_ref') or {}).get('id') in [dev,*expected] for issue in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persist=read(ROOT/'.work/native500-quality-20261025/b01q01/Native-persistence-r2-checks.json');assert persist['passed'] and len(persist['checks'])==20
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':24,'target_item_refs_verified':True,'checks':rows,'paused_job_effective_progress':'Both ordinary Native processes verified identical effective progress and phase during each declared interruption; actual GLB forward/reverse pose and side-by-side Native review are separate evidence.','source_unchanged':True},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('B01-Q01 Native:24 actual Run views, all substrate/consumable/PET refs and absences, plus frozen interruption progress PASS.')
