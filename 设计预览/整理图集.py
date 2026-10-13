from pathlib import Path
import json, html, zipfile
base=Path(__file__).parent
rows=json.loads((base/'图片记录.json').read_text(encoding='utf-8-sig'))
escape=html.escape
cards=[]
for row in rows:
    file=base/row['filename']
    assert file.exists(), str(file)
    status='当前选用设计稿'
    cards.append(f'<section id="{escape(row["id"])}"><h2>{escape(row["id"]+" "+row["title"])}</h2><p>{status} · AI设计预览，非运行截图</p><a href="{escape(row["filename"])}"><img loading="lazy" src="{escape(row["filename"])}"></a></section>')
page='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>工厂方案彩排 · 预览图集</title><style>body{font:16px/1.7 system-ui;background:#f1f4f8;color:#20334e;margin:0}header,main{max-width:1400px;margin:auto;padding:24px}nav{position:sticky;top:0;background:white;padding:12px;display:flex;gap:16px;flex-wrap:wrap}a{color:#126acc}section{background:white;padding:20px;margin:24px 0;border-radius:8px}img{width:100%}h2{font-size:22px}</style><header><h1>工厂方案彩排 · 多层预览</h1><p>常态两张画布：流水线与多厂房。详细/简化/区域是同一流水线画布的三档显示；单击打开对象详情。所有图是AI设计预览，具体交互以最新规格为准。现行SF-17要求目标/场地→模板/九类填充→检查/实际模拟→初案达标直接交付，不足时有限修正/说明→四格式报告；现有22图尚未完整覆盖新增面板。</p><a href="当前交互规格.md">交互规格</a> · <a href="预览阅读说明.md">图稿适用范围与文字修正</a> · <a href="../软件工程/README.md">阶段与模块实现入口</a></header><nav>'''+''.join(f'<a href="#{escape(r["id"])}">{escape(r["id"])}</a>' for r in rows)+'</nav><main>'+''.join(cards)+'</main></html>'
(base/'图集.html').write_text(page,encoding='utf-8')
(base/'生图提示词.md').write_text('# 当前选用图片提示词\n\n原图提示词保留；新增目标生成交互按当前交互规格/视图生成计划，既有图不作该功能运行证据。\n\n'+'\n\n'.join('## '+r['id']+'\n'+r.get('prompt','见视图生成计划.json与生图修订提示词.json') for r in rows),encoding='utf-8')
with zipfile.ZipFile(base/'视图与设计包.zip','w',zipfile.ZIP_DEFLATED) as z:
    for r in rows: z.write(base/r['filename'],'设计预览/'+r['filename'])
    for name in ['图集.html','README.md','预览阅读说明.md','设备适配目录.json','当前交互规格.md','多层视图与对象设计.md','生图提示词.md','视图生成计划.json','生图修订提示词.json','图片记录.json']:
        z.write(base/name,'设计预览/'+name)
    for file in (Path(__file__).resolve().parent / '../软件工程').rglob('*'):
        if file.is_file() and '__pycache__' not in file.parts: z.write(file,'软件工程/'+file.relative_to(Path(__file__).resolve().parent / '../软件工程').as_posix())
print(json.dumps({'selected_images':len(rows),'revised':sum(bool(r.get('revision')) for r in rows),'gallery':'图集.html'},ensure_ascii=False))


