"""Original multi_order DTO mapping, with unresolved family selection explicitly recorded."""
import json,copy
from pathlib import Path
BASE=Path(__file__).parent
clone=copy.deepcopy
q=lambda n,u='piece':{'value':{'coefficient':n,'decimal_places':0},'unit':u}
ref=lambda s:{'id':s,'revision':'r1'}
mref=lambda s:{'id':s,'version':'0.1.0'}
dur=lambda n:{'value':n,'unit':'millisecond','basis':'user_provided','source_ref':None}
for first in ['A','B']:
 d=json.loads((BASE/'p202-ix01-baseline.json').read_text(encoding='utf-8'))
 rename={'Assembly01':'Assembly','Inspection01':'Inspection','Packing01':'Packing','CAP-Assembly01':'CAP-Assembly','CAP-Inspection01':'CAP-Inspection','CAP-Packing01':'CAP-Packing','finished':'dispatch-store'}
 def remap(x):
  if isinstance(x,dict):return {rename.get(k,k):remap(v) for k,v in x.items()}
  if isinstance(x,list):return [remap(v) for v in x]
  return rename.get(x,x) if isinstance(x,str) else x
 d=remap(d);p=d['project'];s=d['scenario'];s.update(id='ORDERS-'+first,title='Original multi-order '+first+' first',window={'start_ms':0,'end_ms':240000})
 p['devices']=[x for x in p['devices'] if x['id'] in ['Assembly','Inspection','Packing']]
 p['resources']=[x for x in p['resources'] if x['id'] in ['CAP-Assembly','CAP-Inspection','CAP-Packing','W-A','W-B','T1']]
 for resource in p['resources']:
  if resource['id']=='T1':resource['home_location']='dispatch-store'
  if resource['id']=='W-A':resource['home_location']='shell-store'
 p['locations']=[x for x in p['locations'] if x['id'] in ['pcb-store','fastener-store','carton-store','inspect-in','passed-in','quarantine','rework-in','dispatch-store']]
 shell=clone(p['locations'][0]);shell.update(id='shell-store',title='shell-store',accepted_item_types=['shell']);p['locations'].append(shell)
 customer=clone(next(x for x in p['locations'] if x['id']=='dispatch-store'));customer.update(id='customer',title='customer');p['locations'].append(customer)
 for l in p['locations']:l['capacity']=q(16 if l['id']=='fastener-store' else 4)
 for cal in p['calendars']:cal['intervals']=[{'start_ms':0,'end_ms':240000}]
 p['lines']=[]
 for dev in p['devices']:dev['line_id']=None
 p['halls'][0].update(input_locations=['shell-store','pcb-store','fastener-store','carton-store'],output_locations=['customer'])
 p['connections']=[x for x in p['connections'] if not any(e['kind']=='port' and e['data']['instance_id'] in ['CNC01','Rework01'] for e in [x['data']['source'],x['data']['destination']])]
 for c in p['connections']:
  a=c['data']
  if a['destination']['kind']=='port' and a['destination']['data']['instance_id']=='Assembly' and a['destination']['data']['port_id']=='shell':a.update(source={'kind':'location','data':'shell-store'},destination_store='shell-store')
 assembly=next(x for x in d['definitions'] if x['id']=='w5.op.assembly');assembly['id']='w5.op.assembly-A';assembly['payload']['data'].update(family='A',duration=dur(30000))
 other=clone(assembly);other['id']='w5.op.assembly-B';other['payload']['data']['family']='B';d['definitions'].append(other)
 for dev in p['devices']:
  if dev['id']=='Assembly':
   dev['operation_binding']=mref('w5.op.assembly-A');dev['route_operation_refs']=[mref('w5.op.assembly-A'),mref('w5.op.assembly-B')]
 for definition in d['definitions']:
  if definition['id']=='w5.station.ASM-01':definition['payload']['data']['behavior']['operation_ref']=mref('w5.op.assembly-A')
  if definition['id']=='w5.op.pack':definition['payload']['data']['duration']=dur(10000)
  if definition['id']=='w5.op.inspect':definition['payload']['data']['quality_and_routes']['max_completed_reworks']=0
 base_route=p['routes'][0];steps=[x for x in base_route['steps'] if x['device_id'] in ['Assembly','Inspection','Packing']]
 base_rows=[x for x in p['boms'][0]['rows'] if x['consume_at_operation'] in [s['id'] for s in steps]]
 p['routes']=[];p['products']=[];p['boms']=[];s['orders']=[];s['production_tasks']=[]
 for family,due in [('A',160000),('B',200000)]:
  route=clone(base_route);route.update(id='ROUTE-'+family,steps=clone(steps))
  for step in route['steps']:
   step['predecessors']=[x for x in step['predecessors'] if x in [st['id'] for st in steps]]
   if step['device_id']=='Assembly':step['operation_ref']=mref('w5.op.assembly-'+family)
  p['routes'].append(route);p['boms'].append({'id':'BOM-'+family,'revision':'r1','rows':clone(base_rows)})
  p['products'].append({'id':family,'revision':'r1','item_type_ref':ref('package'),'bom_ref':ref('BOM-'+family),'routing_ref':ref('ROUTE-'+family),'artifact_refs':[]})
  s['orders'].append({'id':family,'product_ref':ref(family),'quantity':q(2),'release_at_ms':0,'due_at_ms':due,'destination':'customer','acceptance_stage':'unloaded_and_released','allow_partial':False,'priority':0})
  s['production_tasks'].append({'id':'TASK-'+family,'routing_ref':ref('ROUTE-'+family),'order_allocations':[{'order_id':family,'quantity':q(2)}],'planned_quantity':q(2),'release_at_ms':0,'priority':1 if family==first else 0})
 p['setup_policies']=[{'device_id':'Assembly','initial_family':'A','changeovers':[{'from_family':a,'to_family':b,'duration':dur(ms),'resources':[{'role':'worker','quantity':q(1,'person'),'qualifications':[]}],'preparation_mode':'internal'} for a,b,ms in [('A','B',15000),('B','A',25000)]],'first_article':None}]
 sample=clone(s['initial_conditions']['items'][0]);s['initial_conditions']['items']=[]
 for item,loc,n in [('shell','shell-store',4),('pcb','pcb-store',2),('fastener','fastener-store',16),('carton','carton-store',4)]:
  for i in range(1,n+1):x=clone(sample);x.update(id=item+str(i),item_type_ref=ref(item),holder={'kind':'location','id':loc});s['initial_conditions']['items'].append(x)
 s['arrivals'][0].update(id='PCB-100',at_ms=100000,destination='pcb-store')
 s['arrivals'][0]['items']=[]
 for i in [3,4]:x=clone(sample);x.update(id='pcb'+str(i),item_type_ref=ref('pcb'),holder={'kind':'location','id':'pcb-store'});s['arrivals'][0]['items'].append(x)
 preset=clone(s['quality_presets'][0]);s['quality_presets']=[]
 for i in range(1,5):x=clone(preset);x.update(id='QC-'+str(i),item_or_ancestor='shell'+str(i),results=[{'kind':'string','value':'PASS'}]);s['quality_presets'].append(x)
 supply=s['supplies'][0];supply.update(source='dispatch-store',destination='customer',item_type_ref=ref('package'),target_quantity=q(4),transport_ref=mref('w5.transport.package'),window={'start_ms':0,'end_ms':240000},departure_windows=[{'start_ms':t,'end_ms':t+1} for t in [120000,180000]],allow_partial_load=False)
 transport=next(x for x in d['definitions'] if x['payload']['category']=='transport');transport['id']='w5.transport.package';transport['payload']['data'].update(batch_capacity=q(2),times={k:dur(v) for k,v in zip(['loading','outbound','unloading','returning'],[0,20000,5000,20000])})
 s['scope']=[];s['initial_conditions']['initial_setup_families']={'Assembly':'A'}
 p['scenario_refs']=[s['id']];p['module_refs']=[mref(x['id']) for x in d['definitions']];d['input_ref']['scenario_id']=s['id']
 d['source_mapping']={'source':'scenarios/真实业务设计/business-inputs.design.json#multi_order','expected':'expected-business.design.json#multi_a_first/multi_b_first','priorities':'original dispatch order explicitly lowers to TASK family priority, inherited by all stages/setup','family_selection':'actual Device.route_operation_refs explicitly selects exact A/B operation refs on the one physical Assembly; default operation_binding A retained','actual_result':None}
 (BASE/('p104-'+first+'-first.json')).write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
