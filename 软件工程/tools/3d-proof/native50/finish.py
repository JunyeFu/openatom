"""Build the catalogue from actual saved GLB renders, not generated concept art."""
from pathlib import Path
import json
from html import escape
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[3]
HOME = ROOT / 'modules/media/native-equipment-50'
OUT = ROOT / 'scenarios/媒体表现/native50-20261021'
read = lambda p: json.loads(p.read_text(encoding='utf-8'))
inventory = read(HOME / 'inventory.json')
catalog = read(HOME / 'catalog.json')
render = read(OUT / 'render-checks.json')
assert len(render['checks']) == 100
checks = {(c['id'], c['lod']): c for c in render['checks']}
for entry in inventory['entries']:
    check = checks[entry['id'], entry['suffix']]
    assert check['module_ref'] == dict(id=entry['module_id'], version=entry.get('media_version', entry['version']))
    assert check['bytes'] == entry['bytes'] and check['nodes'] == entry['nodes']
    assert check['decode'] and check['phase_seek'] and check['reverse_seek']
    for capture in check['captures']:
        assert (OUT / capture).is_file(), capture

font_path = 'C:/Windows/Fonts/msyh.ttc'
font = ImageFont.truetype(font_path, 23)
big = ImageFont.truetype(font_path, 37)
entries = [e for e in inventory['entries'] if e['suffix'] == 'detailed']
sheet = Image.new('RGB', (2500, 2960), '#e4e9ee')
draw = ImageDraw.Draw(sheet)
draw.rectangle((0, 0, 2500, 110), fill='#183343')
draw.text((30, 15), '50种设备实际模型总览 · 33种纸包装及配套 + 17种常用设备', font=big, fill='white')
draw.text((30, 68), '50个设备定义 / 100个双LOD GLB / 磁盘模型重开渲染 / 原创示例尺寸与工时', font=font, fill='#c7dae6')
for i, entry in enumerate(entries):
    x, y = (i % 5) * 500, 110 + (i // 5) * 285
    image = Image.open(OUT / entry['id'] / 'detailed-iso.png').convert('RGB')
    image = image.crop((0, 110, 1600, 935)).resize((490, 253), Image.Resampling.LANCZOS)
    sheet.paste(image, (x + 5, y))
    draw.text((x + 12, y + 254), f"{entry['index']:02}  {entry['title']}", font=font, fill='#183343')
sheet.save(OUT / '50种设备实际模型总览.jpg', quality=94)

selected = ['flexo-printer', 'platen-die-cutter', 'digital-flatbed-cutter', 'folder-gluer', 'vision-carton-inspector', 'strapper']
core = Image.new('RGB', (1920, 930), '#e4e9ee')
draw = ImageDraw.Draw(core)
draw.rectangle((0, 0, 1920, 90), fill='#183343')
draw.text((28, 12), '纸包装核心设备 · 实际GLB重开画面', font=big, fill='white')
draw.text((28, 58), '50种设备模型库中的6种 · 不是AI效果图', font=font, fill='#c7dae6')
for i, name in enumerate(selected):
    entry = next(e for e in entries if e['id'] == name)
    x, y = (i % 3) * 640, 90 + (i // 3) * 420
    image = Image.open(OUT / name / 'detailed-iso.png').convert('RGB').crop((0, 110, 1600, 935)).resize((640, 330), Image.Resampling.LANCZOS)
    core.paste(image, (x, y))
    draw.text((x + 22, y + 345), entry['title'], font=big, fill='#183343')
core.save(OUT / '纸包装核心设备预览.jpg', quality=95)

cards = []
rows = []
for spec in catalog['equipment']:
    detail = next(e for e in entries if e['id'] == spec['id'])
    simple = next(e for e in inventory['entries'] if e['id'] == spec['id'] and e['suffix'] == 'simple')
    device_dir = HOME if spec['index'] == 1 else HOME / 'devices' / spec['id']
    device = read(device_dir / 'module.json')['data']['definition']['payload']['data']
    operation = device['behavior']['inline']
    phases = ' / '.join(p['id'] + ' ' + str(p['duration']['value']) + 'ms' for p in operation['phases'])
    ports = ' / '.join(p['id'] + ':' + p['direction'] for p in device['ports'])
    rows.append(f"|{spec['index']:02}|{spec['title']}|{spec['flow']}|{operation['builtin']}|{phases}|{spec['definition_id']}|")
    captures = ''.join(f'<a href="{spec["id"]}/{name}"><img loading="lazy" src="{spec["id"]}/{name}" alt="{escape(spec["title"])} {name}"></a>' for name in ['detailed-back.png', 'detailed-bottom.png', 'simple-iso.png', 'simple-back.png', 'simple-bottom.png', 'action-0.png', 'action-50.png', 'action-100.png'])
    cards.append(f'''<article data-group="{spec['group']}"><h2>{spec['index']:02} {escape(spec['title'])}</h2>
<img loading="lazy" src="{spec['id']}/detailed-iso.png" alt="{escape(spec['title'])}实际详细模型">
<p>{escape(spec['flow'])}</p><p><code>{spec['definition_id']}@1.0.0</code></p>
<p>规则：{operation['builtin']} · 声明周期{spec['cycle_seconds']}秒<br>阶段：{escape(phases)}</p>
<p>详细 {detail['triangles']:,}三角面 / {detail['bytes']/1048576:.2f}MiB · 简化 {simple['triangles']:,}三角面 / {simple['bytes']/1048576:.2f}MiB</p>
<p><a href="../../../modules/media/native-equipment-50/{detail['directory']}/{detail['asset']['path']}">详细GLB</a> · <a href="../../../modules/media/native-equipment-50/{simple['directory']}/{simple['asset']['path']}">简化GLB</a></p>
<details><summary>端口、双LOD背底与机构关键姿态</summary><p>{escape(ports)}</p>{captures}</details></article>''')

html = '''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>50种设备实际模型图册</title><style>
body{margin:0;background:#e7edf1;color:#20394a;font:15px 'Microsoft YaHei',sans-serif}header{padding:22px;background:#183343;color:white}h1{margin:0 0 12px}nav{padding:18px;display:flex;gap:12px}button{padding:9px 18px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(370px,1fr));gap:20px;padding:20px}article{background:white;padding:14px;border:1px solid #cad5dc;border-radius:8px}h2{font-size:20px;margin:6px 0}img{width:100%}p{line-height:1.6}code{font-size:12px}a{color:#227797}header a{color:#8ce0ca}details img{margin-top:12px}</style>
<header><h1>50种设备 · 实际模型图册</h1><p>33种纸包装及配套设备 + 17种常用设备；7个成熟L03机型保留精确版本，43种新增。100个详细/简化GLB，450张实际渲染图。</p><p>当前产品Host查询为50个设备、100个媒体包；当前产品GLB解码与场景绑定、声明阶段正反向seek通过。逐设备Native真实持有/等待/保存重开联合验收仍待执行；此处不是生产结果或专业计算证明。</p><a href="http://127.0.0.1:19290/tools/3d-proof/native50/index.html">打开可旋转／动作审阅器</a> · <a href="50种设备实际模型总览.jpg">50种设备总览</a> · <a href="render-checks.json">实际呈现检查</a></header>
<nav><button onclick="filter('all')">全部50种</button><button onclick="filter('paper')">纸包装33种</button><button onclick="filter('common')">常用17种</button></nav><main>''' + '\n'.join(cards) + '''</main><script>function filter(group){document.querySelectorAll('article').forEach(card=>card.hidden=group!=='all'&&card.dataset.group!==group)}</script></html>'''
(OUT / 'gallery.html').write_text(html, encoding='utf-8')
(HOME / '设备清单.md').write_text('# 50种原生设备定义与建模清单\n\n33种纸包装及配套设备优先，17种常用设备补齐50种。设备、LOD和物品分别计数；每行有独立精确设备定义。\n\n所有尺寸、周期与耗材数量是可编辑的声明示例。辅助耗材端口默认每周期领用1个预配用料单元（piece），不是厂家实测用量；工程采用时显式绑定对应物品定义、单位与数量。装配台按壳体1、PCB1、紧固件4领用；搬运保留ID，装箱创建带内容关系的新包件，检验读取显式质量输入。\n\n|序号|设备|物料路线|现有引擎规则|声明阶段|定义ID|\n|---|---|---|---|---|---|\n' + '\n'.join(rows) + '\n', encoding='utf-8')
result = dict(device_count=50, paper=33, common=17, model_count=100, captured_images=450, product_decoder_and_scene='pass', declared_phase_reverse_seek='pass', native_actual_run_wait_save_reopen='pending', formal_per_model_MQ_acceptance='pending')
(OUT / 'delivery.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps(result, ensure_ascii=False))
