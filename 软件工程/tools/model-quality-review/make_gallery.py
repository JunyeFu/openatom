"""Assemble actual GLB capture images; no generated illustration substitutes."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import json
import html

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'scenarios/媒体表现/model-quality-20261019'
MODELS = ROOT / 'modules/media/equipment-quality-v041'
NAMES = dict(assembly='装配台', rework='返工台', packing='装箱台', robot='六轴机器人',
             conveyor='输送机', inspection='检验台', sealer='封箱机', pallet='托盘',
             carton='纸箱', shell='壳体', pcb='PCB', fastener='螺钉')
NAMES.update({'control-box': '控制盒', 'sealing-tape': '胶带卷'})
VIEWS = dict(isometric='整体斜视', front='正面', back='背面', left='左侧',
             right='右侧', top='顶部', bottom='底部', closeup='关键细节')
font = ImageFont.truetype('C:/Windows/Fonts/msyh.ttc', 22)
small = ImageFont.truetype('C:/Windows/Fonts/msyh.ttc', 17)
inventory = json.loads((MODELS / 'inventory.json').read_text(encoding='utf-8'))
entries = [e for e in inventory['entries'] if e['suffix'] == 'detailed']
review_file=OUT/'逐模型审查结果.json'
reviews={r['family']:r for r in json.loads(review_file.read_text(encoding='utf-8'))['models']} if review_file.exists() else {}
sections = []
overview = Image.new('RGB', (2400, 1600), '#edf2f6')
ov = ImageDraw.Draw(overview)
ov.text((24, 12), '14类返修模型 · 实际GLB画面 · 0.4.0 / 0.4.1', font=font, fill='#233a4c')
for entry in entries:
    family = entry['family']
    folder = OUT / family
    if not all((folder / f'{view}.png').exists() for view in VIEWS):
        continue
    sheet = Image.new('RGB', (1600, 540), '#f1f4f7')
    draw = ImageDraw.Draw(sheet)
    draw.text((16, 8), NAMES[family] + ' · 详细档实际GLB · ' + entry['version'], font=font, fill='#223a4c')
    for i, (view, title) in enumerate(VIEWS.items()):
        image = Image.open(folder / f'{view}.png').convert('RGB')
        image.thumbnail((396, 223))
        x, y = (i % 4) * 400, 45 + (i // 4) * 247
        sheet.paste(image, (x + (400 - image.width) // 2, y))
        draw.text((x + 12, y + 225), title, font=small, fill='#223a4c')
    sheet.save(folder / 'views.jpg', quality=95)
    oi = len(sections)
    thumb = Image.open(folder / 'isometric.png').convert('RGB'); thumb.thumbnail((592, 333))
    ox, oy = (oi % 4) * 600, 55 + (oi // 4) * 380
    overview.paste(thumb, (ox + (600-thumb.width)//2, oy))
    ov.text((ox + 16, oy + 335), NAMES[family], font=font, fill='#233a4c')
    motion = sorted(p for p in folder.glob('*.png') if p.stem.endswith(('-start', '-middle', '-end')))
    if motion:
        # Use the exported clip order, rather than putting auto before load.
        phases = ['open', 'fold_preview', 'sealed'] if family == 'carton' else [clip['name'] for clip in entry['animations']]
        motion = sorted(motion, key=lambda p: (phases.index(p.stem.rsplit('-', 1)[0]), ['start', 'middle', 'end'].index(p.stem.rsplit('-', 1)[1])))
        height = 50 + ((len(motion) + 2) // 3) * 263
        action = Image.new('RGB', (1440, height), '#f1f4f7')
        d = ImageDraw.Draw(action)
        d.text((14, 8), NAMES[family] + ' · 声明阶段动作关键帧（非生产结果）', font=font, fill='#223a4c')
        for i, p in enumerate(motion):
            image = Image.open(p).convert('RGB'); image.thumbnail((476, 236))
            x, y = (i % 3) * 480, 45 + (i // 3) * 263
            action.paste(image, (x + (480 - image.width) // 2, y))
            d.text((x + 12, y + 238), p.stem, font=small, fill='#223a4c')
        action.save(folder / 'actions.jpg', quality=94)
        detail = Image.new('RGB', (1440, 50 + ((len(motion) + 2) // 3) * 294), '#f1f4f7')
        dd = ImageDraw.Draw(detail)
        dd.text((14, 8), NAMES[family] + ' · 实际机构近景：开始 / 中段 / 完成', font=font, fill='#223a4c')
        for i, p in enumerate(motion):
            actual = folder / (p.stem + '-detail.png')
            image = Image.open(actual).convert('RGB'); image.thumbnail((476, 268))
            x, y = (i % 3) * 480, 45 + (i // 3) * 294
            detail.paste(image, (x + (480 - image.width) // 2, y))
            dd.text((x + 12, y + 270), p.stem, font=small, fill='#223a4c')
        detail.save(folder / 'action-details.jpg', quality=95)
    path = '../../../modules/media/equipment-quality-v041/' + entry['directory'] + '/' + entry['asset']['path']
    images = ''.join(f'<a href="{family}/{v}.png"><img loading="lazy" src="{family}/{v}.png" alt="{title}"><span>{title}</span></a>' for v, title in VIEWS.items())
    is_static = not entry['animations']
    caption = '静态物品/载具：视频为观察转台，不虚构自身生产动作。' if is_static else '实际GLB声明动作/状态预览；不是生产结果或工业运动学认证。'
    compare = ''
    baseline = family if family in ('assembly','robot','conveyor','inspection','sealer','pallet') else 'assembly' if family in ('rework','packing') else None
    if baseline and (OUT / f'reference-{baseline}/isometric.png').exists():
        comparison = Image.new('RGB', (1600, 1010), '#edf2f6'); cd = ImageDraw.Draw(comparison)
        for ci, (src, label) in enumerate([(f'reference-{baseline}/isometric.png','原监督参照 / 整体'), (f'{family}/isometric.png','本次返修 / 整体'), (f'reference-{baseline}/closeup.png','原监督参照 / 近景'), (f'{family}/closeup.png','本次返修 / 近景')]):
            im=Image.open(OUT/src).convert('RGB'); im.thumbnail((792,445)); xx,yy=(ci%2)*800, (ci//2)*500
            comparison.paste(im,(xx,yy+40));cd.text((xx+12,yy+8),label,font=font,fill='#233a4c')
        comparison.save(folder/'reference-comparison.jpg',quality=95)
        compare=f'<details><summary>与原监督试样同显示条件比较</summary><p>几何和照明来自实际GLB；观察构造与材质层次。参照含原有固定参考工件，本次资产不嵌入重复业务物品。近景目标按机构位置取景，未假装两个不同机构完全相同。</p><img class="wide" src="{family}/reference-comparison.jpg"></details>'
    capture=json.loads((folder/'capture.json').read_text(encoding='utf-8'))
    reverse=''.join(f'<a href="{html.escape(p)}"><img loading="lazy" src="{html.escape(p)}" alt="反向回看"><span>{html.escape(Path(p).stem)}</span></a>' for p in capture.get('reverse_seek',[]))
    reverse_details=f'<details><summary>反向回看实际近景</summary><div class="grid">{reverse}</div></details>' if reverse else ''
    repairs=f'<p><strong>本次返修：</strong>{html.escape(reviews[family]["repair"])}</p>' if family in reviews else ''
    simple = next(e for e in inventory['entries'] if e['family'] == family and e['suffix'] == 'simple')
    simple_path = '../../../modules/media/equipment-quality-v041/' + simple['directory'] + '/' + simple['asset']['path']
    simple_folder = OUT / (family + '-simple')
    simple_images = ''.join(f'<a href="{family}-simple/{v}.png"><img loading="lazy" src="{family}-simple/{v}.png" alt="简化档{title}"><span>{title}</span></a>' for v, title in VIEWS.items())
    simple_sheet = Image.new('RGB', (1600, 540), '#f1f4f7'); sd = ImageDraw.Draw(simple_sheet)
    sd.text((16, 8), NAMES[family] + ' · 简化档实际GLB · ' + simple['version'], font=font, fill='#223a4c')
    for i, (v, title) in enumerate(VIEWS.items()):
        im = Image.open(simple_folder / (v + '.png')).convert('RGB'); im.thumbnail((396, 223))
        x, y = (i % 4) * 400, 45 + (i // 4) * 247
        simple_sheet.paste(im, (x + (400-im.width)//2, y)); sd.text((x+12,y+225), title, font=small, fill='#223a4c')
    simple_sheet.save(simple_folder / 'views.jpg', quality=95)
    simple_preview = f'<details><summary>简化档：七向、近景与适用动作（{simple["version"]}）</summary><p>简化档单独读取其GLB，保留轮廓、端口及运动层级；不替代详细档。</p><video controls preload="none" poster="{family}-simple/isometric.png" src="{family}-simple/motion.mp4"></video><div class="grid">{simple_images}</div></details>'
    sections.append(f'<section id="{family}"><h2>{NAMES[family]} · {entry["version"]}</h2>{repairs}<p>{caption} <a download href="{path}">下载详细GLB</a> · <a download href="{simple_path}">下载简化GLB</a> · <a href="../../../modules/media/equipment-quality-v041/{entry["directory"]}/module.json">节点/动作/锚点定义</a></p><video controls preload="none" poster="{family}/isometric.png" src="{family}/motion.mp4"></video><div class="grid">{images}</div>' + (f'<details open><summary>机构动作近景：按导出阶段顺序</summary><a href="{family}/action-details.jpg"><img class="wide" src="{family}/action-details.jpg"></a></details><details><summary>逐阶段整机关键帧</summary><img class="wide" src="{family}/actions.jpg"></details>' if motion else '') + reverse_details + f'<details><summary>实际视频抽帧</summary><img class="wide" src="{family}/video-frames.jpg"></details>' + compare + simple_preview + '</section>')
nav = ' '.join(f'<a href="#{e["family"]}">{NAMES[e["family"]]}</a>' for e in entries)
page = f'''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>14类模型 · 实际GLB多视角与动作图册</title><style>body{{font:16px "Segoe UI","Microsoft YaHei",sans-serif;background:#edf2f6;color:#24394a;margin:0}}header,main{{max-width:1440px;margin:auto;padding:24px}}h1{{font-size:30px}}nav{{display:flex;flex-wrap:wrap;gap:12px}}a{{color:#245c7e}}section{{background:white;border:1px solid #cad5df;border-radius:8px;padding:20px;margin:24px 0}}video{{display:block;width:min(100%,960px);margin:auto;background:#e4e9ee}}.grid{{display:grid;grid-template-columns:repeat(4,1fr);gap:12px;margin-top:16px}}.grid a{{text-align:center;text-decoration:none}}.grid img{{width:100%;display:block}}.wide{{max-width:100%}}p{{line-height:1.8}}@media(max-width:900px){{.grid{{grid-template-columns:repeat(2,1fr)}}}}</style><header><h1>14类模型 · 实际GLB多视角与动作</h1><p>0.4版实际GLB由产品共享解码与场景绑定代码重开，采用与原监督试样相同类型的环境反射、阴影和表面细节显示。图片均为真实三维画面，可点击原始1920×1080图。尚未据此补签Native、实际生产或真人认可。</p><nav>{nav}</nav></header><main>{''.join(sections)}</main></html>'''
overview.save(OUT/'14类实际模型总览.jpg',quality=95)
page=page.replace('0.4版实际GLB由产品共享解码与场景绑定代码重开', '当前选用0.4.0及0.4.1精确版本，共28个实际GLB；每类提供详细与简化两档的七向、近景及适用动作。实际GLB由产品共享解码与场景绑定代码重开')
page=page.replace('<nav>', '<p><a href="../../../modules/media/equipment-quality-v041/README.md">交付与重建说明</a> · <a href="逐模型审查结果.md">逐模型审查结果</a> · <a href="http://127.0.0.1:19289/tools/model-quality-review/index.html">交互式GLB审阅器（需本机服务）</a></p><img class="wide" src="14类实际模型总览.jpg" alt="14类真实GLB总览"><nav>',1)
(OUT / '模型审阅图册.html').write_text(page, encoding='utf-8')
print(json.dumps({'complete_model_sheets':len(sections),'gallery':str(OUT/'模型审阅图册.html')}, ensure_ascii=False))
