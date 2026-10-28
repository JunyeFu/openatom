from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B02-Q05';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));report=read(BASE/'b02q05-render-03/report.json');catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b02q05/attempt-03/behavior-checks.json');checks=[]
assert report['passed']and report['source_unchanged']
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');key,condition=dev.rsplit('--',1);s=catalog[key];inspect=s['rule']=='inspect';visible={(o['kind'],o['id'])for o in c['visible_objects']};at=c['scene']['view']['view_time_ms'];value=next(x for x in behavior['checks']if x['id']==key)['conditions'][condition];present=[];absent=[];raw=value['input_id'];aux='auxiliary-'+dev;created=s['cycle_seconds']*1000 if condition in ['output-wait','backing-wait']else value['finished_ms']
 assert c['passed']and('device',dev)in visible and c['scene']['evidence_origin']=='actual_software'and c['scene']['view']['run_id']==behavior['run_id']
 if condition=='missing'and at<8000:absent.append(raw)
 elif inspect or at<created:present.append(raw)
 else:absent.append(raw)
 if not inspect:
  if at>=created:present+=value['output_ids']+value['backing_ids']
  else:absent+=value['output_ids']+value['backing_ids']
 if s['auxiliary']:
  if condition=='missing-auxiliary'and at<8000 or at>=created:absent.append(aux)
  else:present.append(aux)
 if condition in ['output-wait','backing-wait','reject-wait']and at<40000:present.append('blocker-'+dev)
 for uid in present:assert('item',uid)in visible,(c['name'],uid)
 for uid in absent:assert('item',uid)not in visible,(c['name'],uid)
 for issue in c['issues']:assert issue['code']=='render_view.presentation_limitation'and issue['object_ref']['id']in absent,issue
 checks.append({'name':c['name'],'present':present,'absent':absent,'passed':True})
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':len(checks),'native':'.work/native500-quality-20261025/b02q05-render-03','source_unchanged':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q05: {len(checks)} Native states verified against actual item identities.')
