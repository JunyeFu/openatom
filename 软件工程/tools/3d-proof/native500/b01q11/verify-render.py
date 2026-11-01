from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q11';native=ROOT/'.work/native500-quality-20261025/b01q11-render-01';attempt=ROOT/'.work/native500-quality-20261025/b01q11/attempt-01';read=lambda p:json.loads(p.read_text(encoding='utf8'))
report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');assert report['passed']and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==19;rows=[]
for c in frames:
 dev,label=c['name'].split(' / ');visible={(o['kind'],o['id'])for o in c['visible_objects']};assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id'];ids=['paper-'+dev,'component-'+dev,'auxiliary-'+dev];out=next(i['id']for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out'))
 if label.startswith('delivered')or label=='holding new composite while blocked':expected=[out];absent=ids
 elif label=='waiting for missing':expected=ids[1:];absent=[ids[0],out]
 elif label=='waiting for missing-component':expected=[ids[0],ids[2]];absent=[ids[1],out]
 elif label=='waiting for missing-auxiliary':expected=ids[:2];absent=[ids[2],out]
 else:expected=ids;absent=[out]
 for i in expected:assert ('item',i)in visible,(c['name'],i)
 for i in absent:assert ('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*expected]for i in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persist=read(ROOT/'.work/native500-quality-20261025/b01q11/Native-persistence-r1-checks.json');assert persist['passed']and len(persist['checks'])==14
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':19,'checks':rows,'source_unchanged':True,'target_item_refs_verified':True,'three_sources_new_composite':True},indent=2)+'\n',encoding='utf8');print('B01-Q11 Native19 views: three actual sources before completion, new composite afterward, exact refs and absences PASS.')
