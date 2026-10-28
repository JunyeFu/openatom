"""Compose the demo carton states from the accepted carton, label and band assets.

Keeps the label-carton's stable six content seats; only the two band subtrees
are added from the strapper asset. No item identity, quantity or timing changes.
"""
import copy
import json
import struct


def read_glb(path):
    raw=path.read_bytes()
    magic,version,total=struct.unpack_from('<III',raw)
    assert magic==0x46546C67 and version==2 and total==len(raw)
    count,kind=struct.unpack_from('<II',raw,12)
    assert kind==0x4E4F534A
    doc=json.loads(raw[20:20+count])
    offset=20+count
    size,kind=struct.unpack_from('<II',raw,offset)
    assert kind==0x004E4942
    binary=bytearray(raw[offset+8:offset+8+size])
    assert len(doc['buffers'])==1 and not doc.get('images') and not doc.get('skins') and not doc.get('extensionsRequired')
    return doc,binary


def write_glb(path,doc,binary):
    binary.extend(b'\0'*(-len(binary)%4))
    doc['buffers']=[{'byteLength':len(binary)}]
    encoded=json.dumps(doc,separators=(',',':')).encode('utf-8')
    encoded+=b' '*(-len(encoded)%4)
    total=12+8+len(encoded)+8+len(binary)
    path.write_bytes(struct.pack('<III',0x46546C67,2,total)+struct.pack('<II',len(encoded),0x4E4F534A)+encoded+struct.pack('<II',len(binary),0x004E4942)+binary)


def prepare_case_media(saved,load_module,write_module,types,packed,output,operations):
    label_item=load_module(types['label-applicator-input']['module_ref'])
    label_wrapper=label_item['definition']['payload']['data']['presentation_or_media_ref']
    label_module=load_module(label_wrapper['source']['data'])
    label=label_module['definition']['payload']['data']
    band_item=load_module(types['strapper-input']['module_ref'])
    band_module=load_module(band_item['definition']['payload']['data']['presentation_or_media_ref']['source']['data'])
    band=band_module['definition']['payload']['data']
    gltf,binary=read_glb(saved/label['source']['data']['path'])
    other,other_binary=read_glb(saved/band['source']['data']['path'])
    assert not gltf.get('textures') and not other.get('textures')
    assert not label['poses'] and not label['actions'] and not band['poses'] and not band['actions']
    base_node_count=len(gltf['nodes'])
    view_offset=len(gltf['bufferViews']);accessor_offset=len(gltf['accessors'])
    material_offset=len(gltf['materials']);mesh_offset=len(gltf['meshes'])
    binary.extend(b'\0'*(-len(binary)%4));byte_offset=len(binary);binary.extend(other_binary)
    for view in other['bufferViews']:
        row=copy.deepcopy(view);row['buffer']=0;row['byteOffset']=row.get('byteOffset',0)+byte_offset;gltf['bufferViews'].append(row)
    for accessor in other['accessors']:
        assert 'sparse' not in accessor
        row=copy.deepcopy(accessor);row['bufferView']+=view_offset;gltf['accessors'].append(row)
    gltf['materials'].extend(copy.deepcopy(other['materials']))
    for mesh in other['meshes']:
        row=copy.deepcopy(mesh)
        for primitive in row['primitives']:
            assert not primitive.get('targets')
            primitive['attributes']={key:value+accessor_offset for key,value in primitive['attributes'].items()}
            if 'indices' in primitive:primitive['indices']+=accessor_offset
            if 'material' in primitive:primitive['material']+=material_offset
        gltf['meshes'].append(row)
    band_roots=[i for i,n in enumerate(other['nodes']) if n.get('name','').startswith('AppliedBand_')]
    assert len(band_roots)==2
    selected=set()
    def select(index):
        selected.add(index)
        for child in other['nodes'][index].get('children',[]):select(child)
    for root in band_roots:select(root)
    mapping={old:base_node_count+i for i,old in enumerate(sorted(selected))}
    for index in sorted(selected):
        row=copy.deepcopy(other['nodes'][index])
        if 'mesh' in row:row['mesh']+=mesh_offset
        if 'children' in row:row['children']=[mapping[x] for x in row['children']]
        gltf['nodes'].append(row)
    stock=next(i for i,n in enumerate(gltf['nodes'][:base_node_count]) if n.get('name')=='StockRoot')
    gltf['nodes'][stock].setdefault('children',[]).extend(mapping[x] for x in band_roots)
    asset={'id':'sf.paperfactory.packed-case.glb','path':'packed-case.glb','kind':'glb'}
    media=copy.deepcopy(label)
    def replace(value):
        if isinstance(value,dict) and value==label['source']['data']:return copy.deepcopy(asset)
        if isinstance(value,dict):return {key:replace(row) for key,row in value.items()}
        if isinstance(value,list):return [replace(row) for row in value]
        return value
    media=replace(media)
    for index in sorted(selected):
        row=copy.deepcopy(band['nodes'][index]);row['id']=mapping[index]
        row['parent_index']=mapping.get(row['parent_index'],stock)
        row['geometry']['data']['asset_ref']=copy.deepcopy(asset)
        row['geometry']['data']['node_index']=mapping[index]
        media['nodes'].append(row)
    # These are the actual canonical fixture's top tape, label root and bands.
    features=[
        [i for i,n in enumerate(gltf['nodes']) if n.get('name') in ['LeadingTape','TopTape','TrailingTape']],
        [i for i,n in enumerate(gltf['nodes']) if n.get('name')=='AppliedShippingLabel'],
        [mapping[x] for x in band_roots],
    ]
    assert [len(x) for x in features]==[3,1,2]
    feature_nodes={i for group in features for i in group}
    source_clip=next(c for c in label['timeline'] if c['phase_ref']=='sealed_unlabelled_pack')
    base_animation=copy.deepcopy(gltf['animations'][source_clip['source']['data']['animation_index']])
    base_animation['channels']=[c for c in base_animation['channels'] if not(c['target']['node'] in feature_nodes and c['target']['path']=='scale')]
    def accessor(values,width):
        binary.extend(b'\0'*(-len(binary)%4));offset=len(binary)
        binary.extend(struct.pack('<'+'f'*len(values),*values))
        view=len(gltf['bufferViews']);gltf['bufferViews'].append({'buffer':0,'byteOffset':offset,'byteLength':4*len(values)})
        index=len(gltf['accessors']);row={'bufferView':view,'componentType':5126,'count':len(values)//width,'type':'SCALAR' if width==1 else 'VEC3'}
        if width==1:row.update(min=[min(values)],max=[max(values)])
        gltf['accessors'].append(row);return index
    times=accessor([0,.7,.85,1],1)
    constant={n:accessor([float(n)]*12,3) for n in [0,1]}
    applied=accessor([0,0,0,0,0,0,1,1,1,1,1,1],3)
    states=[('packed_folded_case',(0,0,0),(1,0,0)),('taped_top_pack',(1,0,0),(1,1,0)),('sealed_labelled_pack',(1,1,0),(1,1,1)),('two_band_pack',(1,1,1),(1,1,1))]
    gltf['animations']=[];media['timeline']=[]
    for state,before,after in states:
        operation=operations[state]
        for phase in [None,'load','auto','unload']:
            name=f"{state}:{operation['id']}:{operation['version']}:{phase}" if phase else state
            animation=copy.deepcopy(base_animation);animation['name']=name
            for group,start,end in zip(features,before,after):
                output_accessor=applied if phase=='auto' and start!=end else constant[end if phase=='unload' else start]
                sampler=len(animation['samplers']);animation['samplers'].append({'input':times,'output':output_accessor,'interpolation':'STEP'})
                for node in group:animation['channels'].append({'sampler':sampler,'target':{'node':node,'path':'scale'}})
            clip_index=len(gltf['animations']);gltf['animations'].append(animation)
            media['timeline'].append({'id':'factory-case-'+name,'phase_ref':name,'start_progress':0,'end_progress':1,'source':{'kind':'model_animation','data':{'asset_ref':asset,'animation_index':clip_index}},'transition':'hold'})
    media['lod']['simplified']={'kind':'asset','data':asset}
    reference={'id':'sf.paperfactory.media.packed-case','version':packed['module_ref']['version']}
    rights=[copy.deepcopy(label_module['rights'][0])];rights[0]['asset_ref']=asset['id']
    rights[0]['source']='Composed from accepted original SF carton/label and two-band meshes; stable six content seats; editable declarative state clips.'
    write_module(reference['id'],'六件装纸箱：折盖、封带、贴标、捆扎','media',media,assets=[asset],files=[{'path':asset['path'],'kind':'asset'}],rights=rights,version=reference['version'])
    write_glb(output/'modules'/reference['id']/asset['path'],gltf,binary)
    source=load_module(types['manual-packing-output']['module_ref'])['definition']['payload']['data']['presentation_or_media_ref']
    old_reference=source['source']['data']
    def rebind(value):
        if value==old_reference:return copy.deepcopy(reference)
        if isinstance(value,dict):return {key:rebind(row) for key,row in value.items()}
        if isinstance(value,list):return [rebind(row) for row in value]
        return value
    write_module(packed['module_ref']['id'],packed['title'],'items',{'presentation_or_media_ref':rebind(source),'unit':packed['unit'],'tracking':packed['tracking'],'spec_ref':packed['spec_ref'],'attributes':packed['attributes'],'attribute_schema':packed['attribute_schema']},dependencies=[reference],version=packed['module_ref']['version'])
