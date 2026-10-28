// Headless adapter: decode the real embedded image bytes with installed Pillow.
// This proves decoded pixels/material inputs, not WebGL upload or visual quality.
import {spawnSync} from 'node:child_process';
const pixels=new Map();
globalThis.self=globalThis;
export function decodePillow(encoded,frameIndex=0){
 const key=frameIndex+':'+encoded.toString('base64');
 let decoded=pixels.get(key);
 if(!decoded){
  const result=spawnSync('D:/MathModelingTools/envs/cumcm/python.exe',['-c',"from PIL import Image; import sys,io,struct; im=Image.open(io.BytesIO(sys.stdin.buffer.read())); im.seek(int(sys.argv[1])); im=im.convert('RGBA'); sys.stdout.buffer.write(struct.pack('<II',*im.size)+im.tobytes())",String(frameIndex)],{input:encoded,maxBuffer:128*1024*1024,windowsHide:true});
  if(result.status!==0)throw Error('Pillow image decode failed: '+result.stderr);
  const raw=result.stdout;decoded={width:raw.readUInt32LE(0),height:raw.readUInt32LE(4),data:raw.subarray(8)};
  if(decoded.data.length!==decoded.width*decoded.height*4)throw Error('Incomplete decoded pixels');
  pixels.set(key,decoded);
 }
 return {...decoded,close(){}};
}
globalThis.createImageBitmap=async value=>value.data&&value.width?{...value,close(){}}:decodePillow(Buffer.from(await value.arrayBuffer()));
export const decodedImageCount=()=>pixels.size;
