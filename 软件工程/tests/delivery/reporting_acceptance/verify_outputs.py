"""Read actual Rust output with independent standard parsers and Poppler."""
import csv
import json
import shutil
import subprocess
import time
from html.parser import HTMLParser
from pathlib import Path
from pypdf import PdfReader
import pdfplumber

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT/'.work/whole-packages/window5-timeline-evaluation'
MOVED = OUT/('moved-reports-'+str(time.time_ns()))
MOVED.mkdir()
CONTRACT = json.loads((ROOT/'packages/reporting/evaluation-report.contract.design.json').read_text(encoding='utf8'))
RESULTS = []

def check(name, action):
    try:
        details = action()
        RESULTS.append({'name': name, 'passed': True, 'details': details})
    except Exception as error:
        RESULTS.append({'name': name, 'passed': False, 'failure': repr(error)})

class DataParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.active = False
        self.data = []
        self.external = []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'script' and attrs.get('id') == 'report-data': self.active = True
        for key in ['src', 'href']:
            if attrs.get(key, '').startswith(('http:', 'https:', '//')): self.external.append(attrs[key])
    def handle_endtag(self, tag):
        if tag == 'script': self.active = False
    def handle_data(self, value):
        if self.active: self.data.append(value)

def rows(path):
    with path.open(encoding='utf-8-sig', newline='') as stream:
        reader = csv.DictReader(stream)
        result = list(reader)
        assert all(None not in row for row in result), path
        return reader.fieldnames, result

for case in ['fractional', 'design']:
    target = MOVED/case
    shutil.copytree(OUT/'exports'/case, target)
    report = json.loads((target/'report.json').read_text(encoding='utf8'))
    data = report['data']
    def csv_contract(target=target, data=data):
        expected = CONTRACT['csv_projection']['tables']
        assert len(list((target/'csv').glob('*.csv'))) == len(expected) == 15
        counts = {}
        for table in expected:
            path = target/'csv'/table['file']
            raw = path.read_bytes()
            assert raw.startswith(b'\xef\xbb\xbf') and raw.endswith(b'\r\n'), table['file']
            columns, records = rows(path)
            required = CONTRACT['csv_projection']['prefix_columns'] + table['columns']
            assert set(required) <= set(columns), (table['file'], sorted(set(required)-set(columns)))
            for record in records:
                assert record['report_id'] == data['id'] and record['project_id'] == data['project_id']
                assert record['run_id'] in data['run_refs'] or (not data['run_refs'] and record['run_id'] == '')
                for nullable in table['nullable_columns']:
                    assert nullable+'_state' in record and nullable+'_reason' in record
                    if record[nullable] == '':
                        assert record[nullable+'_state'] in ['unknown', 'not_applicable', 'empty'], (table['file'], nullable, record)
                        assert record[nullable+'_reason'], (table['file'], nullable)
            counts[table['file']] = len(records)
        return counts
    check(case+'_standard_csv_contract_identity_null_states', csv_contract)
    def html_source(target=target, report=report):
        parser = DataParser()
        parser.feed((target/'index.html').read_text(encoding='utf8'))
        assert not parser.external
        assert json.loads(''.join(parser.data)) == report
        return {'same_complete_json': True, 'active_external_references': 0}
    check(case+'_offline_html_embeds_exact_same_report', html_source)
    def pdf_check(target=target):
        reader = PdfReader(target/'report.pdf')
        texts = [page.extract_text() or '' for page in reader.pages]
        assert len(reader.pages) > 1
        assert '超级工厂' in ''.join(texts)
        for page in reader.pages:
            assert abs(float(page.mediabox.width)-595.276) < 1 and abs(float(page.mediabox.height)-841.89) < 1
        page_stats = []
        with pdfplumber.open(target/'report.pdf') as pdf:
            for index, page in enumerate(pdf.pages):
                assert page.chars, index+1
                outside = [c for c in page.chars if c['x0'] < -1 or c['x1'] > page.width+1 or c['top'] < -1 or c['bottom'] > page.height+1]
                assert not outside, (index+1, outside[:2])
                page_stats.append({'page': index+1, 'characters': len(page.chars), 'start': texts[index][:130]})
        (target/'pdf-text.txt').write_text('\n\n'.join(texts), encoding='utf8')
        (target/'pdf-page-stats.json').write_text(json.dumps(page_stats, ensure_ascii=False, indent=2), encoding='utf8')
        render = target/'render'
        render.mkdir()
        result = subprocess.run(['pdftoppm', '-r', '72', '-png', str(target/'report.pdf'), str(render/'page')], capture_output=True)
        (target/'render.stderr.txt').write_bytes(result.stderr)
        assert result.returncode == 0
        assert len(list(render.glob('page-*.png'))) == len(reader.pages)
        return {'actual_printpdf_pages': len(reader.pages), 'Chinese_text_extracted': True, 'page_bounds_checked': True, 'all_pages_rendered': True, 'visual_inspection_pending': True}
    check(case+'_actual_pdf_chinese_A4_bounds_and_full_Poppler_render', pdf_check)

def fraction():
    report = json.loads((MOVED/'fractional/report.json').read_text(encoding='utf8'))['data']
    line = report['run_evaluations'][0]['costs']['lines'][0]
    _, costs = rows(MOVED/'fractional/csv/costs.csv')
    selected = [r for r in costs if r['cost_line_id'] == line['cost_input_id']]
    assert len(selected) == 1
    assert float(selected[0]['amount']) == line['amount'] == 135000/7
    claims = report['run_evaluations'][0]['resource_claims']
    _, table = rows(MOVED/'fractional/csv/resource_claims.csv')
    assert len(table) == sum(len(c['intervals']) for c in claims)
    for claim in claims:
        for index, interval in enumerate(claim['intervals']):
            row = next(r for r in table if r['claim_id'] == claim['id'] and int(r['interval_index']) == index)
            assert row['end_cause'] == claim['interval_end_causes'][index]
            assert int(row['true_start_ms']) == interval['start_ms'] and int(row['true_end_ms']) == interval['end_ms']
    return {'fractional_amount': line['amount'], 'claim_intervals_preserved': len(table)}
check('fractional_amount_and_real_claim_end_causes_survive_standard_parsers', fraction)
def exact_decimal():
    source = OUT/'exports/exact-decimal'
    report = json.loads((source/'report.json').read_text(encoding='utf8'))['data']
    cost = report['run_evaluations'][0]['costs']
    line = next(line for line in cost['lines'] if line['cost_input_id'] == 'exact-decimal')
    assert line['amount'] == float('0.02')
    assert line['amount'] != 0.1*3/15
    _, records = rows(source/'csv/costs.csv')
    row = next(row for row in records if row['cost_line_id'] == 'exact-decimal')
    assert float(row['amount']) == float('0.02')
    assert json.loads(row['rate_unit'])['value'] == {'coefficient':15,'decimal_places':0}
    return {'actual_Engine_window_ms':[0,3], 'integer_capacity_person':1, 'decimal_rate':0.1, 'rate_denominator_person_ms':15, 'exact_expected_once_converted':float('0.02'), 'rejected_early_binary_result':0.1*3/15}
check('actual_decimal_paid_calendar_cost_survives_independent_standard_readers', exact_decimal)
(OUT/'independent-output-result.json').write_text(json.dumps({'moved_directory': str(MOVED), 'checks': RESULTS, 'whole_task_done': False, 'human_acceptance': False}, ensure_ascii=False, indent=2), encoding='utf8')
for result in RESULTS: print(json.dumps(result, ensure_ascii=False))
raise SystemExit(0 if all(r['passed'] for r in RESULTS) else 1)
