"""Actual-render contact sheets and browseable catalogue; never substitutes for MQ acceptance."""
from pathlib import Path
from html import escape
import json,sys
from PIL import Image,ImageDraw,ImageFont
ROOT=Path(__file__).resolve().parents[3]
HOME=ROOT/'modules/media/native-equipment-500'
OUT=ROOT/'scenarios/媒体表现/native500-20261024'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
inv=read(HOME/'inventory.json');entries=[e for e in inv['entries'] if e['suffix']=='detailed']
font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',23);big=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',37)
completed=[];captures=0
for b in inv['batches']:
 report=OUT/('render-'+b['id'].lower()+'.json')
 if not report.exists():continue
 check=read(report);assert len(check['checks'])==100
 for c in check['checks']:
  e=next(e for e in inv['entries'] if e['id']==c['id'] and e['suffix']==c['lod']);assert c['bytes']==e['bytes'] and c['nodes']==e['nodes']
  assert c['decode'] and c['phase_seek'] and c['reverse_seek']
  assert all((OUT/f).is_file() for f in c['captures']);captures+=len(c['captures'])
 sheet=Image.new('RGB',(2500,2960),'#e4e9ee');d=ImageDraw.Draw(sheet);d.rectangle((0,0,2500,110),fill='#183343')
 d.text((30,14),b['id']+' · '+b['title']+' · 50种实际模型',font=big,fill='white')
 d.text((30,68),'每设备独立定义 / 双档GLB / 七向及动作实拍 / Native联合验收待完成',font=font,fill='#c7dae6')
 subset=[e for e in entries if e['batch']==b['id']]
 for i,e in enumerate(subset):
  x,y=(i%5)*500,110+(i//5)*285
  with Image.open(OUT/e['id']/'detailed-iso.png') as im:sheet.paste(im.convert('RGB').crop((0,110,1600,935)).resize((490,253),Image.Resampling.LANCZOS),(x+5,y))
  d.text((x+12,y+254),f"{e['index']:03d}  {e['title']}",font=font,fill='#183343')
 sheet.save(OUT/(b['id']+'-50种设备.jpg'),quality=94);completed.append(b['id'])
if '--final' in sys.argv:assert len(completed)==10,'not all ten batches rendered'
cards=[];rows=[]
for e in entries:
 folder='../native50-20261021/'+e['id'] if e['batch']=='B01' and not (OUT/e['id']/'detailed-iso.png').exists() else e['id']
 flow=e['flow'] if isinstance(e['flow'],str) else e['flow']['input']+' → '+e['flow']['output']
 image=f'{folder}/detailed-iso.png';has_image=(OUT/image).is_file()
 views=['detailed-back.png','detailed-bottom.png','simple-iso.png','action-0.png','action-50.png','action-100.png']
 links=''.join(f'<a href="{folder}/{v}">{v}</a> ' for v in views if (OUT/folder/v).is_file())
 cards.append(f'''<article data-batch="{e['batch']}" data-text="{escape(e['title']+' '+e['id'])}"><h2>{e['index']:03d} {escape(e['title'])}</h2>
 {'<img loading="lazy" src="'+image+'" alt="'+escape(e['title'])+'实际模型">' if has_image else '<p>实拍尚未完成</p>'}
 <p>{escape(flow)}</p><p><code>{e['definition_id']}@{e['version']}</code></p><p>{e['batch']} · {e['rule']} · 声明周期{e['cycle_seconds']}秒</p>
 <p><a href="/modules/media/native-equipment-500/{e['directory']}/{e['asset']['path']}">详细GLB</a> · <a href="/tools/3d-proof/native500/index.html">旋转／动作审阅</a></p><details><summary>背底、简化及动作关键姿态</summary>{links}</details></article>''')
 rows.append(f"|{e['index']:03d}|{e['batch']}|{e['title']}|{flow}|{e['rule']}|{e['definition_id']}|")
buttons=''.join(f'<button onclick="choose(\'{b["id"]}\')">{b["id"]} {escape(b["title"])}</button>' for b in inv['batches'])
html=f'''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>500种设备 · 分批实际模型图册</title><link rel="icon" href="data:,"><style>
body{{margin:0;background:#e7edf1;color:#20394a;font:15px 'Microsoft YaHei',sans-serif}}header{{padding:22px;background:#183343;color:white}}h1{{margin:0 0 12px}}nav{{padding:18px;display:flex;gap:8px;flex-wrap:wrap}}button,input{{padding:9px 14px;font:inherit}}main{{display:grid;grid-template-columns:repeat(auto-fit,minmax(370px,1fr));gap:20px;padding:20px}}article{{background:white;padding:14px;border:1px solid #cad5dc;border-radius:8px}}article[hidden]{{display:none}}h2{{font-size:20px;margin:6px 0}}img{{width:100%}}p{{line-height:1.6}}code{{font-size:12px}}a{{color:#227797}}header a{{color:#8ce0ca}}</style>
<header><h1>500种设备 · 10批实际模型候选库</h1><p>原50种中45种沿用精确版本、5种动作修订至1.0.1；新增9批×50种。500个独立工艺定义，1000个双档GLB，1500个ModulePackage。</p>
<p>已完成实拍：{' / '.join(completed)}，共{captures}张。图册展示实际GLB。逐模型完整画面认可、Native实际物品持有／四类等待／保存重开仍待联合验收。</p>
<a href="/tools/3d-proof/native500/index.html">打开旋转与动作审阅器</a> · <a href="/modules/media/native-equipment-500/设备清单.md">完整设备清单</a></header>
<nav><button onclick="choose('all')">全部500种</button>{buttons}<input id="search" placeholder="搜索名称或英文ID" oninput="apply()"></nav><main>{''.join(cards)}</main>
<script>let selected='all';function choose(b){{selected=b;apply()}}function apply(){{const q=document.getElementById('search').value.toLowerCase();document.querySelectorAll('article').forEach(a=>a.hidden=(selected!=='all'&&a.dataset.batch!==selected)||!a.dataset.text.toLowerCase().includes(q))}}</script></html>'''
(OUT/'gallery.html').write_text(html,encoding='utf-8')
(HOME/'设备清单.md').write_text('# 500种原生设备建模候选清单\n\n每行是一种独立工艺定义。B01保留既有精确版本，B02—B10每批50种。详细/简化档、实例和相同设备的尺寸配色不重复计数；相同机构可服务不同的明确工艺定义。\n\n所有新增模型为原创可编辑参数化GLB。尺寸、周期及数量为声明示例，尚未逐设备完成正式MQ接收。物料名称表达代表件/代表批，不能直接等同工业连续物料衡算。实际工程须绑定具体物品、原料数量、工艺参数、资源、质量输入与布局。\n\n|序号|批次|设备|工艺路线|规则|定义ID|\n|---|---|---|---|---|---|\n'+'\n'.join(rows)+'\n',encoding='utf-8')
print(json.dumps(dict(rendered_batches=completed,new_captures=captures),ensure_ascii=False))
(OUT/'render-summary.json').write_text(json.dumps(dict(rendered_batches=completed,model_count=len(completed)*100,captured_images=captures,actual_capture_files_checked=True),ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
if (HOME/'reviews/B02-Q01/quality-review.json').is_file() and read(HOME/'reviews/B02-Q01/quality-review.json').get('status')=='accepted_by_AI':
 import runpy
 runpy.run_path(str(Path(__file__).with_name('paper-gallery.py')))
