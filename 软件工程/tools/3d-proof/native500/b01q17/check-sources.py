from pathlib import Path
import json,struct,math,shutil,subprocess
ROOT=Path(__file__).resolve().parents[4];HOME=ROOT/'modules/media/native-equipment-500/reviews/B01-Q17';OUT=ROOT/'.work/native500-quality-20261025/b01q17/source-reexport-02'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
def check(p):
 data=p.read_bytes();magic,version,size=struct.unpack_from('<III',data);assert(magic,version,size)==(0x46546c67,2,len(data));n=struct.unpack_from('<I',data,12)[0];j=json.loads(data[20:20+n]);binary=data[28+n:]
 def vals(i):
  a=j['accessors'][i];v=j['bufferViews'][a['bufferView']];width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}[a['type']];offset=v.get('byteOffset',0)+a.get('byteOffset',0);stride=v.get('byteStride',width*4);return[struct.unpack_from('<'+'f'*width,binary,offset+k*stride)for k in range(a['count'])]
 names=[n.get('name')for n in j['nodes']];assert None not in names,p
 for i,a in enumerate(j['accessors']):
  if a['componentType']==5126:assert all(math.isfinite(v)for row in vals(i)for v in row)
 for clip in j['animations']:
  channels=[(c['target']['node'],c['target']['path'])for c in clip['channels']];assert len(channels)==len(set(channels)),p
  # Static score marks use stable node indices; only animation names must be unambiguous.
  assert all(names.count(names[node])==1 for node,_ in channels),p
  for s in clip['samplers']:
   times=[t[0]for t in vals(s['input'])];assert all(b>a for a,b in zip(times,times[1:]))
 assert not any('uri'in b for b in j.get('buffers',[]))and not any('uri'in i for i in j.get('images',[]))
 return{'bytes':len(data),'nodes':len(j['nodes']),'clips':[a['name']for a in j['animations']],'finite_geometry_and_tracks':True,'unique_channels':True,'self_contained_GLB':True}
assert not OUT.exists(),'re-export directory must be new'
equipment=[];items=[]
for e in read(HOME/'inventory.json')['entries']:
 source=HOME/e['directory'];target=OUT/(e['id']+'-'+e['suffix']);shutil.copytree(source,target);subprocess.run(['node',str(target/'source/export.mjs')],cwd=ROOT,check=True,capture_output=True);before=check(source/e['asset']['path']);after=check(target/e['asset']['path']);assert before['nodes']==after['nodes']and before['clips']==after['clips'];equipment.append({'id':e['id'],'lod':e['suffix'],**before,'own_package_reexport':True})
for e in read(HOME/'items/inventory.json')['metadata']:
 source=HOME/'items'/e['id'];target=OUT/e['id'];shutil.copytree(source,target);subprocess.run(['node',str(target/'source/item-build.mjs')],cwd=ROOT,check=True,capture_output=True);before=check(source/'paper.glb');after=check(target/'paper.glb');assert before['nodes']==after['nodes']and before['clips']==after['clips'];items.append({'id':e['id'],**before,'own_package_reexport':True})
write(HOME/'source-checks.json',{'passed':True,'modeling_profile':'simulation-efficient-v1','scope':'GLB validity and own-package re-export; byte identity is not a gate','checks':equipment});write(HOME/'items/source-checks.json',{'passed':True,'checks':items});print(f'{len(equipment)} equipment and {len(items)} Item GLBs valid and own-package re-exported.')
