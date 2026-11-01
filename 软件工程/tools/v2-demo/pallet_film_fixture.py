"""Match the actual film roll, live web and partial wrap to the one-layer pallet."""
import argparse
import copy
import json
import struct
from pathlib import Path
from prepare_full_factory import read, write


def prepare(source, base_media, output):
    assert not output.exists();output.mkdir(parents=True)
    plan=read(base_media/'media-plan.json')
    package=copy.deepcopy(next(m for m in read(source/'actual-frozen-modules.json') if m['definition']['id']=='qa.paper.b01q20.media.pallet-stretch-wrapper-auxiliary'))
    media=package['definition']['payload']['data'];asset=media['source']['data']
    raw=(source/'project'/asset['path']).read_bytes();n=struct.unpack_from('<I',raw,12)[0];g=json.loads(raw[20:20+n]);binary=bytearray(raw[28+n:]);nodes={x['name']:x for x in g['nodes']}
    # Actual roll is500mm wide. Keep it aligned with the low carriage spindle.
    scale=[1,.24/1.08,.244/.364]
    for idx in nodes['AppliedStretch']['children']:
        node=g['nodes'][idx]
        node['translation']=[v*s for v,s in zip(node.get('translation',[0,0,0]),scale)]
        node['scale']=[v*s for v,s in zip(node.get('scale',[1,1,1]),scale)]
    edited=set()
    for clip in g['animations']:
        phase=clip['name'].rsplit(':',1)[-1] if ':' in clip['name'] else None
        for c in clip['channels']:
            name=g['nodes'][c['target']['node']]['name']
            if c['target']['path']!='translation' or name not in ['MeteredFilmRoll','LiveFilmWeb']:continue
            idx=clip['samplers'][c['sampler']]['output'];assert idx not in edited;edited.add(idx)
            a=g['accessors'][idx];v=g['bufferViews'][a['bufferView']];assert a['type']=='VEC3' and a['componentType']==5126 and 'byteStride' not in v
            start=v.get('byteOffset',0)+a.get('byteOffset',0);values=[]
            for i in range(a['count']):
                at=start+i*12;xyz=list(struct.unpack_from('<3f',binary,at))
                if name=='MeteredFilmRoll':
                    if phase=='load':xyz[1]*=-.01/.12
                    elif phase in ['auto','unload']:xyz[1]=-.01
                else:xyz[1]=.24;xyz[2]*=scale[2]
                struct.pack_into('<3f',binary,at,*xyz);values.append(xyz)
            for key,fn in [('min',min),('max',max)]:
                if key in a:a[key]=[fn(x[k] for x in values) for k in range(3)]
    for row in media['timeline']:
        if ':' in row['phase_ref']:
            state,phase=row['phase_ref'].rsplit(':',1);row['phase_ref']=state+':sf.native50.device.pallet-stretch-wrapper:1.1.0:'+phase
            g['animations'][row['source']['data']['animation_index']]['name']=row['phase_ref']
    for node in media['nodes']:
        raw_node=nodes[node['name']]
        for prop in ['translation','scale']:
            if prop in raw_node:node['transform'][prop]=dict(zip('xyz',raw_node[prop]))
    identity='sf.paperfactory.pallet.media.metered-film';new_asset={'id':identity+'.glb','kind':'glb','path':'metered-film.glb'}
    def relocate(value):
        if isinstance(value,dict):
            if value==asset:return dict(new_asset)
            return {k:relocate(v) for k,v in value.items()}
        if isinstance(value,list):return [relocate(v) for v in value]
        return value
    package=relocate(package);package['definition'].update(id=identity,version='1.0.0',title='9米计量缠绕膜 · 单层托盘')
    package['assets']=[new_asset];package['files']=[{'path':new_asset['path'],'kind':'asset'}];package['dependencies']=[]
    rights=package['rights'];assert rights and all(all(r.get(k)==rights[0].get(k) for k in ['license','redistribution_confirmed','missing_reason']) for r in rights)
    package['rights']=[dict(rights[0],asset_ref=new_asset['id'],source='\n'.join(r['source'] for r in rights)+'\nLow-load roll, live film and one-layer partial-wrap adaptation.')]
    package['definition']['contribution']['assets']=copy.deepcopy(package['rights'])
    directory=output/'modules'/identity;directory.mkdir(parents=True);encoded=json.dumps(g,separators=(',',':')).encode();encoded+=b' '*(-len(encoded)%4)
    (directory/new_asset['path']).write_bytes(struct.pack('<4sII',b'glTF',2,28+len(encoded)+len(binary))+struct.pack('<II',len(encoded),0x4E4F534A)+encoded+struct.pack('<II',len(binary),0x004E4942)+binary)
    write(directory/'module.json',{'schema_version':'1.0','data':package})
    ref={'id':identity,'version':'1.0.0'};plan['modules'].append(dict(ref,directory=str(directory)))
    presentation=copy.deepcopy(package['definition']['payload']['data']);presentation.update(nodes=[],timeline=[],source={'kind':'media_ref','data':ref})
    presentation['lod']={'detailed':None,'simplified':{'kind':'media_ref','data':ref},'region':'device_icons'}
    plan.update(film_presentation=presentation,film_media_ref=ref)
    plan['film_fixture']={'roll_world_center_y_m':.46,'web_world_center_y_m':.46,'roll_width_m':.5,'partial_wrap_height_m':.24,'scoped_to_wrapper':True,'Native_verified':False}
    write(output/'media-plan.json',plan);print('Prepared actual metered film for the low-load wrapper:',output)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['source','base_media','output']:parser.add_argument(name,type=Path)
    args=parser.parse_args();prepare(args.source,args.base_media,args.output)
