from pathlib import Path
import json
from datetime import datetime
root=Path.cwd();home=root/'modules/media/native-equipment-500/reviews/B02-Q04'
write=lambda name,v:(home/name).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
write('acceptance-input.json',{'host':'.work/native500-quality-20261025/q04/attempt-06','native_persistence':'.work/native500-quality-20261025/q04/Native-persistence-r3-checks.json','native_render':'.work/native500-quality-20261025/q04-render-03'})
fixes=[
('Q04-01','保身份工序的液体辅料使用individual跟踪，实际Host拒绝该输入。','三种液体改成明确体积的一次计量batch剂量；不放宽内核准入。'),
('Q04-02','均匀动画采样未包含精确接触时点，烫金模具在u=0.20有0.467mm间隙。','采样加入实际机构及材料转移边界，按产品GLB解码核对模具、PET和纸面。'),
('Q04-03','PET下夹爪会穿过纸材；压凸上下工具和承托/驱动细节不完整。','PET改为上方真空持片；压凸使用下阳模与上阴模；补齐滑车支撑、驱动和连续供给端。'),
('Q04-04','极薄转印层与纸顶共面，UV表面切换可能出现缩放过渡。','纸顶留出标记区域，保留真实层厚；湿/固化层用独立STEP父组切换。'),
('Q04-05','实际Native画面中封闭UV罩隐藏正在固化的纸材。','加入观察窗和开放灯架、移正风机支撑，外部蓝灯只表达固化阶段。'),
('Q04-06','两台液压压机供油管起端低于油站顶，末端距缸体约0.1m。','管起端连接油站顶，末端连接新增缸体接头；双LOD源码接触检查及修正版实际Native全部重验。'),
('Q04-08','压凸纸面共享顶点平滑法线掩盖实际1.2mm凸边。','保留真实几何尺寸，改用逐面法线表现实际平面与斜面；12物品包独立重建、实际GLB及新Host/Native重验。'),
('Q04-07','成品近景被正面操作屏遮挡。','从正面中间避开操作屏拍摄压机成品，UV从观察窗上方取景；不更改模型或业务数据。')]
write('open-defects.json',{'issues':[{'id':id,'observed':a,'fix':b,'status':'resolved'}for id,a,b in fixes]})
common={
'MQ-01':'米制Y向上媒体映射到工程Z向上；双LOD七向与真实工件同尺度，进料/加工/出料槽位明确。',
'MQ-02':'逐机七向、背底及支脚图复审；台腿连接台底、脚垫接地；执行头和导轨由可见立柱/横梁/滑台承托。',
'MQ-04':'保留导向、轴承、气/液压驱动、操作屏、急停、信号灯和原创铭牌；供油端点与滑台供给端按源码及实际解码核对。',
'MQ-05':'与L03和已复审装配台工业结构、PBR表面等级对照；金属/蓝壳/橡胶/纸/油墨/透明罩分材质，简化档保留功能轮廓，源码和微表面原创。',
'MQ-06':'双LOD实际GLB三阶段、正反seek和接触关键时点通过；离散完成状态不缩小工件，机构由实际Job有效时间驱动。',
'MQ-07':'独立ItemType及真实Item绑定槽位，实际Job/Location持有决定画面；纸材面积、耗材体积和适用PET载体均核对，不在设备GLB内烘焙库存。',
'MQ-08':'30个设备工况、36个实际Host任务通过；缺纸/适用缺耗材、阶段资源等待、中断恢复、成品及适用PET阻塞恢复；两普通Native进程各85时点、菜单保存及新进程重开、70个实际Run视图通过。'}
specific={
'hot-foil-stamper':'四柱液压压板、加热模座和上方真空PET持片架；预切片热转印0.12×0.12m金属层，独立0.16×0.16m废PET输出，保留两个真实输入的来源关系。',
'blind-embosser':'下阳模向上压入上阴模，纸张从flat变embossed、凸起1.2mm；实际压凸顶点与模具接触，不新增废料或替换原纸身份。',
'spot-uv-printer':'丝网定量涂布、抬网、滑台进入观察窗UV罩、定时固化；0.2mL涂料成为0.10×0.10m、20µm涂层；原纸身份保持。',
'screen-print-press':'网框/网版开口、刮刀、升降与水平驱动完整；0.2mL油墨转移至0.10×0.10m、20µm层，实际网版/刮刀/纸面接触核对。',
'pad-print-press':'开放凹版、刮墨、硅胶垫取墨、横移和压印；0.08mL油墨在凹版/胶垫/0.04×0.04m、50µm印层之间守恒，原标签身份保持。'}
write('visual-review.json',{'reviewer':'Codex AI','reviewed_at':datetime.now().astimezone().isoformat(),'result':'pass','open_defects':[],'scope':'五设备双LOD七向、机构近景、真实工件/耗材组合、完成近景及实际Native业务状态AI复审；不声明人工验收。','comparison':'保持L03及原生装配台参考的工业框架、支撑/驱动/执行机构、原创PBR细节等级；五种不同整饰工序使用独立工具和真实物料定义。','dimensions':{id:{**common,'MQ-03':reason}for id,reason in specific.items()}})
(home/'review-notes.md').write_text('''# B02-Q04 · 纸包装整饰设备逐机复审

本批为平压烫金机、纸面凹凸压印机、局部UV上光机、平面丝网印刷机、移印机@1.1.0，共5种独立设备、10个双LOD设备GLB；12个物品GLB为业务夹具，不增加设备计数。

## 已实现的业务与边界

逐机尺寸、周期、ItemType/Operation/StockLocation和工艺来源见review-scope.json、catalog.json及包内source。预切PET载体烫金消耗纸与贴片，产生新纸Item和独立废PET，两者保留两个输入来源；不是卷筒烫金。压凸保持原纸身份和0.20m²名义面积，flat→embossed、1.2mm凸起，无废料。UV、丝印各消耗0.2mL batch剂量，形成0.10×0.10m×20µm层；移印消耗0.08mL batch剂量，形成0.04×0.04m×50µm层。batch数量1表示这一明确计量剂量，不表示1mL。

保身份工序只修改已声明的presentation_state枚举；declared_finish_layer_volume_m3、declared_emboss_height_m为配方常量，不表示初始flat纸已涂覆或凸起。保身份耗材消耗由实际Job.input_refs及耗材transformed生命周期证明，当前契约不会向原纸composition_refs追加辅料；烫金新产物则按transform_with_origin保留来源。全部使用实际Item持有、Job进度与工程显式槽位，不在设备模型内放置假库存。

UV为涂布→抬网→滑台→定时固化的原子组合操作，不输出可流转湿半成品；蓝灯只表示固化阶段。不声明光学、热学、软体或制造公差仿真。默认配方针对所列工件和状态，其他产品/串联工艺应显式绑定对应ItemType和Operation，不隐式兼容任意原料。

## 修正与实际验证

缺陷逐条见open-defects.json：修正batch耗材准入、接触关键帧间隙、PET夹持穿纸、上下压凸模具、薄层共面、UV观察遮挡、液压管路断接、压凸纸面平滑法线和成品近景遮挡。原候选与失败试次保留，当前设备GLB均由独立包内源码重导出一致。

最终Host输入与36任务/30工况：`.work/native500-quality-20261025/q04/attempt-06`。包含正常、缺纸、阶段资源等待、途中中断、成品阻塞，以及适用缺耗材与废PET阻塞。数量、原纸身份/变换来源、面积、涂层/载体体积、等待与恢复完成时间通过。6个显式清缓存任务用于解除实际阻塞。

实际产品解码130张图：70张双LOD七向、15张机构近景、10张支脚、30张真实工件/耗材组合、5张完成近景。源码接触与真实GLB联合检查覆盖模具、纸/PET、网版/刮刀、凹版/胶垫和转移层。实际Native最终画面：`q04-render-03`，70个Run视图，精确Item refs及缺料时不存在的Item均核对。

普通Native两个独立进程`q04-ordinary-05`、`q04-ordinary-06`各85个历史时点，实际菜单保存到`q04/native-saved-r3/saved`后新进程重开；60条逐工况检查通过，完整Run相同，暂停时有效进度一致。最终回执`q04/Native-persistence-r3-checks.json`。不以整幅PNG逐像素相等替代机构暂停判定。

逐机MQ八项AI复审见visual-review.json。正式计数仅在accept-paper.py核对上述当前文件后更新accepted-registration.json；Native新版本首启登记另存startup-registration-checks.json。该批不关闭整个P3-02、正式NFR或人工验收。
''',encoding='utf8')
