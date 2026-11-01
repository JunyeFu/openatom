from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q03-R1';native=ROOT/(sys.argv[1] if len(sys.argv)>1 else '.work/native500-quality-20261025/q03r1-render-01');attempt=ROOT/(sys.argv[2] if len(sys.argv)>2 else '.work/native500-quality-20261025/q03r1/attempt-02')
read=lambda p:json.loads(p.read_text(encoding='utf-8'));report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');specs=read(HOME/'catalog.json')['equipment'];assert report['passed'] and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==16;rows=[]
for i,c in enumerate(frames):
 dev,label=c['name'].split(' / ');s=next(s for s in specs if dev.startswith(s['id']+'--'));condition=dev.split('--')[1];visible={(o['kind'],o['id']) for o in c['visible_objects']};assert c['passed'] and ('device',dev) in visible
 assert c['scene']['view']['run_id']==run['id'];expected=[];absent=[]
 if label=='waiting for arrival':absent=['paper-'+dev];expected=['ink-'+dev] if s['auxiliary'] else []
 elif label=='waiting for actual ink':expected=['paper-'+dev];absent=['ink-'+dev]
 elif 'delivered' in label or (s['rule']!='preserve' and condition in ['output-wait','scrap-wait']):
  expected=[i['id'] for i in run['final_items'] if i['holder'] in [dict(kind='location',id=dev+'-out'),dict(kind='location',id=dev+'-scrap')] and i['item_type_ref']['id']!='blocker'];assert len(expected)=={'flexo-slotter':7,'corrugated-slotter':7,'board-edge-trimmer':5,'paper-perforator':26,'platen-creaser':1}[s['id']]
  if s['rule']!='preserve':absent=['paper-'+dev]+(['ink-'+dev] if s['auxiliary'] else [])
 else:expected=['paper-'+dev]+(['ink-'+dev] if s['auxiliary'] else [])
 for item in expected:assert ('item',item) in visible,(c['name'],item)
 for item in absent:assert ('item',item) not in visible,(c['name'],'unexpected transformed/missing item '+item)
 assert not any((issue.get('object_ref') or {}).get('id') in [dev,*expected] for issue in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persist=read(ROOT/'.work/native500-quality-20261025/q03r1/Native-persistence-r1-checks.json');assert persist['passed'] and len(persist['checks'])==14 and persist['samples_per_process']==20
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':16,'target_item_refs_verified':True,'checks':rows,'source_unchanged':True,'paused_progress':'Both ordinary Native processes prove identical auto phase/effective progress during interruption; actual GLB contact and reverse seek are separate evidence.'},ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print('Q03-R1 Native16 views: actual paper/ink/six scrap refs, absences, holds/waits and resumed progress PASS.')
