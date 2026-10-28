"""独立重开fixture四格式并核字段；视觉检查须另用Poppler逐页进行。"""
import json
import re
import csv
from pathlib import Path
from html.parser import HTMLParser
from pypdf import PdfReader

ROOT = Path(__file__).resolve().parent
OUT = ROOT / 'output'


class Table(HTMLParser):
    def __init__(self):
        super().__init__()
        self.rows, self.row, self.cell, self.in_td = [], [], '', False

    def handle_starttag(self, tag, attrs):
        if tag == 'tr':
            self.row = []
        if tag == 'td':
            self.in_td, self.cell = True, ''

    def handle_data(self, value):
        if self.in_td:
            self.cell += value

    def handle_endtag(self, tag):
        if tag == 'td':
            self.in_td = False
            self.row.append(self.cell)
        if tag == 'tr' and self.row:
            self.rows.append(self.row)


expected = json.loads((OUT / 'report-fixture.json').read_text(encoding='utf-8'))
assert expected['source_kind'] == 'fixture_not_factory_result'
with (OUT / 'report-fixture.csv').open(encoding='utf-8', newline='') as source:
    actual = list(csv.DictReader(source))
assert actual == [{k: str(v) for k, v in row.items()} for row in expected['rows']]
table = Table()
table.feed((OUT / 'report-fixture.html').read_text(encoding='utf-8'))
assert table.rows == [[r['id'], r['description'], str(r['count']), str(r['duration_ms'])] for r in expected['rows']]
pdf = PdfReader(OUT / 'report-fixture.pdf')
texts = [page.extract_text() for page in pdf.pages]
assert len(pdf.pages) == 2
assert all('自建输入' in value and '不是工厂结果' in value for value in texts)
whole = '\n'.join(texts)
for row in expected['rows']:
    assert whole.count(row['id']) == 1
parts = re.split(r'(LAB-\d{3})', whole)
for i in range(1, len(parts), 2):
    row = next(r for r in expected['rows'] if r['id'] == parts[i])
    assert row['description'].replace('\n', ' / ') in parts[i + 1]
    assert re.search(r'\b' + str(row['count']) + r'\b', parts[i + 1])
    assert re.search(r'\b' + str(row['duration_ms']) + r'\b', parts[i + 1])
assert [len(page.images) for page in pdf.pages] == [1, 1]
print(json.dumps({'four_formats_fields_reopened': 'passed', 'rows': 48, 'pdf_pages': 2,
                  'Chinese_text_and_images': 'passed', 'actual_product_report': False,
                  'layout_visual_review_not_performed_by_this_script': True}, ensure_ascii=False))
