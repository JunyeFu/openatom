"""Finite file/Windows manifest proof; does not implement a product loader or Host."""
from pathlib import Path, PurePosixPath
from copy import deepcopy
import ctypes
from ctypes import wintypes
import json
import os
import stat
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent
OUT = ROOT / 'output'
sys.path.insert(0, str(ROOT / 'python-libs'))
from jsonschema import Draft202012Validator, ValidationError


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


SCHEMAS = {p.name.removesuffix('.schema.json'): Draft202012Validator(read(p))
           for p in (OUT / 'schemas').glob('*.schema.json')}
ROLE_SCHEMA = {'active_project': 'project', 'scenario_input': 'scenario',
               'module_definition': 'module', 'template_data': 'template',
               'generation_profile': 'generation-profile', 'professional_input': 'packaging-job',
               'professional_result': 'packaging-result', 'artifact_metadata': 'artifact',
               'frozen_run_project': 'project', 'frozen_run_scenario': 'scenario',
               'run_result': 'run', 'evaluation': 'evaluation'}


def safe_member(name):
    parts = PurePosixPath(name).parts
    if (not name or name.startswith('/') or '\\' in name or ':' in name or '\x00' in name
            or not parts or any(p in ('', '.', '..') for p in name.split('/'))):
        raise ValueError('unsafe relative path')
    reserved = {'CON', 'PRN', 'AUX', 'NUL'} | {f'{p}{n}' for p in ('COM', 'LPT') for n in range(1, 10)}
    if any(p.endswith(('.', ' ')) or p.split('.')[0].upper() in reserved for p in parts):
        raise ValueError('unsafe Windows name')
    return name


def rebase(value, paths):
    if isinstance(value, dict):
        for key, item in value.items():
            if key in ('path', 'project', 'generation_profile') and isinstance(item, str) and item in paths:
                value[key] = paths[item]
            else:
                rebase(item, paths)
    elif isinstance(value, list):
        for item in value:
            rebase(item, paths)


def package_fixture():
    fixtures = {p.stem: read(p) for p in (ROOT / 'fixtures').glob('*.json')
                if p.stem not in ('manifest', 'measurement')}
    paths = {name + '.json': 'data/g000001/' + ('project-state.json' if name == 'project' else name + '.json')
             for name in fixtures}
    files = {}
    for name, data in fixtures.items():
        rebase(data, paths)
        files[paths[name + '.json']] = data
    # Frozen inputs are separate documents, not inferred from an active document's file name.
    files['data/g000001/run-project.json'] = deepcopy(files[paths['project.json']])
    files['data/g000001/run-scenario.json'] = deepcopy(files[paths['scenario.json']])
    run = files[paths['run.json']]['data']
    run['input_ref']['frozen_project_file']['path'] = 'data/g000001/run-project.json'
    run['input_ref']['frozen_scenario_file']['path'] = 'data/g000001/run-scenario.json'
    manifest = read(ROOT / 'fixtures/manifest.json')
    rebase(manifest, paths)
    extra = [('packaging-job', 'professional_input', 'packaging_job', 'packaging_job', 'PACKJOB1'),
             ('packaging-result', 'professional_result', 'packaging_result', 'packaging_job', 'PACKJOB1'),
             ('artifact', 'artifact_metadata', 'artifact_record', 'artifact', 'A1'),
             ('run', 'run_result', 'run_result', 'run', 'R1'),
             ('evaluation', 'evaluation', 'evaluation', 'report', 'E1'),
             ('run-project', 'frozen_run_project', 'project', 'project', 'P1'),
             ('run-scenario', 'frozen_run_scenario', 'scenario', 'scenario', 'SC1')]
    for name, role, kind, object_kind, ident in extra:
        manifest['data']['documents'].append({
            'role': role, 'object_ref': {'kind': object_kind, 'id': ident},
            'module_ref': None, 'revision': 1 if role.startswith('frozen_') else None,
            'file_ref': {'path': 'data/g000001/' + name + '.json', 'kind': kind}, 'required': True})
    return files, manifest


def open_staged(folder):
    root = read(folder / 'project.json')
    SCHEMAS['manifest'].validate(root)
    docs = root['data']['documents']
    assert len([e for e in docs if e['role'] == 'active_project']) == 1
    seen, loaded = set(), {}
    for entry in docs:
        path = safe_member(entry['file_ref']['path'])
        assert path.casefold() not in seen
        seen.add(path.casefold())
        data = read(folder / path)
        SCHEMAS[ROLE_SCHEMA[entry['role']]].validate(data)
        loaded[path] = data
        if entry['module_ref'] is not None:
            definition = data['data']['definition']
            assert entry['module_ref'] == {k: definition[k] for k in ('id', 'version')}
    active = next(e for e in docs if e['role'] == 'active_project')
    project = loaded[active['file_ref']['path']]['data']
    assert project['id'] == root['data']['project_id']
    assert project['revision'] == root['data']['project_revision'] == active['revision']
    for entry in docs:
        data = loaded[entry['file_ref']['path']]['data']
        if entry['role'] == 'run_result':
            source = data['input_ref']
            frozen_project = loaded[source['frozen_project_file']['path']]['data']
            frozen_scenario = loaded[source['frozen_scenario_file']['path']]['data']
            assert frozen_project['id'] == source['project_id']
            assert frozen_project['revision'] == source['project_revision']
            assert frozen_scenario['id'] == source['scenario_id']
            assert frozen_scenario['revision'] == source['scenario_revision']
        if entry['role'] == 'artifact_metadata':
            job = loaded[data['input_ref']['path']]['data']
            result = loaded[data['result_ref']['path']]['data']
            assert data['subject_ref'] == job['subject_ref'] == result['subject_ref']
    return root, project, loaded


replace = ctypes.WinDLL('kernel32', use_last_error=True).ReplaceFileW
replace.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR,
                    wintypes.DWORD, wintypes.LPVOID, wintypes.LPVOID]
replace.restype = wintypes.BOOL


def commit(folder, manifest):
    pending = folder / 'project.pending.json'
    write(pending, manifest)
    current = folder / 'project.json'
    if current.exists():
        if not replace(str(current), str(pending), None, 0, None, None):
            raise ctypes.WinError(ctypes.get_last_error())
    else:
        os.rename(pending, current)


def extract_checked(archive, destination):
    with zipfile.ZipFile(archive) as z:
        seen = set()
        for info in z.infolist():
            name = safe_member(info.filename[:-1] if info.is_dir() else info.filename)
            if name.casefold() in seen or stat.S_ISLNK(info.external_attr >> 16):
                raise ValueError('duplicate member or symbolic link')
            seen.add(name.casefold())
        for info in z.infolist():
            output = (destination / info.filename).resolve()
            assert output.is_relative_to(destination.resolve())
            if info.is_dir():
                output.mkdir(parents=True, exist_ok=True)
                continue
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(z.read(info))


def main():
    cases = []
    # tempfile cleanup is confined to a newly allocated proof directory.
    with tempfile.TemporaryDirectory(prefix='sf-type-save-', dir=OUT) as tmp:
        folder = Path(tmp).resolve()
        assert folder.is_relative_to(OUT.resolve())
        files, manifest = package_fixture()
        for name, data in files.items():
            write(folder / safe_member(name), data)
        commit(folder, manifest)
        old, old_project, loaded = open_staged(folder)
        assert old_project['revision'] == 1
        cases.append('new_directory_commit_and_schema_reopen')
        previous_root = (folder / 'project.json').read_bytes()
        write(folder / 'data/g000002/uncommitted-project.json', {'not': 'a committed project'})
        (folder / 'blocked').write_text('file, not directory', encoding='utf-8')
        try:
            write(folder / 'blocked/new.json', {})
        except OSError:
            pass
        else:
            raise AssertionError('write failure was not observed')
        assert (folder / 'project.json').read_bytes() == previous_root
        assert open_staged(folder)[1] == old_project
        cases.append('actual_file_write_failure_preserves_old_manifest')
        assert not replace(str(folder / 'project.json'), str(folder / 'absent.pending'), None, 0, None, None)
        assert ctypes.get_last_error() != 0
        assert (folder / 'project.json').read_bytes() == previous_root
        cases.append('actual_ReplaceFileW_failure_preserves_old_manifest')
        next_manifest = deepcopy(old)
        next_manifest['data'].update(generation=2, project_revision=2)
        current = next(e for e in next_manifest['data']['documents'] if e['role'] == 'active_project')
        current['revision'] = 2
        current['file_ref']['path'] = 'data/g000002/project-state.json'
        next_project = deepcopy(files['data/g000001/project-state.json'])
        next_project['data'].update(revision=2, title='saved frozen R2')
        write(folder / current['file_ref']['path'], next_project)
        editor_revision = 3
        commit(folder, next_manifest)
        saved, project, loaded = open_staged(folder)
        assert project['revision'] == 2 and editor_revision > saved['data']['project_revision']
        assert loaded['data/g000001/run-project.json']['data']['revision'] == 1
        assert loaded['data/g000001/run.json']['data']['input_ref']['project_revision'] == 1
        cases += ['update_commit_preserves_frozen_Run_inputs', 'save_R2_does_not_mark_editor_R3_clean']
        # Invalid open is staged; the caller keeps its previously accepted project.
        accepted = deepcopy(project)
        bad = folder / 'bad-open'
        write(bad / 'project.json', {'schema_version': '99.0', 'data': {}})
        try:
            open_staged(bad)
        except ValidationError:
            pass
        else:
            raise AssertionError('unknown schema accepted')
        assert project == accepted
        cases.append('unknown_version_open_preserves_current_context')
        missing = deepcopy(saved)
        missing['data']['documents'][0]['file_ref']['path'] = 'missing.json'
        write(bad / 'project.json', missing)
        try:
            open_staged(bad)
        except FileNotFoundError:
            pass
        else:
            raise AssertionError('missing file accepted')
        assert project == accepted
        cases.append('required_file_missing_preserves_current_context')
        archive = folder / 'handoff.sfproj'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
            z.write(folder / 'project.json', 'project.json')
            for entry in saved['data']['documents']:
                name = entry['file_ref']['path']
                z.write(folder / name, name)
        handoff = folder / 'new-directory'
        extract_checked(archive, handoff)
        assert open_staged(handoff)[1] == project
        cases.append('selected_sfproj_new_directory_reopen')
        for label, names in [('traversal', ['../escape.json']),
                             ('case_collision', ['Data/a.json', 'data/A.json'])]:
            malicious = folder / (label + '.zip')
            with zipfile.ZipFile(malicious, 'w') as z:
                for name in names:
                    z.writestr(name, '{}')
            try:
                extract_checked(malicious, folder / 'refused')
            except ValueError:
                pass
            else:
                raise AssertionError(label + ' accepted')
            cases.append('ZIP_' + label + '_rejected_before_extraction')
        malicious = folder / 'symlink.zip'
        with zipfile.ZipFile(malicious, 'w') as z:
            info = zipfile.ZipInfo('link.json')
            info.create_system = 3
            info.external_attr = (stat.S_IFLNK | 0o777) << 16
            z.writestr(info, '../escape')
        try:
            extract_checked(malicious, folder / 'refused')
        except ValueError:
            pass
        else:
            raise AssertionError('symbolic link accepted')
        cases.append('ZIP_symlink_rejected_before_extraction')
        for name in ['CON.json', 'file. ', 'x/../out.json', 'C:/absolute', 'x\\file', 'x//file']:
            try:
                safe_member(name)
            except ValueError:
                pass
            else:
                raise AssertionError('unsafe name accepted')
        cases.append('six_unsafe_Windows_paths_rejected')
        assert len(cases) == 12
        doc_count = len(saved['data']['documents'])
    result = {'status': 'passed_actual_file_schema_and_Windows_manifest_primitive_proof',
              'cases': cases, 'selected_document_count': doc_count,
              'commit_primitive': 'ReplaceFileW + initial same-volume rename',
              'evidence_kind': 'isolated_type_fixture_IO_not_product_Save_Open_or_software_handoff',
              'actual_product_runtime': None, 'actual_NFR_performance': None, 'errors': []}
    write(OUT / 'persistence-proof-result.json', result)
    print(json.dumps({'status': result['status'], 'cases': len(cases), 'documents': doc_count}, ensure_ascii=False))


if __name__ == '__main__':
    main()
