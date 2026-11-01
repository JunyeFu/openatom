"""Read actual Rust-produced files; static HTML inspection is not browser acceptance."""
from pathlib import Path
from html.parser import HTMLParser
from PIL import Image, ImageDraw
import json, csv
import pypdfium2 as pdfium
from pypdf import PdfReader

ROOT = Path(__file__).resolve().parent

class HTML(HTMLParser):
    def __init__(self):
        super().__init__(); self.data = ''; self.in_data = False; self.remote = []; self.tables = 0
    def handle_starttag(self, tag, attrs):
        a=dict(attrs)
        if tag=='script' and a.get('id')=='report-data': self.in_data=True
        if tag=='table': self.tables+=1
        for key in ('src','href'):
            if a.get(key,'').startswith(('http:','https:','//')): self.remote.append(a[key])
    def handle_endtag(self, tag):
        if tag=='script': self.in_data=False
    def handle_data(self, data):
        if self.in_data: self.data+=data

def main():
    render=ROOT/'render'; render.mkdir(exist_ok=True); results={}
    for name in ('comparison','single','design'):
        folder=ROOT/'output'/name; expected=json.loads((folder/'report.json').read_text('utf-8'))
        h=HTML(); h.feed((folder/'index.html').read_text('utf-8'))
        assert json.loads(h.data)==expected
        assert not h.remote and h.tables==15
        csv_counts={}; csv_rows={}
        for file in sorted((folder/'csv').glob('*.csv')):
            raw=file.read_bytes(); assert raw.startswith(b'\xef\xbb\xbf')
            with file.open(encoding='utf-8-sig',newline='') as f:
                rows=list(csv.DictReader(f)); csv_counts[file.name]=len(rows); csv_rows[file.name]=rows
                assert all(row['report_id']==expected['data']['id'] for row in rows)
                for row in rows:
                    if not row['run_id']:
                        assert row['computed_until_ms']==''
                        assert row['computed_until_ms_state']=='not_applicable'
                        assert '单一实际Run' in row['computed_until_ms_reason']
        assert len(csv_counts)==15
        doc=pdfium.PdfDocument(str(folder/'report.pdf')); reader=PdfReader(folder/'report.pdf')
        text='\n'.join(p.extract_text() or '' for p in reader.pages)
        assert '超级工厂' in text and '工程评估' in text
        assert expected['data']['id'] in text
        mode=expected['data']['mode']
        if name=='design':
            assert mode=='design_description'
            assert expected['data']['run_refs']==[] and expected['data']['run_evaluations']==[]
            assert expected['data']['scenario_comparison'] is None
            assert not any('产线模拟结果来自真实软件Run' in v for v in expected['data']['conclusions'])
            assert '产线模拟结果来自真实软件Run' not in text
            assert '单次实际Run可直接交付' not in text
            assert '方案比较不适用' in text
        elif name=='single':
            assert mode=='simulation_evaluation' and len(expected['data']['run_refs'])==1
            assert expected['data']['scenario_comparison'] is None
            assert '单次实际Run可直接交付' in text
        else:
            assert mode=='simulation_evaluation' and len(expected['data']['run_refs'])==2
            assert expected['data']['scenario_comparison'] is not None
            assert '受控变更比较' in text
        thumbnails=[]
        for n in range(len(doc)):
            page=doc[n]; pic=page.render(scale=.6).to_pil().convert('RGB'); thumbnails.append(pic)
            if n in {0,1,len(doc)-1}: page.render(scale=1.45).to_pil().save(render/f'{name}-page-{n+1}.png')
        for offset in range(0,len(thumbnails),12):
            sheet=Image.new('RGB',(1440,1590),'#cad4dc'); draw=ImageDraw.Draw(sheet)
            for k,img in enumerate(thumbnails[offset:offset+12]):
                img.thumbnail((340,505)); x=k%4*360+10; y=k//4*530+20
                sheet.paste(img,(x,y));draw.text((x,y-15),f'{name} page {offset+k+1}',fill='black')
            sheet.save(render/f'{name}-contact-{offset//12+1}.png')
        results[name]={'pages':len(doc),'pdf_bytes':(folder/'report.pdf').stat().st_size,
            'chinese_text_readback':True,'html_embedded_report_equal_json':True,
            'html_remote_assets':h.remote,'html_tables':h.tables,'csv_record_counts':csv_counts,
            'mode_consistency_verified':True,
            'non_single_Run_boundary_not_applicable_verified':True,
            'actual_browser_interaction':'not_verified_file_protocol_blocked_by_security_policy'}
        print(name, 'PDF pages',len(doc),'HTML/JSON same source; 15 CSV tables reopened')
    (ROOT/'file-readback.json').write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

if __name__=='__main__': main()
