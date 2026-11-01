"""Assemble an existing Windows candidate and its declared resources; never run it."""
import argparse
import json
from pathlib import Path
import shutil
import zipfile


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def assemble(source, native, demos, output, archive, candidate, build_receipt=None):
    source, native, demos = source.resolve(), native.resolve(), demos.resolve()
    output.mkdir(parents=True, exist_ok=False)
    mapping = read(source/'apps/planner/src-tauri/tauri.conf.json')['bundle']['resources']
    copied = []
    def copy(src, dest, role):
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dest)
        assert src.read_bytes() == dest.read_bytes(), str(dest)
        copied.append({'path':dest.relative_to(output).as_posix(),'bytes':dest.stat().st_size,'role':role})
    assert (native/'superfactory.exe').is_file()
    for name in ('superfactory.exe', 'superfactory-mcp-bridge.exe', 'WebView2Loader.dll'):
        src = native/name
        if src.is_file():
            copy(src, output/name, 'linked_native_binary')
    native_manifest = source/'apps/planner/src-tauri'
    resources = []
    for relative, destination in mapping.items():
        src = (native_manifest/relative).resolve()
        assert src.is_relative_to(source) and src.exists(), relative
        assert not Path(destination).is_absolute() and '..' not in Path(destination).parts
        members = [src] if src.is_file() else sorted(p for p in src.rglob('*') if p.is_file())
        assert members, relative
        for member in members:
            dest = output/destination
            if src.is_dir():
                dest = dest/member.relative_to(src)
            copy(member, dest, 'native_resource')
        resources.append({'source':src.relative_to(source).as_posix(),'destination':destination,'files':len(members)})
    demo_summary = []
    for name in ['missing-material','dense']:
        project = demos/name
        manifest = read(project/'project.json')['data']
        for doc in manifest['documents']:
            assert (project/doc['file_ref']['path']).is_file(), doc
        station = read(project/'module-0.json')
        definition = station.get('data',station)
        if 'definition' in definition:
            definition = definition['definition']
        assert definition['id'] == 'p201.b60.station' and definition['version'] == '1.0.5'
        glb = (project/'assets/b60-station-1.0.5.glb').read_bytes()
        assert glb[:4] == b'glTF'
        runs = []
        for doc in manifest['documents']:
            if doc['role'] == 'run_result':
                result = read(project/doc['file_ref']['path'])
                run = result.get('data',result)
                assert run['id'] == doc['object_ref']['id']
                runs.append({'id':run['id'],'file':doc['file_ref']['path'],
                             'scenario_ref':run.get('scenario_ref'),'computed_until_ms':run.get('computed_until_ms')})
        assert len(runs) == 1
        members = [p for p in project.rglob('*') if p.is_file()]
        for member in members:
            copy(member,output/'demos'/name/member.relative_to(project),'frozen_demo')
        demo_summary.append({'name':name,'project_id':manifest['project_id'],'revision':manifest['project_revision'],
                             'module_ref':{'id':'p201.b60.station','version':'1.0.5'},'files':len(members),'runs':runs,
                             'original_bytes_preserved':True,'rerun_performed':False})
    guide = source/'docs/05_开源与参赛/release-preparation/运行与交付指南.md'
    copy(guide,output/'运行指南.md','guide')
    result = {'candidate_id':candidate,'native_resource_mappings':resources,'demos':demo_summary,
              'files':copied,'native_launch_verified':False,'frozen_binary_link_receipt':build_receipt,
              'root_native_acceptance_pending':True,'published':False}
    (output/'NATIVE_DELIVERY.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    if archive is not None:
        archive.parent.mkdir(parents=True,exist_ok=True)
        with zipfile.ZipFile(archive,'x',compression=zipfile.ZIP_DEFLATED,compresslevel=1) as z:
            for p in sorted(output.rglob('*')):
                if p.is_file():
                    z.write(p,p.relative_to(output).as_posix())
    return {'candidate_id':candidate,'output':str(output),'archive':str(archive) if archive else None,'archive_bytes':archive.stat().st_size if archive else None,
            'resource_mappings':len(resources),'resource_files':sum(x['files'] for x in resources),
            'demos':demo_summary,'native_started':False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for key in ['source-root','native-directory','demo-root','output']:
        parser.add_argument('--'+key,type=Path,required=True)
    parser.add_argument('--zip-output',type=Path,help='Optional; omit for a directory candidate')
    parser.add_argument('--candidate-id',required=True)
    parser.add_argument('--build-receipt',help='Build receipt reference included in the supplied native directory')
    args = parser.parse_args()
    print(json.dumps(assemble(args.source_root,args.native_directory,args.demo_root,args.output,args.zip_output,args.candidate_id,args.build_receipt),ensure_ascii=False,indent=2))


if __name__ == '__main__':
    main()
