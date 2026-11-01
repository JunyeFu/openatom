// Original illustrative equipment, not manufacturer CAD or measured capacity.
export const version = '1.0.0';
const paper = [
 ['sheet-feeder','真空送纸机','feeder','preserve',12,'纸板 → 单张纸板','SuctionBeam'],
 ['pile-lift','纸堆升降上料台','lift','preserve',15,'纸板垛 → 送纸高度','LiftDeck'],
 ['flexo-printer','双色柔版印刷机','printer','transform',25,'纸板 → 印刷纸板','PrintRoller'],
 ['offset-printer','单张胶印机','offset','transform',28,'纸板 → 胶印纸板','BlanketCylinder'],
 ['inkjet-marker','在线喷码机','marker','preserve',8,'成品 → 编码成品','PrintHead'],
 ['varnish-coater','辊式上光机','coater','transform',20,'印刷纸板 → 上光纸板','CoatingRoller'],
 ['uv-curer','UV固化通道','uv','preserve',18,'上光纸板 → 固化纸板','BeltDrive'],
 ['film-laminator','覆膜机','laminator','transform',24,'纸板/膜 → 覆膜纸板','PressureRoller'],
 ['flute-laminator','裱纸机','flute','transform',26,'面纸/底纸 → 裱合纸板','LaminationNip'],
 ['sheet-slitter','分切机','slitter','transform',18,'大张纸板 → 分切纸板','KnifeShaft'],
 ['rotary-scorer','滚轮压痕机','scorer','transform',16,'纸板 → 压痕纸板','CreaseShaft'],
 ['platen-die-cutter','平压平模切机','platen','transform',20,'纸板 → 模切箱坯','MovingPlaten'],
 ['rotary-die-cutter','圆压圆模切机','rotary','transform',18,'纸板 → 模切箱坯','DieCylinder'],
 ['digital-flatbed-cutter','数字切割压痕台','cutter','transform',40,'纸板 → 切割压痕箱坯','ToolHead'],
 ['blank-stripper','箱坯清废机','stripper','transform',17,'模切箱坯 → 清废箱坯','StrippingPlate'],
 ['folder-gluer','直线折叠糊盒机','folder','transform',30,'箱坯/胶 → 折糊纸箱','FoldLeft'],
 ['crash-lock-gluer','勾底折糊机','crash','transform',34,'勾底箱坯/胶 → 勾底纸箱','BottomHook'],
 ['tray-former','纸托盘成型机','tray','transform',25,'纸托坯 → 成型纸托','FormingMandrel'],
 ['carton-erector','纸箱开箱机','erector','transform',22,'平折纸箱 → 开立纸箱','VacuumArm'],
 ['rigid-box-wrapper','硬盒包边机','rigid','transform',35,'盒体/面纸 → 包边硬盒','CornerFolder'],
 ['window-patcher','开窗贴膜机','window','transform',28,'开窗箱坯/膜 → 贴膜箱坯','FilmApplicator'],
 ['carton-stitcher','钉箱机','stitcher','transform',20,'箱坯/钉线 → 钉合纸箱','StitchHead'],
 ['adhesive-dispenser','热熔胶涂布机','glue','transform',15,'箱坯/胶 → 涂胶箱坯','GlueNozzle'],
 ['compression-stack','纸箱压合机','compression','preserve',30,'折糊纸箱 → 压合纸箱','PressBelt'],
 ['vision-carton-inspector','纸箱视觉检验机','inspection','inspect',12,'纸箱 → 合格/不合格支路','OpticalHead'],
 ['carton-counter-stacker','纸箱计数堆叠机','stacker','preserve',18,'纸箱 → 计数后纸箱','StackStop'],
 ['strapper','纸箱捆扎机','strapper','preserve',14,'纸箱束 → 捆扎纸箱束','StrapHead'],
 ['shrink-wrapper','热收缩包装机','shrink','preserve',25,'包件 → 热收缩包件','SealJaw'],
 ['case-packer','龙门装箱机','casepacker','pack',24,'产品/纸箱 → 装箱包件','PackingHead'],
 ['tape-case-sealer','胶带封箱机','sealer','preserve',12,'开口包件/胶带 → 密封包件','TapeHead'],
 ['label-applicator','纸箱贴标机','labeler','preserve',10,'包件/标签 → 贴标包件','TampPad'],
 ['pallet-stretch-wrapper','托盘缠绕包装机','wrapper','preserve',45,'托盘货物 → 缠绕货物','FilmCarriage'],
 ['manual-packing','人工装箱与折盖台','packing','pack',25,'产品/纸箱 → 装箱包件','PackingGate'],
];
const common = [
 ['belt-conveyor','直线皮带输送机','belt','preserve',8,'在制品 → 下游在制品','BeltDrive'],
 ['roller-conveyor','动力辊筒输送机','rollers','preserve',8,'包件 → 下游包件','DriveRoller'],
 ['curved-conveyor','90度弯道输送机','curve','preserve',10,'包件 → 转向后包件','TurnRoller'],
 ['right-angle-transfer','直角顶升移载机','transfer','preserve',12,'包件 → 横向下游包件','PopUpDeck'],
 ['lift-conveyor','垂直升降输送机','liftconveyor','preserve',18,'包件 → 高位下游包件','LiftDeck'],
 ['six-axis-robot','六轴夹爪搬运机器人','robot','preserve',12,'单件 → 搬运后的同一件','GraspCenter'],
 ['scara-robot','SCARA平面搬运机器人','scara','preserve',10,'单件 → 平面移载后的同一件','GraspCenter'],
 ['gantry-pick-place','三轴龙门搬运机','gantry','preserve',14,'单件 → 搬运后的同一件','GraspCenter'],
 ['vacuum-palletizer','真空龙门码垛机','palletizer','preserve',20,'包件 → 码放后的同一包件','GraspCenter'],
 ['screw-assembly','四点螺钉装配台','assembly','assemble',40,'壳体/板件/紧固件 → 总成','ToolTip'],
 ['identity-rework','保留身份返工台','rework','rework',30,'不合格件 → 待复检同一件','ToolTip'],
 ['optical-inspector','通用相机检验台','inspection','inspect',10,'工件 → 合格/不合格支路','OpticalHead'],
 ['cnc-mill','三轴数控铣床','mill','transform',60,'毛坯 → 铣削件','ToolHead'],
 ['cnc-lathe','数控车床','lathe','transform',55,'棒料 → 车削件','LatheChuck'],
 ['drill-press','立式钻床','drill','transform',30,'工件 → 钻孔件','ToolHead'],
 ['hydraulic-press','四柱压力机','press','transform',35,'毛坯 → 压制件','MovingPlaten'],
 ['checkweigher','在线重量检验机','weigher','inspect',10,'包件 → 重量合格/不合格支路','WeighDeck'],
];
export const equipment = [...paper.map(row=>[...row,'paper']),...common.map(row=>[...row,'common'])].map((r,i)=>({
  index:i+1,id:r[0],title:r[1],family:r[2],rule:r[3],cycle_seconds:r[4],flow:r[5],focus_node:r[6],group:r[7],
  definition_id:'sf.native50.device.'+r[0],media_id:'sf.native50.media.'+r[0],version,
  basis:'原创示例尺寸/声明工时；不表示厂家额定参数或物理求解',
  first_release_role:({'flexo-printer':'optional-printing','platen-die-cutter':'DieCut01','folder-gluer':'FoldGlue01','vision-carton-inspector':'Inspection01','strapper':'finished-carton-bundling'})[r[0]]??null,
}));
// Keep the seven mature L03 models at their exact, already authored versions.
const existing={
 'screw-assembly':['assembly','v041','0.4.1'], 'identity-rework':['rework','v041','0.4.1'],
 'manual-packing':['packing','v041','0.4.1'], 'six-axis-robot':['robot','v04','0.4.0'],
 'belt-conveyor':['conveyor','v04','0.4.0'], 'optical-inspector':['inspection','v041','0.4.1'],
 'tape-case-sealer':['sealer','v04','0.4.0'],
};
for(const spec of equipment)if(existing[spec.id]){
 const [family,directory,media_version]=existing[spec.id];
 Object.assign(spec,{existing_media:{family,directory},media_id:'p302.line-grade.'+family,media_version});
}
export const references = [
 {url:'https://media.bobst.com/en/news/detail/article/1768493402-the-essentials-of-carton-converting/',use:'模切、直线折糊和勾底折糊的类别与工艺区别；不取产能或厂家几何'},
 {url:'https://www.kongsbergsystems.com/en/digital-cutting/cad-table',use:'数字切割/压痕刀具与平面运动的类别依据'},
 {url:'https://endoline-automation.com/erectors/',use:'开箱与底部封合的设备类别依据'},
 {url:'https://www.robopac.com/en/business-units/robopac-machinery/vertical-stretch-wrapping',use:'托盘转台与升降膜架的类别依据'},
];
