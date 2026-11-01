import {useEffect,useRef,useState} from 'react';
import type {FileRef} from '../../../../../packages/commands/generated/typescript/FileRef';
import type {PackagingEvidence} from './types';
import {domPainted,type ReadonlyVisualCapture} from '../render-surface/visualCapture';
import './manufacturing-preview.css';

export interface ManufacturingPreviewProps {
  evidence:PackagingEvidence;
  readFile?:(evidence:PackagingEvidence,file:FileRef)=>Promise<ArrayBuffer>;
  capture?:ReadonlyVisualCapture;
}
/** The stored SVG itself, shared by the normal workspace and immutable render tasks. */
export function ManufacturingPreview({evidence,readFile,capture}:ManufacturingPreviewProps){
  const [images,setImages]=useState<{url:string;part:string;path:string;layers:string[];unit:string}[]>([]),[error,setError]=useState('');
  const host=useRef<HTMLElement>(null);
  useEffect(()=>{
    let live=true;const urls:string[]=[];setImages([]);setError('');
    async function load(){
      if(!readFile)throw new Error('宿主尚未接入此Artifact精确制造文件的只读字节入口。');
      const values=[];
      for(const file of evidence.result.manufacturing_files){
        const bytes=await readFile(evidence,file.file);if(!live)return;
        const url=URL.createObjectURL(new Blob([bytes],{type:'image/svg+xml'}));urls.push(url);
        const image=new Image();image.src=url;await image.decode();if(!live)return;
        values.push({url,part:file.part_id,path:file.file.path,layers:file.layers,unit:file.unit});
      }
      if(live)setImages(values);
    }
    load().catch(reason=>{if(live){const message=reason instanceof Error?reason.message:String(reason);setError(message);capture?.onFailure(message);}});
    return()=>{live=false;for(const url of urls)URL.revokeObjectURL(url);};
  },[evidence,readFile,capture]);
  useEffect(()=>{
    if(!capture||error||!readFile||images.length!==evidence.result.manufacturing_files.length)return;
    let live=true;
    Promise.all([...host.current!.querySelectorAll('img')].map(image=>image.decode())).then(()=>domPainted(()=>live)).then(done=>{
      if(!done)return;
      const rect=host.current!.getBoundingClientRect();
      const visible=[...host.current!.querySelectorAll('figure')].some(node=>{const r=node.getBoundingClientRect();return r.width>0&&r.height>0&&r.bottom>Math.max(0,rect.top)&&r.top<Math.min(innerHeight,rect.bottom);});
      capture.onFrame({visible_objects:visible?[capture.object]:[],limitations:['二维制造图不适用三维相机；图纸/构件ID是文件内容身份，可见对象使用真实Artifact父身份。',...(!images.length?['此实际成果没有制造SVG文件。']:[])]});
    }).catch(reason=>{if(live)capture.onFailure(String(reason));});
    return()=>{live=false;};
  },[images,error,evidence,capture,readFile]);
  return <section className="manufacturing-preview" aria-label="实际制造SVG图纸" ref={host}>
    <h3>实际制造图 · {evidence.artifact.id}</h3>
    {error?<p role="alert">{error}</p>:images.length!==evidence.result.manufacturing_files.length?<p role="status">正在读取并解码已存SVG…</p>:!images.length?<p>此实际成果未生成制造图。</p>:null}
    <div className="manufacturing-sheets">{images.map((image,index)=><figure key={`${image.path}:${index}`}><figcaption>构件 {image.part} · {image.unit} · {image.layers.join(' / ')}<small>{image.path}</small></figcaption><img src={image.url} alt={`实际制造图 ${image.part}`} /></figure>)}</div>
  </section>;
}
