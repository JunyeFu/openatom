from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q18';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));report=read(BASE/'b01q18-render-01/report.json');catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b01q18/attempt-06/behavior-checks.json');checks=[]
assert report['passed']and report['source_unchanged']and read(BASE/'b01q18/Native-persistence-checks.json')['passed']
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');key,condition=dev.rsplit('--',1);s=catalog[key];visible={(o['kind'],o['id'])for o in c['visible_objects']};raw='paper-'+dev;aux='auxiliary-'+dev;at=c['scene']['view']['view_time_ms'];value=next(x for x in behavior['checks']if x['id']==key)['conditions'][condition];present=[];absent=[]
 assert c['passed']and('device',dev)in visible and c['scene']['evidence_origin']=='actual_software'and c['scene']['view']['run_id']==behavior['run_id']
 if condition=='missing'and at<8000:absent+=[raw]+['unit-'+str(i)+'-'+dev for i in range(6)]
 else:present+=[raw]+['unit-'+str(i)+'-'+dev for i in range(6)]
 if s['auxiliary']:
  if (condition=='missing-auxiliary'and at<8000)or at>=value['finished_ms']:absent.append(aux)
  else:present.append(aux)
 if condition in ['output-wait','reject-wait']and at<40000:present.append('blocker-'+dev)
 for uid in present:assert('item',uid)in visible,(c['name'],uid)
 for uid in absent:assert('item',uid)not in visible,(c['name'],uid)
 for issue in c['issues']:
  assert issue['code']=='render_view.presentation_limitation'and issue['object_ref']['id']==aux and aux in absent,issue
 checks.append({'name':c['name'],'present':present,'absent':absent,'passed':True})
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':len(checks),'native':'.work/native500-quality-20261025/b01q18-render-01','source_unchanged':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q18 {len(checks)} real Native cases: exact Run, package, six children and consumable visibility verified.')
