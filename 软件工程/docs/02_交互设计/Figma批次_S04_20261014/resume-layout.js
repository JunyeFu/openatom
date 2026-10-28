// Pending execution. The quota rejection means this repair has not changed the Figma file.
// No SET_VARIABLE reactions: Starter rejected them. Subsequent prototype work uses native frames/states.
await figma.loadFontAsync({family:'Noto Sans SC',style:'Regular'});
await figma.loadFontAsync({family:'Noto Sans SC',style:'Bold'});
const page=figma.currentPage,mutated=[],created=[];
const corrections={
 'F01-01':'设备 / devices',
 'F01-02':'装配单元 / assembly-unit / r1',
 'F03-01':'内联行为 · 不同时引用工艺定义',
 'F03-02':'自动阶段 30 s · 参数已公开',
 'F03-03':'IN / OUT · 控制盒 · piece · 容量 1',
 'F03-04':'包络 0.8×0.6×1.2 m · assembly-r1',
 'F03-05':'operators · 已声明资源角色'
};
for(const frame of page.children.filter(n=>n.type==='FRAME'&&/^(VP|CAT-)/.test(n.name))){
 for(const n of frame.findAll(n=>n.type==='FRAME'&&n.layoutMode==='HORIZONTAL'&&['操作行','视图交互示例','设备视图控制'].includes(n.name))){n.resize(n.width,44);mutated.push(n.id);}
 const tree=frame.findOne(n=>n.name==='项目与库');if(tree){tree.overflowDirection='VERTICAL';mutated.push(tree.id);}
 for(const field of frame.findAll(n=>n.type==='INSTANCE'&&/^F\d\d-\d\d/.test(n.name))){
  const id=field.name.slice(0,6);
  if(corrections[id]){const key=Object.keys(field.componentProperties).find(k=>k.startsWith('value#'));field.setProperties({[key]:corrections[id]});mutated.push(field.id);}
 }
}
// The interactive field selection component must be built and visually checked in a later call.
return {mutated,created,status:'layout_and_sample_value_repair_only',prototype_complete:false};
