import {createServer} from 'vite';
import fs from 'node:fs/promises';
import path from 'node:path';
const root=process.cwd(),out=path.join(root,'scenarios/媒体表现/native500-20261024');
await fs.mkdir(out,{recursive:true});
const server=await createServer({root,configFile:false,cacheDir:path.join(out,'vite-cache'),optimizeDeps:{noDiscovery:true,include:[]},server:{host:'127.0.0.1',port:19291,strictPort:true,watch:{ignored:['**/.work/**','**/target/**','**/scenarios/**']}},plugins:[{name:'native500-captures',configureServer(s){s.middlewares.use(async(req,res,next)=>{
 if(!req.url.startsWith('/native500-save/'))return next();
 const name=decodeURIComponent(req.url.slice('/native500-save/'.length));
 if(req.method!=='POST'||!/^([a-z0-9-]+\/)?[a-z0-9.-]+\.(png|json)$/.test(name)){res.statusCode=400;res.end();return;}
 const target=path.join(out,name);await fs.mkdir(path.dirname(target),{recursive:true});await fs.writeFile(target,Buffer.concat(await Array.fromAsync(req)));res.setHeader('Content-Type','application/json');res.end(JSON.stringify({saved:name}));
});}}]});
await server.listen();console.log('Native500 actual GLB reviewer http://127.0.0.1:19291/tools/3d-proof/native500/index.html');
process.on('SIGINT',async()=>{await server.close();process.exit(0);});
