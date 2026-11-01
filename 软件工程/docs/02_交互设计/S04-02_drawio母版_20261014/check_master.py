"""检查本轮母版字段、状态页与源合同；不证明产品交互。"""
from pathlib import Path
import xml.etree.ElementTree as E
import re
import json
from PIL import Image

D=Path(__file__).resolve().parent
S=D.parents[2]
tree=E.parse(D/'industrial-workbench-master.drawio')
pages=tree.findall('diagram')
page_ids={p.attrib['id'] for p in pages}
forms=json.loads((S/'docs/02_交互设计/首版表单与反馈.json').read_text(encoding='utf-8-sig'))
allowed={x['ui_field_id'] for f in forms['forms'] for x in f['fields']}
src=(S/'packages/domain/types.design.rs').read_text(encoding='utf-8')
types={name:set(re.findall(r'pub\s+(\w+)\s*:',body)) for name,body in re.findall(r'pub struct (\w+)\s*\{(.*?)\n\}',src,re.S)}
categories=['items','devices','resources','tasks','operations','transport','templates','professional','media']
fields=bindings=vertices=links=0
errors=[]
for p in pages:
    ids=[]
    for n in p.iter():
        if 'id' in n.attrib: ids.append(n.attrib['id'])
        if n.tag=='mxCell' and n.get('vertex')=='1': vertices+=1
        if n.get('ui_field_id'):
            fields+=1
            if n.get('ui_field_id') not in allowed: errors.append('unknown field group '+n.get('ui_field_id'))
        if n.get('link','').startswith('data:page/id,'):
            links+=1
            if n.get('link').split(',',1)[1] not in page_ids: errors.append('missing page link')
        path=n.get('canonical_binding')
        if path and not path.startswith('UI '):
            for part in path.split(';'):
                name,field=part.split('.',1)
                if name not in types or field.split('.')[0] not in types[name]: errors.append('missing carrier '+part)
                else: bindings+=1
    if len(ids)!=len(set(ids)): errors.append('duplicate IDs on '+p.attrib['id'])
    present=[n.get('category') for n in p.iter() if n.get('category')]
    expected=[] if p.attrib['id'] in ['master-left-collapsed','master-both-collapsed'] else categories
    if sorted(present)!=sorted(expected): errors.append('category visibility mismatch on '+p.attrib['id'])
    if len([n for n in p.iter() if n.get('asset_kind')])!=(0 if p.attrib['id'] in ['master-multi-hall','master-simplified','master-region'] else 1): errors.append('scene-only image count mismatch')
    nodes={n.get('id'):n for n in p.iter() if n.get('id')}
    pid=p.attrib['id']
    expected_time=not (pid.startswith('master-timeline-hidden') or pid.startswith('master-lower-hidden'))
    expected_description=not (pid.startswith('master-description-hidden') or pid.startswith('master-lower-hidden'))
    if (pid+'-timeline' in nodes)!=expected_time: errors.append('timeline visibility mismatch '+pid)
    if (pid+'-event-description' in nodes)!=expected_description: errors.append('description visibility mismatch '+pid)
    for n in p.iter():
        if '-tool-' in n.get('id',''):
            g=n.find('.//mxGeometry')
            if g is None or float(g.get('y'))<1004: errors.append('tool not in bottom strip '+pid)
    for suffix,state in [('-window-option-2',expected_time),('-window-option-3',expected_description)]:
        n=nodes.get(pid+suffix)
        if n is not None:
            label=n.get('label',n.get('value',''))
            if label.startswith('✓')!=state: errors.append('menu checkbox mismatch '+pid+suffix)
    if expected_time:
        n=nodes[pid+'-timeline']
        if n.find('mxGeometry').get('height')!=('320' if pid in ['master-axis-expanded','master-line-axis','master-device-axis','master-input-drag'] else '160'): errors.append('timeline height mismatch '+pid)
        if nodes[pid+'-input-head'].find('mxGeometry').get('width')!='240': errors.append('track name width mismatch '+pid)
        modes=[n.get('source_mode') for n in p.iter() if n.get('source_mode')]
        if modes!=['input','actual','comparison']: errors.append('source modes mismatch '+pid)
        if expected_description and nodes[pid+'-event-description'].find('mxGeometry').get('width')!='300': errors.append('description width mismatch '+pid)
    if pid=='master-line-axis':
        bar=nodes[pid+'-job-envelope']
        if [bar.get(k) for k in ['source_kind','true_start_ms','true_end_ms','visible_start_ms','visible_end_ms']]!=['manual_expected','90000','180000','100000','160000']: errors.append('clipped job identity mismatch')
        if bar.get('continuation_left')!='True' or bar.get('continuation_right')!='True': errors.append('missing continuation caps')
    if pid=='master-device-axis' and any('job-envelope' in n.get('id','') for n in p.iter()): errors.append('invented runtime job')
    if pid=='master-input-drag':
        for key,t in [('ARR01','15000'),('ARR02','25000')]:
            node=nodes[pid+'-preview-marker-'+key]
            if node.get('input_time_ms')!=t or node.get('drag_state')!='preview_not_applied': errors.append('input delta preview mismatch')
    if pid=='master-device-appearance':
        if any(n.get('affects_production')!='false' for n in p.iter() if n.get('clock')=='normalized_u'): errors.append('appearance clock affects production')
        if 'cycle_normalized' not in nodes[pid+'-appearance-mode'].get('value',''): errors.append('single phase binding not explicit')
    if pid in ['master-simplified','master-region']:
        if any('shape=image;' in n.get('style','') for n in p.iter()): errors.append('LOD is a flattened image '+pid)
        expected_lod='simplified' if pid=='master-simplified' else 'region'
        expected_index=1 if expected_lod=='simplified' else 2
        for i in range(3):
            node=nodes[pid+'-lod-'+str(i)]
            if ('fillColor=#146BC2;' in node.find('mxCell').get('style',''))!=(i==expected_index): errors.append('active LOD inconsistent '+pid)
        if pid=='master-region':
            region=nodes[pid+'-region-line']
            if region.get('selected_object_id')!='A01' or region.get('members')!='KIT,A01,I01,P01,R01': errors.append('aggregation loses membership or selection')
    if pid=='master-phase-appearance':
        if any(n.get('affects_production')!='false' for n in p.iter() if n.get('binding_mode')=='phase_local'): errors.append('phase appearance changes production')
        if 'phase_work_intervals' not in nodes[pid+'-phase-setting-3'].get('label',''): errors.append('phase history source missing')
    if pid=='master-revision-conflict':
        preserved=[n for n in p.iter() if n.get('form_state')=='preserved_not_applied']
        if len(preserved)!=12 or any(n.get('base_revision')!='r7' for n in preserved): errors.append('conflict preview loses draft')
    if pid=='master-text-scale':
        values=[n for n in p.iter() if n.get('ui_text_scale')=='1.5']
        if len(values)!=5: errors.append('text scale state missing')
exports=[]
for name in ['母版_精细化预览','左侧收起_预览','右侧收起_预览','两侧收起_预览','面板调宽_预览','设备字段_精细化预览','未应用离开_预览','时间轴隐藏_预览','描述隐藏_预览','底部双面板隐藏_预览','窗口菜单_预览','时间轴隐藏菜单_预览','描述隐藏菜单_预览','双面板隐藏菜单_预览','主轴展开_预览','流水线细化_预览','设备运行缺项_预览','输入吸附联动_预览','多厂房供货_预览','设备表现创作_预览','简化档_预览','区域档_预览','视图菜单_预览','修订冲突_预览','多阶段表现_预览','文字放大_预览']:
    with Image.open(D/(name+'.png')) as im: exports.append({'file':name+'.png','size':list(im.size)})
structural=(D/'结构检查输出.txt').read_text(encoding='utf-8')
result={'pages':len(pages),'native_vertices':vertices,'field_group_locations':fields,'canonical_carrier_checks':bindings,'page_links':links,'exports':exports,'errors':errors,'bottom_layout_checks':'工具下置；默认160/展开320；描述300；窗口菜单；组/设备层级；同尺度裁窗与真实区间；输入增量仅预览；u轴不写生产','structural_summary':'\n'.join(structural.splitlines()[-2:]),'structural_warnings_scope':'UI底色/标题/按钮/事件标记与弹层叠放；通用图形重叠告警不当零告警或交互验收。','binding_check_scope':'核对既有字段组与Rust载体第一层字段，不替代完整命令处理器/Schema/运行验证。','actual_product_runtime':None,'S04_02_passed':False}
(D/'母版核对结果.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({k:result[k] for k in ['pages','native_vertices','field_group_locations','canonical_carrier_checks','page_links','errors','structural_summary']},ensure_ascii=False))
if errors: raise SystemExit(1)
