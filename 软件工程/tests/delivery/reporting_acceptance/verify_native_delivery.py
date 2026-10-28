"""Independent standard readers of Root's real OS-selected Native outputs."""
import csv, json, shutil, subprocess, zipfile
from pathlib import Path
from html.parser import HTMLParser
from pypdf import PdfReader
import pdfplumber
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[3]
SOURCE = ROOT / '.work/whole-packages/zip-delivery-native-ui'
OUT = ROOT / '.work/whole-packages/window5-timeline-evaluation/native-repair-79676'
MOVED = OUT / 'moved-report'
def read(path):
    return json.loads(path.read_text(encoding='utf-8'))
TASK = read(OUT / 'native-report-task.json')['result']['data']
REPORT_SOURCE = SOURCE / 'os-selected-reports/evaluation-00000004'
shutil.copytree(REPORT_SOURCE, MOVED, dirs_exist_ok=True)
REPORT = read(MOVED / 'report.json')
DATA = REPORT['data']
CONTRACT = read(ROOT / 'packages/reporting/evaluation-report.contract.design.json')
RESULTS = []
def check(name, work):
    try:
        RESULTS.append({'name': name, 'passed': True, 'details': work()})
    except Exception as exc:
        RESULTS.append({'name': name, 'passed': False, 'failure': repr(exc)})

class Html(HTMLParser):
    def __init__(self):
        super().__init__(); self.active = False; self.data = []; self.external = []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'script' and attrs.get('id') == 'report-data': self.active = True
        if tag in ['script', 'link', 'img', 'iframe']:
            self.external.extend(v for k, v in attrs.items() if k in ['src', 'href'] and v.startswith(('http:', 'https:', '//')))
    def handle_endtag(self, tag):
        if tag == 'script': self.active = False
    def handle_data(self, data):
        if self.active: self.data.append(data)

def context_files():
    assert TASK['context'] == DATA
    assert len(TASK['files']) == 74
    for ref in TASK['files']:
        parts = Path(ref['path']).parts
        assert parts[0] == 'evaluation-00000004' and '..' not in parts
        file = MOVED.joinpath(*parts[1:])
        assert file.is_file(), ref
        if file.suffix == '.json': read(file)
    html = Html(); html.feed((MOVED / 'index.html').read_text(encoding='utf-8'))
    assert json.loads(''.join(html.data)) == REPORT and not html.external
    assert DATA['editable_project_ref']['path'] == 'project/project.json'
    return {'returned_files_read': 74, 'JSON_HTML_actual_Task_same_context': True}
check('all_actual_FileRefs_JSON_and_HTML', context_files)

def csv_tables():
    assert len(list((MOVED / 'csv').glob('*.csv'))) == 15
    rows_count = 0
    for table in CONTRACT['csv_projection']['tables']:
        path = MOVED / 'csv' / table['file']
        raw = path.read_bytes()
        assert raw.startswith(b'\xef\xbb\xbf') and b'\r\n' in raw
        with path.open(encoding='utf-8-sig', newline='') as stream:
            reader = csv.DictReader(stream); rows = list(reader)
            assert set(CONTRACT['csv_projection']['prefix_columns'] + table['columns']) <= set(reader.fieldnames)
        for row in rows:
            assert row['report_id'] == DATA['id'] and row['project_id'] == DATA['project_id']
            for column in table['nullable_columns']:
                assert column + '_state' in row and column + '_reason' in row
                if row[column] == '':
                    assert row[column + '_state'] in ['unknown', 'not_applicable', 'empty']
                    assert row[column + '_reason']
        rows_count += len(rows)
    return {'tables': 15, 'rows': rows_count, 'BOM_CRLF_and_unknown_states': True}
check('all_15_standard_CSV_tables_and_shared_context', csv_tables)

def semantics():
    assert DATA['project_revision'] == 10
    assert DATA['mode'] == 'simulation_evaluation'
    assert DATA['run_refs'] == ['desktop-host-79676:run:1:1']
    assert DATA['scenario_comparison'] is None
    evaluation, = DATA['run_evaluations']
    assert evaluation['input_ref']['project_revision'] == 2
    assert evaluation['input_ref']['scenario_revision'] == 1
    assert evaluation['goal_met'] is True
    assert [a['at_ms'] for a in evaluation['acceptance_events']] == [70000, 110000]
    assert evaluation['order_metrics'][0]['completion_ms'] == 110000
    assert evaluation['costs']['inputs'] == [] and evaluation['costs']['lines'] == []
    assert evaluation['costs']['missing_conditions']
    assert any(c['code'] == 'evaluation.actual_render_missing' for c in DATA['missing_conditions'])
    assert len(DATA['asset_rights']) == 3
    assert all(a['redistribution_confirmed'] is False and a['license'] is None for a in DATA['asset_rights'])
    return {'current_project_revision': 10, 'frozen_Run_project_revision': 2,
            'accepted_ms': [70000, 110000], 'undeclared_prices_unknown': True,
            'actual_render_and_asset_rights_not_invented': True}
check('single_Run_actual_semantics_frozen_versions_unknown_price_and_rights', semantics)

def inspect_manifest(load, expected_run_numbers):
    manifest = load('project.json')['data']
    active, modules, scenarios, runs = [], {}, [], {}
    for entry in manifest['documents']:
        path = entry['file_ref']['path']
        if entry['role'] in ['supporting_file', 'asset']:
            continue
        value = load(path)['data']
        if entry['role'] == 'active_project': active.append(value)
        elif entry['role'] == 'module_definition':
            ref = entry['module_ref']; definition = value['definition']
            assert (definition['id'], definition['version']) == (ref['id'], ref['version'])
            modules[(ref['id'], ref['version'])] = value
        elif entry['role'] == 'scenario_input': scenarios.append(value)
        elif entry['role'] == 'run_result': runs[value['id']] = value
    project, = active
    assert project['revision'] == 10 and len(scenarios) == 2 and len(modules) == 9
    assert set(runs) == {'desktop-host-79676:run:1:' + str(n) for n in expected_run_numbers}
    for number in expected_run_numbers:
        assert runs['desktop-host-79676:run:1:' + str(number)] == read(
            ROOT / ('.work/whole-packages/window5-timeline-evaluation/native-structured-79676/run' + str(number) + '.json'))
    run = runs['desktop-host-79676:run:1:1']
    assert run == read(ROOT / '.work/whole-packages/window5-timeline-evaluation/native-structured-79676/run1.json')
    frozen = load(run['input_ref']['frozen_project_file']['path'])['data']
    scenario = load(run['input_ref']['frozen_scenario_file']['path'])['data']
    assert frozen['revision'] == 2 and scenario['revision'] == 1
    assert [a['at_ms'] for a in run['acceptance_events']] == [70000, 110000]
    assert project['devices'][0]['configuration']['cycle_seconds']['value']['value'] == 41000
    assert not project['acknowledgments'] and not project['artifact_refs']
    for ref in frozen['module_refs']: assert (ref['id'], ref['version']) in modules
    return {'manifest_documents': len(manifest['documents']), 'exact_modules': 9,
            'current_scenarios': 2, 'stored_historical_Runs': len(runs)}
check('report_portable_editable_Project_copy_and_frozen_inputs',
      lambda: inspect_manifest(lambda name: read(MOVED / 'project' / name), [1, 2, 3]))

def zip_package():
    path = SOURCE / 'native-79676-editable-handoff.zip'
    assert path.stat().st_size == 63793
    with zipfile.ZipFile(path) as archive:
        assert len(archive.namelist()) == 35
        assert archive.testzip() is None
        manifest = json.loads(archive.read('project.json'))['data']
        for entry in manifest['documents']:
            if entry['required']: assert entry['file_ref']['path'] in archive.namelist()
        result = inspect_manifest(lambda name: json.loads(archive.read(name)), [1])
        imported = read(OUT / 'native-zip-import-task.json')['result']['data']
        assert len(imported['missing']) == 3
        for issue in imported['missing']:
            assert issue['code'] == 'handoff.asset_not_redistributed'
            assert issue['blocks'] == ['preview']
            assert issue['field_path'] not in archive.namelist()
        (OUT / 'ZIP-members.json').write_text(json.dumps(archive.namelist(), ensure_ascii=False, indent=2), encoding='utf-8')
    return dict(result, ZIP_members=35, ZIP_bytes=63793, explicitly_omitted_selected_asset_paths=3)
check('standard_ZIP_reader_editable_package_and_explicit_preview_missing', zip_package)

def pdf():
    reader = PdfReader(MOVED / 'report.pdf')
    text = '\n\n'.join(page.extract_text() or '' for page in reader.pages)
    assert '超级工厂' in text and '110000' in text
    fonts = []
    for page in reader.pages:
        for font in page['/Resources'].get_object().get('/Font', {}).get_object().values():
            font = font.get_object()
            if '/DescendantFonts' in font: font = font['/DescendantFonts'][0].get_object()
            descriptor = font.get('/FontDescriptor')
            if descriptor:
                descriptor = descriptor.get_object()
                embedded = [k for k in ['/FontFile', '/FontFile2', '/FontFile3'] if k in descriptor]
                fonts.append({'name': str(descriptor.get('/FontName')), 'embedded': embedded})
    assert fonts and all(f['embedded'] for f in fonts)
    assert any('SourceHanSans' in f['name'] for f in fonts)
    with pdfplumber.open(MOVED / 'report.pdf') as document:
        for page in document.pages:
            assert abs(page.width - 595.276) < 1 and abs(page.height - 841.89) < 1
            assert page.chars
            assert not [c for c in page.chars if c['x0'] < -1 or c['x1'] > page.width+1 or c['top'] < -1 or c['bottom'] > page.height+1]
    assert (MOVED / 'licenses/SourceHanSans-LICENSE.txt').read_bytes() == (ROOT / 'tools/architecture-proof/assets/LICENSE.SourceHanSans.txt').read_bytes()
    (OUT / 'pdf-text.txt').write_text(text, encoding='utf-8')
    render = OUT / 'pdf-render'; render.mkdir(exist_ok=True)
    subprocess.run(['pdftoppm', '-r', '72', '-png', str(MOVED / 'report.pdf'), str(render / 'page')], check=True, capture_output=True)
    pages = sorted(render.glob('page-*.png')); assert len(pages) == len(reader.pages)
    for offset in range(0, len(pages), 10):
        sheet = Image.new('RGB', (1200, 925), '#c9cfd4'); draw = ImageDraw.Draw(sheet)
        for i, path in enumerate(pages[offset:offset+10]):
            image = Image.open(path); image.thumbnail((232, 440))
            x, y = i % 5 * 240, i // 5 * 462
            sheet.paste(image, (x, y+24)); draw.text((x, y+3), path.stem, fill='black')
        sheet.save(OUT / ('pdf-sheet-' + str(offset//10+1) + '.png'))
    return {'pages': len(reader.pages), 'all_pages_Poppler_rendered': True, 'font_descriptors': list({json.dumps(f, sort_keys=True): f for f in fonts}.values()),
            'A4_and_bounds_and_matching_OFL': True, 'visual_inspection_pending': True}
check('real_PDF_CJK_embedded_font_A4_pages_and_license', pdf)

result = {'source': str(SOURCE), 'moved_report_directory': str(MOVED),
          'checks': RESULTS, 'whole_task_done': False, 'human_acceptance': False,
          'actual_new_Host_Open_not_started': 'Prohibited by this batch; same actual Native ZIP Open independently read above, earlier real Core Open retained separately.'}
(OUT / 'file-readback.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
for row in RESULTS: print(json.dumps(row, ensure_ascii=False))
raise SystemExit(0 if all(r['passed'] for r in RESULTS) else 1)
