from pathlib import Path
import json
from copy import deepcopy
D=Path(__file__).resolve().parent
I=D.parent
S=I.parents[1]
A=S/'docs/04_开发与验收'
P=I/'S04_可操作设计原型'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v): p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
state=read(D/'state.json'); qa=read(D/'figma-structural-check.json')
url=f'https://www.figma.com/proto/{state["file_key"]}?page-id=22-1278&node-id=23-1278&starting-point-node-id=23%3A1278&scaling=scale-down&content-scaling=fixed'
actual={'file_key':state['file_key'],'design_url':f'https://www.figma.com/design/{state["file_key"]}?node-id=23-1282','prototype_url':url,'page_id':state['page_id'],'main_frames':24,'support_states':len(state['frames'])-24,'state_frames':len(state['frames']),'category_frames':9,'reactions_connected':qa['reactions'],'native_text_nodes':qa['nativeTexts'],'component_instances':qa['instances'],'full_UI_flattened_image_allowed':False,'actual_design_write':True,'actual_prototype_interaction':'native_Figma_reactions_written_and_read_back','clickable_prototype_complete':True,'visual_review_complete':True,'visual_review_scope':'代表性画面截图；全部66状态结构与主画面图可达性核验，不等于逐控件实际点击','frame_node_ids':state['frames'],'verification_ref':'docs/02_交互设计/Figma批次_20261015/figma-structural-check.json','product_frontend_coding_authorized':False,'actual_product_runtime':None,'actual_user_walkthrough':None}
j=read(I/'Figma执行状态.json')
j['document_kind']='actual_current_native_Figma_clickable_design_prototype'
j['historical_design_issue']=j.get('historical_design_issue') or j.get('current_design_issue')
j['current_design_issue']=None
j.setdefault('previous_execution_before_20261015',{'ref':'Figma节点与执行记录.json 的历史7:/8:节点；attempts保留真实连接/额度失败','execution_state':'historical_unconnected_initial_design'})
j.update(date='2026-10-15 Asia/Shanghai',batch_id='S04-Figma-resume-20261015',authorization_state='user_explicitly_resumed_Figma_design_and_clickable_prototype',execution_state='completed_current_Figma_clickable_design_prototype',prototype_url=url,actual_Figma=actual,frame_node_ids=state['frames'],actual_prototype_interaction=True,blocking_evidence=None,next_required_action='用户可打开五条Figma流程走查；本轮未授权产品前端编码。')
j['current_authorization']={'source':'当前聊天用户直接请求','quote':'使用Figma完成前端原型制作','supersedes':'先不要操作Figma的旧暂停要求，仅限设计原型'}
j['file_key']=state['file_key'];j['design_url']=actual['design_url']
write(I/'Figma执行状态.json',j)
j=read(I/'Figma批次执行与原型连线.json')
j.setdefault('historical_unconnected_batch_20261014',{'ref':'Figma节点与执行记录.json 的历史7:/8:节点；原E001—E070尚未连通'})
j['actual_Figma']=actual;j['actual_execution_status']='completed_current_native_Figma_design_and_reactions';j['blocking_evidence']=None
j['generator_status']='current_20261015_editable_native_builder_and_reaction_readback'
j['historical_visual_defects']=j.get('historical_visual_defects') or j.get('known_visual_defects',[])
j['known_visual_defects']=[]
j['current_batch_ref']='docs/02_交互设计/Figma批次_20261015/state.json'
for f in j.get('frames',[]):
    if f['id'] in state['frames']:
        f['frame_node_id']=state['frames'][f['id']];f['figma_created']=True;f['actual_visual_complete']=True;f['current_batch']='20261015'
for e in j.get('prototype_edges',[]):
    e['historical_figma_control_node_id']=e.pop('figma_control_node_id',None)
    e['actual_connected']=False
    e['current_semantics']='旧E编号用于需求追踪；现行原生交互按66状态及真实reactions回读核验，不用历史E自跳页冒充执行。'
j['current_connected_reactions_ref']=actual['verification_ref']
j['prototype_edges_role']='historical_20261014_unconnected_requirement_edges_not_current_Figma_status'
j['style']['source']='现行draw.io工业母版与本地原型：顶部菜单、底部工具、独立160px时间轴和描述、灰白蓝工程外壳'
write(I/'Figma批次执行与原型连线.json',j)
j=read(I/'Figma节点与执行记录.json')
j['run_id']='S04-Figma-resume-20261015';j['pages']['current']=state['page_id'];j['active_frame_node_ids']=state['frames']
j['historical_before_20261015']=deepcopy(j.get('actual_Figma',{}));j['current_Figma']=actual;j['current_batch']='20261015';write(I/'Figma节点与执行记录.json',j)
for name in ['连续任务与原型制作依据.json','工业工作台母版与字段细化.json','Figma制作依据核对与工作台纠偏.json']:
    j=read(I/name);j['current_Figma_prototype']=actual;j['Figma_hold_superseded']={'date':'2026-10-15','quote':'使用Figma完成前端原型制作','scope':'设计原型；draw.io继续保留为母版源，本地原型保留为实际交互验证源'};write(I/name,j)
review=read(A/'S04-04_可点击原型复审.json');review['actual_current_Figma']=actual;review['current_tool']='Figma原生可点击原型 + draw.io母版 + docs隔离浏览器原型';review['Figma_hold_superseded']=j['Figma_hold_superseded'];write(A/'S04-04_可点击原型复审.json',review)
dq=read(P/'output/dq-state/result.json');walk=read(P/'output/delegated-walkthrough/walkthrough-result.json');reg=read(P/'output/playwright/browser-check-result.json')
packet={'document_kind':'owned_S04_repair_delivery_not_public_gate','date':'2026-10-15 Asia/Shanghai','status':'source_fixed_browser_regression_passed_independent_merge_by_coordinator','source_stable':True,'finding_ids':['DQ-03','DQ-04','DQ-05'],'changed_files':['docs/02_交互设计/S04_可操作设计原型/workbench.js','docs/02_交互设计/S04_可操作设计原型/dq-state-regression.cjs','docs/02_交互设计/S04_可操作设计原型/verify-browser.cjs','docs/02_交互设计/S04_可操作设计原型/delegated-walkthrough.cjs'],'repairs':{'DQ-03':'切设备应用/丢弃/留在三分支；应用失败与修订冲突保持原对象和字段','DQ-04':'候选成功读取后先处理未应用与未保存；成功替换清旧历史、运行、包装、物件选择和定义创作上下文；失败或留在当前保持原工程；旧定义确认失效','DQ-05':'冻结用途相关包装输入并核对；普通输入提交保留当前动作flag，再根据输入失配使认可/绑定失效；只允许匹配Pass显式认可；取消认可解除绑定并产生未保存修订；普通表单和undo不能复活已取消认可，纯视角不失效'},'actual_browser_regression':{'DQ_cases':len(dq['results']),'DQ_passed':dq['completed'],'DQ_ref':'docs/02_交互设计/S04_可操作设计原型/output/dq-state/result.json','five_flows_passed':walk['completed'],'flows_ref':'docs/02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/walkthrough-result.json','existing_16_suites_ref':'docs/02_交互设计/S04_可操作设计原型/output/playwright/browser-check-result.json','page_errors':dq['page_errors']},'Figma':{'status':'native_design_written_reactions_connected','state_frames':len(state['frames']),'prototype_url':url,'verification_ref':actual['verification_ref']},'public_gate_owned_by':'复审窗口1；本文件不修改准入、公共质量清单或产品进度','actual_product_runtime':None,'actual_user_walkthrough':None}
independent=read(S/'tools/design-quality-audit/window3/dq05-result.json')
packet['independent_professional_recheck']={k:independent[k] for k in ['status','passed','failed']}
packet['independent_professional_recheck']['ref']='tools/design-quality-audit/window3/dq05-result.json'
packet['coordinator_sync_requests']=[{'file':'AGENTS.md','change':'将现行Figma暂停改为2026-10-15用户直接恢复制作；产品编码门槛保持。'}]
write(A/'特征细化/DQ_S04修复验收.json',packet)
md=f'''# Figma可点击原型交付 · 2026-10-15

[打开可点击原型]({url}) · [打开可编辑设计]({actual['design_url']})

用户直接恢复Figma制作。本轮在既有文件新建“当前 · 超级工厂可点击原型”页，保留旧页面为历史。

66个任务/状态画面：24个主业务画面、九类定义、菜单与独立停靠、主轴/流水线/设备细化、输入预览、设备片段、包装成型/开合、四格式、修订冲突、切对象/切工程三选择、包装用途认可与失配。另有九类原生居中弹窗，菜单为保留当前背景的透明叠加层。

五条入口：从空厂房到交付；设备创作；Agent目标建厂；纸包装专业闭环；跨厂供货与接手。菜单/按钮均使用原生Figma反应，结构回读有{qa['reactions']}条交互、{qa['nativeTexts']}个可编辑文字节点、{qa['instances']}个组件实例，所有24主画面可达，无失效目标。页面、文字、字段和控件可编辑；只有独立3D视口内容为图片，没有把整张界面截图充当原型。

原型采用固定示例与离散状态。Figma不执行仿真、外部Agent、任意文件读取或真实四格式报告导出；这些动作展示已设计的交互路径。浏览器原型的实际输入/下载/重新打开及16组新状态序列另有证据，不与Figma图层连线混算。真人实际走查、产品运行和产品前端编码保持分开。

视觉核对发现并修复：中文单行标签误换行、树目录对齐、时间轴底栏内容穿透，以及SVG外部CSS丢失造成黑色填充。截图在本目录 `*-figma-final.png`。原生连线结构核验见 `figma-structural-check.json`；构建源与节点清单见 `build-figma.cjs`、`wire-figma.cjs` 和 `state.json`。

## 继续编辑

以当前页为入口；旧“历史 · S04初稿 · 未接交互”不用于现行验收。公共类型/API、S07/S08准入、运行结果和正式报告不由本轮改变。旧E001—E070保留为历史需求边，不把其中的自跳页伪填成当前交互；当前连线以真实Figma回读为准。
'''
(D/'README.md').write_text(md,encoding='utf-8')
write(D/'visual-review.json',{'operator':'AI_design_review_not_human_acceptance','main_states_reviewed':24,'screenshots':'23个本地VP截图及VP01原生内联截图','contact_sheets':[f'QA-sheet-{i}.jpg' for i in range(1,7)],'fixed':['单行中文换行与树目录对齐','时间轴滚动内容穿透底栏','SVG外部CSS丢失造成黑色填充'],'actual_product_runtime':None,'actual_user_walkthrough':None})
for name in ['Figma批次执行与原型连线.md','Figma制作依据核对与工作台纠偏.md','工业工作台母版与字段细化.md','连续任务与原型制作依据.md']:
    p=I/name;text=p.read_text(encoding='utf-8');marker='\n## 2026-10-15 Figma恢复与本轮交付\n';text=text.split(marker)[0].rstrip()+marker+f'\n用户已直接恢复Figma制作；旧暂停条款为历史。当前为66状态原生可点击原型，保留工业工作台布局。详见[Figma交付](Figma批次_20261015/README.md)及[可点击入口]({url})。本轮不改变产品编码准入。\n';p.write_text(text,encoding='utf-8')
print(json.dumps({'Figma_states':len(state['frames']),'DQ_cases':len(dq['results']),'flows_passed':walk['completed'],'synced':'owned Figma MD/JSON and S04-04; public gate untouched'},ensure_ascii=False))
