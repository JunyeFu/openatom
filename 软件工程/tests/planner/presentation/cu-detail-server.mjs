import {createServer} from 'node:http';
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createServer as createViteServer} from 'vite';
import react from '@vitejs/plugin-react';
const root=new URL('../../../',import.meta.url),out=new URL('.work/cu-delivery/',root),cache=new Map(),requests=[];
const load=async label=>{
 if(!['l03-complete','phase-interrupted','phase-resource-wait','l03-robot-output-wait','l03-pallet-output-wait','l03-missing-pcb'].includes(label))throw Error('unknown captured input');
 if(!cache.has(label))cache.set(label,JSON.parse(await readFile(new URL('host/'+label+'.json',out),'utf8')));return cache.get(label);
};
const vite=await createViteServer({configFile:false,root:fileURLToPath(root),plugins:[react()],cacheDir:fileURLToPath(new URL('.work/vite-cache/',root)),
 server:{middlewareMode:true,hmr:false,watch:{ignored:['**/.work/**','**/node_modules/**']},fs:{allow:[fileURLToPath(root),fileURLToPath(new URL('node_modules/',root))]}},appType:'custom'});
const server=createServer(async(req,res)=>{
 try{
  const url=new URL(req.url,'http://127.0.0.1');
  if(url.pathname==='/'){
   const html=await vite.transformIndexHtml('/',`<!doctype html><html lang="zh-CN"><meta charset="UTF-8"><title>P3-02 实际附件独立验证</title><style>body{margin:12px;font:14px 'Microsoft YaHei';color:#17334d;background:#edf4fa}h1{font-size:18px}nav{display:flex;gap:16px;margin:10px 0}label{display:flex;gap:6px}select{min-height:30px}</style><div id="root"></div><script type="module" src="/tests/planner/presentation/cu-detail-browser.tsx"></script></html>`);
   res.setHeader('Content-Type','text/html; charset=utf-8');res.end(html);return;
  }
  if(url.pathname==='/__cu__/input'){
   const data=await load(url.searchParams.get('label')),at=Number(url.searchParams.get('at')),observation=data.points.find(x=>x.at_ms===at);
   if(!observation)throw Error('actual historical time not captured');
   res.setHeader('Content-Type','application/json');res.end(JSON.stringify({project:data.project,run:data.run,observation,modules:data.modules,scope_revision:data.scope_revision,times:data.points.map(x=>x.at_ms).sort((a,b)=>a-b)}));return;
  }
  if(url.pathname==='/__cu__/read'){
   let body='';for await(const chunk of req)body+=chunk;const {label,method,request}=JSON.parse(body),data=await load(label);
   requests.push({label,method,source:request.source??request.params?.source,at_ms:request.at_ms});
   let value;
   if(method==='observation'){
    if(request.run_id!==data.run.id||request.scope_revision!==data.scope_revision)throw Error('wrong captured revision/Run');
    value=data.points.find(x=>x.at_ms===request.at_ms);if(!value)throw Error('unknown historical observation');
   }else{
    const source=request.source??request.params?.source;
    if(source?.data.run_id!==data.run.id||!data.points.some(x=>x.at_ms===source.data.at_ms))throw Error('wrong exact Run/time');
    if(method==='object'){
     if(request.operation!=='get_object')throw Error('no writes in captured consumer');
     value=request.params.object_id.kind==='project'?{kind:'project',data:data.project}:
      {kind:'module',data:data.modules.find(p=>p.definition.id===source.data.module_ref.id&&p.definition.version===source.data.module_ref.version)};
     if(!value.data)throw Error('missing exact captured object');
    }else if(method==='bytes'){
     const ref=source.data.module_ref,pkg=data.modules.find(p=>p.definition.id===ref.id&&p.definition.version===ref.version);
     if(!pkg?.assets.some(a=>a.id===request.asset.id&&a.path===request.asset.path&&a.kind===request.asset.kind))throw Error('not a declared captured asset');
     const file=new URL(`host-media/${ref.id}/${ref.version}/${request.asset.path}`,out);res.setHeader('Content-Type','application/octet-stream');res.end(await readFile(file));return;
    }else throw Error('unknown read method');
   }
   res.setHeader('Content-Type','application/json');res.end(JSON.stringify(value));return;
  }
  if(url.pathname==='/__cu__/close'&&req.method==='POST'){
   await writeFile(new URL('evidence/browser-read-requests.json',out),JSON.stringify(requests,null,2));res.end('closed');setTimeout(async()=>{await vite.close();server.closeAllConnections();server.close();},100);return;
  }
  vite.middlewares(req,res,()=>{res.statusCode=404;res.end('not found');});
 }catch(e){res.statusCode=400;res.end(String(e));}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const state={pid:process.pid,url:`http://127.0.0.1:${server.address().port}/`,scope:'own hidden captured-real-Host browser; no Native or production mutation',ephemeral_port:true};
await writeFile(new URL('server-state.json',out),JSON.stringify(state,null,2));process.stdout.write(JSON.stringify(state)+'\n');
