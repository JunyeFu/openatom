"""把本轮实际图像核读和专项操作结论整理为已有S04-02验收源。"""
from pathlib import Path
from datetime import datetime,timezone,timedelta
import json
D=Path(__file__).resolve().parent;M=D.parent/'S04-02_drawio母版_20261014';O=D/'output/visual-review'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
check=read(O/'browser-result.json');master=read(M/'母版核对结果.json');frames=read(D/'output/playwright/frames.json');mapping=read(M/'母版字段与页面映射.json');w=read(D/'output/delegated-walkthrough/walkthrough-result.json');regression=read(D/'output/playwright/browser-check-result.json')
assert check['completed'] and len(check['checks'])==7 and not check['page_errors']
assert len(frames)==24 and master['pages']==26 and not master['errors']
assert w['completed'] and regression['passed']==16 and regression['failed']==0
now=datetime.now(timezone.utc);local=now.astimezone(timezone(timedelta(hours=8)))
observations=[
 '空工程/模板分开，场地输入与进入按钮在中央任务区。','订单和两行到料明确输入，BOM/身份表与原草稿分开。','实例配置、唯一行为、端口、角色和版本分组可读。','实际上传图、排序片段及配置热点可见，图片在预览容器内。','实际八生产设备+一接受资源原生3D、返工支路和布局字段可见。','同一设备近景，仍有上下栏与只编辑选中实例的属性。','简化外壳保留方向/位置/稳定ID，没有自动切详情。','聚合只包含八生产成员，FG01接受资源在聚合外保留。','多厂房有两种物品供货关系，日目标不标实际到货。','输入联动预览/提交/取消分开，输入轴明确显示。','样例身份/输入修订/已知范围和计算/回看控制分开。','接收等待持有者/已声明窗口/未知解除时间明确。','包装件保留PCB/结构件/装配件组成与追踪时点。','目标/窗口/场地/日历和H04专属字段入口可见。','实际提交目标后形成候选：8设备、FG01新增接受资源、OP01复用人员分行。','初案成功一例零修正直接交付，不强制比较。','不足/供料上界/三轮预算/停止原因明确，允许范围列出。','纸包装尺寸/机制/制造身份为图示代理，未声称专业实测。','角度改变五板片姿态，成型/开合/取物步骤与三态分支可见。','制造SVG/认可/版本/绑定条件清楚，折线不再填黑。','A/B手工输入差异及未知成本明示，选择只读。','四格式按钮及逐样例报告/边界/成本缺项在可滚动任务区。','快照资产来源/版本/结果来源与正式工程区别清楚。','冲突/撤销/保存失败/未保存离开入口及当前草稿分开。'
]
criteria=[
 ('VC01','中文浅灰工业桌面，共同顶部菜单/工程归属和底部选择、移动等工具常驻；24帧及26状态保持同一外壳。'),
 ('VC02','两常用画布、三档只改变呈现；单击详情/区域成员入口和滚轮不入详情，由BP02/V01核验。'),
 ('VC03','左右面板可收起及调宽，中央获得空间；V02实拖左栏至306px并收起两侧。'),
 ('VC04','默认窄dock/左描述右时间轴，窗口菜单独立显隐；V03及DX08验证回扩、同样例和96px滚动。'),
 ('VC05','字段按身份、配置、端口、资源、表现、结果归属分组；编辑区唯一，右侧任务上下文只读。'),
 ('VC06','未应用/已应用草稿/保存、输入/结果、计算/回看/表现分开；显示来源及未知，不冒充产品计算。'),
 ('VC07','Q02八生产设备叙事补齐；FG01作为接受资源，候选与成员列表不再把资源算成设备。'),
 ('VC08','H04的32字段/6状态和R4的9义务规则分别可展开，明确“未取得高度/专业认可/种子输入”不是已完成。'),
 ('VC09','24帧覆盖5连续链及取消、冲突、缺项、三态、有限停止、比较与接手；26母版覆盖布局/菜单/裁窗/多阶段/放大等状态。'),
 ('VC10','1366窗口文字150%下必要输入和键盘操作可达；纸包装黑填色和上传图越界已修正；有限截图/操作审查不宣称任意分辨率全像素证明。')]
fixes=[
 {'id':'SV01','observed':'原7个视觉实例只有6生产设备+接受托盘，未覆盖Q02八设备叙事','fix':'增加CV02装箱前输送和PK01人工装箱，8生产成员+1接受资源；原6类自建GLB只读复用','recheck':['V01','VF-01','BP01']},
 {'id':'SV02','observed':'旧VP15截图是无预览状态，未展示实际候选','fix':'通过VP14真实目标预览取得VP15，导出脚本保留这个前置动作','recheck':['V07']},
 {'id':'SV03','observed':'VP04真实上传图越过260px容器，遮住上传按钮','fix':'固定图像尺寸/object-fit:contain，容器防溢出；图像与按钮不再重叠','recheck':['V06','VF-02','BP10']},
 {'id':'SV04','observed':'上传/片段/标签改变数据后，顶栏与右侧修订仍显示r1','fix':'这些已观察的保存数据编辑使用同一次界面刷新，显示真实当前修订及未保存状态','recheck':['V06','VF-02']},
 {'id':'SV05','observed':'新增设备候选行包含已声明为接受资源的FG01','fix':'新增设备仅8个；FG01放新增接受资源，OP01另列复用人员资源','recheck':['V07']}]
result={'document_kind':'S04_02_current_design_visual_review','completed':True,'operator':'Codex / 复审窗口2','operator_kind':'AI_delegated','reviewed_at':now.isoformat(),'local_review_date':local.strftime('%Y-%m-%d'),'authority':{'source_thread_id':'01a1353e-da58-7671-837b-ee746a8bd9b4','user_message_id':'01a13b06-8349-7340-8f32-a6a2cb7bfb1a','quote':'自主完成整个设计阶段，所有操作我都给你权限独立执行','verification':'本窗口实际read_thread读取到该userMessage；统筹续派S04-02自主视觉审查','scope':'设计视觉与状态覆盖的AI代理审查，Figma仍暂停，不授权产品开发或代填真人认可'},'actual_user_visual_acceptance':None,'actual_product_runtime':None,'method':['实际核读6张当前24帧联系表，原图复核VP04/VP15及关键过程截图','实际核读7张26母版联系表和原生XML/页面映射；使用同一规范检查','真实Chrome运行V01—V07布局/类型区分/独立详情面板/上传图和候选操作','最新5链/8专项与16回归作为交互依据；不以图片代替操作'],'criteria':[{'id':i,'observation':t,'result':'passed_design_scope'} for i,t in criteria],'business_frame_reviews':[{'frame':r['frame'],'image_ref':'docs/02_交互设计/S04_可操作设计原型/'+r['file'],'observation':observations[n],'result':'passed_design_scope'} for n,r in enumerate(frames)],'master_page_reviews':[{'page':p['title'],'image_ref':'docs/02_交互设计/S04-02_drawio母版_20261014/'+e['file'],'observation':'共同工业外壳与此状态的布局/字段/菜单/反馈已核读；示例与实际原型分别标识','result':'passed_design_scope'} for p,e in zip(mapping['pages'],master['exports'])],'fixed_visual_findings':fixes,'browser_ref':'docs/02_交互设计/S04_可操作设计原型/output/visual-review/browser-result.json','master_structure':{'field_page_errors':master['errors'],'generic_summary':master['structural_summary'],'warning_interpretation':'通用容器/背景重叠告警保留，不写全部零警告；实际裁切/遮盖以图像观察修正'},'current_public_type_basis':421,'latest_walkthrough_ref':'docs/02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/walkthrough-result.json','post_full_walkthrough_targeted_recheck':'末次FG01候选分类修正由V07真实重新预览及分行断言验证；未扩大或虚填重跑范围','remaining_product_evidence':['实际产品Engine/Host/外部Agent、工业正确性/性能及正式工程保存接手由相应实现阶段验证','完整纸包装机制和连续无干涉等专业验证不由五板片UI代理替代','S10独立目标用户试用未执行；真人操作和视觉认可仍空'],'historical_Figma_policy':'33旧帧、0连线、旧裁切/外壳缺陷保留为历史未通过成果；不作当前draw.io/隔离原型阻塞'}
(O/'visual-review-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'business_frames':24,'master_pages':26,'visual_checks':7,'criteria':len(criteria),'result':'passed_design_scope','date':result['local_review_date']},ensure_ascii=False))
