from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q06';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));report=read(BASE/'b02q06-render-01/report.json');catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b02q06/attempt-02/behavior-checks.json');checks=[]
assert report['passed']and report['source_unchanged']
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');key,condition=dev.rsplit('--',1);s=catalog[key];visible={(o['kind'],o['id'])for o in c['visible_objects']};at=c['scene']['view']['view_time_ms'];value=next(x for x in behavior['checks']if x['id']==key)['conditions'][condition];present=[];absent=[];created=s['cycle_seconds']*1000 if condition in ['output-wait','waste-wait']else value['finished_ms']
 assert c['passed']and('device',dev)in visible and c['scene']['evidence_origin']=='actual_software'and c['scene']['view']['run_id']==behavior['run_id']
 for uid in value['source_ids']:
  missing=condition=='missing'and uid==value['input_id']or condition=='missing-component'and uid=='component-'+dev or condition=='missing-auxiliary'and uid=='auxiliary-'+dev
  (absent if at>=created or missing and at<8000 else present).append(uid)
 (present if at>=created else absent).extend(value['output_ids']+value['waste_ids'])
 if condition in ['output-wait','waste-wait']and at<40000:present.append('blocker-'+dev)
 for uid in present:assert('item',uid)in visible,(c['name'],uid)
 for uid in absent:assert('item',uid)not in visible,(c['name'],uid)
 for issue in c['issues']:assert issue['code']=='render_view.presentation_limitation'and issue['object_ref']['id']in absent,issue
 checks.append({'name':c['name'],'present':present,'absent':absent,'passed':True})
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':len(checks),'native':'.work/native500-quality-20261025/b02q06-render-01','source_unchanged':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q06: {len(checks)} Native states verified against actual item identities.')
