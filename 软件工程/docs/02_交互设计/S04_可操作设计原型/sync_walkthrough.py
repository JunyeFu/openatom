"""按真实记录同步用户授权的AI代走查；公共准入与真人操作不代填。"""
from pathlib import Path
from copy import deepcopy
import json

D=Path(__file__).resolve().parent
S=D.parents[2]; I=D.parent; A=S/'docs/04_开发与验收'; F=A/'特征细化'
BASE='docs/02_交互设计/S04_可操作设计原型/'
PROOF=BASE+'output/delegated-walkthrough/walkthrough-result.json'

def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

def apply():
    w=read(D/'output/delegated-walkthrough/walkthrough-result.json')
    assert w['completed'] and len(w['flows'])==5 and len(w['failure_checks'])==8 and not w['page_errors']
    assert w['operator_kind']=='AI_delegated' and w['actual_user_walkthrough'] is None and w['actual_product_runtime'] is None
    authority=w['authorization']
    gate=read(A/'完整设计与执行准入.json')['design_freeze']
    acceptance={'status':'passed_user_authorized_AI_design_walkthrough','operator_kind':w['operator_kind'],'operator':w['operator'],'authority':deepcopy(authority),'started_at':w['started_at'],'ended_at':w['ended_at'],'timestamp_timezone':'UTC; local display Asia/Shanghai','entry_url':w['entry_url'],'evidence_ref':PROOF,'continuous_chains':5,'failure_check_groups':8,'actual_browser_interaction':True,'page_errors':[],'actual_user_walkthrough':None,'actual_product_runtime':None,'target_user_trial':None,'scope':'S04设计原型和授权代走查，不是S08产品运行或S10独立目标用户试用'}
    timeline=[
      ('DTC01',['DX06','DX02'],'behavioral_analogue','真实输入标记屏幕坐标反算10s；主/子轴采用绝对时间，未把固定1200px数值当当前屏幕宽度'),
      ('DTC02',['DX02','BP06'],'source_fixture','J-01原90–180s，裁至100–160仍保身份和双续接标记'),
      ('DTC03',['DX07'],'source_fixture','窗100–200s，已知至150；只画100–150，150–200未知，170定位拒绝'),
      ('DTC04',['DX02','BP06'],'behavioral_analogue','子轴主动选150s，回主轴仍150，不退回进入时刻'),
      ('DTC05',['DX06','BP05'],'source_behavior','真实拖动发pointercancel后pointerup不提交；Esc也取消'),
      ('DTC06',['DX02','DX08'],'source_behavior','时间轴/描述独立隐藏/重开，不改变结果与草稿'),
      ('DTC07',['DX02','BP04'],'source_behavior','描述隐藏时选ARR02不弹回；选中状态保留'),
      ('DTC08',['DX08','BP04'],'behavioral_analogue','当前停靠布局重开保尺寸/同结果/滚动96px；未宣称固定180/360px及所有筛选器均已验证'),
      ('DTC09',['DX07','BP07'],'source_behavior','切L03延迟例拒绝旧定位回复，清旧事件/预览'),
      ('DTC10',['DX02','BP16'],'FC15_equivalent_point_fixture','FC15的110s观察末点可见、可选，只计独立点，没有造正时长'),
      ('DTC11',['DX07','BP06'],'source_fixture_and_followup','20→25s仍computed_boundary且同身份一段；随后35s阶段完成例仍不是整窗完成')]
    issues=[
      {'id':'DW01','severity':'P1','observed':'X=8.6未应用时旋转会清掉表单并回到X=8','evidence_kind':'browser_reproduction','before_ref':BASE+'output/delegated-walkthrough/initial-probes.json#/rotation','fix':'破坏性编辑操作先处理未应用字段，应用/丢弃/留在当前分开','source_ref':BASE+'workbench.js','recheck':['VF-02','BP03'],'status':'fixed_rechecked'},
      {'id':'DW02','severity':'P1','observed':'连续两次复制产生28设备但仅21唯一ID','evidence_kind':'browser_reproduction','before_ref':BASE+'output/delegated-walkthrough/initial-probes.json#/copy','fix':'每次生成新COPY前缀，锁定复制预览修订后整体提交','source_ref':BASE+'workbench.js','recheck':['DX04'],'status':'fixed_rechecked'},
      {'id':'DW03','severity':'P1','observed':'A01工时改45s后切R01亦显示45s；配置/外观原来为工程全局','evidence_kind':'browser_reproduction_and_source_review','before_ref':BASE+'output/delegated-walkthrough/walkthrough-first-attempt.json#/flows/1','fix':'instanceState按稳定设备ID持有配置与外观；复制/采用/保存重开保归属，切设备停止外观循环','source_ref':BASE+'workbench.js','recheck':['VF-02','BP10'],'status':'fixed_rechecked'},
      {'id':'DW04','severity':'P1','observed':'pointercancel后pointerup仍把ARR01从10s提交为13.382s','evidence_kind':'browser_reproduction','before_ref':BASE+'output/delegated-walkthrough/timeline-probes.json#/pointercancel','fix':'pointercancel清除拖动与预览，不提交草稿','source_ref':BASE+'workbench.js','recheck':['DX06'],'status':'fixed_rechecked'},
      {'id':'DW05','severity':'P2','observed':'停靠重开把实际滚动96px重置为0；隐藏时的零值会覆盖已存布局','evidence_kind':'browser_reproduction','fix':'按样例/来源/轴保存滚动，只消费可见轨道滚动事件','source_ref':BASE+'workbench.js','recheck':['DX08'],'status':'fixed_rechecked'},
      {'id':'DW06','severity':'P2','observed':'原组头不提示等待/并行；P01过滤packaging而资料使用packing，导致包装轨漏画','evidence_kind':'source_review_and_visual_observation','before_ref':BASE+'output/delegated-walkthrough/DW06-before-group.png','fix':'组头显示手工工作/并行峰值/接收等待摘要；展开显示6包装段与370–420等待','source_ref':BASE+'workbench.js','recheck':['DX08'],'status':'fixed_rechecked'},
      {'id':'DW07','severity':'P2','observed':'1366窗口文字150%时固定24px时间轴栏把多行按钮裁切','evidence_kind':'visual_observation','fix':'放大文字时头/脚栏增加行高，按钮不换行，允许横向滚动和键盘访问','source_ref':BASE+'workbench.css','recheck':['DX01'],'status':'fixed_rechecked'},
      {'id':'DW08','severity':'P2','observed':'SVG折线path默认填黑遮住基板','evidence_kind':'visual_observation','before_ref':BASE+'output/delegated-walkthrough/DW08-before-fold.png','fix':'折线fill:none，明确虚线描边','source_ref':BASE+'workbench.css','recheck':['VF-04','BP13'],'status':'fixed_rechecked'},
    ]
    remaining=['S04-02最终视觉认可仍独立待取得；本轮04/05授权不据此填写整项视觉通过','实际Engine/Host/外部Agent/Windows接点、正式工程打包、工业正确性/性能仍待各对应阶段','纸包装五板片UI显示代理不替代三种机制专业模型、提供者和连续无干涉验证','S10独立目标用户试用未执行，真人actual_user_walkthrough为空']
    fixdoc={'document_kind':'S04_delegated_walkthrough_issue_closure','authority':authority,'actual_operator_kind':'AI_delegated','issues':issues,'open_S04_04_05_issues':[],'timeline_coverage':[{ 'id':i,'checks':cs,'coverage_kind':k,'observed':t,'actual_product_result':None} for i,cs,k,t in timeline],'failure_checks':deepcopy(w['failure_checks']),'walkthrough_script_setup_corrections':['首次脚本使用不存在的盒长标签，改为实际长度；这是定位器错误，不记产品缺陷','首次从实际轨查输入事件按钮，改为先显式切输入轨；不把该超时报产品错误'],'remaining_other_gates':remaining,'walkthrough_ref':PROOF}
    write(D/'output/delegated-walkthrough/issue-closure.json',fixdoc)
    for name in ['S04-04_可点击原型复审.json','S04-05_本人原型走查.json']:
        p=A/name;v=read(p);v['status']='passed_design_prototype_user_authorized_AI_walkthrough' if name.startswith('S04-04') else 'passed_user_authorized_AI_design_walkthrough';v['scope']='S04设计关口，含真实浏览器交互及用户授权AI代走查';v['review']={'status':'passed','scope':v['scope'],'authority':authority};v['passed_scoped_checkpoint']=True;v['public_projection_owner']='复审窗口1';v['passed_public_checkpoint']=v['checkpoint'] not in gate['currently_pending_design_reviews'];v['public_projection_status']='coordinator_evidence_merge' if v['passed_public_checkpoint'] else 'awaiting_owner_merge_not_written_by_this_window';v['public_projection_source']='docs/04_开发与验收/完整设计与执行准入.json#/design_freeze';v['credit_change']=False;v['global_stage_credit_edited_by_this_window']=False;v['actual_user_walkthrough']=None;v['actual_product_runtime']=None;v['actual_delegated_walkthrough']=deepcopy(acceptance);v['issue_closure_ref']=BASE+'output/delegated-walkthrough/issue-closure.json';v['timeline_new_requirement_verified']=True;v['timeline_verification_scope']='当前列举的有限设计样例/浏览器行为，逐DTC覆盖类型见issue-closure，产品结果为空';v['remaining_other_gates']=remaining;v['blocking_evidence']=[]
        v['current_isolated_prototype']['delegated_walkthrough_ref']=PROOF
        if name.startswith('S04-04'):
            v['actual_prototype_interaction']=deepcopy(v['current_isolated_prototype']);v['current_execution_entry']=w['entry_url']
            if 'historical_Figma_handoff' not in v:v['historical_Figma_handoff']=deepcopy(v.get('english_handoff'))
            if 'historical_shared_merge_requests' not in v:v['historical_shared_merge_requests']=deepcopy(v.get('shared_merge_requests',[]))
            v['english_handoff']={'summary':'S04-04 clickable design prototype passed; S04-05 user-authorized AI walkthrough passed.','evidence':'Five fresh continuous flows and eight dedicated failure/timeline checks, real Chrome controls, fixes and rechecks; sixteen regression suites and 24 frame screenshots.','scope':'Design only. The actual operator is an AI delegate, not the human owner. Product runtime and independent target-user trial remain null.','remaining':'S04-02 final visual acceptance remains separate; window 1 owns public admission projections.'};v['shared_merge_requests']=[{'id':'S04-M04','owner':'复审窗口1','authority':authority,'request':'按本轮直接授权合入S04-04与S04-05设计通过；actual_user_walkthrough保持null，使用actual_delegated_walkthrough和issue-closure；S04-02不自动通过，不放行S06-05或编码','evidence_ref':PROOF}]
        else:
            v['document_kind']='S04_05_user_authorized_AI_actual_design_walkthrough';v['operator_required']='human_owner_or_explicitly_authorized_AI_delegate';v['operator_policy_amendment']={'previous':'human_project_owner_only','new':'本轮允许AI代表用户完成S04-04/S04-05真实原型走查；记录AI身份','authority':authority};v['agent_clicks_do_not_count_as_human_walkthrough']=True;v['issue_closure_rule']='本轮由用户明确授权的AI操作真实浏览器，记录动作/观察/时间，修复拥有的源并以浏览器复核；不填写真人操作或S10独立用户证据。';v['failure_checks']=deepcopy(w['failure_checks']);v['timeline_coverage']=deepcopy(fixdoc['timeline_coverage']);v['issue_records']=issues
            records={r['id']:r for r in w['flows']}
            for r in v['flow_records']:
                done=records[r['id']];r.update({k:deepcopy(done[k]) for k in ['operator_kind','started_at','ended_at','steps','actual_observations','status']});r['operator']=w['operator'];r['authorization']=authority;r['issues']=[x for x in issues if r['id'] in x['recheck']];r['recheck']={'operator_kind':'AI_delegated','result':'passed','evidence_ref':PROOF+'#/flows/'+str(w['flows'].index(done))};r['human_recheck']=None;r['evidence_ref']=PROOF;r['actual_browser_log']=deepcopy(done['browser_action_log'])
        write(p,v)
    for p in [I/'连续任务与原型制作依据.json',I/'工业工作台母版与字段细化.json',A/'S04-02_视觉预览与连续任务复审.json',F/'DRF02_完整定义.json',F/'DRF02_缺口与修复.json']:
        v=read(p);v['actual_delegated_walkthrough']=deepcopy(acceptance)
        if 'current_isolated_prototype' not in v:v['current_isolated_prototype']=deepcopy(read(A/'S04-04_可点击原型复审.json')['current_isolated_prototype'])
        v['current_isolated_prototype']['delegated_walkthrough_ref']=PROOF
        if p.name=='工业工作台母版与字段细化.json':v['remaining']=remaining
        if p.name=='DRF02_完整定义.json':
            v['status']='S04_04_05_passed_user_authorized_AI_design_walkthrough_public_projection_pending';v['scope']='S04-02视觉与原型消费，S04-04/05授权AI代走查完成；公共合入归窗口1';v['six_views']['deployment_and_verification']='真实Chrome：新增5完整链/8专项、修复复核，16回归/24帧；操作者AI，真人/产品结果空';v['verification']['delegated_walkthrough']=deepcopy(acceptance)
        if p.name=='DRF02_缺口与修复.json':v['fixed']=[x for x in v['fixed'] if not x['id'].startswith('DW')]+[{'id':x['id'],'result':x['fix'],'source_ref':x['source_ref'],'recheck':x['recheck'],'status':x['status']} for x in issues];v['remaining']=remaining;v['issue_closure_ref']=BASE+'output/delegated-walkthrough/issue-closure.json'
        write(p,v)
    link='[完整动作与观察记录](../02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/walkthrough-result.json) · [问题与复核](../02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/issue-closure.json) · [打开原型](http://127.0.0.1:8921/)'
    for cp,title in [('S04-04','可点击原型复审'),('S04-05','本人原型走查')]:
        text=f'# {cp} {title}\n\n2026-10-14。本项设计关口通过：用户明确要求“代替我完成S04-04/S04-05”，实际操作者为Codex AI（复审窗口2）。本轮按授权代走查记录，未填写真人操作。\n\n'+link+'\n\n## 实际操作与结论\n\n'
        text+='|链路|实走任务|观察与判定|\n|---|---|---|\n'+'\n'.join('|'+r['id']+'|'+r['title']+'|'+str(len(r['steps']))+'步实际浏览器操作，通过；详细观察、截图和UTC时间见记录|' for r in w['flows'])+'\n\n'
        text+='此外独立走查8组：文字150%/键盘、时间轴显隐/裁窗/末刻、取消竞态/阶段占用、复制/采用冲突、九类自定义、pointercancel/冻结历史、未知区/续算/迟到回复、折叠摘要/包装轨/布局保留。浏览器页面错误为0，修复后16组原有回归通过并重导24帧。\n\n'
        text+='## 问题闭环\n\n|问题|实际缺陷|修复与复核|\n|---|---|---|\n'+'\n'.join('|'+x['id']+'|'+x['observed']+'|'+x['fix']+'；'+','.join(x['recheck'])+'通过|' for x in issues)+'\n\n'
        text+='DTC01—11逐项标明原资料样例、等价行为或FC15替代点例，不将不同屏幕尺寸/例值说成原数值试验。DTC03展示150s后未知，DTC11续至25s仍是前缀。五链不是只翻24帧；输入、采用、取消、冲突、返回、导出和文件重开均实际操作。\n\n'
        text+='## 本项完成范围\n\n原型位于docs隔离目录，不改产品API、不执行Engine/Host/外部Agent。业务数值为手工/类型样例。纸包装是五板片显示代理，不替代完整专业模型。S04-02最终视觉认可仍独立待取得；S10目标用户试用未发生。公共准入与成熟度由复审窗口1合入，不把本项通过扩大为S06-05冻结或产品完成。Figma保持暂停。\n\n'
        text+='生成源sync_review.py调用sync_walkthrough.py消费这份真实记录，后续刷新不会把已完成的授权代走查清回空表。原Figma未通过成果仍在JSON作为历史；当前操作入口为本地原型。\n'
        (A/f'{cp}_{title}.md').write_text(text,encoding='utf-8')
    for name in ['DRF02_完整定义.md','DRF02_缺口与修复.md']:
        text='# DRF02 '+('S04原型与设计消费' if '完整定义' in name else '修复与剩余')+'\n\n2026-10-14，复审窗口2。S04-04/S04-05按用户直接授权的AI代走查完成；S04-02视觉认可仍独立待取得。\n\n[完整记录](../../02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/walkthrough-result.json) · [问题闭环](../../02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/issue-closure.json)\n\n26页draw.io母版、24浏览器状态、5新增完整链、8专项检查、16回归均有实际证据。操作者AI，真人/产品结果空。公共派生TS/36API及DRF03—06设计仅消费，17SF/21P/18操作不变，未写公共准入或产品代码。\n\n'+ '\n'.join('- '+x['id']+'：'+x['fix']+'；'+','.join(x['recheck'])+'复核通过。' for x in issues)+'\n\n其他关口仍待：\n\n'+'\n'.join('- '+x for x in remaining)+'\n'
        (F/name).write_text(text,encoding='utf-8')
    (D/'授权代走查报告.md').write_text('# S04-04 / S04-05 授权代走查报告\n\n'+(A/'S04-05_本人原型走查.md').read_text(encoding='utf-8').split('\n\n',1)[1].replace('../02_交互设计/S04_可操作设计原型/','')+'\n',encoding='utf-8')
    print(json.dumps({'S04_04':'passed_design','S04_05':'passed_authorized_AI_delegate','operator_kind':'AI_delegated','chains':5,'failure_groups':8,'fixed_rechecked':len(issues),'actual_user_walkthrough':None,'actual_product_runtime':None,'public_projection_owner':'复审窗口1'},ensure_ascii=False))

if __name__=='__main__':apply()
