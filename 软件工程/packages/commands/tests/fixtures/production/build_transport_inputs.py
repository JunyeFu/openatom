"""Independent DTO mapping of frozen source designs; never reads producer runtime outputs."""
import copy,json
from pathlib import Path
BASE=Path(__file__).parent
load=lambda n:json.loads((BASE/n).read_text(encoding='utf-8'))
clone=copy.deepcopy
q=lambda n,u='piece':{'value':{'coefficient':n,'decimal_places':0},'unit':u}
ref=lambda s:{'id':s,'revision':'r1'}
mref=lambda s:{'id':s,'version':'0.1.0'}
duration=lambda n:{'value':n,'unit':'millisecond','basis':'user_provided','source_ref':None}
def save(name,d):
 d['project']['module_refs']=[mref(x['id']) for x in d['definitions']]
 d['scenario']['scope']=[]
 d['project']['scenario_refs']=[d['scenario']['id']]
 d['input_ref'].update(scenario_id=d['scenario']['id'])
 (BASE/(name+'.json')).write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def transport(d,item,source,dest,target,batch,times,end,carrier='T1',unit='piece',partial=False,windows=None):
 name='w5.transport.'+item
 ports=[{'id':p,'direction':direction,'payload':{'kind':'material','data':{'item_role':'cargo','unit':unit,'quantity_rule':None}}} for p,direction in [('source','input'),('destination','output')]]
 definition=clone(d['definitions'][0]);definition.update(id=name,title=name,parameters=[])
 definition['payload']={'category':'transport','data':{'ports':ports,'builtin':'transport','batch_capacity':q(batch,unit),'resources':[{'role':'carrier','quantity':q(1,'person' if carrier=='worker-transport' else 'vehicle'),'qualifications':[]}],'times':{k:duration(v) for k,v in zip(['loading','outbound','unloading','returning'],times)},'acceptance_conditions':[{'kind':'capacity','target_role':None,'required_value':None},{'kind':'quality_released','target_role':None,'required_value':None}],'identity_policy':'preserve_cargo_ids','presentation':None}}
 d['definitions'].append(definition)
 rd=clone(next(x for x in d['definitions'] if x['payload']['category']=='resources'))
 rd.update(id='w5.resource.vehicle',title='explicit actual cart capacity');rd['payload']['data'].update(kind='worker' if carrier=='worker-transport' else 'vehicle',role='carrier',capacity=q(1,'person' if carrier=='worker-transport' else 'vehicle'),default_calendar_ms=[{'start_ms':0,'end_ms':end}]);d['definitions'].append(rd)
 r=clone(d['project']['resources'][0]);r.update(id=carrier,title=carrier,definition_ref=mref(rd['id']),kind='worker' if carrier=='worker-transport' else 'vehicle',role='carrier',capacity=q(1,'person' if carrier=='worker-transport' else 'vehicle'),calendar_id='ALL',home_location=source)
 d['project']['resources'] = d['project']['resources']+[r] if d['project']['devices'] else [r]
 d['scenario']['supplies']=[{'id':'SUPPLY','source':source,'destination':dest,'item_type_ref':ref(item),'target_quantity':q(target,unit),'window':{'start_ms':0,'end_ms':end},'transport_ref':mref(name),'resources':{'carrier':carrier},'departure_windows':windows if windows is not None else [{'start_ms':0,'end_ms':end}],'allow_partial_load':partial}]
def initial(template,id,item,loc,n=1):
 x=clone(template);x.update(id=id,item_type_ref=ref(item),quantity=q(n),holder={'kind':'location','id':loc},quality='released',components=[],contents=[],source='user_provided');return x
# Original two-hall A/B: exactly one PCB changes, no timing or resource changes.
for case in ['A','B']:
 d=load('local-assembly-pack.json');p=d['project'];s=d['scenario'];s.update(id='HALLS-'+case,title='Original two-hall '+case)
 a=clone(p['halls'][0]);b=clone(a);a.update(title='Assembly hall',input_locations=['shells-A','pcbs-A'],output_locations=['assemblies-A']);b.update(id='H-B',title='Packing hall',input_locations=['assemblies-B','cartons-B'],output_locations=['packages-B']);p['halls']=[a,b]
 loc=clone(next(l for l in p['locations'] if l['id']=='assemblies-A'));loc.update(id='assemblies-B',title='assemblies-B',hall_id='H-B');p['locations'].append(loc)
 for l in p['locations']:
  if l['id'] in ['assemblies-B','cartons-B','packages-B']:l['hall_id']='H-B'
 for dev in p['devices']:
  if dev['id']=='PACK-01':dev['hall_id']='H-B'
 for c in p['connections']:
  data=c['data']
  if data['destination']['kind']=='port' and data['destination']['data']['instance_id']=='PACK-01' and data['destination']['data']['port_id']=='product':
   data['source']={'kind':'location','data':'assemblies-B'};data['destination_store']='assemblies-B'
 if case=='B':s['initial_conditions']['items'].append(initial(s['initial_conditions']['items'][0],'pcb-002','pcb','pcbs-A'))
 transport(d,'assembly','assemblies-A','assemblies-B',2,1,[10000,30000,10000,30000],240000,carrier='worker-transport')
 d['source_mapping']={'source':'scenarios/两厂房装配包装/project.design.json + variants.design.json','unit':'original seconds explicitly multiplied by 1000; two halls retained','actual_result':None}
 save('p202-halls-'+case,d)
# Pure actual cargo tests: no fabricated manufacture or OrderLine. Product route/BOM are retained references, not executed tasks.
for name,stock,target,batch,partial in [('short-stock',150,200,200,True),('whole-batch-short',150,200,200,False),('identity',2,2,1,False),('blocked',2,2,1,False),('departure-missed',1,1,1,False)]:
 d=load('local-assembly-pack.json');p=d['project'];s=d['scenario'];s.update(id='CARGO-'+name,title='Independent explicit cargo '+name)
 p.update(devices=[],lines=[],connections=[],setup_policies=[],products=[],boms=[],routes=[])
 p['locations']=[clone(p['locations'][0]),clone(p['locations'][1])]
 for loc,id in zip(p['locations'],['cargo-source','cargo-destination']):loc.update(id=id,title=id,hall_id='H-A',accepted_item_types=['assembly'],capacity=q(200),purpose='input' if id.endswith('source') else 'output')
 if name=='blocked':p['locations'][1]['capacity']=q(0)
 p['halls'][0].update(input_locations=['cargo-source'],output_locations=['cargo-destination'])
 s['initial_conditions']['items']=[initial(s['initial_conditions']['items'][0],'cargo-'+str(i),'assembly','cargo-source') for i in range(stock)]
 s.update(orders=[],production_tasks=[],supplies=[],arrivals=[])
 windows=[{'start_ms':0,'end_ms':1}] if name=='departure-missed' else None
 if name=='departure-missed':
  incoming=s['initial_conditions']['items'];s['initial_conditions']['items']=[]
  s['arrivals']=[{'id':'LATE-CARGO','at_ms':1000,'destination':'cargo-source','items':incoming,'release_policy':'unchecked_allowed'}]
 transport(d,'assembly','cargo-source','cargo-destination',target,batch,[10000,30000,10000,30000],240000,partial=partial,windows=windows)
 save('p202-'+name,d)
# Full original IX-01: CNC 10/45/5, six real blanks, shared assembly/rework person,
# three initial PCB + declared 240s/420s arrival, six carton containers, third inspection FAIL/PASS.
for variant,arrival in [('baseline',240000),('delayed-pcb',420000)]:
 d=load('p1-parallel-A.json')
 mapping={'H-1':'H1','M-01':'CNC01','ASM-01':'Assembly01','QC-01':'Inspection01','RW-01':'Rework01','PACK-01':'Packing01','CAP-M-01':'CAP-CNC01','CAP-ASM-01':'CAP-Assembly01','CAP-QC-01':'CAP-Inspection01','CAP-RW-01':'CAP-Rework01','CAP-PACK-01':'CAP-Packing01','W-Q':'W-B','W-P':'W-B','W-R':'W-A','raw_shell':'blank','raw-M':'receiving','raw-P':'pcb-store','pcb-ready':'pcb-store','shell-ready':'cnc-out','inspect-in':'inspect-in','passed-in':'passed-in','cartons':'carton-store'}
 def rewrite(x):
  if isinstance(x,dict):return {mapping.get(k,k):rewrite(v) for k,v in x.items()}
  if isinstance(x,list):return [rewrite(v) for v in x]
  return mapping.get(x,x) if isinstance(x,str) else x
 d=rewrite(d);p=d['project'];s=d['scenario'];s.update(id='IX01-'+variant,title='Original full IX-01 '+variant,window={'start_ms':0,'end_ms':600000})
 p['devices']=[x for x in p['devices'] if x['id']!='P-01']
 p['resources']=[x for x in p['resources'] if x['id']!='CAP-P-01'];p['resources']=list({x['id']:x for x in p['resources']}.values())
 p['locations']=[x for x in p['locations'] if x['id'] not in ['raw-P']];p['locations']=list({x['id']:x for x in p['locations']}.values())
 p['connections']=[x for x in p['connections'] if not any(e.get('kind')=='port' and e['data']['instance_id']=='P-01' for e in [x['data']['source'],x['data']['destination']])]
 shell_buffer=clone(next(x for x in p['locations'] if x['id']=='cnc-out'));shell_buffer.update(id='shell-buffer',title='shell-buffer',capacity=q(2));p['locations'].append(shell_buffer)
 f=clone(p['item_types'][0]);f.update(id='fastener',title='fastener');p['item_types'].append(f)
 fs=clone(p['locations'][0]);fs.update(id='fastener-store',title='fastener-store',accepted_item_types=['fastener'],capacity=q(24));p['locations'].append(fs)
 for l in p['locations']:
  if l['id']!='shell-buffer':l['capacity']=q(24 if l['id']=='fastener-store' else 6)
 for hall in p['halls']:hall.update(width_m=24,depth_m=16,height_m=None,input_locations=['receiving','pcb-store','fastener-store','carton-store'],output_locations=['finished'])
 for cal in p['calendars']:cal['intervals']=[{'start_ms':0,'end_ms':600000}]
 for line in p['lines']:line['members']=[x for x in line['members'] if x!='P-01']
 wc=clone(next(x for x in p['resources'] if x['id']=='W-A'));wc.update(id='W-C',title='W-C',home_location='receiving');p['resources'].append(wc)
 for dev in p['devices']:
  if dev['id']=='CNC01':dev['role_bindings']['resources'].update(worker='W-C',machine='CAP-CNC01')
  if dev['id']=='Assembly01':dev['role_bindings']['item_types']['fastener']='fastener'
 for definition in d['definitions']:
  data=definition['payload']['data']
  if definition['id']=='w5.op.shell':
   data.update(duration=None,resource_roles=[{'role':'machine','quantity':q(1,'slot'),'qualifications':[]}],hold_on_output_wait=[])
   data['phases']=[{'id':name,'duration':duration(ms),'phase_resources':[{'role':'worker','quantity':q(1,'person'),'qualifications':[]}] if name!='auto' else [],'interruption_policy':'non_preemptive','work_kind':kind} for name,ms,kind in [('load',10000,'loading'),('auto',45000,'processing'),('unload',5000,'unloading')]]
  if definition['id']=='w5.op.assembly':
   data['duration']=duration(40000);data['item_roles'].append('fastener');material=clone(data['inputs'][0]);material.update(role='fastener',port='fastener',quantity=q(4));data['inputs'].append(material)
  if definition['id']=='w5.op.pack':data['duration']=duration(20000)
 # Explicit fastener material port and physical-store connection, not a BOM-only supply.
 station=next(x for x in d['definitions'] if x['id']==next(x for x in p['devices'] if x['id']=='Assembly01')['module_ref']['id'])
 port=clone(station['payload']['data']['ports'][0]);port.update(id='fastener');port['payload']['data']['item_role']='fastener';station['payload']['data']['ports'].append(port)
 conn=clone(p['connections'][0]);conn['data'].update(id='fastener-input',source={'kind':'location','data':'fastener-store'},destination={'kind':'port','data':{'module_ref':mref(station['id']),'instance_id':'Assembly01','port_id':'fastener'}},item_type='fastener',destination_store='fastener-store');p['connections'].append(conn)
 for c in p['connections']:
  a=c['data']
  if a['destination']['kind']=='port' and a['destination']['data']['instance_id']=='Assembly01' and a['destination']['data']['port_id']=='shell':a.update(source={'kind':'location','data':'shell-buffer'},destination_store='shell-buffer')
 route=p['routes'][0];route['steps']=[x for x in route['steps'] if x['device_id']!='P-01']
 for step in route['steps']:
  step['predecessors']=[x for x in step['predecessors'] if x!='step-pcb']
  if step['device_id']=='CNC01':step['resource_bindings'].update(worker='W-C',machine='CAP-CNC01')
  if step['device_id']=='Assembly01':step['item_bindings']['fastener']='fastener'
 p['boms'][0]['rows']=[r for r in p['boms'][0]['rows'] if r['consume_at_operation']!='step-pcb']
 p['boms'][0]['rows'].append({'item_type_ref':ref('fastener'),'quantity':q(4),'consume_at_operation':next(x['id'] for x in route['steps'] if x['device_id']=='Assembly01')})
 template=s['initial_conditions']['items'][0];s['initial_conditions']['items']=[]
 for item,loc,count,prefix in [('blank','receiving',6,'blank'),('pcb','pcb-store',3,'pcb'),('fastener','fastener-store',24,'fastener'),('carton','carton-store',6,'carton')]:s['initial_conditions']['items'] +=[initial(template,prefix+f'{i:03}',item,loc) for i in range(1,count+1)]
 s['arrivals']=[{'id':'pcb-batch2','at_ms':arrival,'destination':'pcb-store','items':[initial(template,'pcb'+f'{i:03}','pcb','pcb-store') for i in range(4,7)],'release_policy':'unchecked_allowed'}]
 s['orders'][0]['quantity']=q(6);s['orders'][0]['due_at_ms']=600000
 s['production_tasks'][0].update(planned_quantity=q(6),order_allocations=[{'order_id':s['orders'][0]['id'],'quantity':q(6)}])
 preset=clone(s['quality_presets'][0]);s['quality_presets']=[]
 for i in range(1,7):
  a=clone(preset);a.update(id='QC-'+str(i),item_or_ancestor='blank'+f'{i:03}',results=[{'kind':'string','value':x} for x in (['FAIL','PASS'] if i==3 else ['PASS'])]);s['quality_presets'].append(a)
 transport(d,'shell','cnc-out','shell-buffer',6,1,[0,10000,0,5000],600000)
 d['source_mapping']={'source':'scenarios/从零搭建控制盒厂房/project.design.json + variants.design.json','cnc_out':'explicit station output receiving capacity 6; Supply source requires physical location','expected_source':'expected-timeline.design.json','actual_result':None}
 save('p202-ix01-'+variant,d)
