// Production TS/TSX and generated schema, executed by a hook/element harness.
// This is neither a browser nor evidence of OS events, layout or visible frames.
import {readFileSync,existsSync} from 'node:fs';
import {readFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import vm from 'node:vm';
import {transformSync} from 'rolldown/utils';
export const root=new URL('../../../',import.meta.url),out=new URL('.work/p205-closeout/delivery/',root);
const rootPath=fileURLToPath(root),before=fileURLToPath(new URL('before/',out));
export function runtime(){
 let current=null,id=0;
 const cache=new Map(),jobs=new Set(),storage=new Map();
 const Fragment=Symbol('Fragment');
 function element(type,props,key){return {type,props:props??{},key};}
 function slot(initial){const frame=current,i=frame.index++;if(!(i in frame.hooks))frame.hooks[i]=initial();return [frame,frame.hooks[i],i];}
 const dependencies=(a,b)=>a&&b&&a.length===b.length&&a.every((v,i)=>Object.is(v,b[i]));
 const react={
  useState(initial){const [frame,s]=slot(()=>({value:typeof initial==='function'?initial():initial}));return [s.value,next=>{const v=typeof next==='function'?next(s.value):next;if(!Object.is(v,s.value)){s.value=v;frame.owner.dirty=true;}}];},
  useRef(initial){return slot(()=>({current:initial}))[1];},
  useMemo(fn,deps){const [,s]=slot(()=>({}));if(!('value'in s)||!dependencies(s.deps,deps)){s.value=fn();s.deps=deps;}return s.value;},
  useId(){return slot(()=>`p205-${++id}`)[1];},
  useEffect(fn,deps){const [frame,s]=slot(()=>({}));if(!dependencies(s.deps,deps)){s.deps=deps;frame.owner.effects.push(()=>{s.cleanup?.();s.cleanup=fn();});}},
 };
 react.useLayoutEffect=react.useEffect;
 class Element{constructor(){this.clientWidth=700;this.clientHeight=100;this.captures=new Set();}focus(){}addEventListener(){}removeEventListener(){}getBoundingClientRect(){return {left:100,width:700};}setPointerCapture(id){this.captures.add(id);}hasPointerCapture(id){return this.captures.has(id);}releasePointerCapture(id){this.captures.delete(id);}scrollTo({top}){this.scrollTop=top;}}
 class Input extends Element{} class TextArea extends Element{} class Button extends Element{} class Select extends Element{}
 const jsx={jsx:element,jsxs:element,Fragment};
 function track(value){if(value?.then){const pending=Promise.resolve(value);jobs.add(pending);pending.then(()=>jobs.delete(pending),()=>jobs.delete(pending));}return value;}
 const context=vm.createContext({console,URL,structuredClone,crypto:globalThis.crypto,setTimeout,clearTimeout,queueMicrotask,
  h:(type,props,...children)=>element(type,{...props,children:children.length===1?children[0]:children},props?.key),Fragment,
  HTMLElement:Element,HTMLInputElement:Input,HTMLTextAreaElement:TextArea,HTMLButtonElement:Button,HTMLSelectElement:Select,
  document:{fonts:{ready:Promise.resolve()}},window:{addEventListener(){},removeEventListener(){}},
  ResizeObserver:class{observe(){}disconnect(){}},requestAnimationFrame:fn=>setImmediate(()=>fn(0)),cancelAnimationFrame:clearImmediate,
  localStorage:{getItem:key=>storage.get(key)??null,setItem:(key,value)=>storage.set(key,value)},
  fetch(url){return track(readFile(fileURLToPath(url),'utf8').then(raw=>({ok:true,status:200,json:async()=>JSON.parse(raw)})));},
 });
 function source(relative){
  const absolute=path.resolve(rootPath,relative);
  if(cache.has(absolute))return cache.get(absolute).exports;
  const module={exports:{}};cache.set(absolute,module);
  if(relative.endsWith('.css'))return module.exports;
  const frozen=path.join(before,relative),sourceFile=process.env.P205_SOURCE_BEFORE==='1'&&existsSync(frozen)?frozen:absolute;
  const text=readFileSync(sourceFile,'utf8').replaceAll('import.meta.url',JSON.stringify(new URL(relative,root).href));
  const compiled=transformSync(absolute,text,{jsx:{runtime:'classic',pragma:'h',pragmaFrag:'Fragment'}});
  if(compiled.errors.length)throw compiled.errors[0];
  const names=[...compiled.code.matchAll(/export (?:async )?(?:function|const|let|class) (\w+)/g)].map(match=>match[1]);
  const code=compiled.code.replace(/import\s*\{([^}]+)\}\s*from\s*['"]([^'"]+)['"];?/g,(_,names,spec)=>`const {${names.replace(/\bas\b/g,':')}}=require(${JSON.stringify(spec)});`)
   .replace(/import\s*['"][^'"]+\.css['"];?/g,'').replace(/\bexport (?=(?:async )?(?:function|const|let|class) )/g,'')+`\nObject.assign(module.exports,{${names.join(',')}});`;
  vm.runInContext(`(function(require,module,exports){${code}\n})`,context,{filename:absolute})(spec=>{
   if(spec==='react')return react;if(spec==='react/jsx-runtime')return jsx;
   let filename=path.resolve(path.dirname(absolute),spec);
   if(!path.extname(filename))filename=['.ts','.tsx'].map(ext=>filename+ext).find(existsSync);
   if(!filename)throw new Error(`unresolved production import ${spec}`);
   return source(path.relative(rootPath,filename));
  },module,module.exports);
  return module.exports;
 }
 function mount(Component,initial){
  let props=initial,tree;
  const owner={dirty:true,effects:[]},frames=new Map(),nodes=new Map();
  const wrap=(name,value)=>['onCommit','onRefresh','onRun','onCommitInput','onSeek'].includes(name)?(...args)=>track(value(...args)):value;
  function renderNode(raw,key){
   if(Array.isArray(raw))return raw.map((v,i)=>renderNode(v,`${key}/${v?.key??i}`));
   if(raw==null||typeof raw!=='object')return raw;
   if(raw.type===Fragment)return renderNode(raw.props.children,key+'/fragment');
   if(typeof raw.type==='function'){
    const identity=`${key}/${raw.type.name}:${raw.key??''}`;
    let frame=frames.get(identity);if(!frame){frame={hooks:[],index:0,owner};frames.set(identity,frame);}frame.seen=true;frame.index=0;
    const previous=current;current=frame;const result=raw.type(raw.props);current=previous;
    return renderNode(result,identity);
   }
   const type=raw.type,identity=`${key}/${type}:${raw.key??''}`;
   let node=nodes.get(identity);if(!node){node=new(type==='input'?Input:type==='textarea'?TextArea:type==='button'?Button:type==='select'?Select:Element)();nodes.set(identity,node);}
   node.type=type;node.tagName=String(type).toUpperCase();node.props={...raw.props,children:renderNode(raw.props.children,identity)};
   if(node.props.ref)node.props.ref.current=node;
   return node;
  }
  function render(next){if(next)props={...props,...next};let loops=0;
   do{owner.dirty=false;owner.effects=[];for(const f of frames.values())f.seen=false;
    tree=renderNode(element(Component,Object.fromEntries(Object.entries(props).map(([name,value])=>[name,typeof value==='function'?wrap(name,value):value]))),'root');
    for(const [key,f]of frames)if(!f.seen){for(const hook of f.hooks)hook?.cleanup?.();frames.delete(key);}
    for(const effect of owner.effects)effect();if(++loops>40)throw new Error('component did not settle');
   }while(owner.dirty);
   return api;
  }
  const all=n=>Array.isArray(n)?n.flatMap(all):n?.props?[n,...all(n.props.children)]:[];
  const text=n=>Array.isArray(n)?n.map(text).join(''):n?.props?text(n.props.children):n==null||typeof n==='boolean'?'':String(n);
  const api={render,nodes:()=>all(tree),text:()=>text(tree),textOf:text,find:label=>all(tree).find(n=>n.props['aria-label']===label),
   button:label=>all(tree).find(n=>n.type==='button'&&text(n)===label),
   async flush(){for(let i=0;i<20;i++){await Promise.all([...jobs]);await new Promise(r=>setImmediate(r));if(owner.dirty)render();if(!jobs.size&&!owner.dirty)return api;}throw new Error('pending callbacks did not settle');},
   async act(fn){await fn();render();return api.flush();},
   unmount(){for(const f of frames.values())for(const hook of f.hooks)hook?.cleanup?.();frames.clear();},
  };return render();
 }
 return {source,mount,Element,Input,TextArea};
}
export function event(target,extra={}){return {target,currentTarget:target,key:'',ctrlKey:false,metaKey:false,shiftKey:false,altKey:false,nativeEvent:{isComposing:extra.isComposing??false},defaultPrevented:false,propagationStopped:false,preventDefault(){this.defaultPrevented=true;},stopPropagation(){this.propagationStopped=true;},...extra};}
