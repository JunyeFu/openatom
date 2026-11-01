"""Give both manual-station LODs a floor anchor and fit wrapper travel to one layer."""
import argparse
import copy
import json
import struct
from pathlib import Path
from prepare_full_factory import read, write


def prepare(source, base_media, output):
    assert not output.exists(); output.mkdir(parents=True)
    plan=read(base_media/'media-plan.json')
    exact={(m['definition']['id'],m['definition']['version']):m for m in read(source/'actual-frozen-modules.json')}
    devices={d['id']:d for d in read(source/'actual-project.json')['devices']}
    for name in ['manual-packing','pallet-stretch-wrapper']:
        ref=devices['PF-'+name]['module_ref'];presentation=copy.deepcopy(exact[(ref['id'],ref['version'])]['definition']['payload']['data']['presentation'])
        refs={'detailed':presentation['source']['data'],'simplified':presentation['lod']['simplified']['data']};new_refs={}
        for lod,ref in refs.items():
            package=copy.deepcopy(exact[(ref['id'],ref['version'])]);media=package['definition']['payload']['data'];asset=media['source']['data']
            raw=(source/'project'/asset['path']).read_bytes();n=struct.unpack_from('<I',raw,12)[0];gltf=json.loads(raw[20:20+n]);binary=bytearray(raw[28+n:])
            if name=='manual-packing':
                root=gltf['nodes'][0]
                assert root.get('translation',[0,0,0])==[0,0,0] and root.get('rotation',[0,0,0,1])==[0,0,0,1]
                media['anchor_roles']['floor_loading']={'kind':'scene_node','data':0}
            else:
                index=next(i for i,x in enumerate(gltf['nodes']) if x['name']=='FilmCarriage')
                gltf['nodes'][index]['translation'][1]=.18
                accessors=set()
                for clip in gltf['animations']:
                    for c in clip['channels']:
                        if c['target']['node']==index:
                            assert c['target']['path']=='translation'
                            accessors.add(clip['samplers'][c['sampler']]['output'])
                assert accessors
                for i in accessors:
                    a=gltf['accessors'][i];v=gltf['bufferViews'][a['bufferView']]
                    assert a['componentType']==5126 and a['type']=='VEC3' and 'byteStride' not in v
                    offset=v.get('byteOffset',0)+a.get('byteOffset',0)
                    for j in range(a['count']):struct.pack_into('<f',binary,offset+j*12+4,.18)
                    for bound in ['min','max']:
                        if bound in a:a[bound][1]=.18
                next(x for x in media['nodes'] if x['name']=='FilmCarriage')['transform']['translation']['y']=.18
            identity='sf.paperfactory.pallet.media.'+name+'.'+lod
            new_asset={'id':identity+'.glb','path':'station.glb','kind':'glb'}
            def relocate(value):
                if isinstance(value,dict):
                    if value==asset:return dict(new_asset)
                    return {k:relocate(v) for k,v in value.items()}
                if isinstance(value,list):return [relocate(v) for v in value]
                return value
            package=relocate(package);package['definition'].update(id=identity,version='1.0.0',title=('地面装托锚点' if name=='manual-packing' else '单层托盘缠膜行程')+' · '+lod)
            package['definition']['payload']['data']['lod']={'detailed':None,'simplified':{'kind':'asset','data':new_asset},'region':'device_icons'}
            package['assets']=[new_asset];package['files']=[{'path':new_asset['path'],'kind':'asset'}];package['dependencies']=[]
            rights=package['rights'];assert rights and all(all(r.get(k)==rights[0].get(k) for k in ['license','redistribution_confirmed','missing_reason']) for r in rights)
            package['rights']=[dict(rights[0],asset_ref=new_asset['id'],source='\n'.join(r['source'] for r in rights)+'\nPallet floor-anchor / low-load carriage adaptation.')]
            package['definition']['contribution']['assets']=copy.deepcopy(package['rights'])
            directory=output/'modules'/identity;directory.mkdir(parents=True)
            encoded=json.dumps(gltf,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4)
            (directory/new_asset['path']).write_bytes(struct.pack('<4sII',b'glTF',2,28+len(encoded)+len(binary))+struct.pack('<II',len(encoded),0x4E4F534A)+encoded+struct.pack('<II',len(binary),0x004E4942)+binary)
            write(directory/'module.json',{'schema_version':'1.0','data':package})
            new_refs[lod]={'id':identity,'version':'1.0.0'};plan['modules'].append(dict(new_refs[lod],directory=str(directory)))
        presentation['source']={'kind':'media_ref','data':new_refs['detailed']}
        presentation['lod']={'detailed':None,'simplified':{'kind':'media_ref','data':new_refs['simplified']},'region':'device_icons'}
        plan['manual_presentation' if name=='manual-packing' else 'wrapper_presentation']=presentation
    plan['station_fixture']={'floor_anchor':'scene_node0','floor_offset_m':[2.425,0,0],'wrapper_carriage_origin_y_m':.18,'film_center_y_m':.46,'film_width_m':.5,'box_y_extent_on_turntable_m':[.34,.58],'runtime_verified':False}
    write(output/'media-plan.json',plan);print('Prepared both LOD floor anchors and one-layer wrapper carriage:',output)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['source','base_media','output']:parser.add_argument(name,type=Path)
    args=parser.parse_args();prepare(args.source,args.base_media,args.output)
