"""Compare an actual Native save with the independent typed source input."""
import argparse
import copy
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('directory', type=Path)
parser.add_argument('--name', required=True)
parser.add_argument('--revision', type=int, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
source = root / '.work/whole-packages/p001-20261017/complete-input'

def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

manifest = read(args.directory / 'project.json')
data = manifest['data']
assert manifest['schema_version'] == '1.0'
assert data['project_id'] == 'P1' and data['project_revision'] == args.revision
active = next(x for x in data['documents'] if x['role'] == 'active_project')
saved = read(args.directory / active['file_ref']['path'])
expected = copy.deepcopy(read(source / 'payload/project-state.json'))
expected['data']['revision'] = args.revision
next(x for x in expected['data']['devices'] if x['id'] == 'D1')['title'] = args.name
# Adoption materializes the explicit blocking issue declared in input-expectations.json.
assert read(source / 'input-expectations.json')['expected_explicit_draft_issue'] == 'professional_inputs.nonhuman_acknowledgment'
expected['data']['issues'] = [{
    'code': 'professional_inputs.nonhuman_acknowledgment',
    'object_ref': {'kind': 'device', 'id': 'D4'},
    'field_path': '/acceptance_ref',
    'message': '预设或提供者结果不能替代指定用途的人工认可。',
    'blocks': ['execution', 'complete_conclusion'],
}]
assert saved == expected, 'Saved typed project differs from independent input plus the explicit Native edit'
checked = 0
for doc in data['documents']:
    path = args.directory / doc['file_ref']['path']
    assert path.is_file(), path
    if doc['role'] != 'active_project':
        original = next(x for x in read(source / 'project.json')['data']['documents']
                        if x['object_ref'] == doc['object_ref'] and x['role'] == doc['role'])
        expected_document = read(source / original['file_ref']['path'])
        if doc['role'] == 'scenario_input':
            # Each actual project edit rebinds this scenario to the new draft revision.
            expected_document['data']['project_revision'] = args.revision
            expected_document['data']['revision'] += args.revision - 2
        assert read(path) == expected_document, path
    checked += 1
result = dict(passed=True, directory=str(args.directory.resolve()), project_id='P1',
              revision=args.revision, device_D1_title=args.name,
              comparison='Entire typed project equals original plus explicit D1 title, new revision and predeclared nonhuman acknowledgment issue; scenario revisions follow each actual edit and all other referenced documents equal original',
              referenced_documents_checked=checked, ordinary_Native_UI_evidence='validation.json records separate observed actions',
              whole_task_done=False, human_acceptance=None)
args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(result, ensure_ascii=False))
