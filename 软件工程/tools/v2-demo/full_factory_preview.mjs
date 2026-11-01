// Read-only renderer check of actual Host exports. No production IPC replacement.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
const root=fileURLToPath(new URL('../../',import.meta.url)),require=createRequire(path.join(root,'package.json'));
const {createServer}=await import(pathToFileURL(require.resolve('vite')).href),{default:react}=await import(pathToFileURL(require.resolve('@vitejs/plugin-react')).href);
const proof=path.resolve(process.argv[2]),port=Number(process.argv[3]??8922),read=async name=>JSON.parse((await fs.readFile(path.join(proof,name),'utf8')).replace(/^\uFEFF/,''));
const modules=await read('actual-frozen-modules.json'),assets=new Map();
for(const pkg of modules) for(const asset of pkg.assets) assets.set(JSON.stringify([pkg.definition.id,pkg.definition.version,asset.path]),asset.path);
const files=new Set(['actual-frozen-project.json','actual-scenario.json','actual-connected-run.json','actual-observation-frames.json','actual-frozen-modules.json']);
const server=await createServer({root:path.join(root,'apps/planner'),configFile:false,optimizeDeps:{noDiscovery:true,include:['react','react-dom/client','three']},plugins:[react(),{name:'actual-factory-proof',configureServer(vite){vite.middlewares.use(async(req,res,next)=>{
 try{
  const url=new URL(req.url,'http://127.0.0.1');
  if(url.pathname==='/favicon.ico'){res.statusCode=204;res.end();return;}
  if(url.pathname==='/factory-proof'){
   const entry='/@fs/'+path.join(root,'tools/v2-demo/full_factory_preview.tsx').replaceAll('\\','/');
   res.setHeader('Content-Type','text/html;charset=utf-8');res.end(await vite.transformIndexHtml(url.pathname,'<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>超级工厂 · 实际整厂渲染检查</title></head><body><div id="root"></div><script type="module" src="'+entry+'"></script></body></html>'));return;
  }
  if(url.pathname.startsWith('/proof/')){
   const name=url.pathname.slice(7);if(!files.has(name)){res.statusCode=404;res.end();return;}
   res.setHeader('Content-Type','application/json');res.end(await fs.readFile(path.join(proof,name)));return;
  }
  if(url.pathname==='/proof-asset'){
   const key=JSON.stringify([url.searchParams.get('id'),url.searchParams.get('version'),url.searchParams.get('path')]),asset=assets.get(key);
   if(!asset){res.statusCode=404;res.end('Undeclared asset');return;}
   const base=path.resolve(proof,'project'),file=path.resolve(base,asset);
   if(!file.startsWith(base+path.sep))throw new Error('Asset outside saved project');
   res.setHeader('Content-Type','application/octet-stream');res.end(await fs.readFile(file));return;
  }
  next();
 }catch(error){res.statusCode=500;res.end(String(error));}
});}}],server:{host:'127.0.0.1',port,strictPort:true,watch:null,fs:{allow:[root]}},clearScreen:false});
await server.listen();console.log(`Actual Host renderer preview: http://127.0.0.1:${port}/factory-proof`);
