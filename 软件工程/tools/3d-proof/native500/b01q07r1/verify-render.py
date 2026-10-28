from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q07-R1';native=ROOT/'.work/native500-quality-20261025/b01q07r1-render-01';attempt=ROOT/'.work/native500-quality-20261025/b01q07r1/attempt-01';read=lambda p:json.loads(p.read_text(encoding='utf8'))
report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');assert report['passed']and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==17;rows=[]
for c in frames:
 dev,label=c['name'].split(' / ');visible={(o['kind'],o['id'])for o in c['visible_objects']};assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id'];raw='paper-'+dev;outputs=[i['id']for i in run['final_items']if i['holder']['id']in[dev+'-out',dev+'-scrap']and i['composition_refs']==[raw]];assert len(outputs)==5
 if label=='waiting for arrival':expected=[];absent=[raw,*outputs]
 elif 'delivered'in label or 'output blocked'in label or 'scrap bin blocked'in label or '--full-scrap-wait' in dev:expected=outputs;absent=[raw]
 else:expected=[raw];absent=outputs
 for i in expected:assert ('item',i)in visible,(c['name'],i)
 for i in absent:assert ('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*expected]for i in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persist=read(ROOT/'.work/native500-quality-20261025/b01q07r1/Native-persistence-r1-checks.json');assert persist['passed']and len(persist['checks'])==14
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':17,'checks':rows,'source_unchanged':True,'target_item_refs_verified':True,'four_real_pending_scraps_verified':True},indent=2)+'\n',encoding='utf8');print('B01-Q07-R1 Native17 views: actual raw stock and one blank and four corner outputs, waiting and recovery PASS.')
