"""Record this turn's explicit AI visual findings, without granting Native acceptance."""
from pathlib import Path
from datetime import datetime
import json

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'modules/media/equipment-quality-v041'
OUT=ROOT/'scenarios/媒体表现/model-quality-20261019'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def rel(p): return p.resolve().relative_to(ROOT).as_posix()

# These are the reviewer's observations from actual exported views and motion
# frames, not a score computed from mesh counts or test results.
findings={
 'assembly':('装配台','型材截面、台架、下层板、电柜背门、定位夹具、XY/Z滑台与主轴完整。','补角撑/门铰/锁扣/后部服务线；主轴接环改为空心套，保留独立滑台与夹具动作。','load/auto/unload；夹具闭合、主轴行程和随动供线可从近景辨认。','assembly'),
 'rework':('返工台','台架与滑台保留，增加有立柱和连杆支撑的抽吸罩、软管、工具盘。','补服务线、角撑、接环及抽吸罩支撑；通过工装与附件区别于装配台。','load/auto/unload；工装开合和主轴动作，抽吸罩为固定辅助件。','assembly'),
 'packing':('装箱台','台架、箱体定位槽、推杆/气缸、折板、铰轴与双立柱支撑均可辨认。','补台架连接件、电柜/门铰、服务线和实际表面；保持折板与推杆独立层级。','load/auto/unload；推杆平移和折板绕实际铰轴开合，不依靠整机位移。','assembly'),
 'robot':('六轴机器人','底座、回转座、肩/肘/腕、连杆壳、夹爪滑块和夹指层级完整。','修复长连杆过细、端盖间距和肩叉支撑，补夹爪气路；2026-10-21进一步分离立柱/空心密封环/回转轴承的表面，消除黑色锯齿接缝，并降低肩叉底板以避开肩部轴承运动包络。','approach/grip/loaded_move/release/return；五阶段和反向回看均有实际近景，近景相机随当前夹爪姿态重新取景。','robot'),
 'conveyor':('输送机','有脚垫、横撑/斜撑、带面/回程、端滚筒、轴承、电机减速器、传感器与接线盒。','补斜撑、张紧槽与接线口；修复悬空线尾；带接缝改为可见的橙色实体标记。','load/auto/unload；接缝沿带面位移，端滚筒旋转；不添加虚假业务工件。','conveyor'),
 'inspection':('检验台','台架、定位区、相机/镜筒、灯环、相机安装板及供线完整。','补镜筒环、相机固定螺钉、台架连接与电柜细节，增强金属/塑料表面。','load/auto/unload；检测头下探/回位与线缆可辨；未宣称测量精度或检验结果。','inspection'),
 'sealer':('封箱机','滚筒床、侧带、升降柱、手轮、胶带架/卷芯、导带与压辊构造完整。','胶带/卷芯改成真实空心环及硬边端面；补可见压辊标记，修复标记埋在辊内的问题。','infeed/seal/outfeed；滚筒/侧带轮/压辊实际转动，标记可辨；不伪造胶带消耗。','sealer'),
 'pallet':('托盘','承载板、下横板、脚块/叉孔、钉位与背底均有实体。','加入原创木纹/表面细节；保留真实板块层次，不把原参照烘焙箱垛复制成业务库存。','静态载具，无主动动作；视频是观察转台。','pallet'),
 'control-box':('控制盒','开盖装配态：底壳、侧壁、安装耳、连接口、PCB和四角固定件完整。','重做壳壁接缝、支柱及螺钉高度；补芯片、引脚/焊盘、走线、端子和金属/塑料材质。','静态物品，无主动动作；视频是观察转台。','assembly'),
 'carton':('纸箱','有厚度的底/侧壁、四个独立盖片、纸边、接缝、标签与封口胶带。','重建四铰接盖片；去掉悬空接缝线；增加闭合间隙，并在导出关键帧中保留先短盖、后长盖的完整时序。','open/fold_preview/sealed；顺序预览与反向seek，长短盖不再同时压入。','sealer'),
 'shell':('壳体','开口壳体的底面、四壁、四支柱、安装耳和电缆接口完整；没有声称它带上盖。','重做壳壁/底板与支柱；缩短端壁使侧壁接头不再共面重叠。','静态物品，无主动动作；视频是观察转台。','assembly'),
 'pcb':('PCB','板厚、安装位、8个芯片、引脚/焊盘、走线、SMD与端子均可从近景辨认。','以独立几何补芯片引脚、锡焊面、铜色走线与端子，保持板件轮廓和表面区分。','静态物品，无主动动作；视频是观察转台。','assembly'),
 'fastener':('螺钉','有杆身、端部、螺纹和真实内六角凹槽；顶、底视角均可检查。','用挖空的挤出头部和螺旋几何替代实心头加黑点的外观。','静态物品，无主动动作；视频是观察转台。','assembly'),
 'sealing-tape':('胶带卷','恒宽卷材、空心纸芯、端面层纹与胶带末端均有实体。','以直壁环形卷材代替圆环甜甜圈形体，分离端面/侧壁法线，避免不合理的大圆鼓面。','静态物品，无主动动作；视频是观察转台。','sealer')
}
for family in ('assembly','rework','packing','inspection'):
 value=list(findings[family]);value[2]+=' 0.4.1将背面线尾接入立柱上的接线盒，补接头及固定件。'
 if family=='packing':
  value[1]+=' 折板驱动含轴承座、承托架、减速器、电机及随动联轴器。'
  value[2]+=' 补齐折板驱动链与电源接线盒，固定电机不随折板翻转，联轴器标记随原铰轴转动。'
 if family=='inspection':value[2]+=' 补随检测头移动的相机电缆接头和锁紧环，消除壳顶到电缆起点的5毫米间隙。'
 findings[family]=tuple(value)
inventory=read(BASE/'inventory.json'); ledger=read(ROOT/'modules/media/model-quality-ledger.json')
rows={r['model_path']:r for r in ledger['models']}; stamp=datetime.now().astimezone().isoformat(timespec='seconds')
reviews=[];simplified_reviews=[]
for entry in inventory['entries']:
 family=entry['family'];name,structure,repair,motion,reference=findings[family]
 model_file=BASE/entry['directory']/entry['asset']['path']; key=rel(model_file);row=rows[key]
 folder=family+('-simple' if entry['suffix']=='simple' else '')
 capture=read(OUT/folder/'capture.json')
 if capture.get('glb_bytes')!=entry['bytes'] or capture.get('version')!=entry['version'] or capture.get('source',{}).get('id')!=entry['asset']['id']:
  raise RuntimeError('Actual capture does not match final GLB: '+family)
 if capture.get('renderer_profile')!='depth-fitted-v1-vsm':raise RuntimeError('Old capture profile: '+family)
 stat=model_file.stat()
 if row['observed']['bytes']!=stat.st_size or row['observed']['modified_ns']!=stat.st_mtime_ns:raise RuntimeError('Refresh model inventory before recording: '+key)
 images={v:rel(OUT/folder/(v+'.png')) for v in ('isometric','front','back','left','right','top','bottom')}
 closeup=rel(OUT/folder/'closeup.png');video=rel(OUT/folder/'motion.mp4');sheet=rel(OUT/folder/'views.jpg')
 for image in [*images.values(),closeup,video]:
  if not (ROOT/image).is_file():raise RuntimeError('Missing actual evidence: '+image)
 animated=bool(entry['animations'])
 dims={
  'MQ-01':dict(result='pass',reason='已检查整体、正背左右顶底；米/Y-up及示例尺度/包络见实际inventory。未声称工业实测尺寸。',evidence=[sheet,rel(BASE/'inventory.json')]),
  'MQ-02':dict(result='pass',reason=structure,evidence=[images['back'],images['bottom'],closeup]),
  'MQ-03':dict(result='pass' if animated else 'not_applicable',reason=motion if animated else '静态物品/载具没有主动执行机构；实体构造仍按MQ-02/MQ-04检查。',evidence=[closeup,video]),
  'MQ-04':dict(result='pass',reason=repair,evidence=[closeup,images['top'],images['bottom']]),
  'MQ-05':dict(result='pass',reason='逐图比较材质区分、表面与接缝；原创2K表面实际嵌入GLB，没有用铭牌代替金属材质。',evidence=[closeup,rel(OUT/f'reference-{reference}/closeup.png')]),
  'MQ-06':dict(result='pass' if animated else 'not_applicable',reason=motion,evidence=[video,*[rel(OUT/p) for p in capture.get('reverse_seek',[])]]),
  'MQ-07':dict(result='pending',reason='此轮检查资产几何和声明预览，没有在Native内以真实Run核对物品holder/移交/包件内容。',evidence=[rel(OUT/'逐模型审查结果.md')]),
  'MQ-08':dict(result='pending' if animated else 'not_applicable',reason='尚未以真实Run完成四类等待、暂停/倍率/回看及保存重开。' if animated else '静态物品/载具没有自身生产阶段动作；实际持有关系仍由MQ-07待验。',evidence=[rel(OUT/'逐模型审查结果.md')])}
 detailed=entry['suffix']=='detailed'
 review=dict(reviewed_file=key,file_revision={k:row['observed'][k] for k in ('bytes','modified_ns')},reviewer='Codex AI actual-image review',reviewed_at=stamp,human_acceptance='pending',appearance_review='pass' if detailed else 'pass_lod_only',open_defects=[],repair_summary=repair,scope='asset geometry/materials and declared animation preview; no Native acceptance',gates={
  'source_export':dict(result='pass',evidence=[rel(OUT/('batch/source-rebuild-v041.json' if entry['version']=='0.4.1' else 'batch/source-rebuild.json')),rel(model_file.parent/'source/generate.mjs')]),
  'visual':dict(result='pending',reason='详细档外观和声明动作已AI逐图审阅；规范MQ-07/08联合产品环节未完成。' if detailed else '简化档已按七向、近景及适用动作实际审阅；只证明LOD外观与层级，不代替详细档及Native联合验收。',evidence=[sheet,video]),
  'platform_reopen':dict(result='pending',reason='未改动当前有效Native；未补签加载/采用/实际Run/保存重开与适用性能。',evidence=[rel(OUT/'逐模型审查结果.md')])},dimensions=dims if detailed else {},views=images,closeups=[closeup],motion_evidence=[video],reference_comparison=dict(reference_files=[f'tools/3d-proof/output/{reference}.glb'],evidence=[rel(OUT/f'reference-{reference}/isometric.png'),closeup],conclusion='同类构造、装配与材质层次作定性比较；不是按面数或字节打分。物品使用参照内相应材料/装配细节，不假称有一一对应的旧独立GLB。'))
 row['asset_quality']='pending';row['review']=review
 if detailed:reviews.append(dict(family=family,name=name,version=entry['version'],file=key,structure=structure,repair=repair,motion=motion,review=review))
 else:simplified_reviews.append(dict(family=family,name=name,version=entry['version'],file=key,review=review))

(ROOT/'modules/media/model-quality-ledger.json').write_text(json.dumps(ledger,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(OUT/'逐模型审查结果.json').write_text(json.dumps(dict(reviewed_at=stamp,reviewer='Codex AI',official_detailed_acceptance='pending',reason='Native MQ-07/08 and human acceptance are not supplied by asset screenshots.',models=reviews,simplified_models=simplified_reviews),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
lines=['# 14类模型实际返修与画面复核 · 0.4.0 / 0.4.1','',f'复核时间：{stamp}。复核者：Codex AI。','',
'本轮已完成14类的实际GLB返修、多视角/近景、声明动作与反向回看采集，并逐图审阅。外观与声明机构达到本轮可交付候选强度；没有用节点数、文件大小、重建成功或视频可解码代替画面判断。正式详细质量仍为pending：新规范还要求Native中的真实持有/阶段/等待及保存重开，当前有效Native与生产输入未被本轮替换，真人认可也未补签。','',
'[打开完整图册](模型审阅图册.html) · [打开可旋转与阶段拖动审阅器](http://127.0.0.1:19289/tools/model-quality-review/index.html) · [可编辑源与交付说明](../../../modules/media/equipment-quality-v041/README.md)','',
'## 本次确实修了什么','',
'每类详细与简化两档分别检查斜视、正面、背面、左右、顶部、底部和近景，共224张方向/近景图。8类有声明动作，6类为静态物品/载具；后者视频是观察转台，不伪造生产动画。28段视频均为12秒/24fps，左侧整体、右侧真实机构/材质近景。阶段关键帧、反向回看图与原监督参照比较图保存在对应子目录。四类台式设备修订为0.4.1，其余10类保留0.4.0；旧GLB与受影响画面保留供对照。','',
'|类别|本次返修与保留的构造|动作/观察|实际文件与画面|','|---|---|---|---|']
for r in reviews:
 f=r['family'];lines.append(f'|{r["name"]}|{r["repair"]} {r["structure"]}|{r["motion"]}|[GLB](../../../{r["file"]}) · [七向＋近景]({f}/views.jpg) · [实际视频]({f}/motion.mp4)|')
lines+=['','## 与原监督八设备流水线的比较','',
'原八实例实际复用六类GLB。本轮将这六类原文件原样重开，在同一环境反射、照明、阴影和归一取景条件下，与新资产并列查看。原试样带有固定参考工件、部分含箱垛；候选没有把这些烘焙进设备/载具并冒充真实库存。装配/检验的台架边界不同，不以复制原外形作等价证明，而逐项看支撑、工装、执行机构、连接、表面和层级。封箱机补出了原参照较概括的卷芯、导带、压辊、侧带支撑；托盘补出了板块/脚块/钉位及木纹。控制盒、PCB、纸箱等按原参照中的对应部件/材料和本次声明尺度检查，没有虚构六个旧独立物品GLB。','',
'在已检查的外观范围未见需要以“低精简模”豁免的阻断项；该判断不等于工业精确CAD、通用碰撞求解、正式产品性能或真人认可。原参照存在的局部不足也不作为新资产保留缺陷的理由。','',
'## 显示与运动中发现并处理的问题','',
'- 主轴接缝、控制盒壁接头与螺钉高度，已从源几何修正并重拍。机器人连杆变粗后同步调整关节/端盖与肩叉结构，保持原动画关节/锚点。','- 纸箱短盖与长盖的导出关键帧原来跨阶段插值而相撞；现有0、0.4、0.45、0.85、1五个时点，短盖先收平，长盖再下降，胶带最后出现。','- 封箱压辊的观察标记原来埋在辊内，已移到端面；输送带接缝使用可见实体标记，能看到真正的几何移动。','- 审阅器原来极小近裁剪面/极大远裁剪面造成深度冲突，连原监督GLB也出现锯齿。现已按取景距离收紧相机深度范围；最终参照和候选均重新拍摄，诊断关闭阴影的画面没有作为最终验收素材。','',
'## 八维度的边界','',
'MQ-01～05：本轮按实际全视角与近景完成AI外观复核。MQ-06：8类声明动作有顺放关键点与反向seek画面，6类静态资产说明不适用。MQ-07：真实库存身份/holder/夹取移交/入箱接触尚待Native联合检查。MQ-08：8类有动作资产的真实阶段、四类等待、暂停/倍率/回看及保存重开尚待Native，6类静态资产不声明自身生产动作。逐项理由、文件版本与证据已同步JSON和逐模型清单；任一适用pending未被写成正式accepted。','',
'## 交付文件','',
'- 14个详细GLB＋14个简化GLB与各自版本模块定义、精确登记清单。','- 新8包可编辑源在独立副本中重开，导出与交付GLB逐字节相同；未变20包沿用原有效重建结果。这只证明源/导出对应。','- 两档各含七个独立方向、机构/材质近景、12秒视频；适用阶段起中止与反向回看图。','- 原创CC0几何/贴图/铭牌，Three.js依赖及现有项目锁版本说明；不分发外部字体、闭源网格或机器本地可执行程序。','']
(OUT/'逐模型审查结果.md').write_text('\n'.join(lines),encoding='utf-8')
print(json.dumps(dict(reviewed_families=len(reviews),source_packages=28,official_acceptance='pending',report=rel(OUT/'逐模型审查结果.md')),ensure_ascii=False))
