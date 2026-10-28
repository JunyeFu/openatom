"""S05-04版本/路线/部署规范校验；不是依赖安装、编译或发布证明。"""
import json
from pathlib import Path
from datetime import datetime, timedelta, timezone

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def read(ref):
    return json.loads((S / ref).read_text(encoding='utf-8-sig'))


def main():
    design = read('docs/03_架构与契约/首版技术栈与部署设计.json')
    source = read(design['registry_metadata_ref'])
    metadata = {(r['ecosystem'], r['name']): r for r in source['packages']}
    md = (S / design['spec_md_ref']).read_text(encoding='utf-8-sig')
    packages = design['packages']
    assert len(packages) == 22 == len(metadata)
    rust = tuple(int(n) for n in design['toolchain']['rust'].split('.'))
    for package in packages:
        original = metadata[(package['ecosystem'], package['name'])]
        for field in ['version', 'license', 'source']:
            assert package[field] == original[field]
        assert package['version'] in md and package['role']
        assert package['selection_state'] == 'selected_for_design_trial_not_product_lock_or_compile_verified'
        if package.get('rust_version'):
            minimum = tuple(int(n) for n in package['rust_version'].split('.'))
            assert rust >= minimum
    assert metadata[('npm', 'react')]['version'] == metadata[('npm', 'react-dom')]['version']
    assert metadata[('npm', '@tauri-apps/api')]['version'] == metadata[('crates', 'tauri')]['version']
    assert metadata[('npm', '@tauri-apps/plugin-dialog')]['version'] == metadata[('crates', 'tauri-plugin-dialog')]['version']
    node = tuple(int(n) for n in design['toolchain']['node'].split('.'))
    assert node >= (22, 12, 0)  # 本次元数据engine为^20.19.0 || >=22.12.0；所选24.15.0。
    checked_features = 0
    for name, requested in design['feature_design'].items():
        if name.endswith('_default_features'):
            continue
        assert isinstance(requested, list)
        available = metadata[('crates', name)]['available_features']
        assert set(requested) <= set(available), (name, requested)
        checked_features += len(requested)
    assert design['feature_design']['printpdf_default_features'] is False
    assert 'html' not in design['feature_design']['printpdf']
    assert len(design['layer_routes']) == 7
    assert design['mcp_bridge_change']['selected'] == 'Rust rmcp native binary'
    architecture = read('docs/03_架构与契约/全层组件与接点.json')
    bridge = next(c for c in architecture['components'] if c['id'] == 'mcp_bridge')
    assert bridge['technology_family'] == 'Rust/rmcp；MCP stdio'
    assert bridge['code_dependencies'] == ['domain'] and bridge['actual_implementation'] is None
    assert design['deployment']['actual_release'] is None
    assert design['licenses']['actual_inventory'] is None
    assert design['font_release']['actual_font_file'] is None
    assert design['actual_runtime'] is None and design['actual_install_or_product_build'] is None
    result = {'check_kind': 'S05_04_technical_selection_and_deployment_design_not_compile',
              'date': datetime.now(timezone(timedelta(hours=8))).date().isoformat(),
              'status': 'passed_static_technical_and_deployment_design', 'selected_dependencies': len(packages),
              'features_checked_against_registry': checked_features, 'layer_routes': 7,
              'native_bridge_and_component_consistent': True,
              'current_cli_and_registry_evidence': 'read_only_inventory_not_application',
              'actual_product_compile': None, 'actual_install_or_release': None,
              'actual_runtime': None, 'errors': []}
    (A / '技术部署检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
