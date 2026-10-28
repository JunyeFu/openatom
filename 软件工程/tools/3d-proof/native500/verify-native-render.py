"""Check actual Native visibility against an existing paper transform/assembly Host result."""
from pathlib import Path
import json,sys

ROOT=Path(__file__).resolve().parents[3]; BASE=ROOT/'.work/native500-quality-20261025'
batch,attempt,native_pass=sys.argv[1:4]
HOME=ROOT/'modules/media/native-equipment-500/reviews'/batch
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
report=read(BASE/native_pass/'report.json'); behavior=read(BASE/attempt/'behavior-checks.json')
catalog={s['id']:s for s in read(HOME/'catalog.json')['equipment']}; checks=[]
assert report['passed'] and report['source_unchanged']
for case in report['driver']['cases']:
    dev,label=case['name'].split(' / '); key,condition=dev.rsplit('--',1)
    visible={(o['kind'],o['id']) for o in case['visible_objects']}
    at=case['scene']['view']['view_time_ms']
    value=next(s for s in behavior['checks'] if s['id']==key)['conditions'][condition]
    created=catalog[key]['cycle_seconds']*1000 if condition in ['output-wait','waste-wait'] else value['finished_ms']
    assert case['passed'] and ('device',dev) in visible
    assert case['scene']['evidence_origin']=='actual_software' and case['scene']['view']['run_id']==behavior['run_id']
    present=[]; absent=[]
    for uid in value['source_ids']:
        missing=(condition=='missing' and uid==value['input_id'] or condition=='missing-component' and uid=='component-'+dev or condition=='missing-auxiliary' and uid=='auxiliary-'+dev)
        (absent if (not value.get('preserves_identity') and at>=created) or missing and at<8000 else present).append(uid)
    if not value.get('preserves_identity'):
        (present if at>=created else absent).extend(value['output_ids']+value['waste_ids'])
    if condition in ['output-wait','waste-wait'] and at<40000:
        present.append('blocker-'+dev)
    framing=behavior.get('capture_framing',{}).get(case['name'],{})
    outside=framing.get('outside_frame',[])
    assert not outside or framing['reason'] and set(outside)<=set(value['source_ids'])
    for uid in present:
        if uid not in outside: assert ('item',uid) in visible,(case['name'],uid)
    for uid in absent: assert ('item',uid) not in visible,(case['name'],uid)
    for issue in case['issues']:
        assert issue['code']=='render_view.presentation_limitation' and issue['object_ref']['id'] in absent,issue
    checks.append({'name':case['name'],'present':present,'absent':absent,'outside_frame':outside,'passed':True})
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':len(checks),'native':(BASE/native_pass).relative_to(ROOT).as_posix(),'source_unchanged':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(f'{batch}: {len(checks)} Native states verified against actual item identities.')
