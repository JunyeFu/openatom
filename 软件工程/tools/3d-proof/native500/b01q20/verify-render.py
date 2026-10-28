from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q20';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));report=read(BASE/'b01q20-render-03/report.json');catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};behavior=read(BASE/'b01q20/attempt-03/behavior-checks.json');checks=[]
assert report['passed']and report['source_unchanged']and read(BASE/'b01q20/Native-persistence-checks.json')['passed']
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');key,condition=dev.rsplit('--',1);s=catalog[key];pack=s['rule']=='pack';counter=key=='carton-counter-stacker';visible={(o['kind'],o['id'])for o in c['visible_objects']};at=c['scene']['view']['view_time_ms'];value=next(x for x in behavior['checks']if x['id']==key)['conditions'][condition];present=[];absent=[];carriers=value['carrier_ids'];carrier=value['carrier_id'];children=value['content_ids'];container='container-'+dev;aux='auxiliary-'+dev
 assert c['passed']and('device',dev)in visible and c['scene']['evidence_origin']=='actual_software'and c['scene']['view']['run_id']==behavior['run_id']
 if condition=='missing'and at<8000:absent+=children+([]if pack else carriers)
 else:present+=children+([]if pack else carriers)
 if pack:
  created=s['cycle_seconds']*1000 if condition=='output-wait'else value['finished_ms']
  if at>=created:present.append(carrier);absent.append(container)
  else:
   absent.append(carrier)
   if condition=='missing-container'and at<8000:absent.append(container)
   else:present.append(container)
 elif not counter:
  if(condition=='missing-auxiliary'and at<8000)or at>=value['finished_ms']:absent.append(aux)
  else:present.append(aux)
 if condition=='output-wait'and at<40000:present.append('blocker-'+dev)
 for uid in present:assert('item',uid)in visible,(c['name'],uid)
 for uid in absent:assert('item',uid)not in visible,(c['name'],uid)
 for issue in c['issues']:assert issue['code']=='render_view.presentation_limitation'and issue['object_ref']['id']in absent,issue
 checks.append({'name':c['name'],'present':present,'absent':absent,'passed':True})
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':len(checks),'native':'.work/native500-quality-20261025/b01q20-render-03','source_unchanged':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q20 {len(checks)} actual Native states: original identities and material/container visibility verified.')
