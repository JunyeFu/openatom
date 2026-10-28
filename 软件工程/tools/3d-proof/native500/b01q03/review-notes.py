from pathlib import Path
from datetime import datetime
import json
root=Path.cwd();h=root/'modules/media/native-equipment-500/reviews/B01-Q03';write=lambda f,v:(h/f).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
write('acceptance-input.json',{'host':'.work/native500-quality-20261025/b01q03/attempt-03','native_persistence':'.work/native500-quality-20261025/b01q03/Native-persistence-r2-checks.json','native_render':'.work/native500-quality-20261025/b01q03-render-02'})
write('review-scope.json',{'review_batch':'B01-Q03','device':'sf.native50.device.offset-printer@1.1.0','primary':{'size_m':[.50,.0008,.40],'area_m2':.20,'identity':'preserve','initial':'flat','final':'offset_printed'},'ink':{'role':'auxiliary','tracking':'batch','quantity_piece':1,'dose_ml':.2,'mark_m':[.10,.000020,.10]},'required_resource':{'role':'dampening_circuit','kind':'fixture','unit':'slot','quantity':1,'required_phase':'auto','meaning':'Available filled/circulating dampening service. Macro resource availability only; no finite water stock, fluid inventory or consumption quantity claimed.'},'cycle_ms':28000,'phase_ms':{'load':5600,'auto':16800,'unload':5600},'mechanism':'Plate -> blanket -> paper against impression cylinder, separate inking and dampening roller trains, closed feed belts and circulation reservoir/pump/lines.','sources':['https://www.heidelberg.com/global/media/en/global_media/company___publications/heidelberg_news/tips_tricks/hn_271_tips_tricks.pdf','https://www.imaging.org/IST/IST/Resources/Tutorials/Printing_Press.aspx'],'boundaries':'Original example geometry and timings; fixed single blue mark. No arbitrary RIP, ink/water balance, drying, dot gain or fluid/contact-force physics.'})
issues=[('original','旧示例将三只同类上辊平排，未明确胶印三滚筒工艺和润版条件。','分别建印版/橡皮布/压印滚筒、供墨与润版辊组；auto强制需要dampening_circuit资源服务。'),('clamp','印版夹条位于橡皮布接触宽度内。','夹条移到两侧端部，逐阶段正反姿态核对不进入橡皮布宽度。'),('direction','初次动画符号让压印接触面与纸材前进方向相反。','纠正印刷/供墨/润版辊组旋向；源码和实际产品解码分别实测压印上表面、橡皮布下表面同向同速。')]
write('open-defects.json',{'issues':[{'id':'B01-Q03-'+id,'observed':a,'fix':b,'status':'resolved'}for id,a,b in issues]})
d={'MQ-01':'米制Y向上媒体和工程Z向上、0.92m纸路及实际纸张尺寸一致，双LOD七向复审。','MQ-02':'台腿/脚垫/台底实际GLB接触通过；印刷塔、轴承座、电机/墨杯支架和润版泵由机架承托，背底已看图。','MQ-03':'印版→橡皮布→纸张/压印辊三滚筒明确，供墨/润版辊组相切，压印辊最低点不穿机床面，端部印版夹不入橡皮布幅面；实际解码核对纸路与印层。','MQ-04':'供墨杯/管、润版储罐/泵/供回管、透明塔侧护罩、导向、轴承、电机/轴/安装座、操作屏/急停/灯和原创铭牌完整。','MQ-05':'保持L03及已接收设备工业框架和原创PBR等级；金属/蓝壳/橡胶/透明罩分材质，简化档保留三滚筒和两辅助辊组。','MQ-06':'双LOD实际GLB三阶段正反seek通过；新增源码及真实解码的切点速度核对，压印/橡皮布接触面与纸张均为正向2m/归一化auto；等待按实际Job有效进度冻结。','MQ-07':'实际原纸和0.2mL batch油墨有独立身份/槽位，原纸身份、数量、面积保持，油墨真实消耗且体积转到20µm层；润版明确为资源服务，不烘焙虚构液体库存。','MQ-08':'8任务7工况、16个Native视图；缺纸/缺墨、阶段等待、中断、出料阻塞和润版回路不可用均恢复；两普通Native进程各20时点、菜单保存及新进程重开、正常关闭通过。'}
write('visual-review.json',{'reviewer':'Codex AI','reviewed_at':datetime.now().astimezone().isoformat(),'result':'pass','open_defects':[],'scope':'单张胶印双LOD七向、机构/纸墨组合图、真实切点方向及16个实际Native业务视图AI复审，不声明人工验收。','comparison':'沿用L03/装配台工业结构与PBR基准，三滚筒及润版条件区别于柔版。','dimensions':{'offset-printer':d}})
(h/'review-notes.md').write_text('''# B01-Q03 · 单张胶印机逐机复审

原独立身份`sf.native50.device.offset-printer`修订1.1.0。双LOD共2个设备GLB，原纸/墨剂/阻塞夹具3个物品GLB；夹具、LOD和实例不增加设备数。原创CC0结构/表面/铭牌及包内源码，尺寸工时为本项目明确示例。

一张0.50×0.40×0.0008m纸板，面积0.20m²，保持同一Item、数量1，flat→offset_printed。印版先转印到橡皮布，橡皮布向纸材转印，压印辊承托；供墨和润版为分别可辨认的辊组。参考[Heidelberg润版说明](https://www.heidelberg.com/global/media/en/global_media/company___publications/heidelberg_news/tips_tricks/hn_271_tips_tricks.pdf)与[IS&T胶印过程](https://www.imaging.org/IST/IST/Resources/Tutorials/Printing_Press.aspx)核对关系，不导入厂商CAD或额定参数。

油墨为独立batch ItemType，1 piece明确代表0.2mL剂量，形成0.10×0.10m×20µm蓝色层。declared_blue_layer_volume_m3是配方常量，不表示初始已涂墨。实际Job.input_refs和油墨transformed生命周期证明消耗；preserve不向原纸composition_refs追加耗材。

润版循环为auto必需的`dampening_circuit`资源，fixture/slot数量1，表示已经供液并可运行的循环服务。现场不可用通过实际日历/占用阻止印刷，纸/墨仍持有并在恢复后继续。该资源不是一份虚构水Item，不计算有限水存量、液位或水耗；若需要水耗平衡，应建立对应物料工序，不能从这份资源可用性结果推断。外观有封闭循环罐、泵和供回管，不烘焙液体库存。

周期28s，load5.6s/auto16.8s/unload5.6s；装卸operator和自动auto_fixture加dampening_circuit均为真实绑定。默认固定单蓝色块；未声明任意印刷稿/RIP、油水平衡、干燥、网点增大或流体/接触力求解。下游应显式绑定offset_printed状态和对应ItemType。

修正原三上辊泛化、印版夹进入幅面、以及滚筒旋向错误。新增了真实切点速度检查：橡皮布下表面与压印辊上表面沿纸路均为+2m/归一化auto，既有源码重建和实际产品GLB解码分别通过；不以看起来在转代替同向滚动。该复核同时发现旧双色柔版和印刷开槽1.1.0有同类缺陷，已独立撤回计数，等待新修订重验；本批不掩盖它们的待修状态。

2设备GLB与3物品GLB独立源码重建一致；29张实际GLB图：双LOD七向14、机构近景3、支脚2、实际纸墨组合10。三滚筒/供墨润版相切、床面及夹条净空通过。最终Host`.work/native500-quality-20261025/b01q03/attempt-03`：8任务含1清缓存夹具，7设备工况覆盖正常、缺纸、缺墨、阶段等待、中断、出料阻塞和润版服务等待，均恢复完成；同纸身份/数量/面积和墨剂体积核对，新Host保存重开通过。

最终普通Native`b01q03-ordinary-03`、`b01q03-ordinary-04`各20个历史时点，菜单保存至`b01q03/native-saved-r2/saved`后新进程重开；14条逐工况持有检查、完整Run不变及正常关闭通过。最终16视图`b01q03-render-02`核对实际物品出现/缺料消失、等待和润版恢复。源、旧试次和Run保留。准入后首启精确登记另存startup-registration-checks.json；本批不关闭P3-02、正式NFR或人工验收。
''',encoding='utf8')
