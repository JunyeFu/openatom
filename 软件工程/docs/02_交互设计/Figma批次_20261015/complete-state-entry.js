// Run after the base wire pass. Only repairs existing prototype-state entry paths.
const page = await figma.getNodeByIdAsync('22:1278');
await figma.setCurrentPageAsync(page);
const createdNodeIds = [], mutatedNodeIds = [];
async function link(id, target, navigation = 'NAVIGATE') {
  const node = await figma.getNodeByIdAsync(id);
  await node.setReactionsAsync([{trigger:{type:'ON_CLICK'},actions:[{
    type:'NODE',destinationId:target,navigation,transition:null
  }]}]);
  mutatedNodeIds.push(node.id);
}
// Preserve the other panel's state when either rail is toggled.
await link('27:8986', '26:4523');
await link('27:9091', '26:4523');
await link('27:9309', '26:4522');
await link('27:9412', '26:4521');
// VP24 contains unapplied fields: selecting another object must ask first.
await link('25:3600', '34:5120');
// A contextual file overlay preserves the current dirty background.
let menu = page.children.find(n => n.name === 'POP-MENU文件-VP24');
if (!menu) {
  const source = await figma.getNodeByIdAsync('23:1311');
  const fonts = source.findAllWithCriteria({types:['TEXT']})
    .flatMap(t => t.getStyledTextSegments(['fontName']).map(s => s.fontName));
  for (const font of fonts) await figma.loadFontAsync(font);
  const maxX = Math.max(...page.children.map(n => n.x + n.width));
  menu = source.clone();
  page.appendChild(menu);
  menu.name = 'POP-MENU文件-VP24';
  menu.x = maxX + 100;
  createdNodeIds.push(menu.id, ...menu.findAll().map(n => n.id));
}
const open = menu.findAllWithCriteria({types:['INSTANCE']})
  .find(n => n.name.startsWith('操作 / open-project / '));
await link(open.id, '34:5121');
await link('25:3564', menu.id, 'OVERLAY');
return {createdNodeIds,mutatedNodeIds,contextualMenuId:menu.id,
  repairs:['independent_panel_toggles','dirty_object_switch','open_unapplied_fields'],
  scope:'native_Figma_design_reactions_only'};
