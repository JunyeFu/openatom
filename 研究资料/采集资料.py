from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
import json, csv, sqlite3
from urllib.request import Request, urlopen
from urllib.error import URLError
from html.parser import HTMLParser
from pypdf import PdfReader

class TextParser(HTMLParser):
    def __init__(self):
        super().__init__(); self.parts=[]; self.hidden=0; self.in_title=False; self.title=[]
    def handle_starttag(self,tag,attrs):
        if tag in ['script','style','noscript']: self.hidden+=1
        if tag=='title': self.in_title=True
        if tag in ['p','div','tr','li','h1','h2','h3','br']:self.parts.append('\n')
    def handle_endtag(self,tag):
        if tag in ['script','style','noscript']: self.hidden=max(0,self.hidden-1)
        if tag=='title':self.in_title=False
        if tag in ['p','div','tr','li','h1','h2','h3']:self.parts.append('\n')
    def handle_data(self,data):
        if not self.hidden:
            self.parts.append(data)
            if self.in_title:self.title.append(data)

BASE = Path(__file__).parent
SOURCES = BASE / '来源目录.json'

def acquire(row):
    folder = BASE / '原始资料' / row['group']
    folder.mkdir(parents=True, exist_ok=True)
    try:
        request=Request(row.get('download_url',row['url']),headers={'User-Agent':'Mozilla/5.0 (compatible; project-research)'})
        with urlopen(request,timeout=35) as response:
            row['http_status']=response.status
            row['resolved_url']=response.url
            content=response.read()
            mime=response.headers.get('Content-Type','')
            charset=response.headers.get_content_charset()
        row['content_type'] = mime
        if content.startswith(b'%PDF'):
            original = folder / (row['id'] + '.pdf')
            original.write_bytes(content)
            pages = []
            doc=PdfReader(original)
            row['page_count']=len(doc.pages)
            for i,page in enumerate(doc.pages):
                pages.append(f'\n--- PDF第{i+1}页 ---\n'+(page.extract_text() or ''))
            readable = '\n'.join(pages)
        elif 'html' in mime or content.lstrip().lower().startswith((b'<!doctype',b'<html')):
            original = folder / (row['id'] + '.html')
            original.write_bytes(content)
            parser=TextParser()
            parser.feed(content.decode(charset or 'utf-8',errors='replace'))
            row['page_title']=''.join(parser.title).strip()
            readable='\n'.join(line.strip() for line in ''.join(parser.parts).splitlines() if line.strip())
        else:
            original = folder / (row['id'] + '.txt')
            original.write_bytes(content)
            readable = content.decode('utf-8-sig')
        text_path = folder / (row['id'] + '_正文.txt')
        text_path.write_text(readable,encoding='utf-8')
        row['local_original'] = original.relative_to(BASE).as_posix()
        row['local_text'] = text_path.relative_to(BASE).as_posix()
        row['text_characters'] = len(readable)
        blocked = any(t in readable.lower() for t in ['access denied','enable javascript and cookies to continue','just a moment...'])
        row['download_status'] = 'needs_review' if blocked or len(readable) < 120 else 'downloaded'
        row['read_status'] = 'downloaded_not_fully_read'
        row['download_note'] = '页面疑似验证页或正文较少' if row['download_status']=='needs_review' else None
    except (URLError,OSError,ValueError,UnicodeError,RuntimeError) as exc:
        row['download_status']='failed'
        row['download_note']=str(exc)[:300]
        row['read_status']='web_excerpt_only_or_pending'
    return row

def main():
    rows=json.loads(SOURCES.read_text(encoding='utf-8-sig'))
    additional=BASE/'补充来源.json'
    if additional.exists():
        known={r['id'] for r in rows}
        rows.extend(r for r in json.loads(additional.read_text(encoding='utf-8')) if r['id'] not in known)
    pending=[r for r in rows if r['download_status']=='pending']
    with ThreadPoolExecutor(max_workers=6) as pool:
        futures={pool.submit(acquire,dict(r)):r['id'] for r in pending}
        for future in as_completed(futures):
            result=future.result()
            rows[next(i for i,r in enumerate(rows) if r['id']==result['id'])]=result
            print(result['id'],result['download_status'],result.get('text_characters',0),flush=True)
    SOURCES.write_text(json.dumps(rows,ensure_ascii=False,indent=2),encoding='utf-8')
    with (BASE/'来源目录.csv').open('w',encoding='utf-8-sig',newline='') as f:
        fields=['id','group','title','publisher','published_date','retrieved_date','url','download_status','read_status','local_original','local_text','reading_location','summary','limits']
        writer=csv.DictWriter(f,fieldnames=fields,extrasaction='ignore')
        writer.writeheader();writer.writerows(rows)
    con=sqlite3.connect(Path(__file__).resolve().parent / '项目研究资料.sqlite')
    con.execute('CREATE TABLE IF NOT EXISTS sources (id TEXT PRIMARY KEY, category TEXT, title TEXT, url TEXT, status TEXT, metadata TEXT, body TEXT)')
    for r in rows:
        body=(BASE/r['local_text']).read_text(encoding='utf-8') if r.get('local_text') else ''
        con.execute('INSERT OR REPLACE INTO sources VALUES (?,?,?,?,?,?,?)',(r['id'],r['group'],r['title'],r['url'],r['download_status'],json.dumps(r,ensure_ascii=False),body))
    con.commit();con.close()
    print('TOTAL',len(rows),'DOWNLOADED',sum(r['download_status']=='downloaded' for r in rows),'REVIEW',sum(r['download_status']=='needs_review' for r in rows),'FAILED',sum(r['download_status']=='failed' for r in rows))

if __name__=='__main__':main()
