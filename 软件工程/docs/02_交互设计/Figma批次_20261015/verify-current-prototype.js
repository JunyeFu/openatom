const page = await figma.getNodeByIdAsync('22:1278');
await figma.setCurrentPageAsync(page);
const frames = page.children.filter(n => n.type === 'FRAME');
const ids = new Set(frames.map(n => n.id));
const edges = [], brokenTargets = [];
let reactions = 0;
for (const f of frames) {
  for (const node of [f, ...f.findAll()]) {
    if (!('reactions' in node)) continue;
    for (const r of node.reactions) {
      reactions++;
      for (const a of r.actions || (r.action ? [r.action] : [])) {
        if (!a.destinationId) continue;
        edges.push([f.id, a.destinationId]);
        if (!ids.has(a.destinationId)) brokenTargets.push({node:node.id,target:a.destinationId});
      }
    }
  }
}
const reachable = new Set([page.flowStartingPoints[0].nodeId]);
for (let i = 0; i < frames.length; i++) {
  for (const [source,target] of edges) if (reachable.has(source)) reachable.add(target);
}
const states = frames.filter(n => !n.name.startsWith('POP-'));
const main = states.filter(n => n.name.startsWith('VP'));
const text = page.findAllWithCriteria({types:['TEXT']});
return {
  date:'2026-10-16',method:'live_Figma_Plugin_API',
  file_key:'QBtchORv1wTdJI3ldYxexm',page_id:page.id,
  total_top_frames:frames.length,state_frames:states.length,main_frames:main.length,
  reachable_state_frames:states.filter(n => reachable.has(n.id)).length,
  reachable_main_frames:main.filter(n => reachable.has(n.id)).length,
  unreachable_states:states.filter(n => !reachable.has(n.id)).map(n => ({id:n.id,name:n.name})),
  broken_targets:brokenTargets,reactions,native_text_nodes:text.length,
  component_instances:page.findAllWithCriteria({types:['INSTANCE']}).length,
  fonts:[...new Set(text.map(n => typeof n.fontName === 'object' ? n.fontName.family : 'mixed'))],
  flow_starting_points:page.flowStartingPoints,
  design_prototype_only:true,product_frontend_code_changed:false
};
