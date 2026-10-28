"""Original CC0 mechanical-station candidate and controlled media. No imported art."""
from pathlib import Path
import json, math, struct
from PIL import Image, ImageDraw

ROOT=Path(__file__).parent
parts=[]
def part(name,center,size,color,shape='box'):
    parts.append(dict(name=name,center=list(center),size=list(size),color=color,shape=shape))
steel=[.16,.22,.28,1]; blue=[.08,.29,.48,1]; silver=[.57,.64,.69,1]; dark=[.055,.075,.09,1]; orange=[.94,.44,.12,1]
part('Base weldment',(0,.76,0),(1.72,.16,1.10),steel)
part('Table plate',(0,.88,0),(1.8,.07,1.16),silver)
for x in [-.70,.70]:
 for z in [-.43,.43]:
  part('Support leg',(x,.37,z),(.10,.72,.10),steel)
  part('Adjustable foot',(x,.035,z),(.22,.07,.22),dark)
 for y in [.20,.58]:part('Side stretcher',(x,y,0),(.06,.06,.87),steel)
for z in [-.43,.43]:part('Lower cross support',(0,.21,z),(1.4,.06,.06),steel)
for x in [-.67,.67]:
 part('Rear upright',(x,1.37,-.48),(.10,1.00,.12),steel)
 part('Rear gusset',(x,.99,-.40),(.15,.19,.22),blue)
part('Rear beam',(0,1.79,-.48),(1.42,.13,.15),blue)
part('Rear safety panel',(0,1.35,-.56),(1.28,.72,.035),blue)
for x in [-.28,.28]:
 part('Linear vertical rail',(x,1.42,-.36),(.045,.58,.05),silver)
 part('Rail carriage',(x,1.50,-.29),(.13,.17,.12),dark)
part('Contact head',(0,1.50,-.08),(.68,.23,.36),blue)
part('Contact spindle',(0,1.29,.02),(.09,.24,.09),silver)
part('Contact tool',(0,1.12,.02),(.05,.11,.05),orange)
part('Drive housing',(0,1.87,-.32),(.32,.20,.29),dark)
part('Rear drive motor',(0,1.59,-.71),(.22,.34,.23),silver)
part('Motor cover',(0,1.81,-.71),(.27,.08,.27),dark)
part('Screw transmission',(0,1.40,-.35),(.035,.63,.035),silver)
for z in [-.14,.20]:part('Horizontal guide',(0,.95,z),(1.12,.045,.045),silver)
for x,name in [(-.34,'Left clamp jaw'),(.34,'Right clamp jaw')]:
 part(name,(x,1.02,.04),(.12,.14,.34),blue)
 part(name+' pad',(x+(.07 if x<0 else -.07),1.03,.04),(.025,.11,.27),orange)
 part(name+' actuator',(x*1.65,.97,.04),(.25,.08,.14),dark)
part('Workpiece',(0,.97,.04),(.31,.09,.23),silver)
part('Front operator panel',(.64,1.07,.42),(.32,.27,.12),blue)
part('Panel screen',(.64,1.11,.488),(.23,.12,.012),dark)
for x,c in [(.55,[.1,.65,.35,1]),(.65,orange),(.74,[.8,.08,.08,1])]:part('Panel pushbutton',(x,1.015,.492),(.035,.035,.012),c)
part('Control cabinet',(.99,.61,-.23),(.30,.99,.58),blue)
part('Cabinet door',(1.15,.61,-.23),(.025,.83,.49),silver)
part('Cabinet latch',(1.172,.65,-.07),(.02,.14,.025),dark)
for y in [.33,.38,.43,.48]:part('Cabinet rear vent',(.99,y,-.528),(.21,.018,.012),dark)
for x in [-.65,-.45,.45,.65]:
 for z in [-.43,.43]:part('Table fastener',(x,.923,z),(.025,.012,.025),dark)
part('Stacklight stem',(.72,1.65,-.45),(.035,.40,.035),dark)
for y,c in [(1.90,[.1,.7,.3,1]),(1.84,orange),(1.78,[.8,.1,.1,1])]:part('Stacklight lens',(.72,y,-.45),(.075,.055,.075),c)

# One indexed cube with true surface normals, reused by all mechanical parts.
vertices=[];normals=[];indices=[]
faces=[([1,0,0],[(.5,-.5,-.5),(.5,.5,-.5),(.5,.5,.5),(.5,-.5,.5)]),([-1,0,0],[(-.5,-.5,.5),(-.5,.5,.5),(-.5,.5,-.5),(-.5,-.5,-.5)]),([0,1,0],[(-.5,.5,-.5),(-.5,.5,.5),(.5,.5,.5),(.5,.5,-.5)]),([0,-1,0],[(-.5,-.5,.5),(-.5,-.5,-.5),(.5,-.5,-.5),(.5,-.5,.5)]),([0,0,1],[(.5,-.5,.5),(.5,.5,.5),(-.5,.5,.5),(-.5,-.5,.5)]),([0,0,-1],[(-.5,-.5,-.5),(-.5,.5,-.5),(.5,.5,-.5),(.5,-.5,-.5)])]
for normal,points in faces:
 base=len(vertices);vertices.extend(points);normals.extend([normal]*4);indices.extend([base,base+1,base+2,base,base+2,base+3])
binary=bytearray();views=[];accessors=[]
def accessor(values,fmt,kind,count,minv=None,maxv=None):
 while len(binary)%4:binary.append(0)
 offset=len(binary);flat=[v for row in values for v in row] if values and isinstance(values[0],(list,tuple)) else values
 binary.extend(struct.pack('<'+fmt*len(flat),*flat));views.append(dict(buffer=0,byteOffset=offset,byteLength=len(binary)-offset))
 a=dict(bufferView=len(views)-1,componentType={'f':5126,'H':5123}[fmt],count=count,type=kind)
 if minv is not None:a['min']=minv;a['max']=maxv
 accessors.append(a);return len(accessors)-1
pos=accessor(vertices,'f','VEC3',24,[-.5]*3,[.5]*3);normal=accessor(normals,'f','VEC3',24);idx=accessor(indices,'H','SCALAR',36)
materials=[];meshes=[];nodes=[];colors=[]
for item in parts:
 c=item['color']
 if c not in colors:colors.append(c);materials.append(dict(name='original material '+str(len(colors)),pbrMetallicRoughness=dict(baseColorFactor=c,metallicFactor=.45 if c==silver else .15,roughnessFactor=.48)))
 meshes.append(dict(name=item['name'],primitives=[dict(attributes={'POSITION':pos,'NORMAL':normal},indices=idx,material=colors.index(c))]))
 nodes.append(dict(name=item['name'],mesh=len(meshes)-1,translation=item['center'],scale=item['size']))
times=accessor([0.,1.,2.],'f','SCALAR',3,[0.],[2.]);animations=[]
for phase in ['load','auto','unload']:
 samplers=[];channels=[]
 for i,item in enumerate(parts):
  if item['name'] not in ['Contact head','Contact spindle','Contact tool','Left clamp jaw','Right clamp jaw','Left clamp jaw pad','Right clamp jaw pad']:continue
  start=item['center'][:];middle=start[:];end=start[:]
  if 'clamp' in item['name']:
   delta=.11 if start[0]<0 else -.11
   if phase=='load':middle[0]+=delta;end[0]+=delta
   elif phase=='auto':start[0]+=delta;middle[0]+=delta;end[0]+=delta
   else:start[0]+=delta
  elif phase=='auto':middle[1]-=.12
  values=accessor([start,middle,end],'f','VEC3',3);samplers.append(dict(input=times,output=values,interpolation='LINEAR'));channels.append(dict(sampler=len(samplers)-1,target=dict(node=i,path='translation')))
 animations.append(dict(name=phase,samplers=samplers,channels=channels))
doc=dict(asset=dict(version='2.0',generator='P302 original CC0 generate.py'),scene=0,scenes=[dict(nodes=list(range(len(nodes))))],nodes=nodes,meshes=meshes,materials=materials,animations=animations,buffers=[dict(byteLength=len(binary))],bufferViews=views,accessors=accessors)
def glb(document,blob):
 j=json.dumps(document,separators=(',',':')).encode();j+=b' '*((-len(j))%4);blob+=b'\0'*((-len(blob))%4)
 return struct.pack('<III',0x46546c67,2,28+len(j)+len(blob))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(blob),0x004e4942)+blob
(ROOT/'station.glb').write_bytes(glb(doc,binary))
# A separate, explicitly simplified geometry file; never a hidden quality switch.
simple=json.loads(json.dumps(doc));simple['scenes'][0]['nodes']=[i for i,p in enumerate(parts) if not any(word in p['name'] for word in ['fastener','vent','pushbutton','pad','gusset','lens'])]
(ROOT/'station-simple.glb').write_bytes(glb(simple,binary))

def render(progress):
 image=Image.new('RGB',(960,720),(234,241,247));draw=ImageDraw.Draw(image);polygons=[]
 def project(v):
  x,y,z=v;return (int(450+(x-z*.68)*250),int(620+(-y+(x+z)*.23)*250))
 for item in parts:
  x,y,z=item['center'];sx,sy,sz=item['size']
  if 'clamp jaw' in item['name']:x+=(.11 if x<0 else -.11)*progress
  if item['name'] in ['Contact head','Contact spindle','Contact tool']:y-=.12*math.sin(progress*math.pi)
  points=[(x+a*sx,y+b*sy,z+c*sz)for a,b,c in [(-.5,-.5,-.5),(.5,-.5,-.5),(.5,.5,-.5),(-.5,.5,-.5),(-.5,-.5,.5),(.5,-.5,.5),(.5,.5,.5),(-.5,.5,.5)]]
  for face,shade in [([1,5,6,2],.78),([4,5,6,7],.9),([3,2,6,7],1.12)]:
   corners=[points[i]for i in face];rgb=tuple(min(255,int(c*255*shade))for c in item['color'][:3]);polygons.append((sum(v[0]+v[2] for v in corners)/4,[project(v)for v in corners],rgb))
 for _,poly,c in sorted(polygons,key=lambda p:p[0]):draw.polygon(poly,fill=c,outline=tuple(max(0,v-25)for v in c))
 return image
frames=[render(i/7)for i in range(8)]
frames[0].save(ROOT/'station.png');frames[-1].save(ROOT/'station-end.png');frames[0].save(ROOT/'station.gif',save_all=True,append_images=frames[1:],duration=[200]*8,loop=0,disposal=2)
frames[0].save(ROOT/'station.jpg',quality=92)
(ROOT/'anchor.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400" viewBox="0 0 640 400"><rect width="640" height="400" fill="#eaf1f7"/><path d="M100 310V170H500V310M160 170V75H440V170M280 75V210H350V75" fill="none" stroke="#275578" stroke-width="20"/><rect x="215" y="220" width="210" height="40" fill="#c26c28"/></svg>',encoding='utf-8')
assets=[dict(id='p302.'+name,path=name,kind=kind)for name,kind in [('station.png','png'),('station-end.png','png'),('station.gif','gif'),('station.glb','glb'),('station-simple.glb','glb'),('station.jpg','jpeg'),('anchor.svg','svg')]]
rights=[dict(asset_ref=a['id'],source='Original self-authored mechanical station; generate.py in this package',license='CC0-1.0',redistribution_confirmed=True,missing_reason=None)for a in assets]
def source(name):return next(a for a in assets if a['path']==name)
def presentation(kind,name):
 model=kind=='model';asset=source(name)
 declared=[dict(id=i,name=n['name'],parent_index=None,transform=dict(translation=dict(zip('xyz',n['translation'])),rotation=dict(x=0,y=0,z=0,w=1),scale=dict(zip('xyz',n['scale'])),unit='meter',coordinate_frame='right_handed_y_up'),geometry=dict(kind='imported_node',data=dict(asset_ref=asset,node_index=i,unsupported_extensions=[])),material=dict(color_rgba=parts[i]['color'],metallic=.15,roughness=.48,texture=None))for i,n in enumerate(nodes)]if model else []
 clips=[]
 for i,phase in enumerate(['load','auto','unload','cycle']):
  if phase=='cycle':i=1
  clip=dict(kind='model_animation',data=dict(asset_ref=asset,animation_index=i))if model else dict(kind='gif_segment',data=dict(asset_ref=asset,start_ms=0,end_ms=1600))if kind=='gif' else dict(kind='image_asset',data=asset)
  clips.append(dict(id='clip-'+phase,phase_ref=phase,start_progress=0,end_progress=1,source=clip,transition='hold'))
 anchor=dict(kind='scene_node',data=next(i for i,n in enumerate(nodes)if n['name']=='Contact head'))if model else dict(kind='image_normalized',data=dict(x=.48,y=.42))
 return dict(kind=kind,source=dict(kind='asset',data=asset),unit='meter',coordinate_frame='right_handed_y_up',scale=dict(x=1,y=1,z=1),nodes=declared,poses=[],actions=[],timeline=clips,tags=[dict(id='mechanism',label='接触机构',anchor=anchor,progress_range=None,payload=dict(kind='annotation',data='自建候选；工业代表性待认可')),dict(id='effective',label='阶段有效工时',anchor=dict(kind='viewport',data=dict(x=.16,y=.12)),progress_range=None,payload=dict(kind='data',data=dict(source_field='job.effective_progress_ms',object_ref=None,port_ref=None,value_type='duration',unit='ms')))],anchor_roles={'operator':dict(kind='viewport',data=dict(x=.8,y=.8))},port_anchors={'input':dict(kind='viewport',data=dict(x=.14,y=.65))},lod=dict(detailed=None,simplified=None,region='device_icons'),production_effect='none')
presentations={k:presentation(k,n)for k,n in [('image','station.png'),('gif','station.gif'),('model','station.glb')]}
# Simplified asset is separately authored; no timeline animation exists in that asset.
presentations['model']['lod']['simplified']=dict(kind='media_ref',data=dict(id='p302.controlled.model-simple',version='0.1.0'))
simple_p=presentation('model','station-simple.glb')
simple_p['nodes']=[n for i,n in enumerate(simple_p['nodes'])if i in simple['scenes'][0]['nodes']]
for i,n in enumerate(simple_p['nodes']):n['id']=i
simple_p['tags'][0]['anchor']['data']=next(i for i,n in enumerate(simple_p['nodes'])if n['name']=='Contact head')
simple_p['lod']['simplified']=dict(kind='asset',data=source('station-simple.glb'))
presentations['model-simple']=simple_p
for kind,p in presentations.items():
 if kind in ['image','gif']:p['lod']['simplified']=dict(kind='media_ref',data=dict(id='p302.controlled.image-simple',version='0.1.0'))
 p['tags'].append(dict(id='inspect',label='检查设备',anchor=dict(kind='viewport',data=dict(x=.84,y=.12)),progress_range=None,payload=dict(kind='action',data='inspect')))
 dependencies=[p['lod']['simplified']['data']]if p['lod']['simplified']and p['lod']['simplified']['kind']=='media_ref'else []
 package=dict(assets=assets,definition=dict(id='p302.controlled.'+kind,version='0.1.0',title='自建机械工位候选 · '+kind,definition_state='runtime_candidate',contribution=dict(author='P302 controlled media authors',license='CC0-1.0',assets=rights),parameters=[],attribute_schema=[],payload=dict(category='media',data=p)),files=[dict(path=a['path'],kind='asset')for a in assets]+[dict(path='generate.py',kind='asset'),dict(path='LICENSE.txt',kind='asset')],dependencies=dependencies,rights=rights,issues=[])
 (ROOT/(kind+'.module.json')).write_text(json.dumps(package,ensure_ascii=False,indent=2),encoding='utf-8')
simple_image=json.loads((ROOT/'image.module.json').read_text(encoding='utf-8'));simple_image['definition']['id']='p302.controlled.image-simple';simple_image['definition']['payload']['data']['lod']['simplified']=dict(kind='asset',data=source('station.png'));simple_image['dependencies']=[]
(ROOT/'image-simple.module.json').write_text(json.dumps(simple_image,ensure_ascii=False,indent=2),encoding='utf-8')
(ROOT/'candidate-inventory.json').write_text(json.dumps(dict(author='original self-authored CC0',parts=parts,glb_nodes=len(nodes),industrial_acceptance=None),ensure_ascii=False,indent=2),encoding='utf-8')
(ROOT/'LICENSE.txt').write_text('CC0 1.0 Universal\nThe authors dedicate the original generate.py, generated station geometry and imagery in this directory to the public domain under CC0-1.0. No imported third-party art.\nhttps://creativecommons.org/publicdomain/zero/1.0/\n',encoding='utf-8')
print(json.dumps(dict(nodes=len(nodes),assets=len(assets),gif_frames=len(frames),glb_bytes=(ROOT/'station.glb').stat().st_size)))
