import {createServer} from 'vite';
import {mkdir,writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=process.cwd(),delivery=path.join(root,'.work/full-closeout-20261021/delivery');
const server=await createServer({root,configFile:false,cacheDir:path.join(delivery,'vite-cache'),
 optimizeDeps:{noDiscovery:true,include:[]},server:{host:'127.0.0.1',port:0,strictPort:true,watch:{ignored:['**']}},
 plugins:[{name:'owned-p302-evidence',configureServer(s){s.middlewares.use(async(req,res,next)=>{
  if(!req.url.startsWith('/p302-evidence/'))return next();
  try{const name=req.url.slice('/p302-evidence/'.length);
   if(!/^render\/[a-z0-9_.-]+$/.test(name))throw Error('Unknown owned evidence name');
   const target=path.join(delivery,name);await mkdir(path.dirname(target),{recursive:true});
   await writeFile(target,Buffer.concat(await Array.fromAsync(req)));res.end('saved');
  }catch(error){res.statusCode=500;res.end(String(error));}
 });}}]});
await server.listen();const address=server.httpServer.address();
await writeFile(path.join(delivery,'browser-server.json'),JSON.stringify({pid:process.pid,host:'127.0.0.1',port:address.port,
 url:'http://127.0.0.1:'+address.port+'/tests/planner/presentation/full-closeout-browser.html'}));
console.log('P3-02 owned hidden review on ephemeral loopback port '+address.port);
process.on('SIGINT',async()=>{await server.close();process.exit(0);});
