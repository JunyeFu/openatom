"""Relocate only the frozen producer harness outputs; link its exact Core, no Cargo."""
import json
import subprocess
from pathlib import Path

root = Path.cwd()
out = root / 'scenarios/媒体表现/runtime-factory-items'
old = root / 'scenarios/图片时间轴/runtime-port-output'
source = (old / 'core_consumer.rs').read_text(encoding='utf-8')
source = source.replace('scenarios/图片时间轴/runtime-port-output', 'scenarios/媒体表现/runtime-factory-items')
# The Root multi-job fixture has no P303 definition row, while the original
# support manifest still lists these CC0 files. Copy those declared files too.
source = source.replace(' for(i,definition)in input["definitions"]', ' for file in actual["data"]["files"].as_array().unwrap(){let relative=file["path"].as_str().unwrap();let target=directory.join(relative);fs::create_dir_all(target.parent().unwrap()).unwrap();fs::copy(package_root.join(relative),target).unwrap();}\n for(i,definition)in input["definitions"]')
source = source.replace(' entries.push(doc("supporting_file","asset","p303.license"', ' for adoption in project["template_adoptions"].as_array().unwrap(){let relative=adoption["source_archive"]["path"].as_str().unwrap();let target=directory.join(relative);fs::create_dir_all(target.parent().unwrap()).unwrap();fs::copy(root.join("modules/templates/purchased-assembly/candidate/source-project.design.json"),target).unwrap();entries.push(doc("supporting_file","asset","actual-template-source",relative,Value::Null));}\n entries.push(doc("supporting_file","asset","p303.license"')
adopt = 'let snap=core.snapshot();Core::data(core.request(json!({"kind":"project"}),"apply_commands",json!({"base_revision":snap["project"]["revision"],"commands":[{"kind":"adopt_module","data":{"module_ref":{"id":"fixture.exact.item","version":VERSION},"targets":[{"kind":"item_type","id":input["item_media_target"]}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}]})));'
source = source.replace(' let snapshot=core.snapshot();let formal=', ' if input["item_media_fixture"]==true {for version in ["1.0.0","2.0.0"]{core.host.register_bundled_module_directory(&root.join("scenarios/媒体表现/runtime-factory-items/item-packages").join(version)).unwrap();}' + adopt.replace('VERSION', '"1.0.0"') + '}\n let snapshot=core.snapshot();let formal=')
source = source.replace('core.formal=formal.clone();', 'core.formal=formal.clone();if input["item_media_fixture"]==true {' + adopt.replace('VERSION', '"2.0.0"') + '}')
source = source.replace('let commit=Core::data(core.request', 'let authoring_base=core.snapshot()["project"]["revision"].clone();let commit=Core::data(core.request')
source = source.replace('"base_revision":snapshot["project"]["revision"],"commands":[{"kind":"create_definition_draft"', '"base_revision":authoring_base,"commands":[{"kind":"create_definition_draft"')
path = root / 'tests/planner/factory-actual-items/core_consumer.rs'
path.write_text(source, encoding='utf-8')
frozen = json.loads((old / 'core-build.json').read_text(encoding='utf-8'))
core = max((root / 'target/debug/deps').glob('libsf_commands-*.rlib'), key=lambda p: p.stat().st_mtime_ns)
fingerprint = json.loads((root / 'target/debug/.fingerprint' / core.stem.removeprefix('lib').replace('_', '-', 1) / 'lib-sf_commands.json').read_text(encoding='utf-8'))
expected = next(dep[-1] for dep in fingerprint['deps'] if dep[1] == 'serde_json').to_bytes(8, 'little').hex()
matches = [p for p in (root / 'target/debug/.fingerprint').glob('serde_json-*/lib-serde_json') if p.read_text(encoding='utf-8').strip() == expected]
assert len(matches) == 1
frozen['existing_core'] = core.name
frozen['exact_serde_json'] = f'lib{matches[0].parent.name}.rlib'
command = [str(Path('C:/Users/fujunye/.cargo/bin/rustc.exe')), '--edition', '2024', str(path), '-L', 'dependency=target/debug/deps', '--extern', f'sf_commands={root / "target/debug/deps" / frozen["existing_core"]}', '--extern', f'serde_json={root / "target/debug/deps" / frozen["exact_serde_json"]}', '-o', str(out / 'core_consumer.exe')]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
(out / 'core-build.stdout.txt').write_text(result.stdout, encoding='utf-8')
(out / 'core-build.stderr.txt').write_text(result.stderr, encoding='utf-8')
(out / 'core-build.json').write_text(json.dumps({'command': command, 'exit_code': result.returncode, 'Host_Engine_logic_changed': False, 'fixture_support_files_copied': True, 'source': str(old / 'core_consumer.rs')}, ensure_ascii=False, indent=2), encoding='utf-8')
definitions = json.loads((root / 'modules/templates/purchased-assembly/candidate/definitions.json').read_text(encoding='utf-8'))
definitions.append(json.loads((root / 'modules/templates/purchased-assembly/candidate/module.json').read_text(encoding='utf-8'))['data']['definition'])
project = json.loads((root / 'scenarios/模板与交接/runtime/source-clock/root_original.project.json').read_text(encoding='utf-8'))
scenario = json.loads((root / 'scenarios/模板与交接/runtime/source-clock/root_original.scenario.json').read_text(encoding='utf-8'))
(out / 'root-input.json').write_text(json.dumps({'project': project, 'scenario': scenario, 'definitions': definitions}, ensure_ascii=False, indent=2), encoding='utf-8')
phase = json.loads((old / 'tagged-phase.input.json').read_text(encoding='utf-8'))
phase['item_media_fixture'] = True
phase['item_media_target'] = 'raw'
(out / 'phase-item-input.json').write_text(json.dumps(phase, ensure_ascii=False, indent=2), encoding='utf-8')
template = json.loads((root / 'modules/devices/template-support/media/module.json').read_text(encoding='utf-8'))['data']['definition']['payload']['data']
for version, color in [('1.0.0', 'red'), ('2.0.0', 'blue')]:
    directory = out / 'item-packages' / version
    (directory / 'assets').mkdir(parents=True, exist_ok=True)
    (directory / 'dependency/assets').mkdir(parents=True, exist_ok=True)
    asset = {'id': 'item-image', 'path': 'assets/item.svg', 'kind': 'svg'}
    rights = [{'asset_ref': 'item-image', 'source': 'self-created controlled Item media fixture', 'license': None, 'redistribution_confirmed': False, 'missing_reason': 'no redistribution grant'}]
    presentation = json.loads(json.dumps(template))
    presentation.update(kind='image', source={'kind': 'asset', 'data': asset})
    presentation['lod'].update(detailed=None, simplified=None)
    contribution = {'author': None, 'license': None, 'assets': rights}
    dependency = {'definition': {'id': 'fixture.item.dependency', 'version': '1.0.0', 'title': 'owned exact dependency', 'definition_state': 'design', 'contribution': contribution, 'parameters': [], 'attribute_schema': [], 'payload': {'category': 'media', 'data': presentation}}, 'assets': [asset], 'files': [{'path': 'assets/item.svg', 'kind': 'asset'}], 'dependencies': [], 'rights': rights, 'issues': []}
    package = {'definition': {'id': 'fixture.exact.item', 'version': version, 'title': 'shell', 'definition_state': 'design', 'contribution': contribution, 'parameters': [], 'attribute_schema': [], 'payload': {'category': 'items', 'data': {'presentation_or_media_ref': presentation, 'unit': 'piece', 'tracking': 'individual', 'spec_ref': None, 'attributes': {}, 'attribute_schema': []}}}, 'assets': [asset], 'files': [{'path': 'assets/item.svg', 'kind': 'asset'}, {'path': 'dependency/module.json', 'kind': 'asset'}], 'dependencies': [{'id': 'fixture.item.dependency', 'version': '1.0.0'}], 'rights': rights, 'issues': []}
    (directory / 'module.json').write_text(json.dumps({'schema_version': '1.0', 'data': package}), encoding='utf-8')
    (directory / 'dependency/module.json').write_text(json.dumps({'schema_version': '1.0', 'data': dependency}), encoding='utf-8')
    (directory / 'assets/item.svg').write_text(f"<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' fill='{color}'/></svg>", encoding='utf-8')
    (directory / 'dependency/assets/item.svg').write_text("<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' fill='red'/></svg>", encoding='utf-8')
raise SystemExit(result.returncode)

