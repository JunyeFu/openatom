"""Independent readers over actual public Host Task/store output, no rendering model."""
import csv,json,shutil,subprocess,sys,time
from pathlib import Path
from html.parser import HTMLParser
from pypdf import PdfReader
import pdfplumber
from PIL import Image,ImageDraw
ROOT=Path(__file__).resolve().parents[3]
OUT=ROOT/'.work/whole-packages/window5-timeline-evaluation'
SOURCE=Path(sys.argv[1]).resolve()
MOVED=OUT/('moved-host-reports-'+str(time.time_ns()));MOVED.mkdir()
CONTRACT=json.loads((ROOT/'packages/reporting/evaluation-report.contract.design.json').read_text(encoding='utf8'))
RESULTS=[];REPORTS={}
def check(name,work):
    try:RESULTS.append({'name':name,'passed':True,'details':work()})
    except Exception as exc:RESULTS.append({'name':name,'passed':False,'failure':repr(exc)})
class Html(HTMLParser):
    def __init__(self):super().__init__();self.active=False;self.data=[];self.external=[]
    def handle_starttag(self,tag,attrs):
        a=dict(attrs)
        if tag=='script' and a.get('id')=='report-data':self.active=True
        if tag in ['script','link','img','iframe']:
            for key in ['src','href']:
                if a.get(key,'').startswith(('http:','https:','//')):self.external.append(a[key])
    def handle_endtag(self,tag):
        if tag=='script':self.active=False
    def handle_data(self,data):
        if self.active:self.data.append(data)
def read(path):return json.loads(path.read_text(encoding='utf8'))
def csv_rows(path):
    raw=path.read_bytes();assert raw.startswith(b'\xef\xbb\xbf');assert b'\r\n' in raw
    with path.open(encoding='utf-8-sig',newline='') as stream:
        reader=csv.DictReader(stream);return reader.fieldnames,list(reader)
for name in ['single','comparison','incomplete','historical-design','design']:
    target=MOVED/name;shutil.copytree(SOURCE/name,target)
    report=read(target/'report.json');REPORTS[name]=report['data']
    def context_and_files(target=target,report=report):
        output=read(target/'actual-task-output.json');assert output['context']==report['data']
        parser=Html();parser.feed((target/'index.html').read_text(encoding='utf8'))
        assert json.loads(''.join(parser.data))==report and not parser.external
        prefixes={Path(f['path']).parts[0] for f in output['files']};assert len(prefixes)==1
        for f in output['files']:
            relative=Path(*Path(f['path']).parts[1:]);assert relative.parts and '..' not in relative.parts
            assert (target/relative).is_file(),str(relative)
        assert report['data']['editable_project_ref']['path']=='project/project.json'
        assert any(i['code']=='evaluation.actual_render_missing' for i in report['data']['missing_conditions'])
        return {'typed_Task_context_equals_JSON_and_HTML':True,'real_returned_files':len(output['files']),'render_missing_explicit':True}
    check(name+'_actual_Task_context_and_every_returned_file',context_and_files)
    def tables(target=target,report=report):
        assert len(list((target/'csv').glob('*.csv')))==15;rows_total=0
        for table in CONTRACT['csv_projection']['tables']:
            fields,records=csv_rows(target/'csv'/table['file']);required=CONTRACT['csv_projection']['prefix_columns']+table['columns'];assert set(required)<=set(fields)
            for record in records:
                assert record['report_id']==report['data']['id'];assert record['project_id']==report['data']['project_id']
                for nullable in table['nullable_columns']:
                    assert nullable+'_state' in record and nullable+'_reason' in record
                    if record[nullable]=='':assert record[nullable+'_state'] in ['unknown','not_applicable','empty'] and record[nullable+'_reason']
            rows_total+=len(records)
        return {'standard_BOM_CRLF_CSV_tables':15,'rows':rows_total}
    check(name+'_standard_CSV_prefix_fields_and_optional_states',tables)
    def archive(target=target,report=report):
        root=target/'project';manifest=read(root/'project.json')['data'];modules={};runs={};active=[]
        for entry in manifest['documents']:
            file=root/entry['file_ref']['path'];assert file.is_file()
            value=read(file)['data']
            if entry['role']=='module_definition':
                ref=entry['module_ref'];definition=value['definition'];assert definition['id']==ref['id'] and definition['version']==ref['version'];modules[(ref['id'],ref['version'])]=file
            elif entry['role']=='active_project':active.append(value)
            elif entry['role']=='run_result':runs[value['id']]=value
        assert len(active)==1
        for evaluation in report['data']['run_evaluations']:
            run=runs[evaluation['run_id']]
            for field in ['project_id','project_revision','scenario_id','scenario_revision']:assert evaluation['input_ref'][field]==run['input_ref'][field]
            graph=read(root/run['input_ref']['frozen_project_file']['path'])['data'];scenario=read(root/run['input_ref']['frozen_scenario_file']['path'])['data']
            assert graph['revision']==run['input_ref']['project_revision'];assert scenario['revision']==run['input_ref']['scenario_revision']
            for module in graph['module_refs']:assert (module['id'],module['version']) in modules
        return {'real_manifest_documents':len(manifest['documents']),'exact_Module_files':len(modules),'stored_real_Runs':len(runs),'active_project_revision':active[0]['revision']}
    check(name+'_portable_manifest_and_real_frozen_Module_files',archive)
    if name in ['single','design']:
        def pdf(target=target):
            reader=PdfReader(target/'report.pdf');texts=[p.extract_text() or '' for p in reader.pages];assert '超级工厂' in ''.join(texts)
            with pdfplumber.open(target/'report.pdf') as document:
                for page in document.pages:
                    assert abs(page.width-595.276)<1 and abs(page.height-841.89)<1
                    assert page.chars and not [c for c in page.chars if c['x0']<-1 or c['x1']>page.width+1 or c['top']<-1 or c['bottom']>page.height+1]
            assert (target/'licenses/SourceHanSans-LICENSE.txt').read_bytes()==(ROOT/'tools/architecture-proof/assets/LICENSE.SourceHanSans.txt').read_bytes()
            (target/'pdf-text.txt').write_text('\n\n'.join(texts),encoding='utf8');render=target/'render';render.mkdir()
            result=subprocess.run(['pdftoppm','-r','72','-png',str(target/'report.pdf'),str(render/'page')],capture_output=True);assert result.returncode==0,result.stderr.decode(errors='replace')
            pages=sorted(render.glob('page-*.png'));assert len(pages)==len(reader.pages)
            for offset in range(0,len(pages),10):
                sheet=Image.new('RGB',(1200,925),'#c9cfd4');draw=ImageDraw.Draw(sheet)
                for i,p in enumerate(pages[offset:offset+10]):
                    im=Image.open(p);im.thumbnail((232,440));x=i%5*240;y=i//5*462;sheet.paste(im,(x,y+24));draw.text((x,y+3),p.stem,fill='black')
                sheet.save(target/f'sheet-{offset//10+1}.png')
            return {'actual_Host_printpdf_pages':len(reader.pages),'all_Poppler_pages_rendered':True,'Chinese_A4_physical_bounds_and_same_OFL':True,'visual_inspection_pending':True}
        check(name+'_actual_Host_PDF_and_license_render',pdf)
def semantics():
    single=REPORTS['single'];assert len(single['run_evaluations'])==1 and single['scenario_comparison'] is None
    first=single['run_evaluations'][0];assert first['order_metrics'][0]['completion_ms']==75000
    assert first['costs']['lines'][0]['amount']==135000/7
    comparison=REPORTS['comparison'];runs=sorted(comparison['run_evaluations'],key=lambda r:r['input_ref']['scenario_revision']);assert len(runs)==3
    assert runs[0]['costs']['lines'][0]['amount']==135000/7 and runs[1]['costs']['lines'][0]['amount']==270000/7
    assert runs[2]['costs']['inputs']==[] and runs[2]['costs']['missing_conditions']
    assert all(c['complete_total'] is None and c['unit_cost'] is None for c in runs[2]['costs']['currencies'])
    pair=comparison['scenario_comparison']['pairs'][0];assert pair['status']=='controlled_changes';assert any(c['path']=='/scenario/cost_inputs/frozen-carton' for c in pair['input_changes'])
    incomplete=REPORTS['incomplete']['run_evaluations'][0]['order_metrics'][0];assert incomplete['completion_ms'] is None and incomplete['tardiness_ms'] is None and incomplete['outstanding']['value']['coefficient']==1
    for name in ['design','historical-design']:assert REPORTS[name]['run_refs']==[] and REPORTS[name]['run_evaluations']==[]
    design=MOVED/'design/project';manifest=read(design/'project.json')['data'];active=read(design/next(d['file_ref']['path'] for d in manifest['documents'] if d['role']=='active_project'))['data'];assert any(a['id']=='W5-REPORT-PURPOSE' for a in active['acknowledgments']) and active['bindings']==[]
    return {'single_completion_ms':75000,'historical_exact_amounts':[135000/7,270000/7],'new_missing_price_not_zero':True,'comparison':'controlled_changes','incomplete_C_tardiness':None,'retained_design_purpose_not_human_acceptance':True}
check('fixed_independent_single_comparison_unknown_and_design_semantics',semantics)
result={'moved_directory':str(MOVED),'source_actual_Host_output':str(SOURCE),'checks':RESULTS,'whole_task_done':False,'human_acceptance':False}
(OUT/'host-output-readback.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
for item in RESULTS:print(json.dumps(item,ensure_ascii=False))
raise SystemExit(0 if all(i['passed'] for i in RESULTS) else 1)
