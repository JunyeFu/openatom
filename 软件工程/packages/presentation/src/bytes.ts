import type { AssetRef } from '../../commands/generated/typescript/AssetRef';
import { UnsupportedMedia } from './types';

export function staticMime(asset: AssetRef, bytes: ArrayBuffer): string {
  const b = new Uint8Array(bytes);
  if (asset.kind === 'png' && b.length >= 8 && [137,80,78,71,13,10,26,10].every((n,i) => b[i] === n)) return 'image/png';
  if (asset.kind === 'jpeg' && b[0] === 255 && b[1] === 216 && b[2] === 255) return 'image/jpeg';
  if (asset.kind === 'svg') {
    const text = new TextDecoder().decode(bytes);
    if (/<!DOCTYPE|<!ENTITY/i.test(text)) throw new UnsupportedMedia('SVG 不支持外部实体或文档类型声明。', asset);
    const doc = new DOMParser().parseFromString(text, 'image/svg+xml');
    if (doc.querySelector('parsererror') || doc.documentElement.localName !== 'svg') throw new UnsupportedMedia('SVG 不是有效静态文档。', asset);
    for (const element of doc.querySelectorAll('*')) {
      if (['script','foreignObject','animate','animateMotion','animateTransform','set','style'].includes(element.localName)) throw new UnsupportedMedia('SVG 含脚本、动态或未支持样式节点。', asset);
      for (const attr of element.attributes) {
        if (/^on/i.test(attr.name) || /url\s*\(|@import/i.test(attr.value) || (/href$/i.test(attr.name) && !attr.value.startsWith('#'))) throw new UnsupportedMedia('SVG 含外部引用或事件；本加载器仅支持静态内嵌图形。', asset);
      }
    }
    return 'image/svg+xml';
  }
  throw new UnsupportedMedia('素材类型或文件签名不属于支持的静态 PNG/JPEG/SVG。', asset);
}
export async function decodeStatic(asset: AssetRef, bytes: ArrayBuffer): Promise<ImageBitmap> {
  const blob=new Blob([bytes], { type: staticMime(asset, bytes) });
  if(asset.kind!=='svg')return createImageBitmap(blob);
  // Chromium does not decode an SVG Blob directly with createImageBitmap.
  // The already validated static SVG is rasterized by its image decoder first.
  const url=URL.createObjectURL(blob),image=new Image();
  try {image.src=url;await image.decode();return await createImageBitmap(image);} finally {URL.revokeObjectURL(url);}
}

export interface GifInfo { durations: number[]; duration_ms: number; repeat: number | null; width: number; height: number }
/** Parse timing only; the browser decodes and composites the original encoded frames. */
export function inspectGif(bytes: ArrayBuffer): GifInfo {
  const b = new Uint8Array(bytes), view = new DataView(bytes);
  let p = 13, delay = 0, repeat: number | null = null;
  const durations: number[] = [];
  const need = (count: number) => { if (p + count > b.length) throw new UnsupportedMedia('GIF 数据截断。'); };
  if (b.length < 13 || !['GIF87a','GIF89a'].includes(new TextDecoder().decode(b.subarray(0,6)))) throw new UnsupportedMedia('GIF 文件签名无效。');
  if (b[10] & 128) p += 3 * 2 ** ((b[10] & 7) + 1);
  const blocks = () => { const parts: Uint8Array[] = []; for (;;) { need(1); const n = b[p++]; if (!n) return parts; need(n); parts.push(b.subarray(p,p+n)); p += n; } };
  let trailer = false;
  while (p < b.length) {
    const tag = b[p++];
    if (tag === 0x3b) { trailer = true; break; }
    if (tag === 0x21) {
      need(1); const kind = b[p++]; const parts = blocks();
      if (kind === 0xf9) { if (parts.length !== 1 || parts[0].length !== 4) throw new UnsupportedMedia('GIF 控制扩展无效。'); delay = (parts[0][1] + parts[0][2]*256)*10; }
      if (kind === 0xff && parts[0] && ['NETSCAPE2.0','ANIMEXTS1.0'].includes(new TextDecoder().decode(parts[0]))) {
        if (parts[1]?.length === 3 && parts[1][0] === 1) repeat = parts[1][1] + parts[1][2]*256;
      }
    } else if (tag === 0x2c) {
      need(9); const packed = b[p+8]; p += 9;
      if (packed & 128) p += 3 * 2 ** ((packed & 7)+1);
      need(1); p++; blocks(); durations.push(delay); delay = 0;
    } else throw new UnsupportedMedia('GIF 含未支持的块。');
  }
  const duration_ms = durations.reduce((a,b) => a+b,0);
  if (!trailer || !durations.length || duration_ms <= 0) throw new UnsupportedMedia('GIF 缺少可用帧时长或结束块；不虚构帧间隔。');
  return {durations,duration_ms,repeat,width:view.getUint16(6,true),height:view.getUint16(8,true)};
}
export function gifFrameAt(info: GifInfo, time: number, terminal = false): number {
  let t = Math.max(0,time);
  if (terminal) t = Math.min(t,info.duration_ms-0.001);
  else if (info.repeat === 0 || (info.repeat != null && t < info.duration_ms*(info.repeat+1))) t %= info.duration_ms;
  else t = Math.min(t,info.duration_ms-0.001);
  let edge = 0;
  for (let i=0;i<info.durations.length;i++) { edge += info.durations[i]; if (t < edge) return i; }
  return info.durations.length-1;
}
interface Decoder {
  tracks: {ready:Promise<void>};
  decode(options:{frameIndex:number;completeFramesOnly:boolean}):Promise<{image:VideoFrame}>;
  close():void;
}
export class ControlledGif {
  readonly info: GifInfo;
  private decoder: Decoder;
  constructor(bytes:ArrayBuffer) {
    this.info = inspectGif(bytes);
    const ctor = (globalThis as unknown as {ImageDecoder?:new(options:{data:ArrayBuffer;type:string;preferAnimation:boolean})=>Decoder}).ImageDecoder;
    if (!ctor) throw new UnsupportedMedia('当前浏览器未提供 ImageDecoder；不改用自主播放 GIF。');
    this.decoder = new ctor({data:bytes,type:'image/gif',preferAnimation:true});
  }
  async frame(time:number, terminal=false):Promise<ImageBitmap> {
    await this.decoder.tracks.ready;
    const {image} = await this.decoder.decode({frameIndex:gifFrameAt(this.info,time,terminal),completeFramesOnly:true});
    try { return await createImageBitmap(image); } finally { image.close(); }
  }
  close() { this.decoder.close(); }
}
