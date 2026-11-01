from pathlib import Path
import json,sys
ROOT=Path(__file__).resolve().parents[4];h=ROOT/'modules/media/native-equipment-500/reviews/B01-Q15';b=ROOT/'.work/native500-quality-20261025';read=lambda p:json.loads(p.read_text(encoding='utf8'));pass_name=sys.argv[1] if len(sys.argv)>1 else 'b01q15-render-01';attempt=sys.argv[2] if len(sys.argv)>2 else 'b01q15/attempt-01';persist=sys.argv[3] if len(sys.argv)>3 else 'b01q15/Native-persistence-r1-checks.json';report=read(b/pass_name/'report.json');run=read(b/attempt/'PAPER-QA.run.json');rows=[]
assert report['passed']and report['source_unchanged']and len(report['driver']['cases'])==28
for c in report['driver']['cases']:
 dev,label=c['name'].split(' / ');visible={(o['kind'],o['id'])for o in c['visible_objects']};raw='paper-'+dev;aux='auxiliary-'+dev;out=next(i['id']for i in run['final_items']if i['holder']==dict(kind='location',id=dev+'-out'));assert c['passed']and('device',dev)in visible and c['scene']['view']['run_id']==run['id']
 if label.startswith('waiting for arrival'):present=[aux];absent=[raw,out]
 elif label.startswith('waiting for actual consumable'):present=[raw];absent=[aux,out]
 elif 'delivered'in label or 'output blocked'in label:present=[out];absent=[raw,aux]
 else:present=[raw,aux];absent=[out]
 if 'output blocked'in label:assert('item','blocker-'+dev)in visible
 for i in present:assert('item',i)in visible,(c['name'],i)
 for i in absent:assert('item',i)not in visible,(c['name'],i)
 assert not any((i.get('object_ref')or{}).get('id')in[dev,*present]for i in c['issues'])
 rows.append({'name':c['name'],'present':present,'absent':absent,'passed':True})
assert read(b/persist)['passed']
(h/'Native-render-checks.json').write_text(json.dumps({'passed':True,'views':28,'native':'.work/native500-quality-20261025/'+pass_name,'actual_run':'.work/native500-quality-20261025/'+attempt,'source_unchanged':True,'two_sources_new_open_tray':True,'checks':rows},indent=2)+'\n',encoding='utf8');print('B01-Q15 Native28 actual views:blank/dose/new open tray, exact identities and absences PASS.')
