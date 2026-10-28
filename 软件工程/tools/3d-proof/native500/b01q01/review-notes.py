from pathlib import Path
import json
from datetime import datetime
root=Path.cwd();h=root/'modules/media/native-equipment-500/reviews/B01-Q01';write=lambda f,v:(h/f).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
write('acceptance-input.json',{'host':'.work/native500-quality-20261025/b01q01/attempt-04','native_persistence':'.work/native500-quality-20261025/b01q01/Native-persistence-r2-checks.json','native_render':'.work/native500-quality-20261025/b01q01-render-01'})
write('review-scope.json',{'review_batch':'B01-Q01','device_ids':['sheet-feeder','pile-lift'],'version':'1.1.0','definition_namespace':'sf.native50.device; preserve original exact identities','rules':{'sheet-feeder':'One staged 0.50 x 0.40 x 0.0008m sheet Item is vacuum-picked, moved to the feed nip, released and roller-fed to output. No hidden sheet creation or implicit pile splitting.','pile-lift':'One declared 20-sheet pile Item,0.50 x0.40 x0.016m, moves from0.20m loading to0.90m delivery; same identity and sheet_count20. It is an aggregate pile stock unit, not20 independent runtime Items.'},'quantities':'one input and same output Item; sheet_count1/20 and area0.20/4.0m2 retained','work':'12s feeder,15s lift; load20%,auto60%,unload20%; loading/unloading require actual operator capacity, automatic phase requires actual fixture calendar','integration':'Explicit ItemType and Operation bindings required. Aggregate pile and individual sheet are different types; this batch does not automatically convert or split one into the other.','geometry':'Original illustrative mechanical geometry and CC0 micro-surfaces; editable package-local source; no manufacturer CAD or rated performance claim.'})
issues=[('axis','辊筒旋转动画覆盖原初始横轴，导出姿态变成竖轴。','四元数保留圆柱初始轴向并校正送纸旋向；实际解码逐阶段核对。'),('roller-bed','实心出料台会穿入下辊。','改为两侧开放辊床，增至五组辊支撑出料纸张。'),('drive','吸纸横移和真空供给缺少完整可见连接。','增加横移气缸/活塞、支撑、滑动真空总管及伸缩竖管。'),('piston','吸头上升后活塞会伸出短气缸上盖。','气缸加长至覆盖完整有效行程；四模型全部正反行程插入检查通过。'),('identity','验收脚本硬编码sf.native500设备前缀，不能消费原有sf.native50精确身份。','验收Host根据本批实际登记解析完整ID；不改原设备身份。'),('reviewer','审阅器初始化固定选择B02，B01无默认设备。','按本批真实分类初始化；既有B02选择路径保留。')]
write('open-defects.json',{'issues':[{'id':'B01-Q01-'+id,'observed':a,'fix':b,'status':'resolved'}for id,a,b in issues]})
common={'MQ-01':'米制Y向上模型、工程Z向上、Item尺度和端口高度一致；双LOD七向已看图。','MQ-02':'逐台七向、背底、支脚已看图；实际产品解码验证台腿/脚垫/台底零间隙，机构由台架、导轨和底座支撑。','MQ-04':'操作屏、急停、原创铭牌、电柜、驱动、轴承、支架与供给线均保留；详细档有螺栓、散热片、通风及适用丝杆螺纹。','MQ-05':'保持L03与已接收装配台的工业结构/PBR等级；原创金属微表面、蓝色粉末涂层和橡胶分材质，简化档保留机构轮廓和接触关系。','MQ-06':'实际GLB三阶段正反seek和机构近景通过；旋转保留原轴向，吸头/丝杆/台面均以实际Job有效进度驱动，视图不改变业务。','MQ-07':'实际Item引用、显式cargo槽位和Job/Location持有决定纸材画面；单张与整垛保持各自身份、数量、sheet_count和面积，缺料不显示虚构工件。','MQ-08':'12个实际Host任务/10工况、24个Native业务视图；两普通Native进程各28历史时点，菜单保存及新进程重开、等待冻结和恢复、正常关闭通过。'}
specific={'sheet-feeder':'四真空吸杯与分气支路、横移/升降气缸、导轨和滑动供给端、吹气辅助、五组上下辊、减速驱动和开放辊床；实际纸面与吸杯、辊轴向联合复核。','pile-lift':'低位输入台、双导向立柱/丝杆/螺母、顶置伺服和联轴器、升降平台与高位输出台；实际纸垛随平台从0.20升至0.90m，再卸料并空台回落。'}
write('visual-review.json',{'reviewer':'Codex AI','reviewed_at':datetime.now().astimezone().isoformat(),'result':'pass','open_defects':[],'scope':'两设备双LOD七向、机构近景、真实纸材组合及实际Native业务画面AI复审，不声明人工验收。','comparison':'对照既有L03及装配台工业结构和原创PBR基准；两设备机构、工件类型和业务用途分别定义，不以同壳换名计数。','dimensions':{id:{**common,'MQ-03':reason}for id,reason in specific.items()}})
(h/'review-notes.md').write_text('''# B01-Q01 · 送纸与举升两设备复审

两种独立原设备`sf.native50.device.sheet-feeder`、`sf.native50.device.pile-lift`发布修订1.1.0；保留旧1.0.0。双LOD共4个设备GLB、3个纸材/纸垛/阻塞夹具GLB；夹具、LOD与实例均不增加设备计数。素材、铭牌及微表面为本项目原创CC0，尺寸/工时为明确示例。

## 实际业务

送纸机：每周期消费一个已到达输入端的单张0.50×0.40×0.0008m纸板Item，保持同一身份，经吸杯提起40mm、横移、落入辊组、释放和送出。五组辊的横轴和旋向、吸杯接触均使用实际产品GLB解码核对。每张名义面积0.20m²，sheet_count=1。输入是已暂存单张，模型没有烘焙一垛库存；未实现隐式拆垛、双张检测或气动/纸张柔性物理求解。

升降台：一个20张定量纸垛是一个声明的聚合库存Item，尺寸0.50×0.40×0.016m、sheet_count=20、纸材总名义面积4.0m²。整垛装到0.20m平台，双丝杆举升到0.90m，卸到高位台面，空平台复位；全过程保持同一Item。不是20个独立运行态Item，也不隐式拆成下游单张。整垛和单张的ItemType不同，跨工序转换须显式建模；本批不声称两默认配方可直接首尾相接。

送纸12s、举升15s，各load/auto/unload=20%/60%/20%；装卸由实际operator容量、自动由实际auto_fixture日历控制。两操作使用preserve和明确presentation_state，静态状态不改变；设备姿态由实际Job阶段和有效进度驱动。物品数量、资源占用与业务事件归仿真内核，媒体不另造库存或时钟。

## 修正和验证

修正记录见open-defects.json，包括辊轴被四元数覆盖、实心台面穿辊、横移/真空供给缺失和短气缸行程。验收Host改为消费实际登记的完整设备ID，不把原sf.native50强制换名为sf.native500。实际审阅器支持B01初始化。

4个设备GLB从各自包内源码独立重建一致；3个物品GLB亦独立重建。74张实际GLB图：双LOD七向28、详细机构近景6、支脚4、三阶段真实纸材组合36。实际产品解码在正反向共39时点/模型验证吸附、辊轴向和纸垛承托。圆柱接触使用精确顶点边界，并仅容许由16/40段圆柱离散化可计算的径向差，不将保守旋转包围盒误当接触面。

最终Host：`.work/native500-quality-20261025/b01q01/attempt-04`，12任务含2个明确清缓存夹具任务，10设备工况：正常、缺纸、阶段资源等待、途中中断、出料阻塞；全部恢复完成，输入输出同ID、数量1、sheet_count和面积不变。运行保存后新Host重开通过。

普通Native进程`b01q01-ordinary-03`与`b01q01-ordinary-04`各28个历史时点，实际菜单保存到`b01q01/native-saved-r2/saved`再新进程打开；完整Run保持一致、20条逐工况持有检查通过。最终Native24视图：`b01q01-render-01`，实际Item refs、缺料不存在的Item及中断有效进度核对通过。不以整幅PNG相等替代暂停判定。

逐机MQ八项AI复审见visual-review.json。准入后还需普通Native首启登记精确版本；该批不关闭整个P3-02、正式NFR或人工验收。
''',encoding='utf8')
