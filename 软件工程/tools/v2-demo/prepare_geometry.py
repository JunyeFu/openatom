"""New-version display declarations; original saved r2 and original assets remain untouched."""
from pathlib import Path
import copy,json,shutil
ROOT=Path(__file__).resolve().parents[2]
def read(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v):p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def pose(x,y,z):return {'translation':{'x':x,'y':y,'z':z},'rotation':{'x':0,'y':0,'z':0,'w':1},'scale':{'x':1,'y':1,'z':1},'unit':'meter','coordinate_frame':'right_handed_z_up'}
def layout(count,device=False):
 return {'assignment':'item_id_order','slots':[{'item_type_ref':{'id':'benchmark-item-v1','revision':'r1'},'anchor_role':'workpiece' if device else None,
  'offset':pose(0,0,0) if device else pose((i%3-1)*.45,(i//3-.5)*.45,.05)} for i in range(count)]}
def prepare(bundle:Path):
 source=ROOT/'resources/v2-demo/project';dest=bundle/'source'
 if dest.exists():raise FileExistsError('New geometry source required; original inputs are never overwritten')
 bundle.mkdir(parents=True,exist_ok=True);shutil.copytree(source,dest)
 manifest=read(dest/'project.json');m=manifest['data'];entries=m['documents']
 versions={'p201.b60.station':'1.0.6','p201.b60.simple':'1.0.6','p201.b60.workpiece':'1.0.2'}
 fresh=[]
 for entry in list(entries):
  if entry['file_ref']['kind']!='module' or entry['module_ref']['id'] not in versions:continue
  package=read(dest/entry['file_ref']['path']);pkg=package['data'];definition=pkg['definition'];module=definition['id'];old=definition['version'];definition['version']=versions[module]
  # The actual device body and shared GLB are unchanged; only exact display-version closure changes.
  if module=='p201.b60.station':
   definition['payload']['data']['presentation']['lod']['simplified']['data']['version']='1.0.6'
   for dependency in pkg['dependencies']:
    if dependency['id']=='p201.b60.simple':dependency['version']='1.0.6'
  path=f'geometry-module-{module.split(".")[-1]}-{versions[module]}.json';write(dest/path,package)
  new=copy.deepcopy(entry);new['module_ref']['version']=versions[module];new['file_ref']['path']=path;entries.append(new)
  m['module_refs'].append(copy.deepcopy(new['module_ref']))
  fresh.append({'id':module,'old_version':old,'new_version':versions[module],'file':path})
 write(dest/'project.json',manifest)
 active=next(e for e in entries if e['role']=='active_project');project=read(dest/active['file_ref']['path'])['data']
 scenario_entry=next(e for e in entries if e['role']=='scenario_input');scenario=read(dest/scenario_entry['file_ref']['path'])['data']
 # Explicitly adopt the LOD dependency as well; source presence alone does not put it in the frozen Project closure.
 commands=[{'kind':'adopt_module','data':{'module_ref':{'id':'p201.b60.simple','version':'1.0.6'},'targets':[],'bindings':{'item_types':{},'resources':{},'targets':{},'ports':{}},'new_instances':[],'explicit_seed':None}}]
 for device in project['devices']:
  commands.append({'kind':'adopt_module','data':{'module_ref':{'id':device['module_ref']['id'],'version':'1.0.6'},'targets':[{'kind':'device','id':device['id']}],'bindings':device['role_bindings'],'new_instances':[],'explicit_seed':None}})
  d=copy.deepcopy(device);d['module_ref']['version']='1.0.6';d['item_layout']=layout(1,True)
  if d.get('operation_binding') and d['operation_binding']['id']=='p201.b60.station':d['operation_binding']['version']='1.0.6'
  for ref in d.get('route_operation_refs',[]):
   if ref['id']=='p201.b60.station':ref['version']='1.0.6'
  commands.append({'kind':'put_object','data':{'kind':'device','data':d}})
 commands.append({'kind':'adopt_module','data':{'module_ref':{'id':'p201.b60.workpiece','version':'1.0.2'},'targets':[{'kind':'item_type','id':'benchmark-item-v1'}],'bindings':{'item_types':{},'resources':{},'targets':{},'ports':{}},'new_instances':[],'explicit_seed':None}})
 for location in project['locations']:
  d=copy.deepcopy(location);d['item_layout']=layout(6);commands.append({'kind':'put_object','data':{'kind':'location','data':d}})
 for route in project['routes']:
  d=copy.deepcopy(route)
  for step in d['steps']:
   if step['operation_ref']['id']=='p201.b60.station':step['operation_ref']['version']='1.0.6'
  commands.append({'kind':'put_object','data':{'kind':'route','data':d}})
 for connection in project['connections']:
  d=copy.deepcopy(connection)
  for end in ['source','destination']:
   endpoint=d['data'][end]
   if endpoint['kind']=='port' and endpoint['data']['module_ref']['id']=='p201.b60.station':endpoint['data']['module_ref']['version']='1.0.6'
  commands.append({'kind':'put_object','data':{'kind':'connection','data':d}})
 scenario['title']+=' · 物品可视布局';commands.append({'kind':'put_object','data':{'kind':'scenario','data':scenario}})
 write(bundle/'geometry-edit.json',{'base_revision':project['revision'],'commands':commands,'new_modules':fresh,
  'display_only':'Source/sink six declared floor-supported slots; one exact named workpiece anchor per capacity-one device. Business capacity and inventory remain unchanged.'})
 return dest
