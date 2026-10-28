from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q10';native=ROOT/'.work/native500-quality-20261025/b01q10-render-02';attempt=ROOT/'.work/native500-quality-20261025/b01q10/attempt-02';read=lambda p:json.loads(p.read_text(encoding='utf8'))
report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');assert report['passed']and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==16;rows=[]
for c in frames:
 dev,label=c['name'].split(' / ');visible={(o['kind'],o['id'])for o in c['visible_objects']};assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id'];paper='paper-'+dev;aux='auxiliary-'+dev
 if label=='waiting for arrival':expected=[aux];absent=[paper]
 elif label=='waiting for actual consumable':expected=[paper];absent=[aux]
 elif 'delivered'in label or label=='unloading':expected=[paper];absent=[aux]
 else:expected=[paper,aux];absent=[]
 for i in expected:assert ('item',i)in visible,(c['name'],i)
 for i in absent:assert ('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*expected]for i in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persist=read(ROOT/'.work/native500-quality-20261025/b01q10/Native-persistence-r2-checks.json');assert persist['passed']and len(persist['checks'])==14
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':16,'checks':rows,'source_unchanged':True,'target_item_refs_verified':True},indent=2)+'\n',encoding='utf8');print('B01-Q10 Native16 views: actual same board and consumed film-tail identity, holders, waits and absences PASS.')
