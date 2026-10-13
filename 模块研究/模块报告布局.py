"""模块报告的正文顺序与配图布局；原始研究定义仍由主生成器读取。"""
from pathlib import Path
import json, re, html, io
from PIL import Image as PILImage
from reportlab.pdfgen import canvas
from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.platypus import Paragraph, Table, TableStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.lib.utils import ImageReader
from pypdf import PdfReader
from 厂房搭建案例报告 import render_factory_build_case
from 业务设计报告章节 import render_business_design
from 投用与专业报告章节 import render_readiness_and_professional
from 设备创作报告章节 import render_device_authoring
from 路径维修报告章节 import render_logistics_maintenance
from 模板交接报告章节 import render_template_handoff
from 九类模块报告章节 import render_category_contracts

MODULE_NAMES = {
 'R17-CAM':'数控加工工程模块','R20-PACKAGING':'纸包装结构与成型工程模块',
 'N21':'电子装联准备与执行模块','R18-PANEL':'控制柜设计与制造交付模块',
 'R17-ROBOT_OLP':'机器人作业编程模块','N20':'设备行为与控制联调模块',
 'N22':'仓储布局与作业规划模块','R19-TOLERANCE':'公差与装配条件模块',
 'N09':'包装视觉检验工程模块','R13':'过程质量分析与改进模块',
 'R14-PRINT':'印刷与包材生产准备模块','N19':'纺织用料与裁剪模块',
 'N14':'订单承诺与生产计划模块','R09-CARGO':'发运配载与装卸模块',
 'R11':'技术资料与版本交付模块','N16':'电热能源配置与运行模块',
 'R23-AIR':'压缩空气站网模块','R22-WATER':'用水网络与回用工艺模块',
 'R12-PROCESS':'流程工艺与物料能量模块','R10':'金属炉次配料模块',
 'N17':'材料配方实验决策模块','R21-OPTICS':'成像光学与装调模块',
 'R24-THERMAL':'设备散热与冷却回路模块','N13':'电子功能保持与工程变更模块',
 'R22-MOTOR':'电机驱动与热工况模块','R23-GEAR':'机械传动工程模块',
 'R21-MOLDING':'塑料成型与模具工艺模块','R24-STRUCTURE':'装备结构与载荷模块',
 'R21-NVH':'产品声振与声品质模块','R20-RELIABILITY':'可靠性与试验规划模块',
 'R20-SITE':'厂区竖向与土方配置模块','N18':'建设计量计价与变更模块',
 'R18-LIGHTING':'厂房照明设计模块','R18-ACOUSTICS':'厂房声环境设计模块',
 'R23-HVAC':'厂房暖通与改造模块','R-MAINTENANCE':'设备维护窗口与资源模块',
 'R-REMAN':'再制造与拆检修复模块','R-NESTING':'切割套料与余料模块',
 'R-DATA':'工程数据接入模块','R-COSIM':'专业求解协同模块',
 'R17-COLOR':'印刷色彩与测量模块'}

def normalizer(topics):
    names={t['name']:MODULE_NAMES[t['id']] for t in topics}
    names.update({'公差工程':MODULE_NAMES['R19-TOLERANCE'],
                  '超级平台':'超级工厂','专业课题':'专业模块',
                  '各课题':'各模块','跨课题':'跨模块','逐题':'逐模块'})
    for value in list(names.values()):names.setdefault(value,value)
    pattern=re.compile('|'.join(re.escape(k) for k in sorted(names,key=len,reverse=True)))
    return lambda text: pattern.sub(lambda m:names[m.group()],text)

CORE_STEPS={
 'R17-CAM':[
  ('输入与加工条件','给定零件、毛坯、装夹和实际可用工具；先确认固定刀轴三轴2.5D范围。工艺条件由工程师认可。'),
  ('程序驱动的名义去除','刀路经所支持后置形成NC，再按有限方言解释运动，更新名义材料并与目标比较。动画本身不能替代几何计算。'),
  ('条件改变与残留解释','撤回小直径工具后，窄处残留及工序条件需要重新计算。不能只更换工具图标，也不能以动画掩盖不可达区域。'),
  ('可修改的加工交付','交付可编辑工程、支持子集程序、名义结果及认可条件；下游工位消费该作业与结果，不消费宣传图。')],
 'R20-PACKAGING':[
  ('产品与功能要求','输入产品包络、用途、纸材和连接条件。功能要求决定结构选择，盒型模板只作起点。'),
  ('成型与使用过程','检查有限厚度板片、折带、固定连接和声明的开合/直线取物过程；可见过程是设计示意，实际几何检查仍需实现。'),
  ('产品改版与结构重组','产品改宽后，受限面板和内衬可能改变。重新确认容纳、开合及取物条件，不把改尺寸等同工业防护认证。'),
  ('制造资料与工程接手','同一认可结构导出刀、折、胶层及可编辑过程。生产工位另定义装入、封装、人员与数量，不能由盒子几何推导全部机器控制。')],
 'N21':[
  ('实体资源与订单','输入板型、元件、订单、供料器和实际板位。板位限制、物料认可与实体资源不能由排程生成。'),
  ('准备到实际循环','外部准备、内部切换与板位循环分别安排；供料器从预装到使用后拆卸持续占用，首件仍须人工确认。'),
  ('撤回资源后的改排','备用供料器撤回后，等待与配置会改变；重新组织阶段和实际循环，不虚构第二套资源。'),
  ('原生作业与板件交付','按真实板位产生原生Job、准备表与可编辑campaign；完成的板件数量及时点才能进入总装齐套。')],
 'R18-PANEL':[
  ('认可连接与柜体条件','输入24V DC功能、器件端子和认可连接，再设计柜内布局；连接批准与几何摆放是两件事。'),
  ('连接到物理路由','连接图与布局的路由图共同约束导线，长度含声明端接余量。名义填充与空间检查不能替代通电安全证明。'),
  ('增件或缩柜后的修改','改变I/O模块或柜宽后，冲突、路径与线表关联重算，人工锁定与认可决定可接受方案。'),
  ('安装接线资料交付','交付图纸、端子线表、BOM和可编辑工程。它属于设备建设支线，不是每个产品流经的工位，也不自动产生控制逻辑。')],
 'R17-ROBOT_OLP':[
  ('工作站与工艺约束','定义名义六轴、TCP、工装、认可曲线和允许姿态；工艺段与搬运段分别规定。'),
  ('连续运动与动作事件','逐点有逆解不等于路径连续。轴限、速度、加速度及工艺事件共同约束实际支持的运动。'),
  ('工装变化与失败解释','移动工装或收紧姿态后重新检查。不能为避障偏离认可焊缝/工艺曲线；失败应让用户看得见。'),
  ('程序与工作站接手','可编辑运动、事件及声明条件接入工厂工位，再影响资源占用和移交；现场标定与加工质量仍由实际工作确认。')],
 'N20':[
  ('设备行为与信号','定义设备状态、动作顺序和预设信号类型，连接独立控制程序。媒体与行为各自可编辑。'),
  ('反馈与联锁等待','控制提前触发动作时，传感器反馈或接收条件导致等待。模拟的状态变化不是实机连接证明。'),
  ('修改条件后重跑','修改控制条件或设备行为，复用同一物品与场景重跑。接收条件满足后才允许移交，不能强制跳过规则。'),
  ('行为工程与共同编辑','交付设备包、信号映射、时间轴/标签与示例；人工和Agent编辑同一结构，实际协议与工具能力另按实现声明。')]}

class ModuleReport:
    def __init__(self,base,date,normalize):
        self.base,self.date,self.normalize=base,date,normalize
        self.path=base/'输出/报告/超级工厂_九类模块与完整业务设计报告.pdf'
        self.path.parent.mkdir(parents=True,exist_ok=True)
        self.c=canvas.Canvas(str(self.path),pagesize=A4,pageCompression=1)
        self.c.setTitle('超级工厂：模块设计与工程整合报告')
        self.c.setAuthor('ICNSD-KEL 项目研究')
        self.w,self.h=A4;self.left=43;self.width=self.w-86;self.n=0
        self.md=[];self.pageinfo=[];self.illustrations=[];self.current_title='';self.image_cache={}
        self.styles={k:ParagraphStyle(k,fontName='CNB' if k in ('lead','h2') else 'CN',fontSize=size,leading=leading,wordWrap='CJK',textColor=colors.HexColor(color)) for k,size,leading,color in [('body',10.2,17,'#253443'),('small',8.7,14,'#586676'),('h2',12,19,'#126679'),('lead',13,22,'#163343'),('table',9,14.5,'#253443')]}
    def page(self,title,tag='模块研究与设计'):
        title=self.normalize(title);tag=self.normalize(tag)
        if self.n:self.c.showPage()
        self.n+=1;self.y=self.h-99;self.current_title=title
        self.c.setFillColor(colors.HexColor('#126679'));self.c.rect(0,self.h-8,self.w,8,fill=1,stroke=0)
        self.c.setFillColor(colors.HexColor('#61707e'));self.c.setFont('CN',8.5)
        self.c.drawString(43,self.h-30,'超级工厂 | 模块设计与工程整合报告');self.c.drawRightString(self.w-43,self.h-30,self.date)
        size=min(19,19*self.width/max(1,pdfmetrics.stringWidth(title,'CNB',19)))
        self.c.setFont('CNB',size);self.c.setFillColor(colors.HexColor('#153543'));self.c.drawString(43,self.h-62,title)
        self.c.setFont('CN',8.5);self.c.setFillColor(colors.HexColor('#61707e'));self.c.drawString(43,self.h-82,tag)
        self.c.setStrokeColor(colors.HexColor('#d8e1e7'));self.c.line(43,40,self.w-43,40)
        self.c.setFont('CN',7.7);self.c.drawString(43,25,'研究与设计版 | AI图为设计示意，功能状态以正文为准');self.c.drawRightString(self.w-43,25,str(self.n))
        key=f'page{self.n}';self.c.bookmarkPage(key);self.c.addOutlineEntry(title,key,0,False)
        self.pageinfo.append(dict(page=self.n,title=title));self.md+=['\n## '+title+'\n']
    def draw(self,obj,gap=9):
        w,h=obj.wrap(self.width,10000)
        if self.y-h<54:self.page(self.current_title.removesuffix('（续）')+'（续）')
        if self.y-h<54:raise RuntimeError(f'oversized content on page {self.n}')
        obj.drawOn(self.c,self.left,self.y-h);self.y-=h+gap
    def p(self,text,style='body',markup=False):
        text=self.normalize(text)
        self.draw(Paragraph(text if markup else html.escape(text),self.styles[style]),10 if style=='body' else 7);self.md.append(text)
    def heading(self,text):
        if self.y<135:self.page(self.current_title.removesuffix('（续）')+'（续）')
        self.p(text,'h2')
    def table(self,heads,rows,ratio=None):
        rows=[[self.normalize(str(v)) for v in row] for row in [heads]+rows]
        cells=[[Paragraph(html.escape(v),self.styles['table']) for v in row] for row in rows]
        t=Table(cells,colWidths=[self.width*x for x in ratio] if ratio else [self.width/len(heads)]*len(heads))
        t.setStyle(TableStyle([('BACKGROUND',(0,0),(-1,0),colors.HexColor('#e8f0f3')),('VALIGN',(0,0),(-1,-1),'TOP'),('LEFTPADDING',(0,0),(-1,-1),7),('RIGHTPADDING',(0,0),(-1,-1),7),('TOPPADDING',(0,0),(-1,-1),7),('BOTTOMPADDING',(0,0),(-1,-1),7),('LINEBELOW',(0,0),(-1,-1),.4,colors.HexColor('#d9e2e8'))]))
        self.draw(t,14);self.md+=[' | '.join(rows[0]),' | '.join(['---']*len(heads))]+[' | '.join(row) for row in rows[1:]]
    def image(self,path,maxheight=510,panel=None,panel_bounds=None,caption='AI生成的界面设计示意；示例输入、曲线与检查状态不代表已计算结果。'):
        if path not in self.image_cache:
            with PILImage.open(path) as im:
                iw,ih=im.size;data=io.BytesIO()
                im.convert('RGB').save(data,format='JPEG',quality=96,subsampling=0)
            data.seek(0);self.image_cache[path]=(iw,ih,ImageReader(data))
        iw,ih,reader=self.image_cache[path]
        box=[0,0,1,1] if panel is None else [(0,0,.498,.498),(.502,0,1,.498),(0,.502,.498,1),(.502,.502,1,1)][panel]
        if panel_bounds is not None:
            px,py,qx,qy=panel_bounds[panel];box=[px/iw,py/ih,qx/iw,qy/ih]
        sx,sy,ex,ey=box;pw,ph=(ex-sx)*iw,(ey-sy)*ih
        k=min(self.width/pw,maxheight/ph);vw,vh=pw*k,ph*k;x=self.left+(self.width-vw)/2;y=self.y-vh
        if y<54:raise RuntimeError(f'image overflow page {self.n}')
        self.c.saveState();clip=self.c.beginPath();clip.rect(x,y,vw,vh);self.c.clipPath(clip,stroke=0,fill=0)
        self.c.drawImage(reader,x-sx*iw*k,y-(1-ey)*ih*k,width=iw*k,height=ih*k)
        self.c.restoreState();self.y=y-9
        self.p(caption,'small');self.md.append(f'![设计示意]({path.as_posix()})'+(f'（图集中第{panel+1}块）' if panel is not None else ''))
        self.illustrations.append(dict(page=self.n,asset=str(path.relative_to(self.base)) if path.is_relative_to(self.base) else str(path),panel=panel))
    def finish(self):
        self.c.save()
        (self.base/'超级工厂_选题与工程整合报告.md').write_text('\n\n'.join(self.md)+'\n',encoding='utf-8')
        (self.base/'报告页目录.json').write_text(json.dumps(self.pageinfo,ensure_ascii=False,indent=2),encoding='utf-8')
        (self.base/'报告配图目录.json').write_text(json.dumps(self.illustrations,ensure_ascii=False,indent=2),encoding='utf-8')
        return self.path

def render_module_section(r,topics,manifest,base):
    images={}
    for a in manifest:
        if a['key'].startswith('CORE_'):images[a['ids'][0]]=(a,None)
        else:
            for i,key in enumerate(a['ids']):images[key]=(a,i)
    for t in topics:
        r.page(t['name'],t['category'].split('_',1)[1]+' | '+t['status'])
        r.p(t['user'],'lead')
        r.p('当前首版归属：纸包装真实专业闭环，待实现。' if t['id']=='R20-PACKAGING' else '当前首版归属：专业研究储备；真实内核后置。Agent生成只能采用实际支持的声明过程或提供者。','small')
        for title,key in [('完整工作链','chain'),('核心处理与专业责任','core'),('输出与下游接入','interface'),('独立价值与组合价值','value'),('已有覆盖','baseline'),('本次范围与待补能力','boundary')]:
            r.heading(title);r.p(t[key])
        r.heading('真实业务的成果消费者')
        r.p('工程角色：'+t['integration_role']+'；消费者：'+t['consumer']+'；成果类型：'+t['artifact_type']+'。')
        r.p('最低业务字段：'+'、'.join(t['required_payload_fields'])+'。','small')
        r.p('人工认可、成果绑定与实际执行分别保存。当前提供者仍planned；完整专业范围沿用原定义，目录不当作运行插件。','small')
        r.p('课程对应：'+t['course']+'。原始定义与阅读位置见模块资料映射；研究归入模块不等于已经实现。','small')
        a,panel=images[t['id']];asset=base/a['filename']
        if t['id'] in CORE_STEPS:
            for i,(label,text) in enumerate(CORE_STEPS[t['id']]):
                r.page(t['name']+'：'+label,'专业工作台 / 设计预览')
                r.image(asset,panel=i);r.p(text)
        else:
            r.page(t['name']+'：可见的处理过程','专业工作台 / 设计预览')
            r.image(asset,panel=panel,panel_bounds=a.get('panel_bounds_pixels'));r.p('画面用于表达本模块的输入、处理对象与可修改结果。'+t['chain'])
            r.p('需要实现的专业贡献：'+t['core'],'small')

def render_composition_section(r,platform):
    r.page('超级工厂：从模块到完整方案','以下为整合设计；前文模块保持各自专业责任')
    r.p('把可修改的专业工程接到同一工厂方案里。用户从简化设备起步，逐步补充细节；改变一个生产条件后，看见物品、资源、等待和交付的连带变化，再交给别人继续修改。','lead')
    r.p('模块组合要交换实际被下游使用的工程对象、作业和条件。把多个工作台放进菜单、拼接图片或共用聊天框，尚未形成这条工作链。')
    r.p('41个工业专业方向为研究储备；当前九类开放平台/模板目标生成与纸包装专业闭环才是首版必做，其他真实专业内核后置。未形成运行插件或实际Agent工具。')
    r.page('到底什么被模块化了')
    r.table(['层次','可独立替换或贡献的内容','组合时必须保留'],[
     ['对象与资源','物品、设备、人员/移动资源、人工任务、物流关系','身份、单位、数量、可用条件、输入输出与当前归属。'],
     ['专业工程','加工、包装、装联、电气等专业工具及其认可作业','计算支持范围、参数、几何/程序/工艺结果及人工确认。'],
     ['运行规则','转换、齐套装配、缓存、分流、检查、返工、运输','输入输出、资源占用、持续时间、前置条件和完成事件。'],
     ['表现与组合','插图/GIF/模型、时间轴与标签、流水线/厂房模板','表现不代替行为；模板保存成员和边界连接，不复制成员库存。']], [.19,.41,.40])
    r.p('产品模块库按九类组织：物品、固定设备、资源、人工任务、工艺规则、物流与供应、布局与工厂模板、专业计算、表现素材。类别与41个专业研究方向不是同一层概念。')
    r.p('最小设备包可以只有一张插图和一段数据流规则；升级为图片序列、GIF、三维模型或Agent生成的原生外观时，原有端口与行为继续使用。行为细化则需明确映射并重新彩排。')
    r.page('完整工程工作链：六部分接力')
    r.table(['部分','用户与系统完成什么','交给下一部分的内容'],[
     ['1 输入','明确产品、订单、物料、场地、设备、人员和认可条件','带单位/来源范围的工程对象与要求。'],
     ['2 设计计算','按需调用专业模块、配置产线及供应关系','候选几何、程序、工艺、资源与物流方案。'],
     ['3 确认','人检查支持范围与未满足条件，选定可接受方案','已认可作业、接受条件和选定计划。'],
     ['4 运行','设备行为与工厂过程执行；处理齐套、占用、运输和返工','同一物品的状态、数量与事件过程。'],
     ['5 比较解释','改变一个条件，重跑同起点场景并解释连带影响','差异、等待原因与待修改工程对象。'],
     ['6 交付复用','保存可编辑工程、资产与说明；另一人重开修改','工程包、受支持专业资产及贡献样例。']], [.18,.45,.37])
    r.p('这六段是作者的完整工作链，不强制每个零件经过六台机器。专业模块可独立使用；已经采购完成的零件可以从相应工位直接进入运行。')
    r.page('参考生产主链：五段与三条输入支线')
    r.table(['生产部分','物品与任务'],[
     ['来料与暂存','毛坯、PCB/元件、采购件、包材进入各自库位和批次。'],
     ['加工或准备','外壳加工、PCB准备/贴装、采购件备料可并行，完成条件分别定义。'],
     ['齐套总装','按BOM、规格、数量和认可条件汇合；缺件与资源忙分别等待。'],
     ['检查与包装','检查分流、必要返工复检，再装入/封装/装箱，保留内容关系。'],
     ['出库与发运','库位、装卸、在途、接收与资源返程分开，目标供应量不当实际收货。']], [.26,.74])
    r.p('参考产品是自建电子控制盒：外壳加工支线 ∥ 电子板支线 ∥ 外购件支线，汇合后总装、检验、包装和交付。PCB可外购；也能从机床或任意合法入口跟随一件物品到指定终点。')
    r.p('控制柜、结构/散热研发、厂区建设、电热气水属于前置或旁接工程。包装结构设计与实际装盒分别处理；连续流量/组分不直接套用逐件数量规则。')
    r.page('两层责任与真正的工程连接')
    r.table(['层','处理对象','输出'],[
     ['专业工程工具','几何、材料、功能、程序、工艺与资源配置','可编辑作业/产品定义、计算结果及适用范围。'],
     ['工厂过程运行','物品、工序、资源、时间、库存与物流','在制/在途/等待、完工与交付关系。']], [.22,.37,.41])
    r.p('加工后的认可几何进入装配条件；实际板位循环完成产生板件；包装工程提供几何/过程与箱件条件；独立控制反馈影响动作与移交。仅交换PDF或图片不完成计算链。')
    r.p('平台协调共同生产事件，详细几何、运动、热/流体内核保留自己的计算方式。接口明确单位、时间边界与接受条件，不强制全部求解器同一步长。')
    r.page('三种通用模板与现行首版边界')
    r.table(['模板','工作链','展示重点'],[
     ['机加工交付','毛坯→受限加工→转移→声明检验→包装出库','名义材料与物品状态变化。'],
     ['外购件总装','采购件→备料→BOM齐套→装配→检验返工→包装','身份、数量、资源和等待。'],
     ['并行制造总装','外壳加工 ∥ PCB准备/贴装 ∥ 外购件→齐套→总装包装','支线完成时点和共享资源造成的连带影响。']], [.21,.53,.26])
    r.p('三模板复用共同事件、物品、资源核心。成型、纺织、配料和膜处理有研究接入位置，但新增材料机制与专业内核后才能成为支持类型，不能凭模板数量声称覆盖所有工厂。')
    r.page('交互：两画布与三级精细度')
    r.table(['入口','操作与含义'],[
     ['流水线画布','常用工作面；单击设备、线组或可设置对象进入详情，放大不进入设备编辑器。'],
     ['详细档','简化设备外观、端口连接、动作/物品状态；一张图表达一条可整体移动的线。'],
     ['简化档','设备轮廓、状态和数量；缩小后更多线进入视野。'],
     ['区域档','整条线或用户定义区域作为整体；能看到全厂，表现简化不改库存。'],
     ['多厂房画布','菜单切换；厂房整体节点与供应连线，设物品、日目标、批量、时间窗、运输与接收能力。'],
     ['对象详情','设备/人工/物流的参数和操作；时间轴、数据标签、热点、媒体或原生部件。']], [.22,.78])
    r.p('编辑时可整体移动流水线，内部相对连接保留，外部连接需复核。运行中的固定设备不自行迁移；人员、AGV、手推车是移动资源。循环观看不重复生产，生产周期须重新检查资源和物品。')
    r.page('跨模块组合产生什么新价值')
    r.table(['组合','实际交换和消费','连带结果'],[
     ['加工＋公差＋装配','认可几何/偏差、装配条件、工序','加工或基准改变后，装配与检验要求变化。'],
     ['装联＋订单＋总装','准备/板位循环、资源持有、完成板件','供料与准备改变会影响齐套和交付。'],
     ['包装＋印刷＋物流','包材几何、数量、封装与装卸关系','改版影响包材准备、等待与发运。'],
     ['电气＋控制＋机器人','认可信号、控制逻辑、动作与资源','反馈改变作业和物品移交。'],
     ['生产＋公用工程','支持工艺的时序需求与服务边界','供给条件与需求互相影响；缺模型只作离线情景。']], [.23,.40,.37])
    r.p('保留三个组合贡献候选：渐进建模而不用重建工程、同一次变更跨层解释影响、人工/Agent/社区共同交付可编辑资产。是否优于完整近邻仍需同任务比较。')
    r.page('完整视图图组：怎样阅读')
    r.p('后续完整附上现有22张设备、流水线、厂房与跨厂设计预览。它们表达可实现的交互目标，来自同一受限离散工厂设计；不是已经部署的产品截图。','lead')
    r.p('设备外观、人工与运输先看局部；流水线与厂房看汇合和简化；多厂房看供应连线。具体示例人数、工时、数量与读数均为场景输入或预设状态，不能充当行业收益。')
    records=json.loads((platform/'设计预览/图片记录.json').read_text(encoding='utf-8-sig'))
    for im in records:
        r.page('超级工厂：'+im['title'],im['level']+' / 已有AI设计预览')
        r.image(platform/'设计预览'/im['filename'],maxheight=510)
        r.p('该图对应两画布与对象详情规格。图内对象实例编号用于选择具体设备或物品，不是课题代号；运行能力由实际实现阶段决定。','small')

def render_project_overview(r,base,platform,topics):
    records=json.loads((platform/'设计预览/图片记录.json').read_text(encoding='utf-8-sig'))
    pictures={v['id']:platform/'设计预览'/v['filename'] for v in records}
    r.page('超级工厂','整体预览与项目介绍 / 模块设计与工程整合报告')
    r.p('把工厂方案画出来、跑一遍、改明白，再交给别人继续设计。','lead')
    r.p('超级工厂定位为开放的工厂方案设计与彩排软件。用户组合设备、人工、物品和运输规则，提前查看产品生产过程、等待原因与方案变化。')
    r.image(pictures['H01_DETAIL'],maxheight=320,
            caption='图1｜自建电子控制盒工厂的整体设计预览。外壳加工、PCB支线和外购件汇入总装包装；AI图中的状态为示例。')
    r.p('用户可以从一张插图和一段规则开始，再加入图片序列、动图、模型或专业计算。设备外观逐步丰富，物品、资源与输入输出关系保持可编辑。')
    r.p('阅读顺序：先看整体使用与图示，再读各专业模块，最后查看完整整合机制、比赛价值、开源与实施安排。当前成果是研究与设计，图示不作产品运行证明。','small')

    r.page('项目介绍：谁用、什么时候用')
    r.p('面向需要快速提出工厂方案的人，也面向需要把专业方案讲清、反复修改和交接的工程师。','lead')
    r.table(['用户','任务触发','希望拿到什么'],[
     ['产业规划与方案讨论人员','新建产线、扩产、改布局或分配厂房任务','可看懂的流程、设备关系、供应安排与方案解释。'],
     ['厂内工程师与集成商','产品、工艺、到料、设备或人员条件改变','可编辑方案、受影响对象、等待原因及重新安排。'],
     ['设备与专业工具贡献者','已有设备、行为、求解工具需要复用','有明确输入输出与例子的设备包、专业工具或模板。']], [.25,.36,.39])
    r.heading('项目实际要处理的工作')
    r.p('用户输入产品与订单，选择或自定义设备，连接工序和运输，设置人员、工时与接收条件。系统按所支持规则彩排，呈现齐套、占用、等待、返工和交付；专业模块提供认可作业或条件。')
    r.heading('组合价值怎样体现')
    r.p('先用简化设备讨论，再逐步补细节；同一次变更可以沿物品、资源和供应关系传到下游；人工与Agent修改同一工程，下一位接手者能继续编辑。价值须落在这条完整任务上。')
    r.p('首版是九类自由创作/社区扩展的Windows桌面平台，保留三种通用离散模板并增加纸箱制造行业目标生成配置。只选纸包装真实专业闭环；其他专业、建设/公用服务和动态交通后置。','small')

    r.page('整体预览：一座厂房怎样生产')
    r.image(pictures['H01_REGIONS'],maxheight=320,
            caption='图2｜同一厂房缩小到区域档后的预览。线组简化为区域，原有成员、连接和库存关系仍保留。')
    r.table(['图中关系','业务含义'],[
     ['外壳加工 ∥ PCB支线 ∥ 外购件','不同部件分别准备，不能用某一支线完成代替全产品齐套。'],
     ['三支线汇入总装包装','按BOM、规格、数量及资源条件开工；缺件或资源忙会产生不同等待。'],
     ['区域与整体产线','常用工作面仍是流水线画布；缩小只简化表现，单击对象进入详细设置。']], [.36,.64])
    r.p('一件采购件可以从来料入口跟到组装产品及包装；一件机加工件也可以从机床开始跟随。装配体与箱内内容保留关系，不能因视图聚合把零件重复计为库存。','small')

    r.page('整体预览：多厂房怎样接力')
    r.image(pictures['NETWORK_SITES'],maxheight=320,
            caption='图3｜多厂房画布中的分布式生产预览。机械、电子与总装基地通过物品供应关系组成产品生产链。')
    r.p('由菜单切换到多厂房画布，把每座厂房作为可输入、输出物品的整体，再设计厂房之间的供应连线。连线可设置物品、日目标、批量、时间窗、运输与装卸、接收能力和返回资源。')
    r.table(['设计决定','彩排需要区分的状态'],[
     ['哪个厂房供应什么','计划供应、已备料、可发运与实际可接收。'],
     ['何时、分几批送达','装载、在途、卸载、接收和资源返程。'],
     ['某支线延期或接收区满','下游齐套等待与运输阻塞，而非直接把日目标当作收货量。']], [.34,.66])
    r.p('图内数量和时间是场景输入示例。厂房可以属于同一基地或分布式基地；实际支持的运输规则决定计算范围。','small')

    r.page('使用预览：从要求到可编辑交付')
    r.image(base/'模块配图/工程Agent业务预览.png',maxheight=300,
            caption='图4｜人工与工程Agent共同修改方案的界面设计。工具调用仍显示计划、未执行状态，结果不由图像生成。')
    r.table(['步骤','用户与软件的工作','可见变化'],[
     ['输入','明确产品、订单、物料、设备和人员','工程对象与要求。'],
     ['设计计算','组合设备，连接工序与厂房供应','产线、连接及候选作业。'],
     ['人工确认','确认工艺、支持范围和接受条件','选定方案与待解决条件。'],
     ['运行','按认可规则执行生产、运输与检查','物品状态、占用和等待。'],
     ['比较解释','改一个条件，重跑同起点方案','影响对象与差异原因。'],
     ['交付复用','保存工程、资产与说明，交给下一人','可重开、继续修改的工程包。']], [.18,.51,.31])
    r.p('图例为采购PCB延期。Agent可准备变更草稿和请求工具，人确认到料与采用方案；实际计算由支持的规划、专业和运行内核承担。','small')

    r.page('自定义预览：设备外观与行为分开')
    r.image(base/'模块配图/CORE_CONTROL.png',maxheight=380,panel=3,
            caption='图5｜设备包、信号、行为、时间轴与素材绑定的设计预览。没有真实设备连接，按钮和能力随实现阶段提供。')
    r.p('最低设备包可以是一张插图和明确的数据流规则。用户设置端口、参数、工序、资源与操作；图片序列可插入、排序和循环观看，再绑定预设数据标签和操作热点。')
    r.p('表现可升级为GIF、模型或可编辑原生部件；Agent与人工使用同一结构。生产完成由运行规则决定，播放完动画不会自动增加产品数量。设备包、规则、素材和厂房模板分别可贡献。')
    r.p('正文后续区分九类模块资产、41个专业研究方向与首版运行范围，并逐项说明哪些输出真正进入下游。','small')

    r.page('阅读地图：整体、模块与整合')
    r.p('先用一次完整的空厂房搭建任务说明模块如何接力，再逐模块解释专业工作，最后回到超级工厂的完整组合设计。','lead')
    counts={c:sum(t['category']==c for t in topics) for c in dict.fromkeys(t['category'] for t in topics)}
    r.table(['模块研究分组','条目'],[[c.split('_',1)[1],n] for c,n in counts.items()],[.8,.2])
    r.p('各模块分别说明用户、完整工作链、核心处理、输出与下游接入、独立与组合价值、已有覆盖和当前范围，并附设计预览。整合部分随后说明六段工作链、模块化分类、生产因果、比赛与开源。')
    r.p('原始研究名称、内部标识和路径仅用于资料映射。41份是研究与设计条目，详细备选与方向观察仍保持区别。','small')


def render_goal_generation(r,base,platform):
    baseline=json.loads((platform/'软件工程/docs/00_项目基线/项目主线.json').read_text(encoding='utf-8'))
    r.page('现行主线：自由搭建与一句话建厂')
    r.p('超级工厂是开放九类业务概念的工业模拟桌面平台。用户可以自己搭产线，也可以让外部Agent按产品、日产目标及场地长宽高，调用行业模板和九类库生成尽量详细的可编辑厂房。','lead')
    r.table(['输入','平台与Agent处理','交付'],[
      ['行业/产品、日产目标、场地及约束','选择原生或已导入社区模板，读取明确版本/参数来源','候选模板、假设/缺项'],
      ['模板工艺图与九类角色','填物品、设备、资源、任务、工艺、物流、厂房/组合、计算和表现','布局、物料连接、资源和排期'],
      ['同一修订的工程','静态场地/引用检查，真实渲染与实际模拟','可见过程、接受量、等待/占用'],
      ['实际初案结果与许可变量','达标直接交付；不足时有据修改后重跑比较或说明不能修正','四格式评估及可继续编辑工程']], [.27,.43,.30])
    r.p('一句话是用户入口，内部允许多次真实工具调用。计划操作或AI截图不代表完成：首版必须在连接的Agent和软件中真正创建对象、运行并交付。','small')
    r.page('行业模板怎样填九类：纸箱制造例')
    r.p('以采购纸板起步的模切/折糊纸箱产品族为首个行业配置；先确认规格和路线。可选印刷→模切→折糊成箱→检验/返工或隔离→打包→成品接受。生产纸箱不同于把已有产品装入箱。')
    r.table(['角色','实际需要填入'],[
      ['物品/设备','板料、箱坯、箱与打包内容；设备端口、周期、批量、占地/高度'],
      ['资源/任务/工艺','人员、日历、准备/装卸、转换/检验/返工规则'],
      ['物流/厂房与组合','库位/供应、固定运输条件、厂房长宽高和直线/U形/并行布局'],
      ['专业计算/表现','所支持纸结构成果及明确绑定；可编辑原生几何或许可素材']], [.32,.68])
    r.p('用户场地L/W/H映射既有工程width_m(X)/depth_m(Y)/height_m(Z)，单位m；包装结构单位mm。模板必须填入实例新ID和显式供应条件，不复制旧运行库存/订单。原厂工艺页PK06仅支持选定路线，没有本项目机器节拍或产能数据。','small')
    r.page('怎样判断达标与修正：目标不是结果')
    r.p('设目标Q、指定观察窗终点H、实际合格接受量A(H)，两者用同一接受位置和计量单位，不限定纸箱。只有输入和支持能力完整、静态布局条件通过且实际模拟A(H)≥Q，才写该模型条件下达标；缺口为max(0,Q−A(H))。')
    r.p('周期t_i秒、每周期b_i个目标计量单位、净可工作H_i秒时，理想机台下界ceil(Q·t_i/(H_i·b_i))只用于初始填充。来料、共享人力、准备、运输与质量条件仍须实际执行。纸箱算术例：净8小时明确给定时，一万箱对应理想平均2.88秒/箱；不是机器实测参数。')
    r.table(['实际情况','显示及下一步'],[
      ['缺模板/日历/工时','保留草案和缺项，不显示达标'],
      ['占地/高度/间距冲突','定位对象并调整布局；不能缩小图标骗检查'],
      ['实际接受量不足','定位缺料/人员/设备/运输条件，在允许变量内有限修正'],
      ['输入齐全且目标满足','初案直接保存和导出，不强制修改、第二次运行或比较']], [.37,.63])
    r.p('初案达标即可交付；修正能力另以不足案例验收，有支持变量则有限修改/重跑比较，否则说明原因。动态交通/机电布线/现场认证后置。当前种子参数待填，实际接受量为空。','small')
    r.page('交付、社区与当前开发状态')
    r.p('目标/来源/尺寸/班次、布局检查、初案RunId和停止原因汇入同源PDF、离线HTML、CSV分表和JSON。初案达标允许无修正RunId/比较；实际修正时才附新旧运行及差异原因。可编辑工程另存，可改设备或供货后重跑；报告不是库存快照。')
    r.p('社区可以贡献一个行业生成配置、九类角色和来源明确的样例参数，不必重写公共内核。generation_profile是templates的可选载荷，原静态模板仍可打开；实际专业提供者不存在时明确待补。')
    first=baseline['first_release']
    r.table(['当前基线','状态'],[
      ['17项需求、18项操作、21选入任务/7后置','设计已同步；实际任务仍planned'],
      ['3通用离散模板＋纸箱制造生成配置','研究/配置草案；完整运行待实现'],
      ['纸包装真实专业闭环','唯一选入专业内核，待实现'],
      ['3厂房/60设备/3000件','验收目标，非硬规模限制或当前性能'],
      ['其他40专业、交通/维修/建设/公用服务','研究保留、当前后置']], [.56,.44])
    r.p('现行以10月30日前备齐比赛提交物为期限。旧专业研究章节中的范围是候选方向的研究版，不改变当前首版选择；旧AI图不完整覆盖本轮目标面板，保留为设计风格参考。','small')


def build_report(base,platform,date,topics,normalize):
    manifest=json.loads((base/'模块配图/图片记录.json').read_text(encoding='utf-8'))
    r=ModuleReport(base,date,normalize)
    render_project_overview(r,base,platform,topics)
    render_goal_generation(r,base,platform)
    render_factory_build_case(r,base,platform)
    render_business_design(r,base,platform)
    render_readiness_and_professional(r,base,platform)
    render_device_authoring(r,base,platform)
    render_logistics_maintenance(r,base,platform)
    render_template_handoff(r,base,platform)
    render_category_contracts(r,base,platform)
    render_module_section(r,topics,manifest,base)
    render_composition_section(r,platform)
    r.page('完整竞品工作链与原创性结论')
    r.table(['近邻','已经覆盖的完整工作','本项目比较焦点'],[
     ['Visual Components','布局/物料流→运行→比较→共同理解方案','从简化资产到细化，再到开放可修改接手。'],
     ['Plant Simulation / AnyLogic','分层工厂与供应网络建模、分析、修改','设备创作到跨厂交付的作者工作负担。'],
     ['JaamSim','开放对象/子模型→复用→编辑保存','声明设备、流程与完整接手，不声称竞品均需写Java。'],
     ['Simul8 Decision Cloud','模型→决策者输入→实验→协作','参数分享与任意设备工程接手分别比较。'],
     ['Open-Industrials','生成/导入设备→布局动画→Agent反馈→重开交付','公开材料未证明完整库存/齐套厂际链，也不据此断言不存在。']], [.24,.40,.36])
    r.p('已有官方材料、案例与公开工程用于桌面比较；本轮未运行竞品。成熟基础功能按已有能力描述，独立实现有价值，但组合优势与新机制尚待实际工程证明。')
    r.page('需求、市场与可量化价值')
    r.p('面向有自建维护能力的集成商、厂内工程师，以及使用模板参与方案讨论的规划人员。真实任务存在不等于所有人都会采用产品。','lead')
    r.p('条件净工时价值：Σ_j N_j·(h_base,j−h_new,j)−H。任务量N、包含输入/核对/修改/接手的工时h，以及迁入/学习/维护H，都需实际口径。当前没有可代入的用户测量，不编估算值。')
    r.p('制造企业数量既不是厂房数也不是可服务客户数；可售服务需要适配组织、真实采用和价格依据。行业总产值不能直接乘任意比例成为项目市场。')
    r.table(['模型可产生的事实','量化的必要条件'],[
     ['加工去除、包装几何、数量与资源','实际实现的内核和明确输入，不继承实切或工业质量。'],
     ['等待、完成时点、交付差异','同起点、观察窗口与改变项，真实彩排后记录。'],
     ['能源/气/水与费用取舍','相同生产服务、单位一致、设备参数及价格有依据。'],
     ['用户净收益与采用空间','区分软件贡献、改造/组织变化和采用维护成本。']], [.40,.60])
    r.page('比赛与大作业对应')
    r.table(['比赛维度','需要真正交付什么'],[
     ['技术创新 30%','展示自有机制在渐进建模、跨层解释与接手中的作用，按完整任务比较近邻。'],
     ['场景落地 30%','可部署版本、明确边界、可见变化、可重开修改；仿真结果与现场效果分开。'],
     ['开源治理 20%','九类资产的贡献样例、代码/素材许可、运行说明与Issue/PR流程。'],
     ['长期发展 20%','新增类型的真实改动、维护责任、采用条件及支持服务，社区规模据实填写。']], [.28,.72])
    r.p('权重沿用赛事资料与用户提供的官方截图，不据此编造自评总分。赛道一以软件能力交付行业方案，贡献必须落实为软件和可复用资产。')
    r.p('设备行为与控制联调模块可实际自主实现MQTT Broker；包装视觉检验工程模块可实际集成ONNX Runtime；其余模块和总平台可选择真正调用工程工具的AI Agent应用。Rust几何/优化不自动属于推理引擎，数据库调用不等于移植。')
    r.page('AI Agent业务与贡献边界')
    r.p('Agent是工程任务助手：读取当前能力与工程、提出对象或作业草稿、调用受支持专业工具、运行彩排、解释可追溯变化，生成可继续编辑的交付。人确认需求、专业条件与采用方案。','lead')
    r.table(['业务','软件中的操作与可见反馈'],[
     ['工程创建','生成物品/设备/端口/工序/供应草稿，画布显示待确认对象。'],
     ['专业任务','调用相应工程内核，显示实际支持结果与失败；不能以模型回答冒充计算。'],
     ['设备创作','生成可编辑原生部件、时间轴和标签，再根据真实渲染预览修订。'],
     ['方案修改','提交变更、调用彩排，定位受影响物品/资源/订单。'],
     ['接手与社区','整理工程包、说明与示例，贡献者决定提交PR，不自动发布。']], [.24,.76])
    r.p('外形采用已有工程窗口中的任务侧栏与工具执行状态，不需要拟人角色。Agent与人工共享对象格式；没有工具实现时只说明规划，不宣传已接入能力。')
    r.page('超级工厂：工程Agent的业务与外形','任务侧栏 / 两画布 / 工具调用设计预览')
    r.image(base/'模块配图/工程Agent业务预览.png',maxheight=360)
    r.p('采购PCB延期时，Agent读取已认可输入、提出可编辑变更，按顺序请求订单规划与工厂彩排，再整理差异和交付草稿。人确认到料条件和采用方案；侧栏中的未执行/未连接与空结果必须如实保留。')
    r.p('同一方式用于其他专业模块：换工具调用加工内核，产品改版调用包装工程，场地变更调用计量或环境工具。每项业务是否支持，由实际工具契约决定；通用文字回答不算完成专业作业。','small')
    r.page('阶段范围：先做完整闭环')
    r.table(['阶段','交付闭环','当前状态'],[
     ['共同运行','物品、工序、资源、齐套、库存、检验返工','任务和声明样例；未运行。'],
     ['两画布与交付','厂内/跨厂供应、同输入比较、工程导出重开','设计及手算预期；未实现。'],
     ['设备与创作','九类资产、时间轴/标签、媒体与原生共同编辑','目录和规格；未形成运行编辑器。'],
     ['模板与社区','三通用模板＋纸箱行业生成配置、九类贡献采用和许可材料','规划与研究；社区运营据实。'],
     ['专业接入','逐个加入认可作业、适用结果与专业内核','41方向资料映射，仅选纸包装专业内核，其他保留研究。']], [.23,.50,.27])
    r.p('现行首版交付平台闭环、纸包装实际计算、真实外部Agent目标生成及四格式评估，10月30日前准备全部比赛材料。三通用模板和纸箱配置用于验收；加工/电子等研究专业不列本次必做。当前17需求/18操作/21任务仍未实现，期限不是完成证明。')
    r.page('开源资产、贡献与设计权利')
    r.p('贡献者可以复用或改进设备包、表现素材、工艺规则、专业工具、人工/物流任务和工厂模板。普通设备包只使用已支持规则；专业计算通过代码贡献，维护者承担版本与示例责任。')
    r.table(['资产','发布时的处理'],[
     ['代码与依赖','按实际文件和组合核对许可；第三方求解器的许可不由本报告统一改写。'],
     ['模型、素材与数据','优先自建或许可明确资产；代码许可不自动覆盖厂家CAD、物性、模型权重或论文原文。'],
     ['界面与设计预览','沿用自有工程框架，不复制竞品品牌/专有素材；AI图明确标示示意。'],
     ['专利相关功能','独立代码不自动排除专利约束；有限公开检索不作自由实施保证。'],
     ['社区接手','manifest、说明、示例、许可与可重开工程；目录区分提案、可运行样例与发布版本。']], [.24,.76])
    r.p('原始研究原件保留本机位置。公开仓库按授权发布自有报告、允许再分发资产与引用链接，不默认分发整篇原文。')
    r.page('历史研究与当前边界')
    r.p('历史的产能能源排程、制造询价与交期、软硬件再工程、液冷采购、只读点表和零售库存方向保留来源。相邻内容用于当前模块的背景，旧局部探针不成为总平台运行证明。')
    r.p('道路、公交、游戏、农业、市政排水和港口等保留原研究，不因共享算法而归入当前离散工厂。连续流、成型和新材料机制必须重新定义对象与专业计算后接入。')
    r.page('资料库与原始依据入口')
    r.p('模块目录JSON/CSV与SQLite保持当前名称、范围、源文件和摘要关系；模块资料映射保存原始研究名称和内部标识，便于追溯。正文按模块名称阅读即可。','lead')
    r.p('统一来源目录引用已有原件与阅读位置，不把记录数当独立文献数，也不把付费摘要当全文。历史数字、模型输入、厂家案例与本项目结果各按原口径使用。')
    r.p('本轮完成模块化整理及配图，不新增各行业101篇已完成的承诺。更详专业推导与中国/国际原文仍在各模块来源位置。')
    refs=[('工信部制造场景指引','https://www.miit.gov.cn/cms_files/filemanager/1226211233/attach/20253/3ddfcafcac6d410f990d75b85b508b22.pdf'),('Visual Components完整流程','https://www.visualcomponents.com/blog/introduction-to-process-modeling/'),('Agilon具名案例','https://www.visualcomponents.com/case-studies/konecranes-agilon-uses-visual-components-to-accelerate-solution-sales-with-clear-visual-manufacturing-simulation/'),('SCSC-SIMLIB论文','https://www.anylogic.com/upload/iblock/952/952071895e398dd83a5290cdb45c1ea8.pdf'),('JaamSim工程','https://github.com/jaamsim/jaamsim'),('Simul8 Decision Cloud','https://www.simul8.com/software/decision-cloud'),('Open-Industrials公开工程','https://github.com/caid-technologies/Open-Industrials'),('赛事官方入口','https://www.oschina.net/oa2026/')]
    r.page('主要原始依据：可点击入口')
    for label,url in refs:
        r.heading(label);r.p('<link href="'+html.escape(url,quote=True)+'" color="#126679">'+html.escape(url)+'</link>','small',True)
    output=r.finish();reader=PdfReader(output);texts=[p.extract_text() or '' for p in reader.pages]
    assert len(reader.pages)==r.n and len(reader.outline)==r.n
    for t in topics:assert any(t['name'] in s for s in texts),t['name']
    assert not any(re.search(r'铣程|纸构|装程|工境|检程|仓策|印策|布策|载程|能构|气衡|洄策|试衡|镜策|热构|换芯稿|量筑|声图|01[—–-]05',s) for s in texts)
    assert len(r.illustrations)==95,len(r.illustrations)
    return output,r.n
