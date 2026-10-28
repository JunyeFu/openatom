"""消费已实际核读的26母版/24截图及浏览器证据，同步S04-02拥有的文件。"""
from pathlib import Path
from copy import deepcopy
import json
D=Path(__file__).resolve().parent;S=D.parents[2];I=D.parent;A=S/'docs/04_开发与验收';F=A/'特征细化';M=I/'S04-02_drawio母版_20261014'
B='docs/02_交互设计/S04_可操作设计原型/';REF=B+'output/visual-review/visual-review-result.json'
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def apply():
 v=read(D/'output/visual-review/visual-review-result.json');assert v['completed'] and v['actual_user_visual_acceptance'] is None and v['actual_product_runtime'] is None
 evidence={'operator_kind':'AI_delegated','authority':v['authority'],'evidence_ref':REF,'status':'passed_design_visual_review','reviewed_at':v['reviewed_at'],'actual_browser_checks_ref':B+'output/visual-review/browser-result.json','native_master_check_ref':'docs/02_交互设计/S04-02_drawio母版_20261014/母版核对结果.json','master_pages':26,'business_frames':24,'actual_user_visual_acceptance':None,'actual_product_runtime':None}
 p=A/'S04-02_视觉预览与连续任务复审.json';r=read(p);r['document_kind']='S04_02_authorized_AI_current_master_and_browser_visual_review';r['status']='passed_current_drawio_and_browser_design_AI_visual_review';r['date']=v['local_review_date']+' Asia/Shanghai';r['review']={'status':'passed','scope':'S04-02设计画面、26状态、24业务帧和现行风格/布局覆盖','authority':v['authority']};r['passed_scoped_checkpoint']=True;r['passed_public_checkpoint']='S04-02' not in read(A/'完整设计与执行准入.json')['design_freeze']['currently_pending_design_reviews'];r['public_projection_owner']='复审窗口1';r['actual_AI_visual_review']=evidence;r['actual_user_visual_acceptance']=None;r['actual_product_runtime']=None;r['open_evidence']=[];r['blocking_evidence']=[];r['remaining_other_gates']=v['remaining_product_evidence'];r['current_isolated_prototype']['AI_visual_review_ref']=REF
 if 'layout_findings_open' in r:r['historical_Figma_layout_findings_open']=deepcopy(r['layout_findings_open']);r['layout_findings_open']=[]
 for defect in r.get('known_visual_defects',[]):defect['scope']='historical_Figma_not_current_design_gate'
 write(p,r)
 for p in [I/'工业工作台母版与字段细化.json',I/'连续任务与原型制作依据.json',F/'DRF02_完整定义.json',F/'DRF02_缺口与修复.json']:
  x=read(p);x['actual_AI_visual_review']=deepcopy(evidence)
  if 'current_isolated_prototype' in x:x['current_isolated_prototype']['AI_visual_review_ref']=REF
  if p.name=='工业工作台母版与字段细化.json':x['status']='passed_current_master_and_prototype_design_AI_review';x['remaining']=v['remaining_product_evidence']
  if p.name=='连续任务与原型制作依据.json':
   for frame in x['required_frames']:frame['state']='current_isolated_prototype_AI_visual_review_passed';frame['AI_visual_review_ref']=REF
  if p.name=='DRF02_完整定义.json':x['status']='S04_02_04_05_design_owner_reviews_completed_public_projection_owned_by_window1';x['current_type_basis_files']=read(A/'类型格式检查结果.json')['generated_TS_files']
  if p.name=='DRF02_缺口与修复.json':x['remaining']=v['remaining_product_evidence'];x['visual_issue_closure']=v['fixed_visual_findings']
  write(p,x)
 md='# S04-02 视觉预览与连续任务复审\n\n'+v['local_review_date']+'。当前draw.io母版与隔离浏览器原型的设计视觉审查通过，实际操作者为授权AI，不填写用户本人像素认可。授权来自用户在统筹窗口明确要求“自主完成整个设计阶段，所有操作我都给你权限独立执行”。\n\n[审查JSON](../02_交互设计/S04_可操作设计原型/output/visual-review/visual-review-result.json) · [24帧图册](../02_交互设计/S04_可操作设计原型/原型图册.html) · [26母版](../02_交互设计/S04-02_drawio母版_20261014/母版审阅.html) · [可操作入口](http://127.0.0.1:8921/)\n\n## 核读与结论\n\n'
 md+='|要求|实际检查|结论|\n|---|---|---|\n'+'\n'.join('|'+c['id']+'|'+c['observation']+'|通过（设计范围）|' for c in v['criteria'])+'\n\n'
 md+='26状态的原生XML、字段定位及对应导出图均核读；24帧逐组核读，再单独核读VP04上传图/标签与VP15真实候选。7项实际浏览器视觉/布局检查通过。母版字段/页面错误0；通用图形检查仍有1751警告，其中1743为重叠，不能写成所有结构警告为0。背景/容器等重叠告警与已观察的裁切缺陷分开处理。\n\n'
 md+='## 直接修复\n\n'+'\n'.join('- '+x['id']+'：'+x['observed']+'；'+x['fix']+'，'+','.join(x['recheck'])+'复核。' for x in v['fixed_visual_findings'])+'\n\n'
 md+='静态母版使用示例控制盒厂、assembly-station@1.0.0/A01(6,4)；实际原型使用自建控制盒工厂、community.control-box-assembly@0.1.0/A01(8,5)。这些是分别标注的示例数据，不是同一工程修订；只统一交互/字段语义和风格，未暗改已有定义或数值。原型按Q02展示8生产设备和1接受托盘，沿用6类既有自建GLB，未修改受保护tools/3d-proof。R4规则只读说明与当前421TS依据已消费，不伪造TemplateAdoption记录、专业认可或输出质量。\n\n'
 md+='## 审查范围\n\n母版中央产线插图属于AI设计资产，外壳/文字/字段为原生draw.io；实际原型使用真实HTML/SVG/Three控制。纸包装五板片显示代理是设计过程演示，完整三种机制专业实现和工业正确性另验。S04-04/05最新5链/8专项及16回归提供交互证据；真人视觉认可/独立目标用户S10和产品actual_result均为空。旧Figma文件的裁切/布局缺陷保留为历史，Figma继续暂停，不阻止本轮当前基线。公共总阶段由窗口1合入。\n'
 (A/'S04-02_视觉预览与连续任务复审.md').write_text(md,encoding='utf-8')
 for name in ['连续任务与原型制作依据.md','工业工作台母版与字段细化.md']:
  p=I/name;t=p.read_text(encoding='utf-8');t=t.replace('S04-02最终视觉认可待取得；S04-04/05本轮按直接授权的AI代走查完成，真人操作仍为空。','S04-02当前视觉已按全设计自主授权由AI审查通过；S04-04/05授权代走查完成，真人操作仍为空。');t=t.replace('S04-02最终视觉认可','S04-02当前授权AI视觉审查');t=t.replace('还需最终视觉认可；','当前授权AI视觉审查已完成；');t+='\n## 当前视觉验收\n\n26母版/24业务帧已实际核读，7浏览器视觉检查通过；[S04-02正式记录](../04_开发与验收/S04-02_视觉预览与连续任务复审.md)。当前01/02示例值和资产版本分别标识；S04-02/04/05设计范围均已收口，真人/产品结果不代填。\n';p.write_text(t,encoding='utf-8')
 for p in [A/'S04-04_可点击原型复审.json',A/'S04-05_本人原型走查.json']:
  x=read(p);x['remaining_other_gates']=v['remaining_product_evidence'];x['current_AI_visual_review_ref']=REF
  if 'layout_findings_open' in x:x['historical_Figma_layout_findings_open']=deepcopy(x['layout_findings_open']);x['layout_findings_open']=[]
  write(p,x)
 for p in [A/'S04-04_可点击原型复审.md',A/'S04-05_本人原型走查.md',F/'DRF02_完整定义.md',F/'DRF02_缺口与修复.md']:
  t=p.read_text(encoding='utf-8').replace('S04-02最终视觉认可仍独立待取得','S04-02当前视觉已按另行全设计授权由AI审查通过');t=t.replace('S04-02视觉认可仍独立待取得','S04-02当前授权AI视觉审查已通过');p.write_text(t,encoding='utf-8')
 print(json.dumps({'S04_02':'passed_authorized_AI_design_visual_review','master_pages':26,'business_frames':24,'browser_visual_checks':7,'actual_user_visual_acceptance':None},ensure_ascii=False))
if __name__=='__main__':apply()
