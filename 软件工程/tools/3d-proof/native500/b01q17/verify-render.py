from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q17';BASE=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));report=read(BASE/'b01q17-render-02/report.json');run=read(BASE/'b01q17/attempt-02/PAPER-QA.run.json');catalog={e['id']:e for e in read(HOME/'catalog.json')['equipment']};checks=[]
assert report['passed']and report['source_unchanged']
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');key,condition=dev.rsplit('--',1);s=catalog[key];visible={(o['kind'],o['id'])for o in c['visible_objects']};raw='paper-'+dev;aux='auxiliary-'+dev;out=next(i['id']for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out'));assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id']and c['scene']['evidence_origin']=='actual_software'
 if condition=='missing':present=[aux]if s['auxiliary']else[];absent=[raw,out]
 elif condition=='missing-auxiliary':present=[raw];absent=[aux,out]
 elif c['scene']['view']['view_time_ms']>=next(j['finished_at_ms']for j in run['jobs']if j['execution_ref']['data']['device_id']==dev)or(condition=='output-wait'and s['rule']!='preserve'):present=[out];absent=[raw,aux]if s['rule']!='preserve'else[]
 else:present=[raw]+([aux]if s['auxiliary']else[]);absent=[out]if s['rule']!='preserve'else[]
 if condition=='output-wait':present+=['blocker-'+dev]
 for i in present:assert('item',i)in visible,(c['name'],i)
 for i in set(absent)-set(present):assert('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*present]for i in c['issues'])
 checks.append({'name':c['name'],'present':present,'absent':list(set(absent)-set(present)),'passed':True})
assert read(BASE/'b01q17/Native-persistence-checks.json')['passed']
(HOME/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':len(checks),'native':'.work/native500-quality-20261025/b01q17-render-02','source_unchanged':True,'checks':checks},ensure_ascii=False,indent=2)+'\n',encoding='utf8');print(f'Q17 {len(checks)} Native cases: exact Run, actual item identities/presences/absences passed.')
