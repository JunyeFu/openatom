from pathlib import Path
import json,math,struct
root=Path(__file__).resolve().parents[4];home=root/'modules/media/native-equipment-500/reviews/B01-Q15';read=lambda p:json.loads(p.read_text(encoding='utf8'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
rebuild=read(home/'source-rebuild-checks.json');assert rebuild['passed']and len(rebuild['checks'])==6
def check(p):
 data=p.read_bytes();magic,version,length=struct.unpack_from('<III',data);assert magic==0x46546c67 and version==2 and length==len(data)
 jlen=struct.unpack_from('<I',data,12)[0];j=json.loads(data[20:20+jlen]);binary=data[28+jlen:];assert len(binary)>=j['buffers'][0]['byteLength']
 def values(i):
  a=j['accessors'][i];view=j['bufferViews'][a['bufferView']];width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}[a['type']];assert a['componentType']==5126
  offset=view.get('byteOffset',0)+a.get('byteOffset',0);stride=view.get('byteStride',width*4);return[struct.unpack_from('<'+'f'*width,binary,offset+k*stride)for k in range(a['count'])]
 for i,a in enumerate(j['accessors']):
  if a['componentType']==5126:assert all(math.isfinite(v)for row in values(i)for v in row),(p,i)
 names=[n.get('name')for n in j['nodes']];assert None not in names and len(names)==len(set(names)),p
 for clip in j.get('animations',[]):
  pairs=[(c['target']['node'],c['target']['path'])for c in clip['channels']];assert len(pairs)==len(set(pairs)),p
  for sampler in clip['samplers']:
   t=[v[0]for v in values(sampler['input'])];assert all(b>a for a,b in zip(t,t[1:])),(p,clip['name'])
 return dict(bytes=len(data),nodes=len(j['nodes']),finite_geometry_and_tracks=True,unique_channels=True,strictly_increasing_float32_times=True,own_source_rebuild_same_bytes=True)
checks=[]
for e in read(home/'inventory.json')['entries']:
 c=check(home/e['directory']/e['asset']['path']);assert c['bytes']==e['bytes']and c['nodes']==e['nodes'];checks.append(dict(id=e['id'],lod=e['suffix'],**c))
write(home/'source-checks.json',dict(passed=True,models=2,exact_packages=3,layout_revision='finite-tray-corner-tuck-slot-glue-services-r1',checks=checks))
checks=[]
for e in read(home/'items/inventory.json')['metadata']:checks.append(dict(id=e['id'],**check(home/'items'/e['id']/'paper.glb')))
write(home/'items/source-checks.json',dict(passed=True,models=4,scope='actual Item GLB numeric/accessor/timeline validity and own-package rebuild; Native separate',checks=checks))
print('Two equipment and four Item GLBs: finite accessors, unique channels, strict Float32 times and own-source rebuild PASS.')
