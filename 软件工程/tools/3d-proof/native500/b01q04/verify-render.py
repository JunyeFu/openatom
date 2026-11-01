from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q04';native=ROOT/'.work/native500-quality-20261025/b01q04-render-03';attempt=ROOT/'.work/native500-quality-20261025/b01q04/attempt-03';read=lambda p:json.loads(p.read_text(encoding='utf8'))
report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');assert report['passed'] and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==48;rows=[]
for c in frames:
 if c['name'].startswith('coating-chain / '):
  visible={(o['kind'],o['id'])for o in c['visible_objects']};assert c['passed'] and c['scene']['view']['run_id']==run['id'];assert ('item','paper-varnish-coater--chain') in visible
  assert all(('device',key+'--chain')in visible for key in ['varnish-coater','uv-curer']);rows.append({'name':c['name'],'same_item_ref':'paper-varnish-coater--chain','passed':True});continue
 dev,label=c['name'].split(' / ');condition=dev.split('--')[1];visible={(o['kind'],o['id'])for o in c['visible_objects']};assert c['passed'] and ('device',dev)in visible and c['scene']['view']['run_id']==run['id'];paper='paper-'+dev;blue='auxiliary-'+dev;red='auxiliary_red-'+dev
 if label=='waiting for arrival':expected=[blue];absent=[paper]
 elif label=='waiting for actual consumable':expected=[paper];absent=[blue]
 elif 'delivered'in label or label=='unloading':expected=[paper];absent=[blue]
 else:expected=[paper,blue];absent=[]
 if dev.startswith('uv-curer--'):expected=[x for x in expected if x!=blue];absent=[x for x in absent if x!=blue]
 for item in expected:assert ('item',item)in visible,(c['name'],item)
 for item in absent:assert ('item',item)not in visible,(c['name'],item)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*expected]for i in c['issues'])
 rows.append({'name':c['name'],'expected_item_refs':expected,'absent_item_refs':absent,'passed':True})
persistence=read(ROOT/'.work/native500-quality-20261025/b01q04/Native-persistence-r2-checks.json');assert persistence['passed']and len(persistence['checks'])==36
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':48,'source_unchanged':True,'target_item_refs_verified':True,'checks':rows,'paused_progress':'Both ordinary Native processes verify unchanged auto effective progress during the interruption.'},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print('B01-Q04 Native48 views: paper/ink identities and lamp circuit wait, absences and actual paper ownership PASS.')
