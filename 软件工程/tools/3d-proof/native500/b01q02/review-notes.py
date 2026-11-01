from pathlib import Path
import json
from datetime import datetime
root=Path.cwd();h=root/'modules/media/native-equipment-500/reviews/B01-Q02';write=lambda f,v:(h/f).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
write('acceptance-input.json',{'host':'.work/native500-quality-20261025/b01q02/attempt-03','native_persistence':'.work/native500-quality-20261025/b01q02/Native-persistence-r1-checks.json','native_render':'.work/native500-quality-20261025/b01q02-render-01'})
write('review-scope.json',{'review_batch':'B01-Q02','device_ids':['flexo-printer'],'definition':'sf.native50.device.flexo-printer@1.1.0','source_process_reference':{'url':'https://www.bobst.com/cuen/products/flexo-printing-rotary-die-cutting/process/','use':'Anilox -> raised plate -> sheet against impression cylinder; separate print units. No imported geometry or manufacturer performance.'},'primary':{'type':'one flat paperboard sheet','dimensions_m':[.50,.0008,.40],'area_m2':.20,'identity_policy':'preserve','state_before':'flat','state_after':'two_color_printed'},'auxiliaries':[{'role':role,'color':color,'tracking':'batch','quantity_piece':1,'declared_volume_ml':.2,'mark_dimensions_m':[.10,.000020,.10]}for role,color in [('auxiliary','blue'),('auxiliary_red','red')]],'mechanism':'Two independent plate/anilox/impression units; single doctor blades and gravity-fed metered dose cups; three closed feed-belt sections leave impression-roll clearance.','cycle_ms':25000,'phases_ms':{'load':5000,'auto':15000,'unload':5000},'boundaries':'Concrete two-mark recipe, not arbitrary raster/RIP, dot gain, viscosity, recirculation, drying or fluid/contact-force physics. One dose piece means0.2mL, not1mL. Both ink types are independent runtime Items.'})
fixes=[('definition','原印组和辅助耗材没有明确区分两色。','两个独立墨剂ItemType、输入端口、槽位、计量体积及缺料工况；保留原纸身份。'),('geometry','初版下压印辊与0.80m机床面发生20mm交叠。','工作面提高到0.92m，压印辊最低点0.80m；分段闭合带让出两个辊间隙，源码复核通过。'),('paper','薄墨层与纸面易共面，纸顶必须给两色印层留区。','纸顶保留两独立开口与未印区域，按印刷前进方向逐步转移，每色体积独立守恒。'),('reviewer','审阅器0.001m近裁面使0.8mm纸面与皮带出现深度条纹。','审阅器近裁面改0.02m，重新实际拍摄，纸张与两色块清晰；产品Native相机原为0.01m，不改模型厚度。'),('startup','Native启动后立即连接多次早于WebView2端口就绪。','仅验收启动器等待同一存活PID的实际tauri页面，最多30s；不重启、不伪造连接。')]
write('open-defects.json',{'issues':[{'id':'B01-Q02-'+id,'observed':a,'fix':b,'status':'resolved'}for id,a,b in fixes]})
dimensions={'MQ-01':'米制Y向上媒体/工程Z向上及0.92m工作高度一致；双LOD七向和原纸/墨剂同尺度复核。','MQ-02':'七向、背底、支脚图已看；进出料台和墨剂暂存台腿/脚垫/台底实际解码接触通过，印组支架与机床面连接。','MQ-03':'两套独立网纹辊/凸版滚筒/压印辊、单刮刀夹持和重力计量供墨；三段闭合输送带让出压印辊空间，0.2mL蓝、红墨各形成0.10×0.10m×20µm色块。','MQ-04':'轴承座、电机安装/轴、墨杯支架/导管、侧导向、透明侧护罩、操作屏/急停/信号灯、电柜及原创铭牌完整；详细档保留紧固及原创微表面。','MQ-05':'对照已接收L03/装配台工业框架和PBR等级；简化档保留印刷机构、物料路径及接触关系，不以换名共壳充数。','MQ-06':'两LOD实际GLB三阶段正反seek和机构近景通过；所有滚筒按原轴向旋转，纸张和两色转印按同一实际Job有效时间推进，等待不另造时钟。','MQ-07':'真实纸张、蓝墨和红墨为三个独立Item引用，Job/Location持有及三个显式槽位决定画面；两色分别计量守恒，原纸面积/数量/身份不变，耗材按实际阶段消耗。','MQ-08':'8个真实Host任务/7工况通过，分别缺纸/蓝墨/红墨、阶段等待、中断和出料阻塞均恢复；16个实际Native视图、两普通进程各20时点、菜单保存和新进程重开及正常关闭通过。'}
write('visual-review.json',{'reviewer':'Codex AI','reviewed_at':datetime.now().astimezone().isoformat(),'result':'pass','open_defects':[],'scope':'双色柔版一设备双LOD七向、机构/纸墨组合图及16个实际Native业务视图AI复审，不声明人工验收。','comparison':'保持L03与已接收设备的工业结构/材质等级，明确双色独立供给与两个柔版印组。','dimensions':{'flexo-printer':dimensions}})
(h/'review-notes.md').write_text('''# B01-Q02 · 双色柔版印刷机复审

`sf.native50.device.flexo-printer@1.1.0`沿用原独立身份，旧1.0.0保留。本批1种设备、双LOD2个设备GLB；原纸、蓝墨、红墨和阻塞夹具共4个物品GLB，夹具与实例不增加设备计数。

## 可据其运行的业务定义

一张0.50×0.40×0.0008m纸板，面积0.20m²，状态flat→two_color_printed。纸张经过两套网纹辊、凸版滚筒和压印辊印组；保持同一Item，数量和面积不变。蓝墨`auxiliary`、红墨`auxiliary_red`分别是独立batch ItemType、独立运行态Item、端口和持有槽位。每色数量1 piece明确代表0.2mL计量剂量；各自形成0.10×0.10m、20µm印层，独立体积守恒。

25s周期：load5s、auto15s、unload5s；实际装卸资源与自动阶段资源/日历驱动。两种油墨在auto结束真实消耗，原纸枚举更新。耗材来源由Job.input_refs和transformed生命周期证明；preserve不会向原纸composition_refs偷偷追加耗材。declared_blue_layer_volume_m3/declared_red_layer_volume_m3是配方常量，不表示初始纸已印刷。缺任一墨剂都不能启动，缺料时不显示虚构耗材。

工序关系参照[BOBST柔版印刷过程](https://www.bobst.com/cuen/products/flexo-printing-rotary-die-cutting/process/)中的网纹辊→凸版→承印物/压印辊。模型为原创明确示例：微量重力滴供、单刮刀、两个固定色块；不声明工业循环墨路、光栅/RIP、网点增大、粘度、干燥或流体/接触力求解。尺寸与工时不采用制造商额定参数。下游应显式消费two_color_printed状态，任意图案/产品须另行绑定定义。

## 修正与验收证据

修正两色定义缺失、压印辊穿入机床面、薄印层共面及审阅器深度条纹，见open-defects.json。工作面0.92m、压印辊最低点0.80m，三段闭合输送带让出辊组间隙；脚垫/台腿/台面及墨杯支撑核对。源码、微表面和铭牌原创CC0，所有可安装包自带独立重建源码。

2个设备GLB和4个物品GLB独立包内重建一致。29张实际GLB图：双LOD七向14、详细机构近景3、支脚2、蓝/红真实纸墨组合10。实际产品解码正反向16时点/模型核对纸材位移及每色材料守恒；最终重拍已消除审阅相机近裁面造成的纸/带深度条纹。产品Native相机未修改。

最终Host：`.work/native500-quality-20261025/b01q02/attempt-03`，8任务含1个明确清出料缓存夹具，7设备工况：正常、缺纸、缺蓝墨、缺红墨、阶段资源等待、中断、出料阻塞；全部恢复完成。完整Run与文件在新Host重开通过。

普通Native：`b01q02-ordinary-01`与`b01q02-ordinary-02`，各20个历史时点、实际菜单保存到`b01q02/native-saved-r1/saved`再由新进程重开，14条逐工况检查通过，原纸/两色Item身份及持有明确，暂停有效进度不变。最终16视图`b01q02-render-01`核对实际纸、蓝墨、红墨的出现/消失与恢复；观看保持Run不变。不以整幅PNG相等替代暂停判定。

逐机MQ八项AI记录见visual-review.json；首启精确登记另存startup-registration-checks.json。本批不关闭整个P3-02、正式NFR或人工验收。历史素材、失败尝试和Run保留。
''',encoding='utf8')
