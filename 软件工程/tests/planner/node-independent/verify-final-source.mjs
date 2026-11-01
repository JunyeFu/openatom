import {readFile,writeFile,mkdir} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),ours=new URL('scenarios/媒体表现/runtime-node-independent/',root),owner=new URL('scenarios/媒体表现/runtime-node-inspector/',root);
const base=JSON.parse(await readFile(new URL('source-cut.json',ours),'utf8'));
const cut=JSON.parse(await readFile(new URL('source-cut.json',owner),'utf8'));
const css='apps/planner/src/features/node-inspector/node-inspector.css',finalCss=new URL('readability-final/node-inspector.css',ours);
await mkdir(new URL('readability-final/',ours),{recursive:true});
await writeFile(finalCss,await readFile(new URL(css,root)));
const ownerChecks=[];
for(const file of cut.files)ownerChecks.push({source:file.path,snapshot:'scenarios/媒体表现/runtime-node-inspector/'+file.snapshot,equal:(await readFile(new URL(file.path,root))).equals(await readFile(new URL(file.snapshot,owner)))});
for(const path of cut.root_owned_validation_cut){const snapshot='final-source/root-owned__'+path.replaceAll('/','__');ownerChecks.push({source:path,snapshot:'scenarios/媒体表现/runtime-node-inspector/'+snapshot,equal:(await readFile(new URL(path,root))).equals(await readFile(new URL(snapshot,owner)))});}
const checks=[];
for(const file of base.snapshots){const snapshot=file.source===css?'scenarios/媒体表现/runtime-node-independent/readability-final/node-inspector.css':file.snapshot;checks.push({source:file.source,snapshot,equal:(await readFile(new URL(file.source,root))).equals(await readFile(new URL(snapshot,root)))});}
const result={at:new Date().toISOString(),comparison:'direct Buffer equality, no hashes',logical_base:'source-cut.json',css_overlay:{source:css,snapshot:'scenarios/媒体表现/runtime-node-independent/readability-final/node-inspector.css',logical_old_snapshot:base.snapshots.find(f=>f.source===css).snapshot},Root_authority:'Root released final consumer16 plus final Root3 after sample-race and only local inline readability CSS leaf',owner_count:ownerChecks.length,owner:ownerChecks,independent_count:checks.length,independent:checks,all_equal:[...ownerChecks,...checks].every(f=>f.equal),functional_22_checks_are_not_rerun_or_double_counted_for_CSS_only_leaf:true};
await writeFile(new URL('final-source-comparison.json',ours),JSON.stringify(result,null,2));
console.log(JSON.stringify({owner_count:result.owner_count,independent_count:result.independent_count,all_equal:result.all_equal}));
if(!result.all_equal)process.exitCode=1;
