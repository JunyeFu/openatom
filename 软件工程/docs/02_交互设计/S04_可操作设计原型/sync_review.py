"""同步实际原型与已完成的用户授权AI代走查；不代填真人/公共信用。"""
from pathlib import Path
from copy import deepcopy
import json
D=Path(__file__).resolve().parent
S=D.parents[2];I=D.parent;A=S/'docs/04_开发与验收';F=A/'特征细化'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
result=read(D/'output/playwright/browser-check-result.json');frames=read(D/'output/playwright/frames.json')
current_figma=read(I/'Figma执行状态.json').get('actual_Figma',{})
assert result['script_completed'] and result['failed']==0 and not result['page_errors'] and len(frames)==24
base='docs/02_交互设计/S04_可操作设计原型/'
evidence={'entry_url':result['entry'],'entry_file_ref':base+'index.html','guide_ref':base+'README.md','browser_result_ref':base+'output/playwright/browser-check-result.json','frames_ref':base+'output/playwright/frames.json','frame_count':24,'continuous_chains':5,'finite_browser_checks':result['passed'],'browser_errors':result['page_errors'],'data_origin':['manual_expected','type_fixture','independent_UI_visual_fixture'],'actual_browser_interaction':True,'actual_product_runtime':None,'actual_user_walkthrough':None,'Figma_used':False,'product_frontend_started':False}
for filename in ['S04-02_视觉预览与连续任务复审.json','S04-04_可点击原型复审.json']:
    p=A/filename;v=read(p);v['current_isolated_prototype']=deepcopy(evidence)
    v['status']='editable_master_and_24_prototype_frames_browser_verified_human_visual_acceptance_pending' if filename.startswith('S04-02') else 'isolated_clickable_prototype_implemented_finite_browser_verified_human_review_pending'
    v['actual_user_walkthrough']=None;v['passed_public_checkpoint']=False;v['credit_change']=False
    if filename.startswith('S04-04'):
        if 'previous_blocking_evidence' not in v:v['previous_blocking_evidence']=deepcopy(v.get('blocking_evidence',[]))
        v['blocking_evidence']=['本人视觉/关键流程验收尚未取得；本地浏览器原型不能替代产品/真实Windows宿主接点']
        v['prototype_url']=result['entry'];v['actual_prototype_interaction']=deepcopy(evidence)
        v['timeline_finite_browser_verification']={'verified_case_ids':['BP04','BP05','BP06','BP07','BP08','BP16'],'result_ref':evidence['browser_result_ref'],'actual_engine_history_consumed':False,'scope':'实际浏览器交互 + 有限手工区间/点/占用样例'}
        v['timeline_pending_prototype_actions']=['真实产品Run/Events/ObjectsAt与Host接入另验','完整工程语义/工业场景容量、全量并发和纸包装提供者结果另验']
        v['current_tool']='Figma原生可点击原型 + draw.io母版 + docs隔离HTML/Three浏览器原型' if current_figma.get('clickable_prototype_complete') else 'draw.io母版 + docs隔离HTML/Three浏览器原型；Figma状态见现行执行记录'
        if current_figma.get('clickable_prototype_complete'):v['actual_current_Figma']=deepcopy(current_figma)
    else:
        v['open_evidence']=['全24帧的最终视觉质量认可仍需本人','当前纸包装五板片显示代理不替代完整专业模型','实际Engine/Host/Agent及工业验证没有由原型证明']
    write(p,v)

p=A/'S04-05_本人原型走查.json';v=read(p)
v['status']='ready_for_human_owner_walkthrough_isolated_browser_entry_verified'
v['prototype_url']=result['entry'];v['current_isolated_prototype']=deepcopy(evidence);v['plan_ref']=base+'README.md'
v['issue_closure_rule']='本人记录实际URL/VP帧、操作者/时间、操作/观察/期望/问题；窗口2修改拥有的原型或设计源后由本人复核。AI记录不作为本人操作。'
v['actual_user_walkthrough']=None;v['passed_public_checkpoint']=False;v['credit_change']=False
for r in v['flow_records']:
    if 'historical_figma_file_key' not in r:r['historical_figma_file_key']=r.get('prototype_file_key')
    r['prototype_file_key']=None;r['prototype_url']=result['entry'];r['guide_ref']=base+'README.md';r['actual_observations']=None;r['human_recheck']=None;r['operator']=None;r['started_at']=None;r['status']='not_executed'
    r['prototype_frame_refs']=[base+'output/playwright/'+id+'.png' for id in r['frame_ids']]
write(p,v)

for filename in ['连续任务与原型制作依据.json','工业工作台母版与字段细化.json']:
    p=I/filename;v=read(p);v['current_isolated_prototype']=deepcopy(evidence)
    if filename.startswith('连续任务'):
        by_id={r['frame']:r for r in frames}
        for r in v['required_frames']:
            r['new_image_ref']=base+by_id[r['id']]['file'];r['prototype_url']=result['entry']+'?frame='+r['id'];r['state']='created_in_isolated_browser_prototype_final_visual_acceptance_pending';r['actual_product_runtime']=None
    else:
        v['remaining']=['本人对26母版/24业务帧的最终视觉认可待取得','S04-05本人实际操作与问题复核未执行','真实Engine/Host/Agent、正式工程格式与实际Windows接点待产品阶段','纸包装当前显示代理不替代完整专业模型/正确性或连续无干涉验证']
    write(p,v)

p=F/'DRF02_完整定义.json';v=read(p)
if 'previous_public_timeline_merge_record' not in v:v['previous_public_timeline_merge_record']={k:deepcopy(v[k]) for k in ['scope','verification','source_consumer_refs']}
v['owner']='复审窗口2';v['public_contract_owner']='复审窗口1';v['scope']='S04-02→04→05原型、视觉消费、本人走查准备；公共合入历史保留'
v['status']='S04_24_frames_5_chains_16_browser_cases_verified_human_and_product_evidence_pending'
v['six_views']={
 'task_and_interaction':'26页母版 + 24浏览器状态 + VF01—05实际操作；输入、取消/冲突/返回、初案直交与有限修正',
 'structure_and_data':'只读消费当前派生TS/公开类型和DRF03/04/05/06；原型快照自有document_kind，不冒充ProjectState/RunResult',
 'state_and_behavior':'草稿修订/未应用字段、样例冻结输入、计算状态例/回看与表现分开；同样例截断替换不累计重叠',
 'interface_and_collaboration':'18公开操作未变、未调用产品API；H04UI32字段/6状态消费；缺真实授权保持type_fixture',
 'presentation':'真实GLB浏览器预览和原生HTML/SVG交互；纸包装step/u/panel/Pose为五板片UI代理，非专业提供者实际返回',
 'deployment_and_verification':'复用8921/既有Python、Node、Chrome与Playwright；16有限检查/24截图/四格式文件；本人结果空'
}
v['verification']={'current_public_type_basis_ref':'docs/04_开发与验收/类型格式检查结果.json','generated_TS_files':read(A/'类型格式检查结果.json')['generated_TS_files'],'current_public_API_basis_ref':'docs/04_开发与验收/公开操作契约检查结果.json','original_API_operations':18,'original_API_fixtures':36,'browser_prototype':deepcopy(evidence),'actual_product_cases_executed':0,'whole_project_validation_run_this_batch':False}
v['source_consumer_refs']=list(dict.fromkeys(v['source_consumer_refs']+[base+'refresh_sources.py',base+'prototype.data.js',base+'README.md']))
v['actual_user_walkthrough']=None;v['actual_product_runtime']=None;v['checkpoint_credit_change']=False
write(p,v)

p=F/'DRF02_缺口与修复.json';v=read(p);v['owner']='复审窗口2'
if 'previous_remaining' not in v:v['previous_remaining']=deepcopy(v['remaining'])
fixed=[
 ('H-DRF01-03','模式分别取所绑定阶段/周期的duration；去掉多阶段全局duration含糊','packages/module-kit/device-editor-capabilities.design.json'),
 ('DRF02-UI01','从多厂房进入后续任务不再被供货页面拦住；五链可操作',base+'workbench.js'),
 ('DRF02-UI02','输入子表原子提交；取消、RevisionConflict和采用取消保持对象',base+'workbench.js'),
 ('DRF02-UI03','同样例续算替换前缀段，裁窗保原起止，末刻点事件可见',base+'output/playwright/browser-check-result.json'),
 ('DRF02-UI04','纸包装角度实际改变画面；三态/认可/绑定和undo边界修正',base+'workbench.js'),
 ('DRF02-UI05','初案一例直交、无变化/三轮停止、H04字段及六态接入',base+'workbench.js'),
 ('DRF02-UI06','逐样例冻结输入/缺费率与四格式同源；快照/上传图像实际重开',base+'output/playwright/browser-check-result.json'),
]
v['fixed']=[x for x in v['fixed'] if x['id'] not in {x[0] for x in fixed}]+[dict(id=k,result=t,source_ref=r) for k,t,r in fixed]
v['evidence_refs']=list(dict.fromkeys(v['evidence_refs']+[evidence['browser_result_ref'],evidence['frames_ref'],base+'output/playwright/S04_设计说明.pdf']))
v['remaining']=['S04-05本人操作/观察/问题复核尚未执行；S04-02最终视觉认可待本人','产品Engine/Host/外部Agent/Windows接点、正式格式和工业语义/NFR仍未运行','完整专业纸包装模型与提供者结果、真实MaterialEndpoint/库位/并发Engine消费须产品接入']
v['public_merge_wait_list']='旧DRF公共字段待合列表已撤下；当前派生TS/36API合入由窗口1证明，本窗消费设计但不代替各产品消费者运行'
v['current_isolated_prototype']=deepcopy(evidence)
write(p,v)

(A/'S04-04_可点击原型复审.md').write_text('''# S04-04 可点击原型复审

2026-10-14，复审窗口2。当前已实现docs隔离原型，16组有限浏览器检查通过、24帧已导出；最终视觉认可/本人操作待取得，公共关口信用未改变。

[实际入口](http://127.0.0.1:8921/) · [操作说明](../02_交互设计/S04_可操作设计原型/README.md) · [浏览器结果](../02_交互设计/S04_可操作设计原型/output/playwright/browser-check-result.json) · [24帧图册](../02_交互设计/S04_可操作设计原型/原型图册.html)

## 实际完成

沿26页draw.io母版实现顶部菜单、底部工具、两画布三档、设备/区域成员详情、左右调宽/收起、160高独立时间轴/描述、输入联动与取消、样例回看/裁窗/前缀/迟到回复、任务取消/完成竞态、采用冲突、纸包装画面变化及三态绑定、初案直交/最多三轮、四格式说明与快照实际重开。五链的关键输入、修改、取消和返回由真实Chrome操作，数据注明manual/type_fixture。

H04-04的32字段/6状态与当前派生TS设计依据已读取；逐样例报告保留输入、窗口、边界和缺费率。图片/GIF随自有原型快照保存。没有调用产品API、没有实际Engine/Host/Agent，也没有修改受保护3D目录。原型快照不是正式ProjectState。

## 证据边界与剩余

检查仅证明列出的有限浏览器交互，不能证明全规模性能、工业正确性或Windows桌面接点。纸包装是五板片UI显示代理，三种机制的完整专业模型/提供者仍须接入；真实库位/MaterialEndpoint与全量Engine状态另验。本人走查未执行。S04-02仍需你认可视觉质量；S04-05记录保持空。

## 历史Figma

原Figma24主帧/9类页/0反应是2026-10-14历史初稿。2026-10-15用户已直接恢复Figma制作，现行文件页、原型链接和真实连线见Figma执行状态.json及Figma批次_20261015/README.md。本地交互证据与Figma连线分别记录；公共合同/准入归窗口1。
''',encoding='utf-8')
(A/'S04-05_本人原型走查.md').write_text('''# S04-05 本人原型走查

2026-10-14。入口和指南已准备；本人尚未执行、尚未通过。AI浏览器验证不计本人操作。

[打开可操作原型](http://127.0.0.1:8921/) · [五链逐步指南](../02_交互设计/S04_可操作设计原型/README.md) · [本人空记录](S04-05_本人原型走查.json)

在左侧选择VF-01—05并开始任务，按指南完成真实输入、采用、排期、外观、目标、专业三态、比较及文件重开。每链记录实际URL/VP帧、本人和时间、动作、观察、期望差异；出现问题后修改对应原型/设计源，再由本人复核。开始任务会清空当前原型工作，需保留时先下载快照。

另核对：未应用离开/冲突保留、输入Esc取消、隐藏描述后选择不弹回、同结果裁窗保身份、续算替换而非累加、末刻点事件、任务取消/完成竞态、初案直交/无变化/三轮停止、Fail或Undetermined不合格绑定、改版待复核、保存失败和文本缩放。

五条真人flow_records的operator/started_at/actual_observations/human_recheck仍为空。实际AI代走查另据用户授权与执行文件记录，不冒充真人身份。Figma现行入口按执行状态记录；目标用户试用仍留S10。
''',encoding='utf-8')
(F/'DRF02_完整定义.md').write_text('''# DRF02 S04原型与设计消费

2026-10-14，复审窗口2；公共合同合入归窗口1。原17SF/21P/18操作未变。

[完整JSON](DRF02_完整定义.json) · [修复与剩余](DRF02_缺口与修复.json) · [实际原型](http://127.0.0.1:8921/) · [五链指南](../../02_交互设计/S04_可操作设计原型/README.md)

当前交付为26页母版、24浏览器状态、五链关键操作和16组浏览器检查，页面错误0。公共派生TS/36API/TC/IPC/C04与DRF03—06设计已读取；原型业务数据仍manual/type_fixture，产品实际结果和本人操作均空。

结构使用自有原型快照，不复制第二套产品Commands；只读现有GLB和Three。行为明确未应用/草稿/已保存、输入/结果、计算/回看、表现/生产；同样例前缀替换保身份。交互覆盖采用差异/取消/冲突、模板/目标/权限示例、初案直交与有限停止、三态用途绑定/改版、四格式与重开。

纸包装step/u/panel/Pose是五板片UI代理，不能证明完整机制或连续无干涉。当前连接只为视觉关系，真实MaterialEndpoint/库位/工业布局及Engine/Host/Agent由产品接入。S04-02最终视觉认可和S04-05本人观察/问题复核待取得，公共信用未增加。

此前TC01/TC02/验收追踪合入证据保留于JSON的previous_public_timeline_merge_record，不把旧391TS/13例摘要冒充本轮全项目结果。本轮未重跑公共全项目验证或写公共进度/准入。
''',encoding='utf-8')
(F/'DRF02_缺口与修复.md').write_text('# DRF02 修复与剩余\n\n[机器源](DRF02_缺口与修复.json) · [完整定义](DRF02_完整定义.md)\n\n'+'\n'.join('- '+x['id']+'：'+x['result'] for x in v['fixed'])+'\n\n待完成：\n\n'+'\n'.join('- '+x for x in v['remaining'])+'\n\n旧公共字段待合列表已更新为当前消费状态。所有产品actual_result及本人结果仍空，不增加关口信用。\n',encoding='utf-8')

gallery='<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>S04原型24帧</title><style>body{margin:20px;background:#eef3f7;color:#172e48;font:15px Microsoft YaHei}img{width:100%;border:1px solid #c7d4e2}section{margin-bottom:25px}a{color:#146bc2}</style><h1>S04实际浏览器原型 · 24帧</h1><p>业务数据为手工/类型样例；图片不替代操作或本人验收。<a href="http://127.0.0.1:8921/">打开原型</a></p>'
for f in frames:gallery+='<section><h2><a href="http://127.0.0.1:8921/?frame='+f['frame']+'">'+f['frame']+'</a></h2><img loading="lazy" src="'+f['file']+'" alt="'+f['frame']+'实际浏览器设计状态"></section>'
(D/'原型图册.html').write_text(gallery+'</html>',encoding='utf-8')
print(json.dumps({'source_files_updated':12,'browser_cases':result['passed'],'frames':len(frames),'human_walkthrough':None,'public_credit_changed':False},ensure_ascii=False))

walkthrough_path=D/'output/delegated-walkthrough/walkthrough-result.json'
if walkthrough_path.exists() and read(walkthrough_path).get('completed'):
    from sync_walkthrough import apply
    apply()
visual_path=D/'output/visual-review/visual-review-result.json'
if visual_path.exists() and read(visual_path).get('completed'):
    from sync_visual_review import apply as apply_visual
    apply_visual()
