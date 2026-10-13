from pathlib import Path
import json,csv,sqlite3,html

B=Path(__file__).parent

def write_csv(path,rows):
    with path.open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)

def main():
    sources=json.loads((B/'来源目录.json').read_text(encoding='utf-8'))
    sources_by_id={s['id']:s for s in sources}
    tables=json.loads((B/'NBS表格提取临时.json').read_text(encoding='utf-8'))
    counts=next(t for t in tables if any(r and r[0]=='金属制品业' for r in t) and any('企业法人单位' in ' '.join(r) for r in t[:3]))
    assets=next(t for t in tables if any(r and r[0]=='金属制品业' for r in t) and any('营业收入' in ' '.join(r) for r in t[:3]))
    assets={r[0]:r[1:] for r in assets if len(r)==4 and r[1].replace('.','').isdigit()}
    data=[]
    for r in counts:
        if len(r)!=3 or not r[1].replace('.','').isdigit():continue
        a=assets[r[0]]
        data.append({'industry':r[0],'enterprise_legal_units_10k':r[1],'employees_10k':r[2],'assets_100m_cny':a[0],'liabilities_100m_cny':a[1],'revenue_100m_cny':a[2],'statistical_period':'2023年末；营业收入为2023全年','publication_date':'2024-12-26','source_id':'D01','location':'表3-2/表3-3','scope':'工业企业法人单位，不是厂房或本软件客户'})
    d=B/'数据';d.mkdir(exist_ok=True)
    write_csv(d/'五经普工业行业原始数表.csv',data)
    selected={'金属制品业','通用设备制造业','专用设备制造业','电气机械和器材制造业','计算机、通信和其他电子设备制造业','仪器仪表制造业'}
    write_csv(d/'首版相邻行业背景基数.csv',[r for r in data if r['industry'] in selected])
    claims=json.loads((B/'价值主张.json').read_text(encoding='utf-8'))
    carddir=B/'阅读卡';carddir.mkdir(exist_ok=True)
    for s in sources:
        s.setdefault('read_status','not_reviewed')
        if s.get('local_text'):
            lines=(B/s['local_text']).read_text(encoding='utf-8').splitlines()
            s['line_count']=len(lines)
        card=f"# {s['id']} {s['title']}\n\n发布者：{s['publisher']}\n\n发布日期：{s.get('published_date') or '页面未确认；不推测'}；读取：{s['retrieved_date']}。\n\n来源：[官方或作者原始来源]({s['url']})\n\n阅读范围：{s['reading_location']}。状态：{s['read_status']}。\n\n摘要：{s['summary']}\n\n适用限制：{s['limits']}\n\n本地取得状态：{s['download_status']}。"
        if s.get('local_original'):card+=f" [取得的源文件](../{s['local_original']})。"
        if s.get('local_text'):card+=f" [可检索正文](../{s['local_text']})。"
        if s.get('reading_record'):card+=f" [实际核读记录](../{s['reading_record']})。"
        card+=f"\n\n原文再分发：{s['public_redistribution']}。\n\n实施任务：{', '.join(s['implementation_tasks']) or '背景、市场或比赛说明'}。\n"
        (carddir/(s['id']+'.md')).write_text(card,encoding='utf-8')
    (B/'来源目录.json').write_text(json.dumps(sources,ensure_ascii=False,indent=2),encoding='utf-8')
    columns=['id','group','title','publisher','published_date','retrieved_date','url','download_status','read_status','local_original','local_text','reading_location','summary','limits']
    write_csv(B/'来源目录.csv',[{k:s.get(k) for k in columns} for s in sources])
    con=sqlite3.connect(Path(__file__).resolve().parent / '项目研究资料.sqlite')
    con.execute('CREATE TABLE IF NOT EXISTS claims (id TEXT PRIMARY KEY, claim TEXT, metadata TEXT)')
    con.execute('CREATE TABLE IF NOT EXISTS statistics (industry TEXT PRIMARY KEY, metadata TEXT)')
    con.execute('CREATE TABLE IF NOT EXISTS metrics (metric TEXT PRIMARY KEY, source_id TEXT, metadata TEXT)')
    for s in sources:
        text_file=s.get('local_text') or s.get('local_original')
        body=(B/text_file).read_text(encoding='utf-8') if text_file and not text_file.endswith('.pdf') else ''
        con.execute('INSERT OR REPLACE INTO sources VALUES (?,?,?,?,?,?,?)',(s['id'],s['group'],s['title'],s['url'],s['download_status'],json.dumps(s,ensure_ascii=False),body))
    for c in claims:con.execute('INSERT OR REPLACE INTO claims VALUES (?,?,?)',(c['id'],c['claim'],json.dumps(c,ensure_ascii=False)))
    for r in data:con.execute('INSERT OR REPLACE INTO statistics VALUES (?,?)',(r['industry'],json.dumps(r,ensure_ascii=False)))
    for m in json.loads((B/'数据/可引用背景指标.json').read_text(encoding='utf-8')):
        con.execute('INSERT OR REPLACE INTO metrics VALUES (?,?,?)',(m['metric'],m['source_id'],json.dumps(m,ensure_ascii=False)))
    con.commit();con.close()
    cards=[]
    for s in sources:
        key=s['id']; searchable=html.escape(' '.join([s['title'],s['group'],s['summary'],s['limits'],key]),quote=True)
        local=f'<a href="{html.escape(s["local_original"],quote=True)}">取得的源文件</a> · ' if s.get('local_original') else ''
        cards.append(f'<article data-q="{searchable}"><small>{html.escape(s["group"])} · {key}</small><h2>{html.escape(s["title"])}</h2><p>{html.escape(s["summary"])}</p><p class="muted">{html.escape(s["limits"])}</p><p>{local}<a href="阅读卡/{key}.md">阅读卡</a> · <a href="{html.escape(s["url"],quote=True)}">原始网址</a></p><small>下载：{s["download_status"]}；阅读：{html.escape(s["reading_location"])}</small></article>')
    page='''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>工厂方案彩排项目知识库</title><style>body{font:16px/1.65 system-ui;color:#263445;background:#eef1f5;margin:0}header,main{max-width:1100px;margin:auto;padding:24px}input{width:90%;font:inherit;padding:12px;border:1px solid #b6c0cb;border-radius:5px}article{background:white;padding:20px;margin:15px 0;border-radius:6px}h2{font-size:20px}.muted,small{color:#5c6876}a{color:#1266ba}nav{display:flex;gap:15px;flex-wrap:wrap}</style><header><h1>工厂方案彩排项目知识库</h1><p>背景需求、市场原始数表、已有覆盖、技术参考与比赛材料。事实、案例结果和产品假设分别整理。</p><nav><a href="市场与项目价值说明.md">市场与价值</a><a href="路演与提交材料依据.md">路演与提交</a><a href="技术参考与实现映射.md">技术导航</a><a href="已有覆盖与差异审查.md">已有覆盖</a><a href="数据/首版相邻行业背景基数.csv">行业背景数表</a></nav><p><input id="q" placeholder="搜索：物流 / CNC / 开源 / 工时 / P3…" aria-label="搜索资料"></p><p id="count"></p></header><main>'''+''.join(cards)+'''</main><script>const q=document.getElementById('q'),cards=[...document.querySelectorAll('article')];function filter(){let n=0;for(const c of cards){c.hidden=!c.dataset.q.toLowerCase().includes(q.value.toLowerCase());if(!c.hidden)n++}document.getElementById('count').textContent=n+' 条来源'}q.addEventListener('input',filter);filter()</script></html>'''
    page=page.replace('<nav>','<nav><a href="本轮缺口补证.md">最新补证与缺口</a>')
    page=page.replace('<nav>','<nav><a href="../软件工程/docs/00_项目基线/项目主线.json">现行主线</a><a href="../软件工程/docs/03_架构与契约/目标驱动厂房生成.md">目标驱动生成</a>')
    page=page.replace('<nav>','<nav><a href="../软件工程/docs/00_项目基线/行业需求与原文支撑.md">S01-02需求依据</a>')
    (Path(__file__).resolve().parent / '研究资料索引.html').write_text(page,encoding='utf-8')
    print(json.dumps({'sources':len(sources),'downloaded':sum(s['download_status']=='downloaded' for s in sources),'statistics_rows':len(data),'claims':len(claims)},ensure_ascii=False))

if __name__=='__main__':main()
