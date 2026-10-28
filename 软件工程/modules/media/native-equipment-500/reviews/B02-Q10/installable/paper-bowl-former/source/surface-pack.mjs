// Embed original image surfaces into exported GLB core PBR materials.
import fs from 'node:fs/promises';
const home=new URL('./surfaces/',import.meta.url);
export async function embedSurfaces(binary,simple){
  if(simple)return Buffer.from(binary);
  const b=Buffer.from(binary),size=b.readUInt32LE(12),j=JSON.parse(b.subarray(20,20+size).toString()),at=20+size;
  let bin=Buffer.from(b.subarray(at+8,at+8+b.readUInt32LE(at)));
  j.images??=[];j.textures??=[];j.samplers??=[];
  const sampler=j.samplers.length;j.samplers.push({magFilter:9729,minFilter:9987,wrapS:10497,wrapT:10497});
  const cache=new Map();
  async function texture(name){
    if(cache.has(name))return cache.get(name);
    const bytes=await fs.readFile(new URL(name,home));
    bin=Buffer.concat([bin,Buffer.alloc((4-bin.length%4)%4)]);const offset=bin.length;bin=Buffer.concat([bin,bytes]);
    const view=j.bufferViews.length;j.bufferViews.push({buffer:0,byteOffset:offset,byteLength:bytes.length});
    const image=j.images.length;j.images.push({bufferView:view,mimeType:'image/png',name});
    const id=j.textures.length;j.textures.push({source:image,sampler});cache.set(name,id);return id;
  }
  for(const material of j.materials??[]){
    const name=material.name??'',p=material.pbrMetallicRoughness??={};
    if(/Brushed stainless|Anodised aluminium|Copper finish|Tin solder/.test(name)){
      p.baseColorTexture={index:await texture('metal-color.png')};p.metallicRoughnessTexture={index:await texture('metal-orm.png')};
      material.normalTexture={index:await texture('metal-normal.png'),scale:.24};
    }else if(/Pallet pine|Pallet pale grain/.test(name)){
      p.baseColorTexture={index:await texture('wood-color.png')};material.normalTexture={index:await texture('wood-normal.png'),scale:.18};
    }else if(name==='Kraft cardboard'){
      p.baseColorTexture={index:await texture('kraft-color.png')};material.normalTexture={index:await texture('kraft-normal.png'),scale:.16};
    }else if(/Powder coat|Ceramic white shell|Cast iron graphite/.test(name)){
      material.normalTexture={index:await texture('paint-normal.png'),scale:.10};
    }
  }
  bin=Buffer.concat([bin,Buffer.alloc((4-bin.length%4)%4)]);j.buffers[0].byteLength=bin.length;
  const raw=Buffer.from(JSON.stringify(j)),json=Buffer.concat([raw,Buffer.alloc((4-raw.length%4)%4,32)]),out=Buffer.alloc(28+json.length+bin.length);
  out.writeUInt32LE(0x46546c67);out.writeUInt32LE(2,4);out.writeUInt32LE(out.length,8);out.writeUInt32LE(json.length,12);out.writeUInt32LE(0x4e4f534a,16);json.copy(out,20);out.writeUInt32LE(bin.length,20+json.length);out.writeUInt32LE(0x004e4942,24+json.length);bin.copy(out,28+json.length);return out;
}
