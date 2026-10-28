"""Connect actual upstream cases to shared manual pallet packing and real wrapping."""
import argparse
import copy
import math
from pathlib import Path
from prepare_full_factory import ROOT, DESIGN, read, write
from connect_full_factory_routes import quantity, pose
from connect_factory_families import replace


def prepare(source, media_source, output):
    assert not output.exists()
    output.mkdir(parents=True)
    p = read(source/'actual-project.json'); s = copy.deepcopy(read(source/'actual-scenario.json'))
    spec = read(DESIGN/'pallet-line-spec.json'); media = read(media_source/'media-plan.json')
    devices = {x['id']: copy.deepcopy(x) for x in p['devices']}
    locations = {x['id']: copy.deepcopy(x) for x in p['locations']}
    types = {x['id']: copy.deepcopy(x) for x in p['item_types']}
    connections = {x['data']['id']: copy.deepcopy(x) for x in p['connections']}
    exact = {(x['definition']['id'],x['definition']['version']): x for x in read(source/'actual-frozen-modules.json')}
    manual='PF-manual-packing'; wrapper='PF-pallet-stretch-wrapper'; case='PF-packed-case'
    empty='PF-PALLET-empty'; loaded='PF-PALLET-loaded'
    changed={k:{} for k in ['item_type','device','location','connection','route','bom','product']}
    modules=copy.deepcopy(media['modules'])
    def module(name,title,category,data,deps=None):
        identity='sf.paperfactory.pallet.'+name; ref={'id':identity,'version':spec['version']}
        directory=output/'modules'/identity
        write(directory/'module.json',{'schema_version':'1.0','data':{'definition':dict(ref,title=title,definition_state='runtime_candidate',
            contribution={'author':'Super Factory pallet composition','license':'CC0-1.0','assets':[]},parameters=[],attribute_schema=[],payload={'category':category,'data':data}),
            'assets':[],'files':[],'dependencies':deps or [],'rights':[],'issues':[]}})
        modules.append(dict(ref,directory=str(directory)));return ref
    def inline(device):
        ref=devices[device]['module_ref'];return copy.deepcopy(exact[(ref['id'],ref['version'])]['definition']['payload']['data']['behavior']['inline'])
    def slot(item,offset,anchor=None):return {'item_type_ref':{'id':item,'revision':'r1'},'anchor_role':anchor,'offset':offset}
    source_type=types['pallet-stretch-wrapper-input']; ref=source_type['module_ref']
    source_payload=exact[(ref['id'],ref['version'])]['definition']['payload']['data']
    for identity,title,count in [(empty,'1200×800mm装托用木托盘',0),(loaded,'三箱整托（含18件内装物）',3)]:
        t=copy.deepcopy(source_type);t.update(id=identity,title=title)
        t['attributes']['declared_case_contents']['value']=count
        t['contents_layout']={'assignment':'item_id_order','slots':[slot(case,pose(0,0,0),'ContentSeat_'+str(i)) for i in range(count)]} if count else None
        payload=copy.deepcopy(source_payload);payload.update(attributes=t['attributes'],presentation_or_media_ref=media['presentation'])
        t['module_ref']=module('items.'+('empty' if not count else 'loaded'),title,'items',payload,[media['media_ref']])
        types[identity]=t;changed['item_type'][identity]=t
    film_type=copy.deepcopy(types['pallet-stretch-wrapper-auxiliary']);film_ref=film_type['module_ref']
    film_payload=copy.deepcopy(exact[(film_ref['id'],film_ref['version'])]['definition']['payload']['data'])
    film_payload['presentation_or_media_ref']=media['film_presentation']
    film_type.update(title='9米计量缠绕膜',module_ref=module('items.metered-film','9米计量缠绕膜','items',film_payload,[media['film_media_ref']]))
    changed['item_type'][film_type['id']]=film_type
    op=inline(manual);op['inputs'][0]['quantity']=quantity(3)
    for phase in op['phases']:
        phase['duration'].update(value=spec['operation']['manual_cycle_ms'][phase['id']],basis='user_provided')
    opref=module('operation.load','三箱装托','operations',op)
    devices[manual].setdefault('route_operation_refs',[]).append(opref)
    devices[manual]['presentation_override']=media['manual_presentation']
    # The case output also stages on this floor position; no second overlapping table slot.
    slots=devices[manual]['item_layout']['slots']
    slots[:]=[x for x in slots if x['item_type_ref']['id']!=case]
    slots.extend(slot(case,pose(2.425+x,0,.12),'floor_loading') for x in [-.34,0,.34])
    slots.extend(slot(t,pose(2.425,0,0),'floor_loading') for t in [empty,loaded])
    changed['device'][manual]=devices[manual]
    mapping={'pallet-stretch-wrapper-input':loaded}
    devices[wrapper]=replace(devices[wrapper],mapping);changed['device'][wrapper]=devices[wrapper]
    if 'wrapper_presentation' in media: devices[wrapper]['presentation_override']=media['wrapper_presentation']
    for identity,c in list(connections.items()):
        if any(e['kind']=='port' and e['data']['instance_id']==wrapper for e in [c['data']['source'],c['data']['destination']]):
            c=replace(c,mapping);connections[identity]=c;changed['connection'][identity]=c
            lid=c['data']['destination_store'];locations[lid]=replace(locations[lid],mapping);changed['location'][lid]=locations[lid]
    def port_stock(device,role,direction):
        return next(c['data']['destination_store'] for c in connections.values() if
            (e:=c['data']['destination' if direction=='input' else 'source'])['kind']=='port'
            and e['data']['instance_id']==device and e['data']['port_id']==role)
    def stock(identity,title,item,count,position):
        loc=copy.deepcopy(next(iter(locations.values())))
        loc.update(id=identity,title=title,hall_id='PF-H1',purpose='buffer',ordinary_issue_enabled=True,position=position,
                   accepted_item_types=[item],capacity=quantity(count),item_layout={'assignment':'item_id_order','slots':[slot(item,pose((i-(count-1)/2)*.5,0,0)) for i in range(count)]})
        locations[identity]=loc;changed['location'][identity]=loc;return identity
    incase=stock('PF-PALLET-cases','装托待料 · 三只成品箱',case,3,pose(34.8,72.3,0))
    inpallet=stock('PF-PALLET-empty-stock','空托盘待料',empty,1,pose(34.8,76,0))
    outpallet=stock('PF-PALLET-loaded-stock','已装托待缠绕',loaded,1,pose(34.8,73.5,0))
    for lid,item,role,direction in [(incase,case,'input','input'),(inpallet,empty,'container','input'),(outpallet,loaded,'output','output')]:
        endpoint={'kind':'port','data':{'module_ref':devices[manual]['module_ref'],'instance_id':manual,'port_id':role}}
        c={'kind':'material','data':{'id':'PF-PALLET-CONN-'+role,'source':{'kind':'location','data':lid} if direction=='input' else endpoint,
            'destination':endpoint if direction=='input' else {'kind':'location','data':lid},'item_type':item,'unit':'piece','quantity_rule':None,'destination_store':lid,'capacity_rule':None}}
        changed['connection'][c['data']['id']]=c
    shipping=locations['PF-SHIPPING'];shipping['accepted_item_types']=[loaded if x==case else x for x in shipping['accepted_item_types']]
    shipping['capacity']=quantity(100);shipping['item_layout']['slots']=[x for x in shipping['item_layout']['slots'] if x['item_type_ref']['id']!=case]+[slot(loaded,pose(-5.5,-2,-.9))]
    changed['location'][shipping['id']]=shipping
    # Remove only the standalone wrapper demonstration load and its nested sample boxes.
    initial={x['id']:x for x in s['initial_conditions']['items']}
    removed={x['id'] for x in initial.values() if x['item_type_ref']['id']=='pallet-stretch-wrapper-input'}
    while True:
        before=set(removed)
        for identity in list(removed):removed.update(initial[identity]['contents']+initial[identity]['components'])
        if before==removed:break
    s['initial_conditions']['items']=[x for x in initial.values() if x['id'] not in removed]
    s['initial_conditions']['items'].append({'id':'PF-PALLET-BASE-01','item_type_ref':{'id':empty,'revision':'r1'},'quantity':quantity(),
        'holder':{'kind':'location','id':inpallet},'quality':'released','contents':[],'components':[],'source':'user_provided'})
    s.update(id='PF-PALLET-FACTORY',title='纸包装整厂 · 三箱装托与缠绕发运',revision=1,scope=[])
    s['production_tasks']=[x for x in s['production_tasks'] if x['id']!='task-pallet-stretch-wrapper--normal']
    next(x for x in s['production_tasks'] if x['id']=='PF-TASK-CASE')['order_allocations']=[]
    order=next(x for x in s['orders'] if x['id']=='PF-ORDER-CASE')
    order.update(id='PF-ORDER-PALLET',product_ref={'id':'PF-PRODUCT-PALLET','revision':'r1'},quantity=quantity())
    base=next(x for x in p['routes'] if x['id']=='PF-ROUTE-CASE')['steps'][0]
    loadstep=copy.deepcopy(base);loadstep.update(id='pallet-load',predecessors=[],operation_ref=opref,item_bindings={'input':case,'container':empty,'output':loaded})
    wrapstep=copy.deepcopy(next(x for x in p['routes'] if x['id']=='route-pallet-stretch-wrapper--normal')['steps'][0])
    wrapstep=replace(wrapstep,mapping);wrapstep.update(id='pallet-wrap',predecessors=['pallet-load'])
    changed['route']['PF-ROUTE-PALLET']={'id':'PF-ROUTE-PALLET','revision':'r1','steps':[loadstep,wrapstep]}
    rows=[]
    for step,operation in [(loadstep,op),(wrapstep,inline(wrapper))]:
        rows.extend({'item_type_ref':{'id':step['item_bindings'][r['role']],'revision':'r1'},'quantity':r['quantity'],'consume_at_operation':step['id']} for r in operation['inputs'])
    changed['bom']['PF-BOM-PALLET']={'id':'PF-BOM-PALLET','revision':'r1','rows':rows}
    changed['product']['PF-PRODUCT-PALLET']={'id':'PF-PRODUCT-PALLET','revision':'r1','item_type_ref':{'id':loaded,'revision':'r1'},
        'bom_ref':{'id':'PF-BOM-PALLET','revision':'r1'},'routing_ref':{'id':'PF-ROUTE-PALLET','revision':'r1'},'artifact_refs':[]}
    s['production_tasks'].append({'id':'PF-TASK-PALLET','routing_ref':{'id':'PF-ROUTE-PALLET','revision':'r1'},'planned_quantity':quantity(),
        'order_allocations':[{'order_id':'PF-ORDER-PALLET','quantity':quantity()}],'release_at_ms':0,'priority':10})
    original=next(x for x in s['supplies'] if x['id']=='PF-LOGISTICS-TRANSFER-4');s['supplies']=[x for x in s['supplies'] if x['id']!=original['id']]
    legs=[(original['source'],incase,case,3),(outpallet,port_stock(wrapper,'input','input'),loaded,1),(port_stock(wrapper,'output','output'),'PF-SHIPPING',loaded,1)]
    for i,(a_id,b_id,item,count) in enumerate(legs):
        a=locations[a_id]['position']['translation'];b=locations[b_id]['position']['translation'];road=lambda y:16 if y<42 else 40 if y<66 else 64
        points=[dict(a),{'x':a['x'],'y':road(a['y']),'z':.9},{'x':52,'y':road(a['y']),'z':.9},{'x':52,'y':road(b['y']),'z':.9},{'x':b['x'],'y':road(b['y']),'z':.9},dict(b)]
        points=[v for j,v in enumerate(points) if j==0 or v!=points[j-1]]
        travel=math.ceil(sum(math.dist([a[k] for k in ['x','y','z']],[b[k] for k in ['x','y','z']]) for a,b in zip(points,points[1:]))/1.2*1000)
        definition=copy.deepcopy(next(x['payload']['data'] for x in read(ROOT/'packages/commands/tests/fixtures/production/p202-identity.json')['definitions'] if x['payload']['category']=='transport'))
        definition['batch_capacity']=quantity();definition['loading_resources']=[{'role':'dispatch_gate','quantity':quantity(1,'slot'),'qualifications':[]}]
        for phase,ms in [('loading',1000),('outbound',travel),('unloading',1000),('returning',travel)]:definition['times'][phase]={'value':ms,'unit':'millisecond','basis':'user_provided','source_ref':None}
        transport=module('transport.'+str(i),'装托转运 · '+str(i+1),'transport',definition)
        supply=copy.deepcopy(original);supply.update(id='PF-PALLET-TRANSFER-'+str(i),source=a_id,destination=b_id,item_type_ref={'id':item,'revision':'r1'},target_quantity=quantity(count),transport_ref=transport)
        supply['presentation_path'].update(waypoints_m=points,cargo_layout={'assignment':'item_id_order','slots':[slot(item,pose(0,0,.08))]});s['supplies'].append(supply)
    checks=[]
    raw=[x['id'] for x in s['initial_conditions']['items'] if x['item_type_ref']['id']=='manual-packing-input']
    for name,missing in [('SHORT-CASES',set(raw[:6])),('MISSING-FILM',{'auxiliary-pallet-stretch-wrapper--normal'}),('MISSING-PALLET',{'PF-PALLET-BASE-01'})]:
        check=copy.deepcopy(s);check.update(id='PF-PALLET-'+name,title='整托边界 · '+name)
        check['initial_conditions']['items']=[x for x in check['initial_conditions']['items'] if x['id'] not in missing];checks.append(check)
    commands=[{'kind':'put_object','data':{'kind':kind,'data':value}} for kind,values in changed.items() for value in values.values()]
    commands.append({'kind':'put_object','data':{'kind':'scenario','data':s}})
    write(output/'commands.json',{'base_revision':p['revision'],'commands':commands,'new_modules':modules,'scenario_id':s['id'],
        'quality_checks':checks,'material_checks':{c['id']:{'PF-ORDER-PALLET':0} for c in checks},'expected':spec['expected']})
    write(output/'design-consumption.json',{'specification':spec,'source':str(source),'isolated_items_removed':sorted(removed),
        'canonical_device_modules_unchanged':all(devices[d['id']]['module_ref']==d['module_ref'] for d in p['devices'])})
    print('Prepared real three-case pallet route and three material boundary scenarios:',output)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['source','media','output']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();prepare(args.source,args.media,args.output)
