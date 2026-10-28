"""Record actual inspected images. No geometry count or test result grants visual acceptance."""
from pathlib import Path
from datetime import datetime
import json

ROOT = Path(__file__).resolve().parents[3]
BASE = ROOT / 'modules/media/equipment-quality-v042'
OUT = ROOT / 'scenarios/媒体表现/model-quality-20261022'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def rel(p): return p.resolve().relative_to(ROOT).as_posix()

# Individual observations from the full views and mechanism close-ups reviewed this turn.
FINDINGS = {
 'assembly': ('装配台', '保留型材截面、接地脚垫、电柜、定位夹紧、XY导轨和独立Z主轴；补丝杠端支承、夹头锁环/槽口和上料暂存盘。主轴最低点专门补拍，未把50%恰好回升的姿态当作完整动作。', 'assembly'),
 'rework': ('返工台', '保留装配机构和返工工装；把抽吸软管接入有支承的过滤单元，补进气接口、排气口和安装件，消除软管止于空中的解释缺口。', 'assembly'),
 'packing': ('装箱台', '推杆按同一推板的39毫米行程伸缩，固定缸体、夹座和随动杆分开；补夹座底板/垫高支承并重拍，解决新增夹座悬空。折板加强筋随原铰轴运动，电机与减速器保持固定。', 'assembly'),
 'robot': ('六轴机器人', '保留六关节连续壳体、端盖、轴承、线束卡箍、两独立夹爪和原取放层级；补底座压紧垫环及末端气路分配块。没有用整机位移代替关节动作。', 'robot'),
 'conveyor': ('输送机', '保持完整顶带/回程、滚筒、减速电机、机架脚垫和传感器；补回程托辊、张紧调节及电机/出口传感器供线。用非整圈进度拍摄滚动关键点，避免静帧看不出转动。', 'conveyor'),
 'inspection': ('检验台', '保留镜筒、灯环支撑、检测头与随动供线；补调焦端挡、调节丝杆/旋钮、镜头安装板及固定件；下探和回位能在原GLB层级中直接观察。', 'inspection'),
 'sealer': ('封箱机', '保持滚筒床、侧带、升降柱、空心胶带卷芯、导带和压辊；补有安装板的切刀护罩及卡匣检修紧固件。压辊/侧带轮按原阶段转动，不伪造胶带消耗。', 'sealer'),
 'pallet': ('托盘', '保留承载板、横板、支脚块、叉孔、木纹、倒角和钉位；逐面复核接地、背底与木板接触，没有把原参照中的固定箱垛烘焙为库存。', 'pallet'),
 'control-box': ('控制盒', '侧壁连接口、安装耳和内部PCB使用真实贯通孔；电缆接头保留空心孔。PCB焊盘改成贴板铜环并补孔壁镀层，壳壁、支柱、元件、引脚与端子分别保留。', 'assembly'),
 'carton': ('纸箱', '保持有厚度的箱壁、底面及四个铰接盖片；折痕附着在各自盖片上，短盖先收、长盖后合，封口胶带最后出现；补拍六个进度及俯视。', 'sealer'),
 'shell': ('壳体', '安装耳和侧壁接口切出真实孔洞，接头改为空心截面；保留底壳、四壁、四个板件支柱与接缝。它是开口壳体，没有假称带上盖。', 'assembly'),
 'pcb': ('PCB', '从FR4板切出四个安装孔，去掉深色假孔；以贴板铜环和薄孔壁替代突起金属圆环，补底面铜色走线。元件/焊盘用于可视化，不宣称是可制造的电路设计。', 'assembly'),
 'fastener': ('螺钉', '去掉圆线绕制的螺纹，生成V形截面的连续螺旋牙及对应芯径；保留真实内六角槽、头部和端部。螺距/直径是声明设计值，未作工业标准件认证。', 'assembly'),
 'sealing-tape': ('胶带卷', '保留直壁卷材、贯通纸芯和末端片；背面也补卷绕层纹，纸芯改为纸质表面。静态卷材不伪造放卷或消耗动作。', 'sealer'),
}

stamp = datetime.now().astimezone().isoformat(timespec='seconds')
inventory = read(BASE / 'inventory.json')
ledger = read(ROOT / 'modules/media/model-quality-ledger.json')
rows = {r['model_path']: r for r in ledger['models']}
mechanisms = {r['family']: r for r in read(OUT/'batch/mechanisms.json')['evidence']}
checks = read(OUT/'batch/source-and-motion-checks.json')['results']
videos = read(OUT/'video-decode.json')['results']
assert len(checks) == len(videos) == 28
models, simple_models = [], []
for e in inventory['entries']:
 family, suffix = e['family'], e['suffix']
 name, finding, reference = FINDINGS[family]
 file = BASE/e['directory']/e['asset']['path']
 folder = family + ('-simple' if suffix == 'simple' else '')
 capture = read(OUT/folder/'capture.json')
 assert capture['version'] == '0.4.2' and capture['glb_bytes'] == file.stat().st_size
 assert capture['source']['id'] == e['asset']['id']
 row = rows[rel(file)]
 assert row['observed']['bytes'] == file.stat().st_size and row['observed']['modified_ns'] == file.stat().st_mtime_ns
 views = {v:rel(OUT/folder/(v+'.png')) for v in ('isometric','front','back','left','right','top','bottom')}
 closeups = [rel(OUT/folder/'closeup.png')]
 if suffix == 'detailed':
  assert mechanisms[family]['bytes'] == file.stat().st_size
  closeups += [rel(OUT/x['file']) for x in mechanisms[family]['images']]
 video, sheet = rel(OUT/folder/'motion.mp4'), rel(OUT/folder/'views.jpg')
 for x in [*views.values(), *closeups, video, sheet]: assert (ROOT/x).is_file(), x
 animated = bool(e['animations'])
 dimensions = {
  'MQ-01': {'result':'pass','reason':'已逐类查看七向、机构/材质近景；声明米/Y-up、原点和包络。物品尺度没有改作工业实测。','evidence':[sheet,rel(BASE/'inventory.json')]},
  'MQ-02': {'result':'pass','reason':'已查看机架/壳壁/背底、接地与支承；本轮看见的夹座悬空已修复后重拍。','evidence':[views['back'],views['bottom'],*closeups[:2]]},
  'MQ-03': {'result':'pass' if animated else 'not_applicable','reason':finding if animated else '静态物品/载具没有主动机构；实体构造仍检查，不为它编造生产动作。','evidence':[video,*closeups[:2]]},
  'MQ-04': {'result':'pass','reason':finding,'evidence':closeups},
  'MQ-05': {'result':'pass','reason':'在等效环境反射、照明和阴影条件下检查金属/塑料/橡胶/纸木材料及表面细节；简化档只按其LOD身份判断。','evidence':[sheet,*closeups[:2],rel(OUT/f'reference-{reference}/closeup.png')]},
  'MQ-06': {'result':'pass' if animated else 'not_applicable','reason':'实际GLB命名活动件、顺放/反向关键帧和12秒视频已检查；静帧重复整圈的问题由非整圈机位补拍解决。' if animated else '静态物品/载具只提供观察转台，非生产动作。','evidence':[video,*[rel(OUT/x) for x in capture['reverse_seek']]]},
  'MQ-07': {'result':'pending','reason':'本轮是模型级GLB与机构画面验收；尚未把0.4.2逐件采用到Native真实Run核对holder/移交/包件内容。','evidence':[rel(OUT/'逐模型审查结果.md')]},
  'MQ-08': {'result':'pending' if animated else 'not_applicable','reason':'0.4.2的真实阶段、四类等待、暂停/倍率/回看和保存重开仍待产品联合验收。' if animated else '没有自身生产阶段；真实持有关系仍由MQ-07待验。','evidence':[rel(OUT/'逐模型审查结果.md')]},
 }
 review = dict(reviewed_file=rel(file),file_revision={k:row['observed'][k] for k in ('bytes','modified_ns')},reviewer='Codex AI actual GLB image review',reviewed_at=stamp,human_acceptance='pending',appearance_review='pass' if suffix=='detailed' else 'pass_lod_only',open_defects=[],repair_summary=finding,scope='Actual GLB geometry/material and declared-motion image review; no actual-Run or Native acceptance',gates={
  'source_export':dict(result='pass',evidence=[rel(OUT/'batch/source-and-motion-checks.json'),rel(file.parent/'source/generate.mjs')]),
  'visual':dict(result='pending',reason='模型外观与声明机构已逐图审查，MQ-07/08产品联合部分未完成。',evidence=[sheet,video,*closeups[:2]]),
  'platform_reopen':dict(result='pending',reason='共享产品解码与场景绑定已实际渲染，完整Native采用/真实Run/保存重开未补签。',evidence=[rel(OUT/'逐模型审查结果.md')])},dimensions=dimensions if suffix=='detailed' else {},views=views,closeups=closeups,motion_evidence=[video],reference_comparison=dict(reference_files=[f'tools/3d-proof/output/{reference}.glb'],evidence=[rel(OUT/f'reference-{reference}/isometric.png'),sheet],conclusion='已按结构、机构可解释度、材质和命名动作与监督基准定性比较；物品参照原线中的部件，不虚构对应的旧独立模型。'))
 row['asset_quality']='pending';row['review']=review
 result=dict(family=family,name=name,version=e['version'],file=rel(file),repair=finding,review=review)
 (models if suffix=='detailed' else simple_models).append(result)
(ROOT/'modules/media/model-quality-ledger.json').write_text(json.dumps(ledger,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
(OUT/'逐模型审查结果.json').write_text(json.dumps(dict(reviewed_at=stamp,appearance_review='14 detailed and 14 simplified GLBs inspected',official_detailed_acceptance='pending',models=models,simplified_models=simple_models),ensure_ascii=False,indent=2)+'\n',encoding='utf8')
lines=['# 14类模型返修与实际画面审查 · 0.4.2','',f'审查日期：{stamp}。复核者：Codex AI。','',
 '本轮完成14类、详细/简化共28个GLB返修候选，逐类查看七向、近景和适用动作。模型级外观与声明机构达到本轮交付候选要求；不把数量、面数、源码重建、孔洞射线或视频解码当作画面合格依据。正式详细质量仍待MQ-07/08的真实产品联合验收与真人认可，未填accepted。','',
 '[完整图册](模型审阅图册.html) · [可旋转/拖动阶段的实际GLB审阅器](http://127.0.0.1:19289/tools/model-quality-review/v042/index.html) · [源码与登记](../../../modules/media/equipment-quality-v042/README.md)','',
 '|模型|逐项处理与画面观察|实际GLB与预览|','|---|---|---|']
for m in models:
 f=m['family'];lines.append(f'|{m["name"]}|{m["repair"]}|[GLB](../../../{m["file"]}) · [七向/近景]({f}/views.jpg) · [动作或观察视频]({f}/motion.mp4)|')
lines+=['','## 与监督基准的比较','',
 '原八设备实际复用六类GLB。本轮使用原文件的同显示条件捕获，逐类比较结构、支撑、机构、材料和动作，而非强制相同轮廓或面数。装配/返工台保留独立滑台和真实夹紧结构，装箱台有独立折板驱动及连续推杆；机器人保留曲面连杆、关节和夹爪。物品没有旧独立模型的，按原流水线内对应材料和装配部件作参照。原线的固定样件和箱垛不烘焙为新的业务库存。','',
 '在本轮实际观察范围，未见需要降低为灰盒、纸片或低精代理才能放行的模型；这是一项逐图AI判断，不能替代下述尚未完成的产品接触与真实Run检查。','',
 '## 实际捕获','',
 '- 每个GLB含斜视、正、背、左、右、顶、底及近景，共224张标准方向/近景。另含动作起中止、反向seek和专门的机构/孔洞机位。','- 28段视频均为1280×720、24fps、12秒，左侧整体、右侧近景。8类有声明动作，6类静态物品/载具是观察转台。','- 主轴另拍四个真正的最低行程点；装箱推杆单独取景；输送/封箱滚轮采用非整圈进度；纸箱另有六个折盖进度及俯视。','- 出现夹座悬空、铜环突起等缺陷时已改源、重导出并重拍，没有仅改报告。','',
 '## 仍须单独完成的产品联合检查','',
 'MQ-01至05为本轮模型外观检查；MQ-06为声明机构/状态预览。MQ-07的真实物品身份、夹持移交、入箱支承仍待0.4.2在实际工程采用后检查；适用模型的MQ-08真实阶段、四类等待、暂停回看和保存重开也待联合检查。已打开工程、冻结旧Run、B60及原版本未被本轮自动替换。','',
 '## 工具与可编辑源','',
 '使用现有Three.js参数化网格、曲面/挤出/螺旋几何和GLTFExporter，不假称调用Blender，不使用AI效果图替代模型。新几何与沿用原创表面按CC0声明，Three.js保持MIT依赖。28份包内源码在独立目录重开，导出与交付GLB一致；这些检查只辅助确认源与导出对应。','']
(OUT/'逐模型审查结果.md').write_text('\n'.join(lines),encoding='utf8')
print(json.dumps(dict(families=len(models),lod_models=len(simple_models),model_visual_review='complete',native_joint_acceptance='pending'),ensure_ascii=False))
