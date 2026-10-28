"""Create new owned illustrations and actual typed fixtures; never rewrite design inputs."""
import copy
import json
from pathlib import Path

root = Path.cwd()
out = root / 'scenarios/图片时间轴/runtime'
bundle = root / 'modules/devices/manual-assembly/presentation/p303-owned'
(bundle / 'assets').mkdir(parents=True, exist_ok=True)
out.mkdir(parents=True, exist_ok=True)
license_text = '''MIT License

Copyright (c) 2026 Super Factory contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
'''
(bundle / 'LICENSE.txt').write_text(license_text, encoding='utf-8')
names = ['align', 'lower', 'hold', 'return']
for i, name in enumerate(names):
    head_y = [80, 119, 148, 90][i]
    # Original simple vector drawings authored for this task; no external image.
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="800" height="450" viewBox="0 0 800 450">
<rect width="800" height="450" fill="#edf3f7"/><rect x="55" y="35" width="690" height="365" rx="6" fill="#f7fafc" stroke="#95b0c3"/>
<path d="M170 300h460v24H170zM202 324h25v58h-25zM575 324h25v58h-25z" fill="#557080"/>
<path d="M340 65h120v20H340zM352 85h20v162h-20zM428 85h20v162h-20z" fill="#7894a7"/>
<rect x="313" y="{head_y}" width="174" height="29" rx="3" fill="#31749d"/><path d="M383 {head_y+29}h34v58h-34z" fill="#bdcbd4" stroke="#738a99"/>
<path d="M310 266h180v34H310z" fill="#344d60"/><rect x="333" y="245" width="134" height="21" fill="#679e8b"/><path d="M342 252h112M355 246v18M430 246v18" stroke="#e1c16a" stroke-width="3"/>
<rect x="85" y="209" width="112" height="68" fill="#d7e4ea" stroke="#93acbc"/><text x="98" y="249" font-family="sans-serif" font-size="16" fill="#34566a">Shell / PCB</text>
<rect x="605" y="209" width="112" height="68" fill="{'#91b9a3' if i==3 else '#d7e4ea'}" stroke="#93acbc"/><text x="622" y="249" font-family="sans-serif" font-size="16" fill="#34566a">Output</text>
<text x="77" y="68" font-family="sans-serif" font-size="21" fill="#24475d">{i+1}. {name.upper()}</text><text x="77" y="427" font-family="sans-serif" font-size="15" fill="#617d8e">P303 original illustration - assumed geometry - visual only - MIT</text>
</svg>'''
    (bundle / 'assets' / f'{name}.svg').write_text(svg, encoding='utf-8')
assets = [{'id': 'p303.'+n, 'path': f'assets/{n}.svg', 'kind': 'svg'} for n in names]
rights = [{'asset_ref': a['id'], 'source': 'Original vector illustration authored for P303; no external image', 'license': 'MIT', 'redistribution_confirmed': True, 'missing_reason': None} for a in assets]
template = json.loads((root/'modules/devices/template-support/media/module.json').read_text(encoding='utf-8'))['data']['definition']['payload']['data']
presentation = copy.deepcopy(template)
presentation.update(kind='image_sequence', source={'kind':'asset','data':assets[0]}, nodes=[], poses=[], actions=[], timeline=[], tags=[], anchor_roles={}, port_anchors={}, production_effect='none')
presentation['lod'] = {**copy.deepcopy(template['lod']), 'detailed':None, 'simplified':None}
start = 0
for name, weight, asset in zip(names,[.2,.35,.3,.15],assets):
    end = min(1, round(start+weight, 10))
    presentation['timeline'].append({'id':'frame-'+chr(97+names.index(name)), 'phase_ref':None, 'start_progress':start, 'end_progress':end, 'source':{'kind':'image_asset','data':asset}, 'transition':'hold'})
    start = end
presentation['tags']=[{'id':'illustration-note','label':'示例说明','anchor':{'kind':'image_normalized','data':{'x':.04,'y':.92}},'progress_range':None,'payload':{'kind':'annotation','data':'原创示意图；外观不决定工时或产量'}}]
fixture = json.loads((root/'packages/commands/tests/fixtures/production/p1-phase-calendar.json').read_text(encoding='utf-8'))
project, scenario = fixture['project'], fixture['scenario']
operation = next(copy.deepcopy(d['payload']['data']) for d in fixture['definitions'] if d['payload']['category']=='operations')
operation.update(builtin='assemble',item_roles=['raw','pcb','product'],duration={'value':60000,'unit':'millisecond','basis':'user_provided','source_ref':None},phases=[],identity_policy='assembly_with_composition')
operation['inputs'].append({**copy.deepcopy(operation['inputs'][0]),'role':'pcb','port':'pcb'})
operation['resource_roles']=[{'role':'worker','quantity':{'value':{'coefficient':1,'decimal_places':0},'unit':'person'},'qualifications':[]}]
definition=next(copy.deepcopy(d) for d in fixture['definitions'] if d['payload']['category']=='devices')
definition.update(id='p303.owned.manual-assembly',version='0.1.0',title='四图装配台（明确演示输入）',definition_state='design',contribution={'author':'Super Factory contributors; AI-assisted original SVG','license':'MIT','assets':rights},parameters=[])
definition['payload']['data'].update(presentation=presentation,behavior={'selected':'inline','inline':operation,'operation_ref':None},operations=['configure','inspect','preview_cycle'])
definition['payload']['data']['ports'].append({'id':'pcb','direction':'input','payload':{'kind':'material','data':{'item_role':'pcb','unit':'piece','quantity_rule':None}}})
package={'definition':definition,'assets':assets,'files':[{'path':a['path'],'kind':'asset'} for a in assets]+[{'path':'LICENSE.txt','kind':'asset'}],'dependencies':[],'rights':rights,'issues':[]}
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
write(bundle/'module.json',{'schema_version':'1.0','data':package})
device=project['devices'][0];old_ref=device['module_ref'];new_ref={'id':definition['id'],'version':definition['version']}
project['module_refs']=[r for r in project['module_refs'] if r!=old_ref]+[new_ref]
device.update(module_ref=new_ref,operation_binding=None,route_operation_refs=[new_ref],title='P303 自有四图装配台')
device['role_bindings']['item_types']['pcb']='pcb'
route=project['routes'][0]['steps'][0];route['operation_ref']=new_ref;route['item_bindings']['pcb']='pcb'
pcb=copy.deepcopy(project['item_types'][0]);pcb.update(id='pcb',title='PCB');project['item_types'].append(pcb);project['item_types'][0]['title']='壳体'
for connection in project['connections']:
    if connection.get('kind')=='material' and connection['data']['destination'].get('kind')=='port' and connection['data']['destination']['data']['instance_id']==device['id']:
        pcb_connection=copy.deepcopy(connection);pcb_connection['data']['id']+='-pcb';pcb_connection['data']['item_type']='pcb';pcb_connection['data']['destination']['data']['module_ref']=new_ref;pcb_connection['data']['destination']['data']['port_id']='pcb';project['connections'].append(pcb_connection);connection['data']['destination']['data']['module_ref']=new_ref;break
project['locations'][0]['capacity']['value']['coefficient']=3;project['locations'][0]['accepted_item_types'].append('pcb');project['locations'][1]['capacity']['value']['coefficient']=2
row=copy.deepcopy(project['boms'][0]['rows'][0]);row['item_type_ref']['id']='pcb';project['boms'][0]['rows'].append(row)
project['calendars'][0]['intervals'][0]['end_ms']=120000;project['resources'][1]['calendar_id']='ALL'
initial=scenario['initial_conditions']['items'][0];initial['id']='shell-1';other=copy.deepcopy(initial);other['id']='shell-2';pcb=copy.deepcopy(initial);pcb.update(id='pcb-1',item_type_ref={'id':'pcb','revision':'r1'});scenario['initial_conditions']['items']=[initial,other,pcb]
scenario['production_tasks'][0]['planned_quantity']['value']['coefficient']=2;scenario.update(id='P303-FIXED',title='壳体2/PCB1明确输入',window={'start_ms':0,'end_ms':120000});project['scenario_refs']=[scenario['id']];project['title']='P303 actual fixed-cycle consumer'
goal=copy.deepcopy(json.loads((root/'packages/commands/tests/fixtures/quality/quality-original.json').read_text(encoding='utf-8'))['scenario']['acceptance_goal'])
goal.update(industry='self-built P303 example',product_family='assembly',product_specification_ref={'id':'PART','revision':'r1'},accepted_quantity_per_day={'value':{'coefficient':2,'decimal_places':0},'unit':'piece'},production_calendar_ref='ALL',observation_window=scenario['window'],existing_equipment=[{'kind':'device','id':device['id']}])
goal['acceptance_scope'].update(product_ref={'id':'PART','revision':'r1'},destination='finished',stage='factory_released',unit='piece');scenario['acceptance_goal']=goal
fixture['definitions']=[d for d in fixture['definitions'] if d['id']!=old_ref['id']]+[definition]
def remap_refs(node):
    if isinstance(node, dict):
        if node == old_ref:
            node.update(new_ref)
        for child in node.values():
            remap_refs(child)
    elif isinstance(node, list):
        for child in node:
            remap_refs(child)
remap_refs(project)
write(out/'fixed.input.json',fixture)
write(out/'expectations.json',{'source':'Original frozen authoring design inputs, fixed before product runs','initial':{'shell':2,'pcb':1},'fixed_duration_ms':60000,'changed_duration_ms':30000,'completed':1,'remaining':{'shell':1,'pcb':0},'next_cycle_wait':'pcb','original_weights':[.2,.35,.3,.15],'reordered_ids':['frame-b','frame-a','frame-c','frame-d'],'new_Engine_result_from_watch':False,'phase_expected':{'load':[0,10000],'auto':[10000,55000],'unload':[70000,75000],'resource_wait':[55000,70000],'output_wait':[75000,85000],'effective_ms':60000,'work_finished_ms':75000,'received_ms':85000,'machine_held_ms':85000}})
print('Original four SVG assets / exact typed package / fixed2shell1PCB fixture created within owned paths')

# Original phase acceptance: load0-10 / auto10-55 / wait55-70 /
# unload70-75 / output blocked75-85; every wait comes from real jobs/material.
phase=copy.deepcopy(fixture);pr,sc=phase['project'],phase['scenario'];main=phase['definitions'][-1];main['version']='0.2.0';main['title']='P303 explicit phased/output-hold example';ref={'id':main['id'],'version':'0.2.0'}
for node in [pr['devices'][0]['module_ref'],pr['devices'][0]['route_operation_refs'][0],pr['routes'][0]['steps'][0]['operation_ref']]:node.update(ref)
for node in pr['module_refs']:
    if node['id']==ref['id']:node.update(ref)
for connection in pr['connections']:
    for side in ['source','destination']:
        target=connection.get('data',{}).get(side,{})
        if target.get('kind')=='port' and target['data']['module_ref']['id']==ref['id']:target['data']['module_ref'].update(ref)
op=main['payload']['data']['behavior']['inline'];base_op=copy.deepcopy(next(d['payload']['data'] for d in json.loads((root/'packages/commands/tests/fixtures/production/p1-phase-busy.json').read_text())['definitions'] if d['payload']['category']=='operations' and d['id']=='w5.op.phased'))
op.update(duration=None,phases=base_op['phases'],resource_roles=base_op['resource_roles'],hold_on_output_wait=[])
for p,v in zip(op['phases'],[10000,45000,5000]):p['duration']['value']=v
for clip,phase_id,a,b in zip(main['payload']['data']['presentation']['timeline'],['load','auto','auto','unload'],[0,0,.5,0],[1,.5,1,1]):clip.update(phase_ref=phase_id,start_progress=a,end_progress=b)
sc.update(id='P303-PHASE',title='Explicit actual worker50-70 and output75-85 waits',scope=[]);pr['scenario_refs']=[sc['id']];sc['production_tasks'][0]['planned_quantity']['value']['coefficient']=1;sc['acceptance_goal']['accepted_quantity_per_day']['value']['coefficient']=1
pr['locations'][1]['capacity']['value']['coefficient']=1
for kind in ['aux_raw','aux_part']:
    t=copy.deepcopy(pr['item_types'][0]);t.update(id=kind,title=kind);pr['item_types'].append(t)
for name,item,capacity in [('block-raw','aux_raw',1),('block-finished','aux_part',2)]:
    loc=copy.deepcopy(pr['locations'][0]);loc.update(id=name,title=name,accepted_item_types=[item]);loc['capacity']['value']['coefficient']=capacity;pr['locations'].append(loc)
old=copy.deepcopy(sc['initial_conditions']['items'][0]);old.update(id='old-output-block',item_type_ref={'id':'part','revision':'r1'},holder={'kind':'location','id':'finished'});sc['initial_conditions']['items'].append(old)
aux=copy.deepcopy(old);aux.update(id='block-material',item_type_ref={'id':'aux_raw','revision':'r1'},holder={'kind':'location','id':'block-raw'});sc['initial_conditions']['items'].append(aux)
prototype=next(copy.deepcopy(d) for d in json.loads((root/'packages/commands/tests/fixtures/production/p1-phase-busy.json').read_text())['definitions'] if d['id']=='w5.op.blocker')
for name,release,duration,input_type,input_location,worker in [('BLOCKER',50000,20000,'aux_raw','block-raw',True),('CLEARER',85000,1000,'part','finished',False)]:
    cap=copy.deepcopy(pr['resources'][0]);cap.update(id='CAP-'+name,title='CAP-'+name);pr['resources'].append(cap)
    rule=copy.deepcopy(prototype);rule.update(id='p303.op.'+name,title='Explicit '+name);rule['payload']['data']['duration']['value']=duration
    if not worker:rule['payload']['data']['resource_roles']=[r for r in rule['payload']['data']['resource_roles'] if r['role']!='worker']
    station=copy.deepcopy(main);station.update(id='p303.station.'+name,title='Explicit '+name);station['payload']['data'].update(behavior={'selected':'operation_ref','inline':None,'operation_ref':{'id':rule['id'],'version':rule['version']}},presentation=None,operations=[]);station['contribution']={'author':None,'license':None,'assets':[]};station['payload']['data']['ports']=[p for p in station['payload']['data']['ports'] if p['id']!='pcb']
    for d in [rule,station]:phase['definitions'].append(d);pr['module_refs'].append({'id':d['id'],'version':d['version']})
    dev=copy.deepcopy(pr['devices'][0]);dev.update(id=name,title=name,module_ref={'id':station['id'],'version':station['version']},operation_binding={'id':rule['id'],'version':rule['version']},route_operation_refs=[]);dev['role_bindings']['item_types']={'raw':input_type,'product':'aux_part'};dev['role_bindings']['resources'].update(execution_capacity='CAP-'+name,machine='CAP-'+name);pr['devices'].append(dev)
    route={'id':name+'-ROUTE','revision':'r1','steps':[{'id':name+'-STEP','operation_ref':{'id':rule['id'],'version':rule['version']},'device_id':name,'predecessors':[],'item_bindings':{'raw':input_type,'product':'aux_part'},'resource_bindings':{'worker':'W-C'}}]};pr['routes'].append(route)
    pr['boms'].append({'id':name+'-BOM','revision':'r1','rows':[{'item_type_ref':{'id':input_type,'revision':'r1'},'quantity':{'value':{'coefficient':1,'decimal_places':0},'unit':'piece'},'consume_at_operation':name+'-STEP'}]});pr['products'].append({'id':name+'-PRODUCT','revision':'r1','item_type_ref':{'id':'aux_part','revision':'r1'},'bom_ref':{'id':name+'-BOM','revision':'r1'},'routing_ref':{'id':route['id'],'revision':'r1'},'artifact_refs':[]});sc['production_tasks'].append({'id':name+'-TASK','routing_ref':{'id':route['id'],'revision':'r1'},'order_allocations':[],'planned_quantity':{'value':{'coefficient':1,'decimal_places':0},'unit':'piece'},'release_at_ms':release,'priority':0})
    for port,location in [('input',input_location),('output','block-finished')]:
        endpoint={'kind':'port','data':{'instance_id':name,'module_ref':dev['module_ref'],'port_id':port}};pr['connections'].append({'kind':'material','data':{'id':name+'-'+port,'source':{'kind':'location','data':location} if port=='input' else endpoint,'destination':endpoint if port=='input' else {'kind':'location','data':location},'item_type':input_type if port=='input' else 'aux_part','unit':'piece','quantity_rule':None,'capacity_rule':None,'destination_store':location}})
phase['package_file']='phase-module.json';phase_package=copy.deepcopy(package);phase_package['definition']=main;write(bundle/'phase-module.json',{'schema_version':'1.0','data':phase_package});write(out/'phase.input.json',phase)

# A second standard installable package; Root scans module.json only.
import shutil
phase_standard=bundle/'phase'
phase_standard.mkdir(parents=True,exist_ok=True)
for file in phase_package['files']:
    relative=Path(file['path']);(phase_standard/relative).parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(bundle/relative,phase_standard/relative)
write(phase_standard/'module.json',{'schema_version':'1.0','data':phase_package})
