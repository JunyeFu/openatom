from pathlib import Path
from PIL import Image
import json,sys
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q03';native=ROOT/(sys.argv[1] if len(sys.argv)>1 else '.work/native500-quality-20261025/q03-render-04');attempt=ROOT/(sys.argv[2] if len(sys.argv)>2 else '.work/native500-quality-20261025/q03/attempt-06')
read=lambda p:json.loads(p.read_text(encoding='utf-8'));report=read(native/'report.json');run=read(attempt/'PAPER-QA.run.json');specs=read(HOME/'catalog.json')['equipment'];assert report['passed'] and report['source_unchanged'];frames=report['driver']['cases'];assert len(frames)==70;rows=[]
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
freeze=[];raster=[];pose=read(ROOT/'scenarios/媒体表现/native500-20261024/b02-q03-native-pause-pose-checks.json');assert pose['passed']
for s in specs:
 indices=[next(i for i,c in enumerate(frames) if c['name']==s['id']+'--interrupted / '+label) for label in ['paused A','paused B']]
 a,b=[Image.open(native/f'case-{i+1:02}.png').convert('RGB') for i in indices];assert a.size==b.size
 changed=sum(x!=y for x,y in zip(a.get_flattened_data(),b.get_flattened_data()))
 if changed:assert any(c['id']==s['id'] and c['exact_device_matrices'] and c['exact_held_item_phase_and_attachment_inputs'] for c in pose['checks'])
 else:freeze.append(s['id'])
 raster.append({'id':s['id'],'different_pixels':changed,'pixel_identical':changed==0,'pose_evidence':'actual Native input + product phase/GLB matrix check' if changed else 'pixel identical pair'})

(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'native':native.relative_to(ROOT).as_posix(),'actual_run':attempt.relative_to(ROOT).as_posix(),'views':70,'target_item_refs_verified':True,'checks':rows,'paused_pixel_identical_devices':freeze,'paused_raster_diagnostics':raster,'actual_pause_pose_check':'scenarios/媒体表现/native500-20261024/b02-q03-native-pause-pose-checks.json','source_unchanged':True},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('Native:70 actual Run views, all expected actual Item refs and five paused-pose checks (pixel equality and actual Native phase/pose evidence) PASS.')
