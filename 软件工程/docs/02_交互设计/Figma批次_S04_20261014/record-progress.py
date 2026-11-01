import json
from pathlib import Path
p=Path(__file__).resolve().parents[1]
docs=p.parent
def read(q):return json.loads(q.read_text(encoding='utf-8-sig'))
def write(q,d):q.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
ledger=read(p/'Figma节点与执行记录.json')
url=ledger['design_url'];frame_ids=ledger['frames']
ledger['planned_interactions_not_connected']=ledger.pop('stateControls',{})
ledger['completed_steps']+=['viewport_assets_uploaded','24_main_frames_created','9_category_frames_created','VP05_screenshot_inspected']
ledger['native_prototype_reactions_connected']=0
ledger['prototype_url']=None
ledger['execution_status']='partial_design_saved_blocked_by_Figma_Starter_MCP_quota'
ledger['known_visual_defects']=[
 {'id':'S04-FIG-01','severity':'P1','observed':'VP05截图中操作行高度为10px，按钮被裁切；状态行亦受同一布局设置影响。','fix_file':'docs/02_交互设计/Figma批次_S04_20261014/resume-layout.js','fix_executed':False},
 {'id':'S04-FIG-02','severity':'P2','observed':'旧24帧左侧九类列表底部被裁切；需启用滚动或压缩行距。','fix_executed':False},
 {'id':'S04-FIG-03','severity':'P1','observed':'部分F01/F03示例值与实际字段标签不符；本地生成器已修正，新九类页采用新值，旧主帧仍待修正。','fix_executed_on_existing_main_frames':False}
]
ledger['uncommitted_attempts']=[
 {'action':'prototype SET_VARIABLE','result':'当前Starter计划拒绝；包含这些反应的画布调用未提交'},
 {'action':'fix layout and interactive fields','result':'MCP tool call limit reached；该修复没有写入Figma'}
]
write(p/'Figma节点与执行记录.json',ledger)
actual=dict(file_key=ledger['file_key'],design_url=url,main_frames=24,category_frames=9,form_groups=28,field_ids=106,
 frame_node_ids=frame_ids,prototype_url=None,reactions_connected=0,actual_design_write=True,
 screenshot_ref='docs/02_交互设计/Figma批次_S04_20261014/VP05.png',
 visual_review_complete=False,clickable_prototype_complete=False)
block=dict(code='FIGMA_STARTER_MCP_TOOL_CALL_LIMIT',
 message="You've reached the Figma MCP tool call limit on the Starter plan.",
 upgrade_url='https://www.figma.com/files/team/1618474521835639526/all-projects?upgrade=mcp_rate_limit_paywall',
 variable_action_rejection='You cannot use Set Variable actions in Reactions with your current plan.',
 browser_fallback=dict(chrome='not available in current control channel',iab='navigation timed out; editor unavailable'),
 next_required_action='恢复该账户/团队的可用MCP额度；继续现有文件，不重复建文件或重新申请设计授权。')
state=read(p/'Figma执行状态.json');state.update(document_kind='actual_Figma_connection_and_partial_design_execution',
 connector_state='connected',execution_state='blocked_by_plan_tool_call_limit',actual_design_write=True,
 actual_prototype_interaction=False,frame_node_ids=frame_ids,actual_Figma=actual,blocking_evidence=block,
 user_report='用户确认Figma已授权；whoami/create_new_file/use_figma已成功',
 user_report_kind='current_user_authorization_confirmed_by_connector',next_required_action=block['next_required_action'])
state['attempts'] += [dict(tool='use_figma',isError=False,committed_main_frames=24,committed_category_frames=9),
 dict(tool='use_figma prototype reactions',isError=True,message=block['variable_action_rejection'],canvas_committed=False),
 dict(tool='use_figma layout repair',isError=True,message=block['message'],canvas_committed=False)]
write(p/'Figma执行状态.json',state)
plan=read(p/'Figma批次执行与原型连线.json');plan['document_kind']='Figma_authorized_batch_partial_actual_design_unconnected_prototype'
plan['actual_Figma']=actual;plan['actual_execution_status']='blocked_by_Figma_Starter_MCP_tool_call_limit'
plan['style']['font_source']='实际listAvailableFontsAsync确认Noto Sans SC Regular/Bold，已采用本地中文工程组件。'
plan['component_discovery'].update(existing_Figma_file=ledger['file_key'],existing_Figma_components=ledger['components'],library_search='completed_Simple_Design_System_scoped_search_and_Button_Input_import',variables_and_styles='local_semantic_color_aliases_spacing_and_prototype_string_variables_created')
plan['actual_library_imports']=dict(button_set='4:56',input_set='4:142',design_system='Simple Design System',local_component_reason='工程界面中文字体/灰白蓝青样式采用本地可编辑组件；已发现的社区库保留为基础参考。')
for f in plan['frames']:f.update(frame_node_id=frame_ids[f['id']],figma_created=True,actual_visual_complete=False)
for e in plan['prototype_edges']:e.update(figma_control_node_id=ledger['controls'].get(e['id']),actual_connected=False)
plan['plan_compatibility']=dict(set_variable_reactions_supported=False,basis='actual plan rejection',replacement='native frame/state navigation with isolated demo contexts',replacement_executed=False)
plan['known_visual_defects']=ledger['known_visual_defects'];plan['blocking_evidence']=block
write(p/'Figma批次执行与原型连线.json',plan)
brief=read(p/'连续任务与原型制作依据.json');brief['actual_Figma_execution_ref']='docs/02_交互设计/Figma节点与执行记录.json';brief['actual_current_design']=actual;write(p/'连续任务与原型制作依据.json',brief)
for name in ['S04-02_视觉预览与连续任务复审','S04-04_可点击原型复审']:
 q=docs/'04_开发与验收'/f'{name}.json';d=read(q)
 d['status']='partial_design_created_blocked_by_Figma_Starter_MCP_quota'
 d['actual_Figma']=actual;d['blocking_evidence']=block;d['known_visual_defects']=ledger['known_visual_defects']
 d['actual_prototype_interaction']=False;d['passed_public_checkpoint']=False
 if name.startswith('S04-02'):
  d['document_kind']='S04_02_partial_native_Figma_design_actual_screenshot_review'
  d['actual_new_visual_or_Figma']=actual
  d['open_evidence']=['修复实际截图发现的操作行裁切/库列表裁切/旧主帧字段示例值错配','补齐纸包装可编辑几何与制造层、完整设备多视角/动作及上下文隔离','完成真实原生连线、视觉复审及本人走查']
 else:
  d['shared_merge_requests'][0]['request']='合入连接成功和24主帧/九类页实际创建证据；当前Starter MCP额度耗尽、连线未建、视觉缺陷仍开放，S04-02/04/05未通过。'
  d['english_handoff']=dict(summary='Figma authorization succeeded. A real editable design file was created; the requested clickable prototype is incomplete.',
   actual_evidence='File QBtchORv1wTdJI3ldYxexm contains 24 main frames and 9 category pages using native editable layers and components. Eight discrete 3D viewport assets were uploaded. 28 form groups / 106 field IDs have returned node records. VP05 was rendered and inspected.',
   blocker='Starter rejects SET_VARIABLE prototype reactions; subsequent calls hit the account/team MCP tool-call limit. Chrome is unavailable to the current browser-control channel, and the in-app editor navigation timed out.',
   defects='The screenshot exposed 10px action-row clipping. The old main frames also need scrollable category lists and corrected F01/F03 sample values. resume-layout.js is prepared but NOT executed.',
   verification='Creation return records and the actual VP05 screenshot confirm partial design only. Zero prototype reactions were committed; no clickable prototype or human-walkthrough result is claimed.',
   remaining='Restore available Figma MCP quota, repair the existing nodes, finish native state/navigation and isolate four demo contexts, then verify all five flows and provide a real prototype link for owner S04-05 operation.',
   shared_state='Public maturity/gates/tasks, product code, dependencies and tools/3d-proof were not changed. Keep S04-02/04/05 unpassed.')
 write(q,d)
q=docs/'04_开发与验收/S04-05_本人原型走查.json';d=read(q);d['design_url']=url;d['prototype_url']=None;d['status']='pending_completed_clickable_prototype_and_human_operation'
for f in d['flow_records']:f.update(prototype_file_key=ledger['file_key'],design_node_ids={x:frame_ids[x] for x in f['frame_ids']})
write(q,d)
print('Updated owned execution sources and S04-02/04/05 reports; public pass fields remain false.')
