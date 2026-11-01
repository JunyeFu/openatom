from pathlib import Path
from html import escape
import json,re
ROOT=Path(__file__).resolve().parents[3];HOME=ROOT/'modules/media/native-equipment-500';OUT=ROOT/'scenarios/媒体表现/native500-quality-20261025';OLD=ROOT/'scenarios/媒体表现/native500-20261024'
read=lambda p:json.loads(p.read_text(encoding='utf8'))
registry=read(HOME/'accepted-registration.json');count=registry['device_count'];pending=500-count;cards=[];page=(OLD/'gallery.html').read_text(encoding='utf8');batch_links=[]
accepted_refs={(e['id'],e['version'])for e in registry['register_order']if '.device.'in e['id']}
scope=read(HOME/'paper-packaging-review-scope.json');scope_text=f'当前纸包装目标：{scope["accepted_count"]}/{scope["target_count"]}种通过，{scope["remaining_count"]}种待审；完成本范围后停止。'
for batch in registry['review_batches']:
 review=HOME/'reviews'/batch;quality=read(review/'quality-review.json');assert quality['status']=='accepted_by_AI';version=quality['version'];prefix=batch.lower()
 batch_links.append(f'<a href="/modules/media/native-equipment-500/reviews/{batch}/review-notes.md">{escape(batch)} · {escape(version)} 复审记录</a>')
 for e in read(review/'catalog.json')['equipment']:
  if (e['definition_id'],version)not in accepted_refs:continue
  image=f'../native500-20261024/{prefix}-{e["id"]}/detailed-iso.png'
  cards.append(f'<article><h2>{escape(e["title"])} · {escape(version)}</h2><p>{escape(batch)} · MQ八项、实际持有/等待恢复及普通Native保存重开通过（AI审查）。</p><img loading="lazy" src="{image}" alt="已复审实际GLB"><details><summary>实际Native业务时点</summary><img loading="lazy" src="{batch}/{e["id"]}-native.jpg" alt="实际Native业务状态"></details><p><a href="/tools/3d-proof/native500/index.html?review={batch}">旋转与动作</a> · <a href="/modules/media/native-equipment-500/reviews/{batch}/installable/{e["id"]}/{e["id"]}.glb">详细GLB</a></p></article>')
  pattern=r'<article data-batch="'+e['batch']+'" data-text="[^"]*'+re.escape(e['id'])+r'"[\s\S]*?</article>'
  card=f'<article data-batch="{e["batch"]}" data-text="{escape(e["title"]+" "+e["id"])}"><h2>{e["index"]:03d} {escape(e["title"])}</h2><img loading="lazy" src="{prefix}-{e["id"]}/detailed-iso.png" alt="已复审GLB"><p>正式支持 · {escape(version)} · MQ八项及Native保存重开通过</p><p><a href="../native500-quality-20261025/review.html">逐机复审与Native状态</a> · <a href="/tools/3d-proof/native500/index.html?review={batch}">旋转／动作</a></p></article>'
  page,n=re.subn(pattern,lambda _:card,page);assert n==1,e['id']
banner=f'<p id="quality-review-summary"><strong>{scope_text}</strong> 全目录正式通过{count}/500种，{pending}种待审。仅accepted清单中的批次准入；B02-Q01@1.1.0因接地缺陷撤回，其1.1.1另行重验。<a href="../native500-quality-20261025/review.html">查看本批修正及实际Native画面</a>；原候选构建图不代替正式接收。</p>'
page=re.sub(r'<p style="background:#8a371f[\s\S]*?</p>','',page);page=re.sub(r'<p id="quality-review-summary">[\s\S]*?</p>','',page);page=page.replace('<header>','<header>'+banner,1)
page=re.sub(r'未通过的\d+种仍待逐机MQ及Native联合验收。',f'未通过的{pending}种仍待逐机MQ及Native联合验收。',page)
(OLD/'gallery.html').write_text(page,encoding='utf8')
css='body{font:16px Microsoft YaHei,sans-serif;background:#e7edf1;color:#20394a;margin:24px}header{background:#183343;color:white;padding:20px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(480px,1fr));gap:20px;margin-top:20px}article{background:white;padding:18px}img{width:100%}a{color:#1c7895}p{line-height:1.6}'
html=f'<!doctype html><html lang="zh-CN"><meta charset="utf-8"><link rel="icon" href="data:,"><title>Native500 · {count}种通过复审</title><style>{css}</style><header><h1>{scope_text}</h1><p>候选500种，正式支持{count}种，待审{pending}种。每批逐机核对双LOD、源重导出、MQ八项、真实物品/等待、普通Native保存和独立进程重开；其余候选不提前计数。</p></header><p>'+ ' · '.join(batch_links)+f' · <a href="/modules/media/native-equipment-500/quality-review-progress.json">{pending}种剩余状态</a></p><main>'+''.join(cards)+'</main></html>'
(OUT/'review.html').write_text(html,encoding='utf8');print(f'Gallery now reflects{count} exact accepted device types in{len(registry["review_batches"])} reviewed batches.')
