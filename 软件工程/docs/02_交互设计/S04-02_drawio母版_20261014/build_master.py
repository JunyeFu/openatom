"""生成可编辑draw.io工作台设计资产；不生成产品代码或运行结果。"""
from pathlib import Path
from copy import deepcopy
import base64
import json
import xml.etree.ElementTree as E

HERE = Path(__file__).resolve().parent
I = HERE.parent
S = I.parent.parent
FONT = 'Microsoft YaHei'
C = dict(ink='#172E48', muted='#64748B', line='#C7D4E2', shell='#F4F7FA', blue='#146BC2', tint='#E7F1FC', white='#FFFFFF', scene='#E8EDF1', red='#B42318', cyan='#008C99')
IMAGE = 'data:image/png,' + base64.b64encode((HERE/'产线场景素材.png').read_bytes()).decode()
M = E.Element('mxfile', host='drawio', version='31.3.2', compressed='false')
PAGES = []
FIELDS = []

def page(pid, title):
    d = E.SubElement(M, 'diagram', id=pid, name=title)
    g = E.SubElement(d, 'mxGraphModel', dx='1920', dy='1080', grid='1', gridSize='10', guides='1', tooltips='1', connect='1', arrows='1', fold='1', page='1', pageScale='1', pageWidth='1920', pageHeight='1080', background=C['white'], adaptiveColors='auto')
    root = E.SubElement(g, 'root')
    E.SubElement(root, 'mxCell', id='0')
    E.SubElement(root, 'mxCell', id='1', parent='0')
    PAGES.append(dict(id=pid, title=title))
    return root

def cell(root, cid, text, x, y, w, h, parent='1', fill=None, stroke=None, size=16, bold=False, align='left', extra='', metadata=None, link=None):
    text=text.replace('\n','<br>')
    style = f'rounded=0;whiteSpace=wrap;html=1;fontFamily={FONT};fontSize={size};fontColor={C["ink"]};align={align};verticalAlign=middle;spacingLeft=8;spacingRight=6;fillColor={fill or "none"};strokeColor={stroke or "none"};'
    if bold: style += 'fontStyle=1;'
    style += extra
    holder = root
    attrs = dict(id=cid, value=text, style=style, vertex='1', parent=parent)
    if metadata or link:
        data = dict(id=cid, label=text)
        if metadata: data.update({k:str(v) for k,v in metadata.items()})
        if link: data['link'] = 'data:page/id,'+link
        holder = E.SubElement(root, 'object', **data)
        attrs.pop('id'); attrs.pop('value')
    node = E.SubElement(holder, 'mxCell', **attrs)
    E.SubElement(node, 'mxGeometry', x=str(x), y=str(y), width=str(w), height=str(h), **{'as':'geometry'})
    return cid

def group(root, cid, x, y, w, h, fill=None, stroke=None):
    return cell(root,cid,'',x,y,w,h,fill=fill,stroke=stroke,extra='container=1;pointerEvents=0;collapsible=0;')

def text(root,cid,t,x,y,w,h,parent='1',size=16,bold=False,color=None,align='left'):
    return cell(root,cid,t,x,y,w,h,parent=parent,size=size,bold=bold,align=align,extra=f'fontColor={color or C["ink"]};')

def button(root,cid,label,x,y,w,h=30,parent='1',primary=False,disabled=False,link=None):
    fill=C['blue'] if primary else C['white']
    if disabled: fill='#F0F2F5'
    return cell(root,cid,label,x,y,w,h,parent=parent,fill=fill,stroke=C['line'],size=15,align='center',extra='rounded=1;arcSize=8;'+f'fontColor={C["white"] if primary else C["muted"] if disabled else C["ink"]};',metadata={'control_kind':'button','design_only':'true','enabled':not disabled},link=link)

def field(root,cid,label,value,x,y,w=360,parent='1',fid=None,path=None,readonly=False,missing=False,unit=None):
    text(root,cid+'-label',label,x,y,110,28,parent=parent,size=14)
    control='readonly_reference' if readonly else 'reference_selector' if fid=='F03-05' else 'ui_grid_setting' if fid=='F11-02' else 'numeric_with_unit' if unit else 'input'
    attrs={'control_kind':control,'read_only':readonly,'design_example':'true'}
    if fid: attrs['ui_field_id']=fid
    if path: attrs['canonical_binding']=path
    cid=cell(root,cid,value,x+112,y,w-112-(35 if unit else 0),28,parent=parent,fill='#F5F7FA' if readonly else C['white'],stroke=C['red'] if missing else C['line'],size=14,metadata=attrs)
    if unit: text(root,cid+'-unit',unit,x+w-34,y,34,28,parent=parent,size=14)
    FIELDS.append(dict(cell_id=cid,ui_field_id=fid,canonical_binding=path,readonly=readonly,missing=missing))
    return cid

def section(root,cid,label,x,y,w,parent='1'):
    cell(root,cid,label,x,y,w,28,parent=parent,fill='#EDF2F7',stroke=C['line'],size=15,bold=True)

def edge(root,cid,source,target,label='',color=None,parent='1',ex=1,ey=.5,ix=0,iy=.5):
    c=E.SubElement(root,'mxCell',id=cid,value=label,edge='1',parent=parent,source=source,target=target,style=f'edgeStyle=orthogonalEdgeStyle;rounded=1;orthogonalLoop=1;jettySize=auto;html=1;strokeColor={color or C["cyan"]};strokeWidth=2;endArrow=block;endSize=9;fontFamily={FONT};fontSize=14;labelBackgroundColor=#FFFFFF;exitX={ex};exitY={ey};entryX={ix};entryY={iy};')
    E.SubElement(c,'mxGeometry',relative='1',**{'as':'geometry'})

CATEGORIES=[('items','物品'),('devices','固定设备'),('resources','资源'),('tasks','任务'),('operations','工艺'),('transport','物流'),('templates','厂房与组合'),('professional','专业计算'),('media','表现素材')]

def left(root,pid,w):
    g=group(root,pid+'-left',0,100,w,900,C['white'],C['line'])
    if w==32:
        button(root,pid+'-restore-left','›',2,8,28,parent=g,link='master-right-collapsed' if pid=='master-both-collapsed' else 'master-main')
        text(root,pid+'-left-title','工<br>程<br>与<br>库',2,60,28,140,parent=g,size=14,align='center')
        return
    section(root,pid+'-tree-title','工程对象',0,0,w,parent=g)
    button(root,pid+'-collapse-left','‹',w-30,0,30,28,parent=g,link='master-both-collapsed' if pid=='master-right-collapsed' else None if pid=='master-wide-panels' else 'master-left-collapsed')
    cell(root,pid+'-search','搜索工程对象…',10,40,w-20,30,parent=g,fill=C['white'],stroke=C['line'],size=14)
    rows=[('⌄ 示例控制盒厂','',0),('⌄ 厂房 A','',12),('⌄ 总装检查与包装线','',24),('装配工位 A01','selected',42),('视觉检查 I01','',42),('包装工位 P01','',42),('返工工位 R01','',42),('› 物品、BOM与订单','',12),('› 资源与日历','',12)]
    if pid=='master-multi-hall':
        rows=[('⌄ 示例控制盒厂','',0),('› 厂房 A · 装配基地','',12),('› 厂房 B · 包装基地','',12),('› 厂房 C · 纸盒基地','',12),('⌄ 跨厂供货','',12),('S01 · 半成品 → B','selected',24),('S02 · 纸盒 → B','',24),('› 物品、BOM与订单','',12),('› 资源与日历','',12)]
    for n,(name,state,indent) in enumerate(rows):
        y=80+n*31
        if state: cell(root,pid+'-tree-selection','',0,y,w,31,parent=g,fill=C['tint'])
        text(root,pid+f'-tree-{n}',name,indent+10,y,w-indent-20,31,parent=g,size=14,bold=bool(state))
    section(root,pid+'-library-title','模块库 · 九类定义',0,390,w,parent=g)
    text(root,pid+'-library-scope','采用定义 ≠ 新增库存',10,421,w-20,28,parent=g,size=13,color=C['muted'])
    for n,(cat,name) in enumerate(CATEGORIES):
        cell(root,pid+'-cat-'+cat,'›  '+name,8,453+n*32,w-16,30,parent=g,size=15,metadata={'category':cat,'object_scope':'module_definition'})
    button(root,pid+'-new-module','新建定义',10,767,100,parent=g)
    button(root,pid+'-import-module','导入模块',116,767,w-126,parent=g)

def right(root,pid,x,w):
    g=group(root,pid+'-right',x,100,w,900,C['white'],C['line'])
    if w==32:
        button(root,pid+'-restore-right','‹',2,8,28,parent=g,link='master-left-collapsed' if pid=='master-both-collapsed' else 'master-main')
        text(root,pid+'-right-title','属<br>性',2,60,28,80,parent=g,size=14,align='center')
        return
    if pid in ['master-device-detail','master-device-appearance','master-phase-appearance','master-device-axis','master-line-axis']:
        section(root,pid+'-right-outline-title','设备工作台 · 上下文',0,0,w,parent=g)
        caption='A01 / 表现编辑' if pid in ['master-device-appearance','master-phase-appearance'] else 'A01 / 运行明细（只读）' if pid=='master-device-axis' else '总装线 / 手工预期' if pid=='master-line-axis' else 'A01 / 实例配置'
        text(root,pid+'-outline-selected',caption,12,42,w-24,32,parent=g,size=18,bold=True)
        field(root,pid+'-outline-id','实例ID','A01',12,90,w-24,g,None,'DeviceInstance.id',True)
        field(root,pid+'-outline-hall','所属厂房','厂房 A',12,125,w-24,g,None,'DeviceInstance.hall_id',True)
        field(root,pid+'-outline-module','定义版本','assembly.station @ 1.0.0',12,160,w-24,g,'F01-02','DeviceInstance.module_ref',True)
        section(root,pid+'-outline-sections','当前工作区段',12,220,w-24,parent=g)
        for n,label in enumerate(['定义引用与行为来源','端口与接料规则','阶段与资源','表现资产']):
            text(root,pid+'-outline-nav-'+str(n),label,16,260+n*36,w-32,30,parent=g,size=15)
        text(root,pid+'-outline-rule','这里仅显示上下文。<br>可编辑字段与提交入口在中央，<br>避免同一实例出现两套未提交表单。',16,440,w-32,100,parent=g,size=15,color=C['muted'])
        section(root,pid+'-outline-result-title','结果归属',12,570,w-24,parent=g)
        text(root,pid+'-outline-result','Run：未生成<br>计算：未运行<br>结果：暂无<br><br>工时来源：待补充<br>当前页面：设计示例',16,610,w-32,160,parent=g,size=15)
        return
    section(root,pid+'-right-title','对象属性 · 草稿编辑',0,0,w,parent=g)
    button(root,pid+'-collapse-right','›',w-30,0,30,28,parent=g,link='master-both-collapsed' if pid=='master-left-collapsed' else None if pid=='master-wide-panels' else 'master-right-collapsed')
    text(root,pid+'-selected-title','装配工位 A01',10,36,w-150,32,parent=g,size=18,bold=True)
    button(root,pid+'-device-detail','设备详情',w-130,38,116,parent=g,link='master-device-detail')
    for n,(lab,width) in enumerate([('实例',86),('定义引用',112),('连接',86)]):
        xx=10+[0,90,206][n]
        button(root,pid+f'-scope-{n}',lab,xx,78,width,parent=g,primary=n==0)
    text(root,pid+'-scope-note','只改 A01；不改模块库及其他设备',10,114,w-20,25,parent=g,size=13,color=C['muted'])
    section(root,pid+'-identity-section','身份与行为来源',10,145,w-20,parent=g)
    field(root,pid+'-instance-id','实例ID','A01',12,181,w-24,g,None,'DeviceInstance.id',True)
    field(root,pid+'-title','名称','装配工位',12,214,w-24,g,None,'DeviceInstance.title')
    field(root,pid+'-module','定义版本','assembly.station @ 1.0.0',12,247,w-24,g,'F01-02','DeviceInstance.module_ref',True)
    field(root,pid+'-behavior','唯一行为源','工艺引用 · assembly.rule',12,280,w-24,g,'F03-01','DeviceInstance.operation_binding',True)
    section(root,pid+'-layout-section','布局 · 厂房坐标 / Z向上',10,323,w-20,parent=g)
    for i,(label,value,path) in enumerate([('X','6.2' if pid in ['master-dirty-leave','master-revision-conflict'] else '6.0','translation.x'),('Y','4.0','translation.y'),('Z / 高度坐标','0.0','translation.z'),('朝向 / 绕Z','0','rotation')]):
        field(root,pid+'-layout-'+str(i),label,value,12,359+i*33,w-24,g,'F11-01','DeviceInstance.pose.'+path,unit='°' if i==3 else 'm')
    field(root,pid+'-footprint','占地 / 高度','包络 2.4 × 1.8 m / 高 2.0 m',12,491,w-24,g,'F11-01','DeviceInstance.footprint_m;DeviceInstance.height_m',True)
    field(root,pid+'-snap','网格吸附','☑ 启用  / 步长 0.1 m',12,524,w-24,g,'F11-02','UI grid / snap (not domain)')
    section(root,pid+'-ports-section','端口与接料规则',10,567,w-20,parent=g)
    field(root,pid+'-port-in','输入端口','kit_in · 物品 · 套件 / 件',12,603,w-24,g,'F03-03','DeviceDefinition.ports',True)
    field(root,pid+'-port-out','输出端口','assembled_out · 半成品 / 件',12,636,w-24,g,'F03-03','DeviceDefinition.ports',True)
    section(root,pid+'-resource-section','阶段与资源绑定',10,678,w-20,parent=g)
    field(root,pid+'-operator','操作员角色','operator → OP01',12,714,w-24,g,'F03-05','DeviceInstance.role_bindings.resources')
    field(root,pid+'-duration','阶段工时','未填写 · 打开设备详情补充',12,747,w-24,g,'F14-02','PhaseDefinition.duration',missing=True)
    text(root,pid+'-missing-help','必需工时未填，当前工艺不能执行。',12,781,w-24,25,parent=g,size=13,color=C['red'])
    text(root,pid+'-apply-note','应用：提交草稿；保存：另行写入工程。',12,817,w-24,22,parent=g,size=13,color=C['muted'])
    button(root,pid+'-apply','应用更改',w-226,850,106,parent=g,primary=pid=='master-dirty-leave',disabled=pid!='master-dirty-leave')
    button(root,pid+'-discard','放弃更改',w-112,850,100,parent=g,disabled=pid not in ['master-dirty-leave','master-revision-conflict'],link='master-dirty-leave' if pid=='master-dirty-leave' else None)

def scene(root,pid,x,y,w,h):
    g=group(root,pid+'-scene',x,y,w,h,C['scene'],C['line'])
    scene_parent=cell(root,pid+'-scene-image','',0,0,w,h,parent=g,extra='shape=image;imageAspect=0;container=1;pointerEvents=0;image='+IMAGE+';',metadata={'asset_kind':'AI_scene_illustration','editable_3d':'false','runtime_proof':'false'})
    g=scene_parent
    def anchor(cid,xx,yy,ww,hh):
        return cell(root,pid+'-anchor-'+cid,'',xx*w,yy*h,ww*w,hh*h,parent=g,stroke=C['blue'] if cid=='A01' else None,extra='strokeWidth=2;rounded=1;arcSize=6;' if cid=='A01' else '',metadata={'object_id':cid,'design_example':'true'})
    anchors={
      'B01':anchor('B01',.085,.22,.09,.14),'B02':anchor('B02',.075,.49,.11,.15),
      'KIT':anchor('KIT',.195,.35,.095,.14),'A01':anchor('A01',.36,.32,.11,.18),
      'I01':anchor('I01',.592,.32,.095,.19),'P01':anchor('P01',.73,.32,.09,.18),
      'FG01':anchor('FG01',.864,.35,.07,.18),'R01':anchor('R01',.556,.612,.09,.14)}
    for cid,label,xx,yy,ww in [('B01','外购电子件 · B01',.03,.13,.19),('B02','外购结构件 · B02',.02,.66,.19),('KIT','齐套缓存 · KIT',.19,.255,.15),('A01','装配 A01',.35,.23,.14),('I01','检验 I01',.58,.19,.10),('P01','包装 P01',.73,.23,.10),('FG01','成品待入库 · FG01',.84,.28,.15),('R01','返工 R01',.54,.785,.13)]:
        cell(root,pid+'-scene-label-'+cid,label,xx*w,yy*h,ww*w,26,parent=g,fill='#F8FAFC',size=14,bold=cid=='A01',align='center',metadata={'object_id':cid})
    edge(root,pid+'-feed1',anchors['B01'],anchors['KIT'],parent=g)
    edge(root,pid+'-feed2',anchors['B02'],anchors['KIT'],parent=g)
    edge(root,pid+'-kit-assembly',anchors['KIT'],anchors['A01'],parent=g)
    edge(root,pid+'-assembly-inspection',anchors['A01'],anchors['I01'],parent=g)
    edge(root,pid+'-inspection-pack',anchors['I01'],anchors['P01'],parent=g)
    edge(root,pid+'-pack-finished',anchors['P01'],anchors['FG01'],parent=g)
    edge(root,pid+'-NG',anchors['I01'],anchors['R01'],'待返工',C['red'],g,.28,1,.82,0)
    edge(root,pid+'-reinspection',anchors['R01'],anchors['I01'],'复检',C['cyan'],g,1,.6,1,.92)
    text(root,pid+'-scene-dimensions','场地声明：24 × 16 × 4 m',w-265,h-36,255,28,parent=g,size=14)
    text(root,pid+'-scene-modes','仅设计示例 / 未运行',12,h-36,260,28,parent=g,size=14,color=C['muted'])

def lower_panels(root,pid,x,w,show_timeline=True,show_description=True,mode='input',scope='factory',height=160):
    if not (show_timeline or show_description): return
    y=1000-height
    cell(root,pid+'-bottom-dock-grip','⋯',x,y-8,w,6,fill='#DAE3EC',size=10,align='center',metadata={'ui_setting':'bottom_dock_height','state_scope':'presentation'})
    dw=300 if show_timeline else w
    manual=pid=='master-line-axis'
    dragging=pid=='master-input-drag'
    multi=pid=='master-multi-hall'
    runtime=mode=='actual'
    expanded=height==320
    if show_description:
        d=group(root,pid+'-event-description',x,y,dw,height,C['white'],C['line'])
        cell(root,pid+'-description-title','选中描述 · 只读',0,0,dw-32,24,parent=d,fill='#EDF2F7',stroke=C['line'],size=14,bold=True)
        button(root,pid+'-close-description','×',dw-32,0,32,24,parent=d,link='master-description-hidden' if show_timeline else 'master-lower-hidden')
        if show_timeline:
            if multi:
                lines=[('选中','S01 · 半成品供货'),('来源','人工供货输入 / 方案A'),('窗口','0–86400 s / 24 h'),('目标量','120 件 / 天'),('计算','未运行 / 到货暂无')]
            elif manual:
                lines=[('选中','J-01 · 装配作业'),('来源','手工预期示例 / 非实际Run'),('真实区间','90–180 s'),('观察窗口','100–160 s'),('观察时点','135 s / 保留父轴时点'),('边界','两端续接，未缩短作业')]
            elif runtime:
                lines=[('选中','A01 · 设备运行明细'),('Run','尚未生成'),('执行历史','尚未提供'),('有效阶段','尚未提供'),('等待归属','尚未提供'),('显示','不按名义工时补画')]
            elif dragging:
                lines=[('选中','ARR01、ARR02 / 同方案A'),('来源','人工输入 / 拖动预览'),('改动','10 / 20 s → 15 / 25 s'),('增量','Δ +5 s / 等时差联动'),('未改','交期、日历、旧Run'),('提交','松开一次应用 / Esc取消')]
            elif expanded:
                lines=[('选中','总装检查与包装线'),('成员','A01 / I01 / P01 / R01'),('来源','当前草稿 / 未运行'),('加工/等待','尚无实际区间'),('设备占用','尚无实际区间'),('接受量','暂无；不累加工位产出')]
            else:
                lines=[('选中','ARR02 · 结构件到料'),('来源','人工输入示例 / 方案A'),('时点 / 量','60 s / 6 件'),('目的','B02 / 预计到料'),('计算','未运行 / 不代表库存')]
            for n,(lab,val) in enumerate(lines):
                row_h=32 if expanded else 23
                text(root,pid+f'-description-key-{n}',lab,6,26+n*row_h,78,row_h,parent=d,size=13,color=C['muted'])
                text(root,pid+f'-description-value-{n}',val,84,26+n*row_h,dw-90,row_h,parent=d,size=13)
            if expanded:
                text(root,pid+'-description-note','先选择对象与时段，再细化。\n名称选择不移动时间；定位使用同一时间。',8,228,dw-16,44,parent=d,size=13,color=C['muted'])
                button(root,pid+'-description-locate','定位选中对象',8,280,146,28,parent=d,disabled=runtime)
                button(root,pid+'-description-edit','编辑对应输入',162,280,dw-170,28,parent=d,disabled=runtime or manual or not dragging)
        else:
            for n,(lab,val) in enumerate([('选中事件','ARR02 · 结构件到料'),('目的库位','B02'),('时点 / 数量','60 s / 6 件'),('输入来源','人工设计示例')]):
                column=n%2; row=n//2
                xx=12+column*(dw/2)
                text(root,pid+f'-description-key-{n}',lab,xx,36+row*33,104,30,parent=d,size=14,color=C['muted'])
                text(root,pid+f'-description-value-{n}',val,xx+106,36+row*33,dw/2-128,30,parent=d,size=14)
            text(root,pid+'-description-note','时间轴已隐藏；描述仍保留原选择。隐藏面板只改变布局，不改输入、Run或观察时间。',12,111,dw-24,34,parent=d,size=14,color=C['muted'])
    if not show_timeline: return
    tx=x+308 if show_description else x
    tw=w-308 if show_description else w
    g=group(root,pid+'-timeline',tx,y,tw,height,C['white'],C['line'])
    cell(root,pid+'-timeline-title','时间轴',0,0,74,24,parent=g,fill='#EDF2F7',size=14,bold=True)
    for n,(key,lab) in enumerate([('input','输入'),('actual','实际'),('comparison','对比')]):
        cell(root,pid+'-time-tab-'+str(n),lab,76+n*58,0,54,24,parent=g,fill=C['tint'] if mode==key else C['white'],stroke=C['line'],size=13,align='center',metadata={'source_mode':key,'actual_run_exists':'false'})
    head='多厂房 / 主轴' if multi else '设备 / A01' if scope=='device' else '流水线 / 总装线' if scope=='line' else '厂房 A / 主轴'
    text(root,pid+'-axis-scope',head,256,0,188,24,parent=g,size=13,bold=True)
    button(root,pid+'-axis-back','返回主轴',tw-258,0,108,24,parent=g,disabled=scope=='factory',link='master-axis-expanded' if scope!='factory' else None)
    button(root,pid+'-axis-detail','缺项明细' if manual else '细化当前段',tw-146,0,110,24,parent=g,disabled=scope!='line',link='master-device-axis' if scope=='line' else None)
    button(root,pid+'-close-timeline','×',tw-32,0,32,24,parent=g,link='master-timeline-hidden' if show_description else 'master-lower-hidden')
    label_w=240; graph_w=tw-label_w-28
    start,end=(0,86400) if multi else (100,160) if manual else (0,30) if dragging else (0,300)
    def xpos(t): return label_w+8+(t-start)/(end-start)*graph_w
    text(root,pid+'-input-head','对象 / 来源',0,24,label_w,28,parent=g,size=13,color=C['muted'])
    divisions=4 if multi else 6 if dragging or manual else 5
    for n in range(divisions+1):
        t=start+n*(end-start)/divisions; xx=xpos(t)
        tick=str(int(t/3600))+' h' if multi else str(int(t))+' s'
        text(root,pid+'-ruler-'+str(n),tick,xx-(44 if n==divisions else 10 if n else 0),24,52,28,parent=g,size=13)
        cell(root,pid+'-grid-'+str(n),'',xx,52,1,height-80,parent=g,fill='#E2E8F0')
    def row(key,label,yy,hh=32,level=0,group_row=False):
        row_label=label[1:].strip() if group_row and label[:1] in ['›','⌄'] else label
        cell(root,pid+'-axis-row-'+key,row_label,level*14,yy,label_w-level*14,hh,parent=g,fill='#EDF2F7' if group_row else C['tint'] if key in ['A01','ARR02'] else C['white'],stroke=C['line'],size=13,extra='spacingLeft=28;' if group_row else '',metadata={'track_key':key,'scope_kind':scope,'state_scope':'UI row only','group_row':group_row})
        if group_row:
            target='master-main' if pid=='master-axis-expanded' else 'master-axis-expanded' if key=='line' and not (runtime or dragging or multi) else None
            cell(root,pid+'-group-toggle-'+key,label[0] if label[:1] in ['›','⌄'] else '›',level*14+2,yy+4,22,hh-8,parent=g,size=14,align='center',link=target,metadata={'action_scope':'UI group expansion only'})
        cell(root,pid+'-row-fill-'+key,'',label_w,yy,tw-label_w,hh,parent=g,stroke=C['line'])
    if multi:
        row('halls','› 全厂 · 3个厂房',52,40,group_row=True)
        row('S01','S01 · 供货输入叠层',92,32,1)
        text(root,pid+'-multi-no-run','无实际Run / 运输轨暂无',label_w+12,56,graph_w-14,32,parent=g,size=13,color=C['muted'])
        cell(root,pid+'-supply-window','目标120件 / 24h窗口',xpos(0),98,graph_w,20,parent=g,fill=C['white'],stroke=C['cyan'],size=13,align='center',extra='dashed=1;',metadata={'source_kind':'input','supply_id':'S01','start_ms':0,'end_ms':86400000,'actual_arrival':False})
    elif manual:
        row('line','⌄ 总装线 · 手工预期',52,40,group_row=True)
        row('A01','A01 / J-01 · 只读',92,32,1)
        row('resource','› OP01 · 共享资源',124,32)
        bar=cell(root,pid+'-job-envelope','J-01 · 原区间90–180 s',xpos(100),98,graph_w,20,parent=g,fill='#EDF6FF',stroke=C['blue'],size=13,align='center',extra='dashed=1;',metadata={'source_kind':'manual_expected','job_id':'J-01','true_start_ms':90000,'true_end_ms':180000,'visible_start_ms':100000,'visible_end_ms':160000,'continuation_left':True,'continuation_right':True,'read_only':True})
        text(root,pid+'-continuation-left','◀',xpos(100),98,24,20,parent=g,size=13,color=C['blue'],align='center')
        text(root,pid+'-continuation-right','▶',xpos(160)-24,98,24,20,parent=g,size=13,color=C['blue'],align='center')
        text(root,pid+'-resource-no-history','实际资源区间尚未提供',label_w+12,126,graph_w,28,parent=g,size=13,color=C['muted'])
        text(root,pid+'-phase-rule','预期外包络仅作视图设计；有效阶段/等待不由名义时长推断。',8,174,tw-16,40,parent=g,size=14,color=C['muted'])
        at=xpos(135)
        cell(root,pid+'-view-time','',at,52,2,112,parent=g,fill=C['blue'],metadata={'view_time_ms':135000,'source_kind':'manual_expected'})
        text(root,pid+'-view-time-label','观察 135 s',at-60,221,138,28,parent=g,size=13,color=C['blue'])
    elif runtime:
        row('A01','A01 / 作业明细 · 只读',52,40,group_row=True)
        row('phase','有效阶段',92,32,1)
        row('wait','等待 / 阻塞',124,32,1)
        row('resource','› OP01 · 共享资源',156,32)
        for n,msg in enumerate(['尚未生成实际Run，不能显示作业历史','阶段历史尚未提供','等待归属尚未提供','资源历史尚未提供']):
            text(root,pid+'-runtime-gap-'+str(n),msg,label_w+12,52+(40 if n else 0)+max(0,n-1)*32,graph_w-14,40 if n==0 else 32,parent=g,size=13,color=C['muted'])
        text(root,pid+'-uncomputed-label','暂无已计算区间 / 回看不可用',8,220,tw-16,38,parent=g,size=14,color=C['muted'])
        cell(root,pid+'-uncomputed-zone','',xpos(0),52,graph_w,136,parent=g,fill='#DDE4EB',extra='opacity=18;',metadata={'computed_until_ms':'unknown','actual_result':'missing','read_only':True})
    elif dragging:
        row('line','› 总装检查与包装线',52,40,group_row=True)
        row('ARR01','△ ARR01 · 选中到料',92,32,1)
        row('ARR02','△ ARR02 · 选中到料',124,32,1)
        for n,(key,old,new) in enumerate([('ARR01',10,15),('ARR02',20,25)]):
            yy=96+n*32
            cell(root,pid+'-old-marker-'+key,'△',xpos(old)-10,yy,20,24,parent=g,size=16,align='center',extra='opacity=45;',metadata={'source_kind':'input','input_time_ms':old*1000,'drag_state':'original'})
            cell(root,pid+'-preview-marker-'+key,'▲',xpos(new)-10,yy,20,24,parent=g,size=16,align='center',extra='fontColor='+C['blue']+';',metadata={'source_kind':'input','input_time_ms':new*1000,'drag_state':'preview_not_applied'})
        cell(root,pid+'-snap-guide','',xpos(15),52,1,110,parent=g,fill=C['cyan'],metadata={'snap_target_ms':15000,'snap_radius_css_px':6})
        text(root,pid+'-timeline-snap-label','15 s / 可见网格',xpos(15)-72,170,160,28,parent=g,size=13,color=C['cyan'])
        text(root,pid+'-drag-rule','Ctrl：同类输入多选   Alt：暂停吸附   Esc：取消预览',8,214,tw-16,28,parent=g,size=14)
        text(root,pid+'-drag-rule-2','松开只提交一次；冲突保留预览。依赖/日历/旧Run不随拖动改变。',8,244,tw-16,32,parent=g,size=13,color=C['muted'])
    else:
        row('line',('⌄' if expanded else '›')+' 总装检查与包装线',52,40,group_row=True)
        if expanded:
            button(root,pid+'-manual-axis-example','查看裁窗设计例',tw-200,56,174,30,parent=g,link='master-line-axis')
        if expanded:
            for n,(key,label) in enumerate([('A01','装配'),('I01','检验'),('P01','包装'),('R01','返工')]):
                row(key,key+' / '+label,92+n*32,32,1)
                text(root,pid+'-no-history-'+key,'未运行 / 无实际区间',label_w+12,92+n*32,graph_w-14,32,parent=g,size=13,color=C['muted'])
            row('OP01','› OP01 · 共享资源',220,32)
            yy=252
        else: yy=92
        row('ARR02','△ ARR02 · 临时输入叠层',yy,32,1)
        cell(root,pid+'-input-marker-ARR02','△',xpos(60)-9,yy+2,20,28,parent=g,size=17,align='center',extra='fontColor='+C['blue']+';',metadata={'source_kind':'input','input_event_time_ms':60000,'calculated_event':'false'})
        text(root,pid+'-group-no-run','无实际Run / 摘要尚未提供',label_w+12,56,graph_w-14,32,parent=g,size=13,color=C['muted'])
    note='手工预期示例 · 非实际运行；子轴保留绝对时间' if manual else '回看只读 · 无Run不可播放；缺失历史保留未知' if runtime else '供货窗口仅声明目标 / 不预画实际运输' if multi else '输入编辑 · 轨道组/顺序仅改变显示，不改变排程'
    text(root,pid+'-timeline-no-run',note,6,height-26,tw-160,24,parent=g,size=13,color=C['muted'])
    button(root,pid+'-review-play','回看 ▶',tw-147,height-26,82,24,parent=g,disabled=True)
    text(root,pid+'-timeline-scroll','↕',tw-38,60,24,height-90,parent=g,size=14,color=C['muted'],align='center')

def window_menu(root,pid,show_timeline,show_description):
    g=group(root,pid+'-windows-popup',370,62,300,240,C['white'],C['line'])
    text(root,pid+'-view-menu-heading','窗口 → 面板显示',6,2,286,28,parent=g,size=14,bold=True)
    time_target=('master-timeline-hidden' if show_description else 'master-lower-hidden') if show_timeline else ('master-main' if show_description else 'master-description-hidden')
    desc_target=('master-description-hidden' if show_timeline else 'master-lower-hidden') if show_description else ('master-main' if show_timeline else 'master-timeline-hidden')
    options=[('✓ 工程对象与模块库','master-left-collapsed' if show_timeline and show_description else None),('✓ 对象属性','master-right-collapsed' if show_timeline and show_description else None),('✓ 时间轴' if show_timeline else '□ 时间轴',time_target),('✓ 选中描述' if show_description else '□ 选中描述',desc_target)]
    for n,(label,target) in enumerate(options):
        cell(root,pid+'-window-option-'+str(n),label,4,32+n*34,292,32,parent=g,size=15,link=target,metadata={'action_scope':'UI visibility only','design_only':'true'})
    cell(root,pid+'-window-divider','',10,174,280,1,parent=g,fill=C['line'])
    button(root,pid+'-layout-restore','恢复默认布局',10,192,280,32,parent=g,link='master-main')

def shell(pid,title,lw=256,rw=400,kind='main',show_timeline=True,show_description=True,dock_height=160,timeline_mode='input',axis_scope='factory',lod='detailed'):
    r=page(pid,title)
    frame=group(r,pid+'-window',0,0,1920,1080,C['shell'],C['line'])
    text(r,pid+'-app-title','超级工厂  —  示例控制盒厂 · 未保存',10,0,1140,32,parent=frame,size=16,bold=True)
    text(r,pid+'-window-controls','—     □     ×',1790,0,120,32,parent=frame,align='center')
    cell(r,pid+'-menubg','',0,32,1920,30,parent=frame,fill=C['white'],stroke=C['line'])
    for n,l in enumerate(['文件','编辑','视图','工程','模块','仿真','窗口','帮助']):
        menu_target={'master-main':'master-window-menu','master-window-menu':'master-window-menu','master-timeline-hidden':'master-timeline-hidden-menu','master-timeline-hidden-menu':'master-timeline-hidden-menu','master-description-hidden':'master-description-hidden-menu','master-description-hidden-menu':'master-description-hidden-menu','master-lower-hidden':'master-lower-hidden-menu','master-lower-hidden-menu':'master-lower-hidden-menu'}.get(pid)
        cell(r,pid+'-menu-'+str(n),l,10+n*60,32,58,30,parent=frame,size=15,link=menu_target if l=='窗口' else 'master-view-menu' if l=='视图' else None)
    cell(r,pid+'-contextbg','',0,66,1920,30,parent=frame,fill='#E9EFF5',stroke=C['line'])
    context='手工预期视图例  |  J-01 / 90–180 s  |  实际Run未生成' if pid=='master-line-axis' else '输入拖动预览  |  方案 A / 基础修订 r7  |  ARR01 / ARR02  |  尚未应用' if pid=='master-input-drag' else '编辑草稿  |  当前 r8 / 表单基础 r7  |  方案 A  |  未应用字段已保留' if pid=='master-revision-conflict' else '编辑草稿  |  工程：示例控制盒厂  |  修订 r1  |  方案 A  |  Run 未生成'
    text(r,pid+'-context',context,10,66,1280,30,parent=frame,size=14)
    text(r,pid+'-prototype-disclaimer','draw.io设计母版 · 非运行截图',1530,66,380,30,parent=frame,size=14,color=C['muted'],align='right')
    left(r,pid,lw); right(r,pid,1920-rw,rw)
    x=lw+8; w=1920-lw-rw-16
    multi=kind=='multi_hall'
    for n,(lab,ww) in enumerate([('流水线画布',140),('多厂房画布',140)]): button(r,pid+'-canvas-tab-'+str(n),lab,x+n*144,108,ww,32,primary=n==(1 if multi else 0),link='master-main' if n==0 else 'master-multi-hall')
    text(r,pid+'-hall','跨厂供货 / 批次交换' if multi else '厂房 A  ▾',x+300,108,220 if multi else 145,32,size=15)
    if not multi:
        for n,lab in enumerate(['详细','简化','区域','□ 自动']): button(r,pid+'-lod-'+str(n),lab,x+w-342+n*80,108,96 if n==3 else 76,32,primary=n=={'detailed':0,'simplified':1,'region':2}[lod],link=['master-main','master-simplified','master-region',None][n])
    h=842-dock_height if show_timeline or show_description else 850
    if multi: multi_hall(r,pid,x,148,w,h)
    elif lod!='detailed': lod_scene(r,pid,x,148,w,h,lod)
    else: scene(r,pid,x,148,w,h)
    for sx in [lw+1,1920-rw-7]:
        cell(r,pid+'-split-'+str(sx),'⋮',sx,410,6,54,fill='#DCE5EF',size=16,align='center',metadata={'state_scope':'UI presentation only','resizable':'true'})
    lower_panels(r,pid,x,w,show_timeline,show_description,timeline_mode,axis_scope,dock_height)
    group(r,pid+'-bottom-tools',0,1004,1920,36,C['white'],C['line'])
    for n,l in enumerate(['选择','移动','旋转','连接','编组','撤销','重做','保存','布局检查']): button(r,pid+'-tool-'+str(n),l,10+n*94,1007,86,30,primary=n==0)
    button(r,pid+'-run','彩排 · 缺项',860,1007,128,30,disabled=True)
    button(r,pid+'-agent','Agent建厂',1000,1007,122,30)
    button(r,pid+'-report','评估与交付',1134,1007,128,30)
    text(r,pid+'-toolbar-hint','缺项：工艺阶段工时',1290,1007,390,30,size=14,color=C['red'])
    cell(r,pid+'-statusbg','',0,1044,1920,36,fill=C['white'],stroke=C['line'])
    text(r,pid+'-status','工程未保存  |  选中 '+('S01' if multi else 'A01')+'  |  '+('表单有未应用字段' if kind in ['dirty','conflict'] else '表单无未提交更改')+'  |  计算未运行',10,1044,1200,36,size=14)
    text(r,pid+'-status-view','视图：'+('多厂房' if multi else {'detailed':'详细','simplified':'简化','region':'区域'}[lod]+'（手动）')+'  |  图像/标记不代表库存',1360,1044,550,36,size=14,color=C['muted'],align='right')
    if kind=='detail': detail(r,pid,x,w)
    if kind=='dirty': dirty(r,pid)
    if kind=='window_menu': window_menu(r,pid,show_timeline,show_description)
    if kind=='appearance': appearance(r,pid,x,w)
    if kind=='phase_appearance': phase_appearance(r,pid,x,w)
    if kind=='view_menu': view_menu(r,pid)
    if kind=='conflict': conflict(r,pid)
    if kind=='text_scale': text_scale(r,pid)
    return r

def multi_hall(root,pid,x,y,w,h):
    g=group(root,pid+'-multi-hall-scene',x,y,w,h,'#ECF1F5',C['line'])
    for n in range(1,10):
        cell(root,pid+'-site-grid-x-'+str(n),'',n*w/10,0,1,h,parent=g,fill='#DDE6EE')
    for n in range(1,7): cell(root,pid+'-site-grid-y-'+str(n),'',0,n*h/7,w,1,parent=g,fill='#DDE6EE')
    text(root,pid+'-multi-head','厂房作为输入/输出节点，连线声明物品与供货窗口',12,10,w-24,36,parent=g,size=17,bold=True)
    specs=[('hall-a','厂房 A · 装配基地','外购零件 → 控制盒半成品',.07,.28),('hall-b','厂房 B · 包装基地','半成品 + 纸盒 → 已包装成品',.58,.28),('hall-c','厂房 C · 纸盒基地','纸板 → 成型纸盒',.25,.68)]
    nodes={}
    for key,name,flow,xx,yy in specs:
        nodes[key]=cell(root,pid+'-'+key,'',xx*w,yy*h,.28*w,110,parent=g,fill=C['white'],stroke=C['blue'],extra='container=1;rounded=1;arcSize=6;',metadata={'hall_id':key,'design_example':True})
        text(root,pid+'-'+key+'-title',name,8,5,.28*w-16,32,parent=nodes[key],size=17,bold=True)
        text(root,pid+'-'+key+'-flow',flow,8,40,.28*w-16,32,parent=nodes[key],size=14)
        button(root,pid+'-'+key+'-open','进入流水线画布',8,76,.28*w-16,28,parent=nodes[key],link='master-main' if key=='hall-a' else None)
    edge(root,pid+'-supply-a-b',nodes['hall-a'],nodes['hall-b'],'S01 · 半成品 / 目标120件·天',parent=g)
    edge(root,pid+'-supply-c-b',nodes['hall-c'],nodes['hall-b'],'S02 · 纸盒 / 按批供货',parent=g,ex=1,ey=.35,ix=.25,iy=1)
    text(root,pid+'-supply-note','供货目标 ≠ 实际到货；装载、去程、卸货、返程分别记录。\n当前节点/数量为人工设计示例，尚无运输或产能结果。',12,h-74,w-24,64,parent=g,size=14,color=C['muted'])
    # Native right panel replaces the device form; no unrelated device fields remain.
    right_id=pid+'-right'
    for node in list(root):
        c=node if node.tag=='mxCell' else node.find('mxCell')
        if c is not None and c.get('parent')==right_id: root.remove(node)
    section(root,pid+'-supply-title','供货关系 · S01 / 草稿',0,0,400,parent=right_id)
    for n,(lab,val,path) in enumerate([
        ('起点','厂房 A / 半成品输出','SupplyRelation.source'),('终点','厂房 B / 接料输入','SupplyRelation.destination'),
        ('物品定义','控制盒半成品 @ 1.0.0','SupplyRelation.item_type_ref'),('目标数量','120 件 / 天（人工示例）','SupplyRelation.target_quantity'),
        ('供货窗口','0–86400 s / 相对工程时间','SupplyRelation.window'),('物流定义','batch.transport @ 1.0.0','SupplyRelation.transport_ref'),
        ('批次资源','carrier → CART01（示例）','SupplyRelation.resources'),('发车窗口','尚未填写','SupplyRelation.departure_windows'),
        ('部分装载','允许 / 需接料规则接受','SupplyRelation.allow_partial_load')]):
        field(root,pid+'-supply-field-'+str(n),lab,val,12,48+n*36,376,right_id,None,path,readonly=n in [2,5])
    text(root,pid+'-supply-help','设置供货量不创建库存或即时到货。\n运行需齐备运输时间、资源日历与接料容量。',16,392,368,92,parent=right_id,size=15,color=C['muted'])
    text(root,pid+'-supply-state','计算未运行 / 到货、接收、返程暂无',16,500,368,54,parent=right_id,size=15)

def appearance(root,pid,x,w):
    g=group(root,pid+'-appearance-editor',x,108,w,892,C['white'],C['line'])
    text(root,pid+'-appearance-head','设备创作 · A01 / 表现编辑',12,8,w-180,36,parent=g,size=18,bold=True)
    button(root,pid+'-appearance-back','返回流水线',w-156,10,142,parent=g,link='master-main')
    text(root,pid+'-appearance-mode','当前 auto 阶段 / phase_local；cycle_normalized 仅用于单阶段。这里是独立外观预览，不生成产能。',12,50,w-24,32,parent=g,size=15,color=C['muted'])
    for n,lab in enumerate(['配置','表现编辑','运行明细']): button(root,pid+'-appearance-tab-'+str(n),lab,14+n*128,98,120,32,parent=g,primary=n==1,link='master-device-axis' if n==2 else 'master-device-detail' if n==0 else None)
    preview=group(root,pid+'-device-pose',x+24,260,w-520,360,'#EDF2F6',C['line'])
    cell(root,pid+'-pose-base','',48,184,w-616,84,parent=preview,fill='#96AAB7',stroke='#718696')
    cell(root,pid+'-pose-table','',88,133,w-696,60,parent=preview,fill='#CCD6DC',stroke='#718696')
    cell(root,pid+'-pose-arm','',144,73,54,65,parent=preview,fill='#D5AF58',stroke='#A78944')
    cell(root,pid+'-pose-arm2','',184,52,118,28,parent=preview,fill='#D5AF58',stroke='#A78944')
    cell(root,pid+'-pose-part','零件',274,128,68,44,parent=preview,fill='#EFF4F6',stroke='#718696',size=14,align='center')
    cell(root,pid+'-tag-duration','阶段工时：未填写',18,18,240,28,parent=preview,fill=C['white'],stroke=C['line'],size=14,metadata={'tag_kind':'duration','source_kind':'parameter','runtime_value':'unknown'})
    button(root,pid+'-tag-operation','打开配置',18,306,126,28,parent=preview,link='master-device-detail')
    text(root,pid+'-preview-caption','原生几何与标签示意 / 姿态可编辑',164,306,w-690,30,parent=preview,size=14,color=C['muted'])
    for n,(label,value) in enumerate([('资产类型','图片 / GIF / 几何姿态 / 模型'),('参数标签','类型 + 绑定字段 + 单位'),('操作标签','配置 / 检查 / 周期预览'),('运行值','只读 / 缺少历史显示未知')]):
        text(root,pid+'-appearance-param-label-'+str(n),label,w-452,152+n*66,110,26,parent=g,size=14,bold=True)
        cell(root,pid+'-appearance-param-value-'+str(n),value,w-452,180+n*66,428,30,parent=g,fill='#F5F7FA',stroke=C['line'],size=14)
    section(root,pid+'-appearance-track-title','auto 阶段表现轨 · 拖动排序 / 图像关键帧 / 类型标签',14,545,w-28,parent=g)
    for n,t in enumerate(['0.00','0.25','0.50','0.75','1.00']): text(root,pid+'-u-ruler-'+str(n),t,240+n*(w-310)/4,580,70,30,parent=g,size=14)
    for n,label in enumerate(['外观 / 姿态','阶段注释','数据与操作标签']):
        cell(root,pid+'-u-row-'+str(n),label,14,620+n*46,208,40,parent=g,fill='#EDF2F7',stroke=C['line'],size=14)
        for z,part in enumerate(['阶段起姿态','加工准备','加工动作','阶段终姿态']):
            cell(root,pid+f'-u-cell-{n}-{z}',part if n==0 else '手工注释' if n==1 else '类型标签',240+z*(w-290)/4,620+n*46,(w-310)/4,40,parent=g,fill=C['white'],stroke=C['line'],size=14,align='center',metadata={'clock':'normalized_u','binding_mode':'phase_local','phase_id':'auto','source_kind':'presentation','affects_production':'false'})
    text(root,pid+'-appearance-cycle-note','循环预览只重复外观。实际生产按物料、阶段资源和真实有效进度驱动；等待时保持当前姿态。',14,774,w-28,54,parent=g,size=14,color=C['muted'])
    button(root,pid+'-appearance-play','外观预览 ▶',14,842,142,32,parent=g)
    button(root,pid+'-appearance-insert','插入图片/姿态',168,842,152,32,parent=g)
    button(root,pid+'-appearance-tag','添加标签',332,842,122,32,parent=g)
    button(root,pid+'-appearance-binding','全部阶段绑定',466,842,154,32,parent=g,link='master-phase-appearance')

def lod_scene(root,pid,x,y,w,h,lod):
    """三档只改变呈现。原生简图保持对象与连线身份，不推断实际队列。"""
    g=group(root,pid+'-lod-scene',x,y,w,h,'#EAF0F4',C['line'])
    for n in range(1,13): cell(root,pid+'-lod-grid-x-'+str(n),'',n*w/13,0,1,h,parent=g,fill='#DCE5ED')
    for n in range(1,8): cell(root,pid+'-lod-grid-y-'+str(n),'',0,n*h/8,w,1,parent=g,fill='#DCE5ED')
    text(root,pid+'-lod-head',('简化档 · 轮廓、方向与对象身份' if lod=='simplified' else '区域档 · 只聚合已编组的线，未编组对象保留'),14,12,w-28,32,parent=g,size=18,bold=True)
    specs=[('B01','电子件',.035,.27),('B02','结构件',.035,.65),('KIT','齐套缓存',.21,.45),('A01','装配',.39,.45),('I01','检验',.58,.45),('P01','包装',.75,.45),('FG01','成品待入库',.9,.45),('R01','返工',.58,.77)]
    nodes={}
    if lod=='simplified':
        for key,name,xx,yy in specs:
            ww=104 if key!='FG01' else 98
            node=cell(root,pid+'-lod-'+key,'',xx*w,yy*h,ww,70,parent=g,fill='#D6E1E8',stroke=C['blue'] if key=='A01' else '#7893A9',extra='container=1;rounded=1;arcSize=5;strokeWidth=2;',metadata={'object_id':key,'presentation_lod':lod,'actual_queue':'unknown'},link='master-device-detail' if key=='A01' else None)
            nodes[key]=node
            cell(root,pid+'-glyph-'+key,'',16,15,ww-32,30,parent=node,fill='#F8FAFC',stroke='#7893A9')
            if key in ['A01','R01']: cell(root,pid+'-glyph-arm-'+key,'',52,4,12,24,parent=node,fill='#C6A653',stroke='#927834')
            elif key=='I01': cell(root,pid+'-glyph-eye','',43,19,18,18,parent=node,fill='#3D748D',extra='ellipse;')
            elif key=='P01': cell(root,pid+'-glyph-box','',40,17,28,25,parent=node,fill='#D8B88B',stroke='#A18A67')
            text(root,pid+'-lod-label-'+key,name+' '+key,-16,-34,ww+32,28,parent=node,size=14,bold=key=='A01',align='center')
        for n,(a,b) in enumerate([('B01','KIT'),('B02','KIT'),('KIT','A01'),('A01','I01'),('I01','P01'),('P01','FG01')]): edge(root,pid+'-lod-flow-'+str(n),nodes[a],nodes[b],parent=g)
        edge(root,pid+'-lod-rework',nodes['I01'],nodes['R01'],'返工路径',C['red'],g,.5,1,.5,0)
        edge(root,pid+'-lod-return',nodes['R01'],nodes['I01'],'复检',C['cyan'],g,1,.5,1,.8)
        cell(root,pid+'-lod-selected','A01 仍被选中 · 单击打开详情',.32*w,.21*h,326,30,parent=g,fill=C['white'],stroke=C['blue'],size=14,align='center',link='master-device-detail')
    else:
        for key,name,xx,yy in specs:
            if key not in ['B01','B02','FG01']: continue
            nodes[key]=cell(root,pid+'-region-un-grouped-'+key,name+' / '+key,xx*w,yy*h,104,64,parent=g,fill=C['white'],stroke='#7893A9',size=14,align='center',metadata={'object_id':key,'aggregation':'not_grouped'})
        region=cell(root,pid+'-region-line','',.24*w,.35*h,.57*w,150,parent=g,fill='#D7E7F5',stroke=C['blue'],extra='container=1;rounded=1;arcSize=6;strokeWidth=2;',metadata={'group_id':'LINE01','members':'KIT,A01,I01,P01,R01','selected_object_id':'A01','quantities_source':'none'})
        text(root,pid+'-region-title','总装检查与包装线 / LINE01',12,12,.57*w-24,32,parent=region,size=20,bold=True)
        text(root,pid+'-region-chain','齐套 → 装配 → 检验 → 包装　↺ 返工 / 复检',12,51,.57*w-24,32,parent=region,size=16)
        text(root,pid+'-region-values','加工、等待、接受量：尚无实际Run',12,94,.57*w-200,32,parent=region,size=14,color=C['muted'])
        button(root,pid+'-region-members','查看 5 个成员',.57*w-182,94,168,32,parent=region,link='master-simplified')
        for a,b in [('B01',region),('B02',region)]: edge(root,pid+'-region-in-'+a,nodes[a],b,parent=g)
        edge(root,pid+'-region-out',region,nodes['FG01'],parent=g)
        members=group(root,pid+'-members',x+.32*w,y+.65*h,.45*w,174,C['white'],C['line'])
        text(root,pid+'-members-title','成员入口 · 已选中 A01 身份不变',8,2,.45*w-16,30,parent=members,size=16,bold=True)
        for n,(key,name) in enumerate([('KIT','齐套缓存'),('A01','装配'),('I01','检验'),('P01','包装'),('R01','返工')]):
            cell(root,pid+'-member-'+key,key+' · '+name,8,36+n*26,.45*w-16,24,parent=members,fill=C['tint'] if key=='A01' else C['white'],size=14,metadata={'object_id':key},link='master-device-detail' if key=='A01' else None)
    text(root,pid+'-lod-foot','缩放只改变显示精细度；不进入设备页、不切多厂房、不改变对象数量或计算精度。',12,h-42,w-24,30,parent=g,size=14,color=C['muted'])

def view_menu(root,pid):
    g=group(root,pid+'-view-popup',130,62,366,412,C['white'],C['line'])
    text(root,pid+'-view-title','视图 → 相机 / 显示 / 时间比例',6,4,354,30,parent=g,size=15,bold=True)
    options=[('相机 · 固定斜俯视',None),('相机 · 背面 / 底部 / 近景',None),('✓ 详细档（手动）','master-main'),('简化档','master-simplified'),('区域档','master-region'),('□ 自动 LOD · 独立开关',None),('时间轴比例 · 不更改生产时间',None),('界面文字 150% · 不缩放相机','master-text-scale'),('显示/关闭面板 → 窗口菜单','master-window-menu')]
    for n,(label,target) in enumerate(options): cell(root,pid+'-view-item-'+str(n),label,4,39+n*34,358,32,parent=g,size=15,link=target,metadata={'command_scope':'UI_presentation_only'})
    text(root,pid+'-view-rule','缩放不打开设备详情，不切换画布。',8,356,350,46,parent=g,size=14,color=C['muted'])

def conflict(root,pid):
    g=group(root,pid+'-conflict-dialog',590,305,740,382,C['white'],C['line'])
    text(root,pid+'-conflict-title','当前工程已更新 · 尚未应用的字段仍保留',16,8,708,42,parent=g,size=22,bold=True)
    text(root,pid+'-conflict-source','A01 / 基础修订 r7；当前修订 r8。先核对差异，不能直接覆盖。',16,58,708,38,parent=g,size=15)
    for n,(label,old,new,draft) in enumerate([('字段','r7 原值','r8 当前值','本次未应用'),('位置 X / m','6.0','6.4','6.2'),('绑定操作员','OP01','OP02','OP01（未改）')]):
        for j,(v,ww) in enumerate(zip([label,old,new,draft],[190,155,155,192])):
            xx=16+sum([190,155,155,192][:j])
            cell(root,pid+f'-conflict-field-{n}-{j}',v,xx,109+n*37,ww,37,parent=g,fill='#EDF2F7' if n==0 else C['white'],stroke=C['line'],size=14,metadata={'form_state':'preserved_not_applied','base_revision':'r7'})
    text(root,pid+'-conflict-rule','重新核对后由用户确认合并；当前不提交命令。关闭提示仍保留表单。\n放弃本次更改只丢弃未应用字段，不回滚 r8 或任何运行。',16,238,708,60,parent=g,size=14,color=C['muted'])
    button(root,pid+'-conflict-keep','留在当前',16,320,160,36,parent=g)
    button(root,pid+'-conflict-discard','放弃本次更改',190,320,180,36,parent=g)
    button(root,pid+'-conflict-review','重新核对当前版本',454,320,270,36,parent=g,primary=True)

def phase_appearance(root,pid,x,w):
    g=group(root,pid+'-phase-editor',x,108,w,892,C['white'],C['line'])
    text(root,pid+'-phase-head','设备创作 · A01 / 多阶段表现绑定',14,8,w-196,36,parent=g,size=20,bold=True)
    button(root,pid+'-phase-back','返回流水线',w-166,10,150,parent=g,link='master-main')
    text(root,pid+'-phase-caption','表现按阶段有效进度驱动，阶段等待保持姿态；图像时长不修改真实工时。',14,53,w-28,32,parent=g,size=15,color=C['muted'])
    for n,(lab,val) in enumerate([('绑定模式','phase_local / 按阶段'),('阶段选择','auto / 自动加工'),('工时来源','冻结 PhaseDefinition.duration · 未提供'),('有效进度','phase_work_intervals · 尚无真实Run')]): field(root,pid+'-phase-setting-'+str(n),lab,val,14,104+n*36,w-28,g,readonly=n>1)
    section(root,pid+'-phase-table-head','分阶段片段 · 排序与比例只作用于所选阶段',14,263,w-28,parent=g)
    heads=['phase_id','归一化范围','外观片段','过渡 / 标签']
    widths=[170,218,390,w-806]
    rows=[heads,['load','[0.00, 1.00]','取料图片 → 上料结束姿态','状态 / 配置标签'],['auto','[0.00, 0.25)','加工前姿态','瞬时切换'],['auto','[0.25, 0.75)','加工动图 / 已声明部件动作','阶段短循环'],['auto','[0.75, 1.00]','加工完成姿态','数据读数只读'],['unload','[0.00, 1.00]','放料图片 → 待移交姿态','当前阶段结束点']]
    for n,row in enumerate(rows):
        xx=14
        for j,(v,ww) in enumerate(zip(row,widths)):
            cell(root,pid+f'-phase-segment-{n}-{j}',v,xx,305+n*38,ww,38,parent=g,fill='#EDF2F7' if n==0 else C['tint'] if n==2 else C['white'],stroke=C['line'],size=14,metadata={'source_kind':'presentation','binding_mode':'phase_local','affects_production':'false'})
            xx+=ww
    section(root,pid+'-phase-axis-head','auto 阶段的外观 / 注释 / 标签 · u = 0 → 1',14,568,w-28,parent=g)
    for n,t in enumerate(['0.00','0.25','0.50','0.75','1.00']): text(root,pid+'-phase-tick-'+str(n),t,200+n*(w-270)/4,603,70,28,parent=g,size=14)
    for n,name in enumerate(['外观片段','阶段注释','类型标签']):
        cell(root,pid+'-phase-row-'+str(n),name,14,642+n*38,168,32,parent=g,fill='#EDF2F7',stroke=C['line'],size=14)
        for j,name2 in enumerate(['前姿态','加工动图','终姿态']):
            frac=[.25,.5,.25][j]; start=[0,.25,.75][j]
            cell(root,pid+f'-phase-track-{n}-{j}',name2 if n==0 else '手工注释' if n==1 else 'ratio / enum',200+start*(w-228),642+n*38,frac*(w-228),32,parent=g,fill=C['white'],stroke=C['line'],size=14,align='center',metadata={'clock':'phase_local_u','phase_id':'auto','source_kind':'presentation','affects_production':'false'})
    text(root,pid+'-phase-limit','单阶段正工时可用 cycle_normalized；多阶段没有统一 duration。零时长取完成事件，未知不除算。\n循环观看仅属于编辑器；等待不推进有效进度，不增加正式产量。',14,778,w-28,56,parent=g,size=14,color=C['muted'])
    button(root,pid+'-phase-insert','插入图片 / 姿态',14,846,172,30,parent=g)
    button(root,pid+'-phase-tags','数据与操作标签',200,846,172,30,parent=g)
    button(root,pid+'-phase-cycle','打开 auto 外观编辑',w-210,846,196,30,parent=g,link='master-device-appearance')

def text_scale(root,pid):
    """文字放大示例重排属性内容，不将整张画面一起放大。"""
    right_id=pid+'-right'
    for node in list(root):
        c=node if node.tag=='mxCell' else node.find('mxCell')
        if c is not None and c.get('parent')==right_id: root.remove(node)
    section(root,pid+'-scale-title','对象属性 · 文字 150%',0,0,400,parent=right_id)
    text(root,pid+'-scale-selected','装配工位 A01',16,48,368,42,parent=right_id,size=24,bold=True)
    button(root,pid+'-scale-detail','设备详情',16,100,170,44,parent=right_id,link='master-device-detail')
    for n,(lab,value) in enumerate([('实例ID · 只读','A01'),('定义版本 · 只读','assembly.station @ 1.0.0'),('位置 X · m','6.0'),('操作员角色','operator → OP01'),('阶段工时 · s','未填写')]):
        text(root,pid+'-scale-label-'+str(n),lab,16,170+n*110,368,36,parent=right_id,size=21)
        cell(root,pid+'-scale-value-'+str(n),value,16,210+n*110,368,44,parent=right_id,fill=C['white'],stroke=C['red'] if n==4 else C['line'],size=21,metadata={'ui_text_scale':'1.5','object_id':'A01'})
    text(root,pid+'-scale-scroll-help','其余字段沿面板滚动访问；\n不缩放相机，不隐藏提交入口。',16,744,352,76,parent=right_id,size=20,color=C['muted'])
    button(root,pid+'-scale-apply','应用更改',16,840,172,44,parent=right_id,disabled=True)
    button(root,pid+'-scale-discard','放弃更改',202,840,182,44,parent=right_id,disabled=True)
    cell(root,pid+'-scale-scroll','↕',382,170,16,552,parent=right_id,size=20,align='center')

def detail(root,pid,x,w):
    g=group(root,pid+'-detail-workspace',x,108,w,892,C['white'],C['line'])
    text(root,pid+'-detail-head','设备详情 · A01 / 实例配置',12,8,w-240,36,parent=g,size=18,bold=True)
    button(root,pid+'-detail-back','返回流水线',w-156,10,142,parent=g,link='master-main')
    text(root,pid+'-detail-origin','同工程 r1 / 方案 A；返回保留厂房、相机、选择与时间轴位置。',12,50,w-24,28,parent=g,size=14,color=C['muted'])
    section(root,pid+'-detail-identity','定义引用与唯一行为来源',12,89,w-24,parent=g)
    field(root,pid+'-detail-module','设备定义','assembly.station @ 1.0.0',20,127,w-40,g,'F01-02','DeviceInstance.module_ref',True)
    field(root,pid+'-detail-behavior','行为来源','工艺定义引用 · assembly.rule @ 1.0.0',20,160,w-40,g,'F03-01','DeviceInstance.operation_binding',True)
    field(root,pid+'-detail-params','公开参数','本示例未声明可覆盖参数',20,193,w-40,g,'F03-02','DeviceInstance.configuration',True)
    section(root,pid+'-detail-ports','端口 / 类型 / 单位 / 接料容量',12,238,w-24,parent=g)
    vals=[['稳定端口ID','方向','载荷类别 / 类型','单位','容量 / 依据'],['kit_in','输入','物品 / 套件','件','未填写 / 人工声明'],['assembled_out','输出','物品 / 半成品','件','未填写 / 人工声明']]
    widths=[180,90,260,80,max(160,w-650)]
    for n,row in enumerate(vals):
        xx=20
        for z,(val,ww) in enumerate(zip(row,widths)):
            cell(root,pid+f'-ports-{n}-{z}',val,xx,276+n*32,ww,32,parent=g,fill='#EDF2F7' if n==0 else C['white'],stroke=C['line'],size=14,metadata={'ui_field_id':'F03-03','canonical_binding':'DeviceDefinition.ports','design_example':'true'})
            xx+=ww
    section(root,pid+'-detail-resources','阶段与真实资源角色',12,395,w-24,parent=g)
    vals=[['phase_id / 阶段','工时 / s','来源','所需角色 → 实例','占用规则'],['load / 上料','未填写','人工声明','operator → OP01','按阶段占用'],['auto / 自动加工','未填写','未取得','无人工角色','设备保持占用'],['unload / 下料','未填写','人工声明','operator → OP01','按阶段占用']]
    widths=[220,130,180,270,max(180,w-840)]
    for n,row in enumerate(vals):
        xx=20
        for z,(val,ww) in enumerate(zip(row,widths)):
            cell(root,pid+f'-phases-{n}-{z}',val,xx,433+n*32,ww,32,parent=g,fill='#EDF2F7' if n==0 else C['white'],stroke=C['line'],size=14,metadata={'ui_field_id':'F14-02' if z<3 else 'F03-05','canonical_binding':'PhaseDefinition.duration' if z==1 else 'PhaseDefinition.phase_resources;DeviceInstance.role_bindings.resources','design_example':'true'})
            xx+=ww
    field(root,pid+'-resource-calendar','人员日历','OP01 → CAL01 / 0–300 s（人工示例）',20,574,w-40,g,'F14-01','ResourceInstance.calendar_id;Calendar.intervals;Calendar.source',True)
    field(root,pid+'-media-readonly','表现资产','独立图像素材 / 不推断工时、容量或库存',20,609,w-40,g,'F03-04','DeviceInstance.presentation_override',True)
    section(root,pid+'-detail-error-heading','缺项定位与处理',12,656,w-24,parent=g)
    text(root,pid+'-detail-error','阶段工时尚未声明：补充 load / auto / unload 后，再检查当前用途。未知不填 0。',20,691,w-40,42,parent=g,size=15,color=C['red'])
    text(root,pid+'-detail-apply-rule','改定义：需显式进入定义编辑；改本实例：只写当前实例参数/角色绑定。保存草稿不等于可执行。',20,747,w-40,45,parent=g,size=14,color=C['muted'])
    button(root,pid+'-detail-apply','应用更改',w-245,831,110,parent=g,disabled=True)
    button(root,pid+'-detail-discard','放弃更改',w-124,831,110,parent=g,disabled=True)

def dirty(root,pid):
    g=group(root,pid+'-leave-dialog',630,380,660,300,C['white'],C['line'])
    section(root,pid+'-leave-title','离开前处理未应用字段',0,0,660,parent=g)
    text(root,pid+'-leave-text','A01的位置字段已修改，尚未提交到当前工程修订。<br>这是表单更改，不是正在运行的计算任务。',20,46,620,74,parent=g,size=17)
    text(root,pid+'-leave-help','应用成功才离开；冲突/错误时保留字段与当前页。<br>工程保存是独立步骤，不能把“应用”当作已写入磁盘。',20,138,620,62,parent=g,size=15,color=C['muted'])
    button(root,pid+'-leave-apply','应用后离开',20,240,178,parent=g,primary=True)
    button(root,pid+'-leave-discard','丢弃后离开',220,240,178,parent=g,link='master-main')
    button(root,pid+'-leave-stay','留在当前',420,240,218,parent=g,link='master-device-detail')

shell('master-main','M01 · 工业工作台 / 紧凑主轴')
shell('master-left-collapsed','M02 · 左侧收起 / 中央扩展',32,400)
shell('master-right-collapsed','M03 · 右侧收起 / 中央扩展',256,32)
shell('master-both-collapsed','M04 · 两侧收起 / 大画布',32,32)
shell('master-wide-panels','M05 · 左右调宽 / 状态保留',320,480)
shell('master-device-detail','M06 · 设备详细字段 / 缺项定位',256,400,'detail')
shell('master-dirty-leave','M07 · 未应用更改 / 离开选择',256,400,'dirty')
shell('master-timeline-hidden','M08 · 时间轴隐藏 / 保留描述',show_timeline=False)
shell('master-description-hidden','M09 · 描述隐藏 / 时间轴铺满',show_description=False)
shell('master-lower-hidden','M10 · 底部双面板隐藏 / 大画布',show_timeline=False,show_description=False)
shell('master-window-menu','M11 · 窗口菜单 / 面板显示关闭',kind='window_menu')
shell('master-timeline-hidden-menu','M12 · 时间轴关闭后的窗口菜单',kind='window_menu',show_timeline=False)
shell('master-description-hidden-menu','M13 · 描述关闭后的窗口菜单',kind='window_menu',show_description=False)
shell('master-lower-hidden-menu','M14 · 双面板关闭后的窗口菜单',kind='window_menu',show_timeline=False,show_description=False)
shell('master-axis-expanded','M15 · 主轴展开 / 设备与共享资源',dock_height=320)
shell('master-line-axis','M16 · 流水线细化 / 裁窗保身份',dock_height=320,timeline_mode='actual',axis_scope='line')
shell('master-device-axis','M17 · 设备运行明细 / 缺失历史',dock_height=320,timeline_mode='actual',axis_scope='device')
shell('master-input-drag','M18 · 输入多选 / 吸附与联动预览',dock_height=320)
shell('master-multi-hall','M19 · 多厂房画布 / 批次供货',kind='multi_hall')
shell('master-device-appearance','M20 · 设备表现创作 / 独立u轴',kind='appearance')
shell('master-simplified','M21 · 简化显示 / 身份与流向',lod='simplified')
shell('master-region','M22 · 区域显示 / 成员入口',lod='region')
shell('master-view-menu','M23 · 视图菜单 / 相机与显示',kind='view_menu')
shell('master-revision-conflict','M24 · 修订冲突 / 未应用字段保留',kind='conflict')
shell('master-phase-appearance','M25 · 多阶段表现 / 有效进度绑定',kind='phase_appearance')
shell('master-text-scale','M26 · 文字150% / 属性重排',kind='text_scale')
E.indent(M,space='  ')
E.ElementTree(M).write(HERE/'industrial-workbench-master.drawio',encoding='utf-8',xml_declaration=True)
source=json.loads((I/'首版表单与反馈.json').read_text(encoding='utf-8-sig'))
valid={f['ui_field_id'] for form in source['forms'] for f in form['fields']}
assert all(f['ui_field_id'] is None or f['ui_field_id'] in valid for f in FIELDS)
present_ids={n.get('id') for n in M.iter() if n.get('id')}
FIELDS=[f for f in FIELDS if f['cell_id'] in present_ids]
(HERE/'母版字段与页面映射.json').write_text(json.dumps(dict(document_kind='S04_02_drawio_editable_master_projection',source_forms='../首版表单与反馈.json',timeline_source='../时间轴主轴与子轴完整定义.json',pages=PAGES,field_controls=FIELDS,all_fields_reference_existing_group=True,unit_rule='布局m、秒仅显示/ms持久化、绕Z朝向转换共同Quaternion，不直接写Three.js坐标',actual_runtime=None,accepted_by_user=None),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'pages':len(PAGES),'mapped_controls':len(FIELDS),'source':str(HERE/'industrial-workbench-master.drawio')},ensure_ascii=False))
