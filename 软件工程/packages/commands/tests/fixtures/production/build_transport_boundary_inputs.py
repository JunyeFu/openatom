"""Consume actual explicit phase-resource fields, never special-case role names."""
import json,copy
from pathlib import Path
BASE=Path(__file__).parent
clone=copy.deepcopy
load=lambda n:json.loads((BASE/(n+'.json')).read_text(encoding='utf-8'))
q=lambda n,u='piece':{'value':{'coefficient':n,'decimal_places':0},'unit':u}
dur=lambda n:{'value':n,'unit':'millisecond','basis':'user_provided','source_ref':None}
def save(name,d):
 d['scenario']['scope']=[];d['project']['scenario_refs']=[d['scenario']['id']];d['input_ref']['scenario_id']=d['scenario']['id']
 (BASE/(name+'.json')).write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def phase_person(d,start,end):
 d['project']['calendars'].append({'id':'DOCK','title':'explicit actual dock worker calendar','source':'user_provided','intervals':[{'start_ms':start,'end_ms':end}]})
 worker=clone(load('local-assembly-pack')['project']['resources'][0]);worker.update(id='W-DOCK',title='W-DOCK',calendar_id='DOCK',home_location=d['scenario']['supplies'][0]['destination'],definition_ref={'id':'fixture.worker','version':'0.1.0'},kind='worker',role='worker',capacity=q(1,'person'))
 d['project']['resources'].append(worker);d['scenario']['supplies'][0]['resources']['dock_hand']='W-DOCK'
 tr=next(x for x in d['definitions'] if x['payload']['category']=='transport')['payload']['data'];tr['unloading_resources']=[{'role':'dock_hand','quantity':q(1,'person'),'qualifications':[]}]
 return tr
# Frozen B14 values, one pre-existing source cargo, actual unloader calendar.
d=load('p202-identity');d['scenario'].update(id='B14',window={'start_ms':0,'end_ms':200000});d['scenario']['initial_conditions']['items']=d['scenario']['initial_conditions']['items'][:1]
next(loc for loc in d['project']['locations'] if loc['id']=='cargo-destination')['capacity']=q(1)
tr=phase_person(d,120000,150000);tr['times']={k:dur(v) for k,v in zip(['loading','outbound','unloading','returning'],[0,10000,5000,10000])}
sup=d['scenario']['supplies'][0];sup.update(target_quantity=q(1),departure_windows=[{'start_ms':100000,'end_ms':100001}],window={'start_ms':0,'end_ms':200000})
d['project']['calendars'][0]['intervals']=[{'start_ms':0,'end_ms':200000}];d['source_mapping']={'source':'business-rule-boundaries.design.json#B14','actual_result':None};save('p202-B14',d)
# Explicit generic role dock_hand in both phase arrays, same actual worker identity.
d=load('p202-identity');d['scenario']['id']='PHASE-PEOPLE';tr=phase_person(d,0,240000);tr['loading_resources']=clone(tr['unloading_resources']);d['source_mapping']={'source':'self-built ALG-04/09 variant of original transport','role_rule':'no role-name dispatch; exact loading_resources/unloading_resources fields','actual_result':None};save('p202-phase-people',d)
# Frozen B13 transport/Order timing. Two legal source packages are original A task outputs
# before departure; order allocation is preserved, never attached by a guessed item name.
d=load('p104-A-first');s=d['scenario'];s.update(id='B13-ORDER',window={'start_ms':0,'end_ms':172800000});s['production_tasks']=[t for t in s['production_tasks'] if t['id']=='TASK-A'];s['orders']=[o for o in s['orders'] if o['id']=='A'];s['orders'][0].update(id='O-day1',due_at_ms=86425000);s['production_tasks'][0]['order_allocations'][0]['order_id']='O-day1'
for c in d['project']['calendars']:c['intervals']=[{'start_ms':0,'end_ms':172800000}]
tr=phase_person(d,86420000,172800000);tr['times']={k:dur(v) for k,v in zip(['loading','outbound','unloading','returning'],[0,20000,5000,10000])}
s['supplies'][0].update(target_quantity=q(2),window={'start_ms':0,'end_ms':172800000},departure_windows=[{'start_ms':86390000,'end_ms':86390001}])
d['source_mapping']={'source':'business-rule-boundaries.design.json#B13 + actual original A source production','supply':'exact two A packages physically ready before the frozen departure, real TASK-A/O-day1 quota','expected':'B13 arrival86410000/receive+accept86425000/return86435000, day0=0/day1=2, lateness0','actual_result':None};save('p202-B13',d)
# B16 real order and real two-package producer. Explicit zero durations make the source
# stock available at t=0 through the existing material rules, never an injected RuntimeItem.
d=load('p104-A-first');s=d['scenario'];s.update(id='B16-ORDER',window={'start_ms':0,'end_ms':30000});s['orders']=[o for o in s['orders'] if o['id']=='A'];s['production_tasks']=[t for t in s['production_tasks'] if t['id']=='TASK-A'];s['arrivals']=[]
for c in d['project']['calendars']:c['intervals']=[{'start_ms':0,'end_ms':30000}]
for op in d['definitions']:
 if op['id'] in ['w5.op.assembly-A','w5.op.inspect','w5.op.pack']:op['payload']['data']['duration']=dur(0)
for loc in d['project']['locations']:
 if loc['id']=='customer':loc['capacity']=q(1)
tr=next(x for x in d['definitions'] if x['payload']['category']=='transport')['payload']['data'];tr.update(batch_capacity=q(1),times={k:dur(v) for k,v in zip(['loading','outbound','unloading','returning'],[0,10000,0,0])})
s['supplies'][0].update(target_quantity=q(2),window={'start_ms':0,'end_ms':30000},departure_windows=[{'start_ms':0,'end_ms':30000}])
d['source_mapping']={'source':'business-rule-boundaries.design.json#B16 + actual source-task provenance','source_rule':'explicit three zero durations lower two source products through real existing rules at t=0; no injected runtime records','actual_result':None};save('p202-B16',d)
