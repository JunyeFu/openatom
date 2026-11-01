"""Adapt the existing pallet media to three real nested cases and a one-layer film."""
import argparse
import copy
import json
import struct
from pathlib import Path
from prepare_full_factory import DESIGN, read, write


def prepare(source, output):
    assert not output.exists()
    spec = read(DESIGN / 'pallet-line-spec.json')
    packages = read(source / 'actual-frozen-modules.json')
    original = next(m for m in packages if m['definition']['id'] == 'qa.paper.b01q20.media.pallet-stretch-wrapper-input')
    package = copy.deepcopy(original); media = package['definition']['payload']['data']; asset = media['source']['data']
    raw = (source / 'project' / asset['path']).read_bytes()
    n = struct.unpack_from('<I', raw, 12)[0]; gltf = json.loads(raw[20:20+n]); binary = raw[28+n:]
    nodes = {node['name']: node for node in gltf['nodes']}
    # Source is already a pallet with independent content anchors, not baked-in boxes.
    for i, (x, y, z) in enumerate(spec['pallet']['case_bottom_centers_m']):
        nodes[f'ContentSeat_{i}']['translation'] = [x, z, -y]
    media['anchor_roles'] = {f'ContentSeat_{i}': media['anchor_roles'][f'ContentSeat_{i}'] for i in range(3)}
    # A shared manual station also has load/auto/unload phases. Only the wrapper
    # may rotate this pallet or reveal its film; idle state clips stay generic.
    for clip in media['timeline']:
        if ':' in clip['phase_ref']:
            state, phase = clip['phase_ref'].rsplit(':', 1)
            clip['phase_ref'] = state + ':sf.native50.device.pallet-stretch-wrapper:1.1.0:' + phase
            gltf['animations'][clip['source']['data']['animation_index']]['name'] = clip['phase_ref']
    # Change only film children. FinalStretch's animation still controls when film appears.
    # Original film is 1.08m high and 0.364m deep, actual cases are 0.24m high/deep.
    scale = [1, .24 / 1.08, .244 / .364]
    film_nodes = nodes['FinalStretch']['children']
    for index in film_nodes:
        node = gltf['nodes'][index]
        node['translation'] = [v * s for v, s in zip(node.get('translation', [0, 0, 0]), scale)]
        node['scale'] = [v * s for v, s in zip(node.get('scale', [1, 1, 1]), scale)]
    for node in media['nodes']:
        g = nodes[node['name']]
        for prop, keys in [('translation', 'xyz'), ('scale', 'xyz')]:
            if prop in g: node['transform'][prop] = dict(zip(keys, g[prop]))
    identity = 'sf.paperfactory.pallet.media.three-case-load'
    new_asset = {'id': identity + '.glb', 'path': 'three-case-pallet.glb', 'kind': 'glb'}
    def relocate(value):
        if isinstance(value, dict):
            if value == asset: return dict(new_asset)
            return {k: relocate(v) for k, v in value.items()}
        if isinstance(value, list): return [relocate(v) for v in value]
        return value
    package = relocate(package)
    package['definition'].update(id=identity, version=spec['version'], title='三箱单层托盘与缠膜')
    package['assets'] = [new_asset]; package['files'] = [{'path': new_asset['path'], 'kind': 'asset'}]
    package['dependencies'] = []
    rights = package['rights']
    assert rights and all(all(r.get(k) == rights[0].get(k) for k in ['license', 'redistribution_confirmed', 'missing_reason']) for r in rights)
    package['rights'] = [dict(rights[0], asset_ref=new_asset['id'], source='\n'.join(r['source'] for r in rights) + '\nThree-case anchor and film envelope adaptation.')]
    package['definition']['contribution']['assets'] = copy.deepcopy(package['rights'])
    directory = output / 'modules' / identity; directory.mkdir(parents=True)
    encoded = json.dumps(gltf, separators=(',', ':')).encode(); encoded += b' ' * (-len(encoded) % 4)
    (directory / new_asset['path']).write_bytes(struct.pack('<4sII', b'glTF', 2, 28+len(encoded)+len(binary)) + struct.pack('<II', len(encoded), 0x4E4F534A) + encoded + struct.pack('<II', len(binary), 0x004E4942) + binary)
    write(directory / 'module.json', {'schema_version': '1.0', 'data': package})
    reference = {'id': identity, 'version': spec['version']}
    presentation = copy.deepcopy(package['definition']['payload']['data'])
    presentation.update(source={'kind': 'media_ref', 'data': reference}, nodes=[], timeline=[])
    presentation['lod'] = {'detailed': None, 'simplified': {'kind': 'media_ref', 'data': reference}, 'region': 'device_icons'}
    write(output / 'media-plan.json', {'modules': [dict(reference, directory=str(directory))], 'presentation': presentation,
        'media_ref': reference, 'source_ref': {k: original['definition'][k] for k in ['id', 'version']},
        'case_bottom_centers_m': spec['pallet']['case_bottom_centers_m'], 'film_child_scale': scale,
        'node_indices_and_animation_binary_preserved': True, 'runtime_verified': False})
    print('Prepared three-case pallet anchors and one-layer film:', output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path); parser.add_argument('output', type=Path)
    args = parser.parse_args(); prepare(args.source, args.output)
