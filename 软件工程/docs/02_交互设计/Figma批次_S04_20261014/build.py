"""Generate authorized native Figma batches from the current owned design sources."""
import json,sys
from pathlib import Path
here=Path(__file__).parent
root=here.parent
source=json.loads((root/'Figma批次执行与原型连线.json').read_text(encoding='utf-8-sig'))
forms=json.loads((root/'首版表单与反馈.json').read_text(encoding='utf-8-sig'))['forms']
ledger=json.loads((root/'Figma节点与执行记录.json').read_text(encoding='utf-8-sig'))
start=int(sys.argv[1]);stop=int(sys.argv[2]);case=sys.argv[3] if len(sys.argv)>3 else None
frames=source['frames'][start:stop]
if start>=24 and not case:
 categories=['items','devices','resources','tasks','operations','transport','templates','professional','media']
 titles=['物品','设备','资源','任务','工艺','运输','模板','专业','表现素材']
 frames=[dict(id='CAT-'+c,title=t+'定义与契约',form_ids=['F01','F%02d'%(i+2)],navigation_node='device_config',category=c) for i,(c,t) in enumerate(zip(categories,titles))][start-24:stop-24]
needed={i for f in frames for i in f['form_ids']}
compact_forms={f['id']:dict(title=f['title'],fields=[{k:x[k] for k in ['ui_field_id','label','unit_or_reference','mode']} for x in f['fields']]) for f in forms if f['id'] in needed}
compact_frames=[{k:f[k] for k in ['id','title','form_ids','navigation_node','category'] if k in f} for f in frames]
compact_edges=[{k:e[k] for k in ['id','from','control']} for e in source['prototype_edges'] if e['from'] in {f['id'] for f in frames}]
contexts={
 'DEMO-L03':dict(caseTitle='L03 · 总装检查与包装线',caseFacts='八台设备布局示例 · 无本批模拟结果',resultSummary='L03 是新界面/布局示例；不混用旧 440 / 570 s 结果。'),
 'DEMO-AGENT':dict(caseTitle='Agent · 冻结购买装配样例',caseFacts='GF-07 目标 2 / GF-05 目标 3 · 300000 ms · OUT',resultSummary='冻结手算：接受 2 piece / 70、110 s；不足例目标 3、缺口 1。'),
 'DEMO-PAPER':dict(caseTitle='纸包装 · 专业交互示例',caseFacts='三机制 / 三变化 · 无冻结真实几何作业',resultSummary='专业状态与用途确认仅供原型演示；没有真实求解或认可记录。'),
 'DEMO-TWO-HALL':dict(caseTitle='两厂房 · 冻结旧样例',caseFacts='2 套 / 240 s 窗口 · 与 L03、Agent 隔离',resultSummary='冻结样例：A 125 s / B 205 s；目的接受 110 s，资源返回 140 s。')}
compact_frames=[dict(f,context=contexts[case or next(x['default_demo_case'] for x in source['frames'] if x['id']==f['id'])]) if not f['id'].startswith('CAT-') else dict(f,context=contexts['DEMO-L03']) for f in compact_frames]
payload=dict(ledger={k:ledger[k] for k in ['components','variables','assets']},frames=compact_frames,forms=compact_forms,edges=compact_edges,offset=start+(40 if case else 0),suffix=('~'+case) if case else '')
print('const D='+json.dumps(payload,ensure_ascii=False)+';\n'+(here/'screens.js').read_text(encoding='utf-8'))
