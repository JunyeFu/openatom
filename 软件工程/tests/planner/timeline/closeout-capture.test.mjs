import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const out=new URL('../../../.work/p204-closeout-20261022/delivery/',import.meta.url);
const {mount,event}=await import(new URL('staged-tests/noncu-component.mjs',out));
const a=JSON.parse(readFileSync(new URL('producer-root/.work/delivery/host/baseline.actual.json',out),'utf8'));
function view(){const seeks=[],before=JSON.stringify(a),v=mount({project:a.project,run:a.run,runScenario:a.scenario,atMs:0,onSelect(){},onTime(){},onSeek:q=>seeks.push(q)});return {v,seeks,before};}
for(const kind of ['ruler','range'])test(`unexpected lost ${kind} capture cancels preview and abandoned pointer-up`,()=>{
 const {v,seeks,before}=view(),n=v.class(kind==='ruler'?'sf-tl-ruler-axis':'sf-tl-chart');
 n.props.onPointerDown(event(n,{clientX:150}));n.props.onPointerMove(event(n,{clientX:250}));v.render();
 const preview=kind==='ruler'?'sf-tl-head sf-tl-ghost':'sf-tl-range';assert.ok(v.class(preview));
 n.releasePointerCapture(9);n.props.onLostPointerCapture?.(event(n));v.render();
 assert.equal(v.class(preview),undefined);
 n.props.onPointerUp(event(n,{clientX:150}));v.render();assert.equal(seeks.length,0);assert.equal(JSON.stringify(a),before);
});
for(const kind of ['ruler','range'])test(`normal ${kind} completion survives subsequent lost capture`,()=>{
 const {v,seeks,before}=view(),n=v.class(kind==='ruler'?'sf-tl-ruler-axis':'sf-tl-chart');
 n.props.onPointerDown(event(n,{clientX:150}));n.props.onPointerMove(event(n,{clientX:250}));
 n.props.onPointerUp(event(n,{clientX:250}));n.props.onLostPointerCapture?.(event(n));v.render();
 assert.equal(seeks.length,kind==='ruler'?1:0);if(kind==='range')assert.ok(v.class('sf-tl-range'));assert.equal(JSON.stringify(a),before);
});
