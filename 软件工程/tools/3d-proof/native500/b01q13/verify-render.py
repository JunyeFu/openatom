from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[4];h=ROOT/'modules/media/native-equipment-500/reviews/B01-Q13';b=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf8'));report=read(b/'b01q13-render-04/report.json');run=read(b/'b01q13/attempt-04/PAPER-QA.run.json');rows=[]
assert report['passed']and report['source_unchanged']and len(report['driver']['cases'])==18
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');visible={(o['kind'],o['id'])for o in c['visible_objects']};raw='paper-'+dev;aux='auxiliary-'+dev;out=next(i['id']for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out'));assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id']
 if label=='waiting for arrival':present=[aux];absent=[raw,out]
 elif label=='waiting for actual consumable':present=[raw];absent=[aux,out]
 elif 'delivered'in label or 'output blocked'in label:present=[out];absent=[raw,aux]
 else:present=[raw,aux];absent=[out]
 for i in present:assert('item',i)in visible,(c['name'],i)
 for i in absent:assert('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*present]for i in c['issues'])
 rows.append({'name':c['name'],'present':present,'absent':absent,'passed':True})
assert read(b/'b01q13/Native-persistence-r3-checks.json')['passed']
(h/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':18,'native':'.work/native500-quality-20261025/b01q13-render-04','actual_run':'.work/native500-quality-20261025/b01q13/attempt-04','source_unchanged':True,'two_sources_new_sleeve':True,'checks':rows},indent=2)+'\n',encoding='utf8');print('B01-Q13 Native18 actual views:blank/dose/new sleeve, exact identities and absences PASS.')
