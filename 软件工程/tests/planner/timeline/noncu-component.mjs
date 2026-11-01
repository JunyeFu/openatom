// Execute production TSX handlers with the existing Oxc compiler and a small
// hook/element harness. No browser, native UI or OS pointer behavior is claimed.
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import {transformWithOxc} from 'vite';
import {source} from './load-source.mjs';
const time=await source('time'),input=await source('input'),projection=await source('projection');
const compiled=await transformWithOxc(readFileSync(new URL('../../../apps/planner/src/features/timeline/RunTimeline.tsx',import.meta.url),'utf8'),'RunTimeline.tsx',{jsx:{runtime:'classic',pragma:'h',pragmaFrag:'Fragment'}});
const code=compiled.code.replace(/^import .*;\r?\n/gm,'').replace('export function RunTimeline','function RunTimeline')+'\nmodule.exports.RunTimeline=RunTimeline;';
export function mount(initial,{rectWidth=700}={}) {
  let states=[],refs=[],sc=0,rc=0,tree,props=initial,key,active;
  const captured=new Map(),nodesByKey=new Map();
  class Element {
    focus(){active=this;} setPointerCapture(id){captured.set(id,this);} hasPointerCapture(id){return captured.get(id)===this;}
    releasePointerCapture(id){captured.delete(id);} getBoundingClientRect(){return {left:100,width:rectWidth};}
    scrollTo({top}){this.scrollTop=top;}
  }
  class Input extends Element {} class Button extends Element {} class Select extends Element {}
  let prefs={version:1,group_overrides:[],independent_device_ids:[],row_order:[],collapsed_group_ids:[],pinned_resource_ids:[],axis_windows:{},row_scroll:0,snap_enabled:true};
  function h(type,attributes,...children){
    const p={...attributes,children:children.length===1?children[0]:children};
    if(typeof type==='function'){
      if(p.key!==key){key=p.key;states=[];refs=[];sc=rc=0;}
      return type(p);
    }
    const stable=type+':'+(p['aria-label']??p.className??'')+':'+(p.key??'');
    const keep=!!p.ref||!!p['aria-label'];
    const node=(keep?nodesByKey.get(stable):null)??new(type==='input'?Input:type==='button'?Button:type==='select'?Select:Element)();
    Object.assign(node,{type,props:p});if(keep)nodesByKey.set(stable,node);if(p.ref)p.ref.current=node;return node;
  }
  const module={exports:{}};
  vm.runInNewContext(code,{...projection,...time,...input,h,Fragment:'fragment',module,
    useState(initial){const i=sc++;if(!(i in states))states[i]=typeof initial==='function'?initial():initial;return [states[i],v=>{states[i]=typeof v==='function'?v(states[i]):v;}];},
    useRef(initial){return refs[rc++]??=( {current:initial});},useMemo:fn=>fn(),useEffect(){},useLayoutEffect(){},
    HTMLInputElement:Input,HTMLButtonElement:Button,HTMLSelectElement:Select,
    preferenceKey:()=>key,useViewPreferences:()=>({value:prefs,update:v=>{prefs=v;},storageError:''})});
  function nodes(n){if(Array.isArray(n))return n.flatMap(v=>nodes(v));return n?.props?[n,...nodes(n.props.children)]:[];}
  function text(n){if(Array.isArray(n))return n.map(v=>text(v)).join('');if(n?.props)return text(n.props.children);return n==null||typeof n==='boolean'?'':String(n);}
  const api={render(next){if(next)props={...props,...next};sc=rc=0;tree=module.exports.RunTimeline(props);return api;},
    nodes:()=>nodes(tree),find:label=>nodes(tree).find(n=>n.props['aria-label']===label),
    button:label=>nodes(tree).find(n=>n.type==='button'&&text(n)===label),class:name=>nodes(tree).find(n=>n.props.className===name),
    text:()=>text(tree),get root(){return tree;},get captures(){return captured;},get key(){return key;},get active(){return active;}};
  return api.render();
}
export function event(target,extra={}) {
  // A real pointer-up carries the last coordinates/modifier state even when
  // an individual test only supplies its pointer identity at release.
  const value={currentTarget:target,target,clientX:target.lastPointer?.clientX??100,pointerId:9,button:0,altKey:target.lastPointer?.altKey??false,ctrlKey:false,shiftKey:false,preventDefault(){},stopPropagation(){},...extra};
  target.lastPointer={clientX:value.clientX,altKey:value.altKey};return value;
}
