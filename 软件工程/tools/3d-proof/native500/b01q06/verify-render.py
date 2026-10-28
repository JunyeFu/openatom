from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q06';native=ROOT/'.work/native500-quality-20261025/b01q06-render-03';attempt=ROOT/'.work/native500-quality-20261025/b01q06/attempt-03';read=lambda p:json.loads(p.read_text(encoding='utf8'))
report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');assert report['passed']and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==15;rows=[]
for c in frames:
 dev,label=c['name'].split(' / ');visible={(o['kind'],o['id'])for o in c['visible_objects']};assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id'];raw='paper-'+dev;outputs=[i['id']for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out')];assert len(outputs)==2
 if label=='waiting for arrival':expected=[];absent=[raw,*outputs]
 elif 'delivered'in label or 'output blocked'in label or '--full-output-wait' in dev:expected=outputs;absent=[raw]
 else:expected=[raw];absent=outputs
 for i in expected:assert ('item',i)in visible,(c['name'],i)
 for i in absent:assert ('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*expected]for i in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persist=read(ROOT/'.work/native500-quality-20261025/b01q06/Native-persistence-r2-checks.json');assert persist['passed']and len(persist['checks'])==12
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':15,'checks':rows,'source_unchanged':True,'target_item_refs_verified':True,'two_real_pending_outputs_verified':True},indent=2)+'\n',encoding='utf8');print('B01-Q06 Native15 views: actual raw stock and two new outputs, waiting and recovery PASS.')
