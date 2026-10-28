import assert from 'node:assert/strict';
import {test} from 'node:test';
import {writeFile,mkdir} from 'node:fs/promises';
import {root,out,json,bytes} from './runtime.mjs';
import {decodePillow} from './pixel-decoder.mjs';
const {ControlledGif,inspectGif}=await import(new URL('bytes.mjs',out));
test('actual Host GIF bytes retain encoded frames and controlled timing through explicit headless decoder adapter',async()=>{
 const result=await json('.work/delivery/controlled-host/result.json');assert.equal(result.passed,true);
 const row=result.byte_reads.find(x=>x.variant==='gif'&&x.asset.kind==='gif'),encoded=await bytes(row.file),info=inspectGif(encoded),before=JSON.stringify(result);
 assert.deepEqual(info.durations,Array(8).fill(200));assert.equal(info.duration_ms,1600);
 delete globalThis.ImageDecoder;assert.throws(()=>new ControlledGif(encoded),/ImageDecoder/);
 const requests=[];
 globalThis.ImageDecoder=class {
  constructor(options){assert.equal(options.type,'image/gif');assert.equal(options.preferAnimation,true);this.bytes=Buffer.from(options.data);this.tracks={ready:Promise.resolve()};}
  async decode({frameIndex,completeFramesOnly}){assert.equal(completeFramesOnly,true);requests.push(frameIndex);return {image:decodePillow(this.bytes,frameIndex)};}
  close(){this.closed=true;}
 };
 const gif=new ControlledGif(encoded),samples=[];
 for(const [time,terminal,expected]of [[0,false,0],[199,false,0],[200,false,1],[800,false,4],[1600,false,0],[1600,true,7],[800,false,4]]){
  const frame=await gif.frame(time,terminal);assert.equal(requests.at(-1),expected);assert.equal(frame.width,960);assert.equal(frame.data.length,frame.width*frame.height*4);samples.push({time,terminal,frame:expected,width:frame.width,height:frame.height});frame.close();
 }
 assert.notDeepEqual(decodePillow(Buffer.from(encoded),0).data,decodePillow(Buffer.from(encoded),4).data);
 gif.close();assert.equal(JSON.stringify(result),before);
 await mkdir(new URL('.work/delivery/consumer-results/',root),{recursive:true});
 await writeFile(new URL('.work/delivery/consumer-results/controlled-gif.json',root),JSON.stringify({passed:true,actual_host_source:row.source,asset:row.asset,samples,decoder:'Pillow real GIF frame decoder behind explicit ImageDecoder-shaped test adapter; not browser WebCodecs or GUI evidence',production_clock_created:false},null,2));
});
