"""同步母版的直接消费者；不更新公共关口信用、产品状态或其他窗口源。"""
from pathlib import Path
import json
import re

D=Path(__file__).resolve().parent
I=D.parent
S=I.parent.parent
A=S/'docs/04_开发与验收'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,obj): p.write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
mapping=read(D/'母版字段与页面映射.json')
check=read(D/'母版核对结果.json')
spec_path=I/'工业工作台母版与字段细化.json'
spec=read(spec_path)
spec['owner']='复审窗口2'
spec['master_revision']='drawio-r3-20261014'
spec['status']='current_requirements_consumed_26_editable_states_full_checkpoint_pending'
spec['authoring']['state_pages']=len(mapping['pages'])
spec['current_state_pages']=mapping['pages']
spec['requirement_refresh']['appearance_binding_ref']='packages/module-kit/device-editor-capabilities.design.json#/timeline'
spec['requirement_refresh']['scope']='当前外壳、三档呈现、阶段表现、修订冲突与文字重排；24业务帧/实际原型/本人走查分别验收'
ref='packages/module-kit/device-editor-capabilities.design.json'
if ref not in spec['source_refs']: spec['source_refs'].append(ref)
findings=[
    ('WM-25','三档按钮没有对应的不同画面','M01详细、M21简化、M22区域可相互跳转；A01选择保留，未编组设备不被聚合。'),
    ('WM-26','视图菜单与窗口菜单职责未在画面分开','M23明确相机、LOD、时间比例和文字缩放；显隐回到窗口菜单。'),
    ('WM-27','修订冲突缺少字段保留画面','M24展示r7/r8/未应用值，提供留在当前、放弃本次、重新核对入口；不直接覆盖。'),
    ('WM-28','归一化u容易被解释成多阶段全作业时钟','M20编辑auto阶段的u轴，M25查看全部阶段绑定；cycle_normalized仅适用单阶段正工时，无多阶段全局duration。'),
    ('WM-29','区域档没有成员进入设备详情路径','M22明确LINE01与五个成员，先选区域、再点成员，保持A01身份。'),
    ('WM-30','文字缩放未给出字段可访问示例','M26仅属性文字150%并重排，滚动访问其余字段，底部提交入口保持；不缩放相机。'),
]
spec['resolved_findings']=[f for f in spec['resolved_findings'] if f['id'] not in {x[0] for x in findings}]
spec['resolved_findings'] += [dict(id=k,observed=o,correction=c,scope='原生母版呈现；实际行为仍须原型和产品验证') for k,o,c in findings]
spec['presentation_state_contract']={
    'lod_pages':{'detailed':'master-main','simplified':'master-simplified','region':'master-region'},
    'selected_object_preserved':'A01',
    'region_group':'LINE01',
    'region_members':['KIT','A01','I01','P01','R01'],
    'not_grouped':['B01','B02','FG01'],
    'zoom_does_not_enter_device':True,
    'zoom_does_not_change_canvas':True,
    'view_menu_page':'master-view-menu',
    'window_menu_page':'master-window-menu',
    'appearance_modes':{'cycle_normalized':'one positive known fixed-duration phase only','phase_local':'bound frozen phase effective progress; waiting holds pose'},
    'conflict_page':'master-revision-conflict',
    'text_scale_page':'master-text-scale',
    'text_scale_example':1.5,
    'actual_behaviors_verified':False,
}
spec['verification_ref']='docs/02_交互设计/S04-02_drawio母版_20261014/母版核对结果.json'
write(spec_path,spec)
review_path=A/'S04-02_视觉预览与连续任务复审.json'
review=read(review_path)
review['owner']='复审窗口2'
review['status']='drawio_26_state_master_current_requirements_full_checkpoint_pending'
current=review['current_drawio_master']
current['pages']=len(mapping['pages'])
current['mapped_field_rows']=len(mapping['field_controls'])
current['latest_layout_source']='两画布、三档实际画面；顶部菜单/底部工具；独立160高双dock；多阶段绑定、修订冲突与文字150%重排'
current['current_revision']=spec['master_revision']
current['static_check']={k:check[k] for k in ['pages','native_vertices','field_group_locations','canonical_carrier_checks','page_links','errors','structural_summary']}
current['review_entry_ref']='docs/02_交互设计/S04-02_drawio母版_20261014/母版审阅.html'
write(review_path,review)
brief_path=I/'连续任务与原型制作依据.json'
brief=read(brief_path)
brief['current_master_status']='drawio_native_master_26_states_LOD_phase_binding_conflict_text_scale_not_full_24_business_frame_acceptance'
brief['current_master_revision']=spec['master_revision']
brief['current_master_state_pages_ref']='docs/02_交互设计/工业工作台母版与字段细化.json#/current_state_pages'
write(brief_path,brief)

upgrade='''
## 当前母版升级：26个状态页

重新以现行导航、28组表单、时间轴完整定义和设备能力合同校准。原20页保留并同步更新；新增M21简化、M22区域及成员入口、M23视图菜单、M24修订冲突、M25多阶段表现绑定、M26文字150%重排。顶部菜单、底部工具和默认160高的双dock保持；描述与时轴独立显隐。

三档仅改变呈现。详细/简化/区域分别有画面；A01仍为同一选择，区域只聚合KIT/A01/I01/P01/R01，B01/B02/FG01保留独立。缩放不进入设备详情、不切多厂房；详情由成员入口进入。简化和区域使用draw.io原生轮廓示意，不作为真实3D或产能证据。

M20编辑auto阶段的u轴，M25查看全部阶段绑定；cycle_normalized仅用于单阶段正工时，多阶段按phase_id及阶段有效进度，没有全局工时。等待保持姿态，零时长取完成事件，缺历史显示未知。外观、注释和标签不创造库存或执行历史。M24的冲突三列保留基础r7、当前r8和未应用值，重新核对后才可提交；背景也保留当前r8与基础r7，不静默覆盖。M26的文字放大重排属性内容，剩余字段用滚动访问，不放大相机或把提交入口挤出工作台。

[26页审阅入口](S04-02_drawio母版_20261014/母版审阅.html) · [可编辑源](S04-02_drawio母版_20261014/industrial-workbench-master.drawio) · [核对结果](S04-02_drawio母版_20261014/母版核对结果.json)。本轮是母版升级，26个状态页不等同24个业务帧的完成；S04-04交互和S04-05走查单独消费授权代走查记录；S04-02最终视觉与产品运行分别验收。
'''
md_path=I/'工业工作台母版与字段细化.md'
md=md_path.read_text(encoding='utf-8')
md=md.replace('复审窗口2/W00','复审窗口2').replace('当前含20个状态页','当前含26个状态页')
md=md.replace('20页是设计状态例','26页是设计状态例')
md=re.sub(r'\n## 当前母版升级：26个状态页\n[\s\S]*$','',md)
md_path.write_text(md.rstrip()+'\n'+upgrade,encoding='utf-8')

review_md=A/'S04-02_视觉预览与连续任务复审.md'
md=review_md.read_text(encoding='utf-8').replace('复审窗口2/W00','复审窗口2')
md=md.replace('24项母版缺陷处理落实为20个原生可编辑状态页和208个字段行映射','30项母版呈现缺陷处理落实为26个原生可编辑状态页和'+str(len(mapping['field_controls']))+'个字段行映射')
md=md.replace('当前20页含关闭/恢复菜单状态','当前26页含关闭/恢复菜单及三档/阶段/冲突状态')
md=re.sub(r'本轮母版检查：20页[^\n]*',f"当前母版检查：{check['pages']}页、{check['native_vertices']}原生元素、{check['field_group_locations']}处字段组定位、{check['canonical_carrier_checks']}次载体核对、{check['page_links']}页链接，字段及页面错误{len(check['errors'])}。通用图形检查保留重叠告警，详见机器结果；不作为实际产品或本人操作证据。",md)
md=md.replace('3. 可编辑Figma组件/文字/层级重建；不能用整张AI图导入冒充UI。S04-04连线及S04-05本人操作分别验收。','3. draw.io作为母版源；Figma当前制作状态取Figma执行状态.json。文字/字段须原生可编辑，不能用整张AI图冒充UI。S04-04交互及S04-05走查分别验收。')
md+='\n## 本轮直接更新\n\n26页、三档显示的选择身份、多阶段表现、冲突字段保留和文字放大画面已更新到母版、规范JSON和24帧制作依据；[母版审阅入口](../02_交互设计/S04-02_drawio母版_20261014/母版审阅.html)。真实产品/真人操作为空；S04-04/05消费授权代走查报告，公共投影归统筹合并。\n'
review_md.write_text(md,encoding='utf-8')

gallery_data=[dict(**p,file=e['file']) for p,e in zip(mapping['pages'],check['exports'])]
gallery='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>超级工厂 · 母版审阅 r3</title><style>
*{box-sizing:border-box}body{margin:0;background:#e9eff4;color:#172e48;font:15px/1.5 "Microsoft YaHei",sans-serif}header{height:100px;background:#f8fafc;border-bottom:1px solid #c7d4e2;padding:12px 24px}h1{font-size:21px;margin:0 0 6px}p{margin:0;color:#64748b}a{color:#146bc2}main{display:grid;grid-template-columns:272px 1fr;height:calc(100vh - 100px)}nav{overflow:auto;padding:10px;background:white;border-right:1px solid #c7d4e2}button{font:inherit;cursor:pointer;background:white;border:1px solid #c7d4e2;color:#172e48;padding:7px 12px;border-radius:3px}nav button{display:block;width:100%;text-align:left;margin-bottom:5px;font-size:13px}button[aria-pressed=true]{background:#146bc2;color:white}button:focus-visible,a:focus-visible{outline:3px solid #008c99;outline-offset:2px}section{min-width:0;display:flex;flex-direction:column}.controls{display:flex;gap:10px;align-items:center;padding:10px 16px;background:#f8fafc;border-bottom:1px solid #c7d4e2;flex-wrap:wrap}#status{flex:1;min-width:260px}figure{margin:0;padding:16px;overflow:auto;flex:1}img{display:block;width:100%;height:auto;background:white;box-shadow:0 2px 14px #172e4818}figure.original img{width:auto;max-width:none}footer{padding:8px 16px;background:#f8fafc;border-top:1px solid #c7d4e2;color:#64748b;font-size:13px}@media(max-width:850px){main{grid-template-columns:190px 1fr}header{height:auto;min-height:100px}nav{padding:5px}figure{padding:5px}}
</style><header><h1>超级工厂 · 工业工作台母版审阅</h1><p>draw.io r3 · 26个设计状态 · <a href="industrial-workbench-master.drawio">打开可编辑源</a> · <a href="README.md">画面说明</a> · 图稿审阅目录，实际业务交互另验。</p></header><main><nav aria-label="母版状态列表"></nav><section><div class="controls"><button id="previous">上一页</button><button id="next">下一页</button><span id="status" role="status" aria-live="polite"></span><button id="size" aria-pressed="false">原尺寸查看</button><a id="original" target="_blank" rel="noopener">打开原图</a></div><figure><img id="image" alt=""></figure><footer>顶部菜单、底部工具；时间轴与描述独立显隐。图中数量/时间为设计样例。场景插图不作为产品运行、3D模型或产能证明。</footer></section></main><script>
const pages=__DATA__;let current=0;const nav=document.querySelector('nav'),figure=document.querySelector('figure'),image=document.querySelector('#image');
pages.forEach((p,i)=>{const b=document.createElement('button');b.textContent=p.title;b.dataset.page=i;b.onclick=()=>show(i);nav.append(b)});
function show(i){current=Math.max(0,Math.min(pages.length-1,i));const p=pages[current];image.src=p.file;image.alt=p.title+'，设计母版图稿';document.querySelector('#status').textContent=(current+1)+' / '+pages.length+' · '+p.title;document.querySelector('#original').href=p.file;document.querySelectorAll('nav button').forEach((b,n)=>b.setAttribute('aria-pressed',n===current));document.querySelector('#previous').disabled=current===0;document.querySelector('#next').disabled=current===pages.length-1;history.replaceState(null,'','#'+(current+1));}
document.querySelector('#previous').onclick=()=>show(current-1);document.querySelector('#next').onclick=()=>show(current+1);document.querySelector('#size').onclick=e=>{const full=figure.classList.toggle('original');e.currentTarget.setAttribute('aria-pressed',full);e.currentTarget.textContent=full?'适应窗口':'原尺寸查看'};
document.addEventListener('keydown',e=>{if(e.key==='ArrowRight'){e.preventDefault();show(current+1)}if(e.key==='ArrowLeft'){e.preventDefault();show(current-1)}});show(Number(location.hash.slice(1)||1)-1);
</script></html>'''.replace('__DATA__',json.dumps(gallery_data,ensure_ascii=False))
(D/'母版审阅.html').write_text(gallery,encoding='utf-8')
print(json.dumps({'synced':['母版规范MD/JSON','连续任务制作依据JSON','S04-02复审MD/JSON'],'pages':len(mapping['pages']),'mapped_controls':len(mapping['field_controls']),'errors':check['errors']},ensure_ascii=False))

# The current authorized review is a separate real evidence source, not this static export.
visual_path=I/'S04_可操作设计原型/output/visual-review/visual-review-result.json'
if visual_path.exists() and read(visual_path).get('completed'):
    import sys
    sys.path.insert(0,str(I/'S04_可操作设计原型'))
    from sync_visual_review import apply
    apply()
