"""Calibrate existing vacuum tooling to the declared taller case, keeping its floor path."""
import argparse
import copy
import json
import struct
from pathlib import Path
from prepare_full_factory import DESIGN, read, write


def prepare(source, output):
    assert not output.exists()
    output.mkdir(parents=True)
    spec = read(DESIGN / 'logistics-line-spec.json')
    offsets = spec['vacuum_height_fixture']['node_y_offsets_m']
    packages = read(source / 'actual-frozen-modules.json')
    exact = {(m['definition']['id'], m['definition']['version']): m for m in packages}
    project = read(source / 'actual-project.json')
    device = next(d for d in project['devices'] if d['id'] == 'PF-vacuum-palletizer')
    original = exact[(device['module_ref']['id'], device['module_ref']['version'])]['definition']['payload']['data']['presentation']
    refs = {'detailed': original['source']['data'], 'simplified': original['lod']['simplified']['data']}
    modules = []; replacements = {}; evidence = []
    for lod, ref in refs.items():
        package = copy.deepcopy(exact[(ref['id'], ref['version'])])
        media = package['definition']['payload']['data']; asset = media['source']['data']
        data = (source / 'project' / asset['path']).read_bytes()
        length = struct.unpack_from('<I', data, 12)[0]; gltf = json.loads(data[20:20+length]); binary = bytearray(data[28+length:])
        parents = {child: i for i, node in enumerate(gltf['nodes']) for child in node.get('children', [])}
        by_name = {node['name']: i for i, node in enumerate(gltf['nodes'])}
        assert gltf['nodes'][parents[by_name['PaperSeat']]]['name'] == 'J3_VerticalTool'
        for name, delta in offsets.items():
            node = gltf['nodes'][by_name[name]]
            node.setdefault('translation', [0, 0, 0])[1] += delta
        accessors = {}
        for clip in gltf['animations']:
            for channel in clip['channels']:
                name = gltf['nodes'][channel['target']['node']]['name']
                if name not in offsets: continue
                assert channel['target']['path'] == 'translation'
                idx = clip['samplers'][channel['sampler']]['output']
                assert idx not in accessors or accessors[idx] == offsets[name]
                accessors[idx] = offsets[name]
        for idx, delta in accessors.items():
            accessor = gltf['accessors'][idx]; view = gltf['bufferViews'][accessor['bufferView']]
            assert accessor['componentType'] == 5126 and accessor['type'] == 'VEC3' and 'byteStride' not in view
            start = view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
            for i in range(accessor['count']):
                at = start + i*12 + 4; value = struct.unpack_from('<f', binary, at)[0]
                struct.pack_into('<f', binary, at, value + delta)
            for bound in ['min', 'max']:
                if bound in accessor: accessor[bound][1] += delta
        # Keep imported-node transforms aligned with the actual modified GLB.
        for node in media['nodes']:
            if node['name'] in offsets: node['transform']['translation']['y'] += offsets[node['name']]
        identity = 'sf.paperfactory.logistics.media.vacuum-height.' + lod
        new_asset = {'id': identity+'.glb', 'path': 'vacuum-height.glb', 'kind': 'glb'}
        def relocate(v):
            if isinstance(v, dict):
                if v == asset: return dict(new_asset)
                return {k: relocate(x) for k, x in v.items()}
            if isinstance(v, list): return [relocate(x) for x in v]
            return v
        package = relocate(package); package['definition'].update(id=identity, version='1.0.0', title='320×240×240mm箱体真空工具高度 · '+lod)
        package['definition']['payload']['data']['lod'] = {'detailed': None, 'simplified': {'kind': 'asset', 'data': new_asset}, 'region': 'device_icons'}
        package['assets'] = [new_asset]; package['files'] = [{'path': new_asset['path'], 'kind': 'asset'}]; package['dependencies'] = []
        rights = package['rights']
        assert all(all(r.get(k) == rights[0].get(k) for k in ['license', 'redistribution_confirmed', 'missing_reason']) for r in rights)
        package['rights'] = [dict(rights[0], asset_ref=new_asset['id'], source='\n'.join(r['source'] for r in rights)+'\nHeight calibration from '+ref['id']+'@'+ref['version'])]
        package['definition']['contribution']['assets'] = copy.deepcopy(package['rights'])
        directory = output / 'modules' / identity; directory.mkdir(parents=True)
        encoded = json.dumps(gltf, separators=(',', ':')).encode(); encoded += b' '*(-len(encoded)%4)
        blob = struct.pack('<4sII', b'glTF', 2, 28+len(encoded)+len(binary)) + struct.pack('<II', len(encoded), 0x4E4F534A) + encoded + struct.pack('<II', len(binary), 0x004E4942) + binary
        (directory / new_asset['path']).write_bytes(blob)
        write(directory / 'module.json', {'schema_version': '1.0', 'data': package})
        replacements[lod] = {'id': identity, 'version': '1.0.0'}; modules.append(dict(replacements[lod], directory=str(directory)))
        evidence.append({'lod': lod, 'source_ref': ref, 'offsets': offsets, 'accessors_changed': len(accessors), 'node_indices_preserved': True, 'case_bottom_world_path_unchanged': offsets['J3_VerticalTool']+offsets['PaperSeat']==0})
    presentation = copy.deepcopy(original)
    presentation['source'] = {'kind': 'media_ref', 'data': replacements['detailed']}
    presentation['lod'] = {'detailed': None, 'simplified': {'kind': 'media_ref', 'data': replacements['simplified']}, 'region': 'device_icons'}
    write(output / 'media-plan.json', {'modules': modules, 'presentation': presentation, 'height_calibration': evidence})
    print('Both exact vacuum LODs calibrated; cargo-seat world trajectory retained:', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path); parser.add_argument('output', type=Path)
    args = parser.parse_args(); prepare(args.source, args.output)
