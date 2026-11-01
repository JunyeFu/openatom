"""Connect the designed remaining branches to actual receiving and dispatch.

Preserves the original manufacturing definitions, quantities and nested material
identities. Results are computed later by the existing product Host.
"""
import argparse
import copy
import math
from pathlib import Path
from prepare_full_factory import ROOT, DESIGN, read, write
from connect_full_factory_routes import quantity, pose

WINDOW = 28800000  # Explicit eight-hour demonstration shift, not a speed multiplier.


def amount(q):
    c, d = q['value']['coefficient'], q['value']['decimal_places']
    assert c % (10 ** d) == 0, 'Discrete transport needs whole physical units'
    return c // (10 ** d)


def prepare(source, output):
    assert not output.exists(), 'Use a new attempt directory'
    spec=read(DESIGN/'remaining-branches-spec.json')
    assert Path(spec['source']).resolve()==source.resolve()
    p=read(source/'actual-project.json');s=copy.deepcopy(read(source/'actual-scenario.json'))
    packages=read(source/'actual-frozen-modules.json')
    definitions={(m['definition']['id'],m['definition']['version']):m['definition'] for m in packages}
    devices={d['id']:d for d in p['devices']};routes={r['id']:r for r in p['routes']}
    types={t['id']:t for t in p['item_types']};locations={l['id']:l for l in p['locations']}
    changed={kind:{} for kind in ['location','bom','product','calendar']}
    s.update(id='PF-ALL-BRANCHES',title='纸包装整厂 · 多产品支线、统一配送与发运',revision=1,
             project_revision=p['revision'],window={'start_ms':0,'end_ms':WINDOW},scope=[])
    for supply in s['supplies']:
        supply['window']['end_ms']=WINDOW;supply['departure_windows']=[{'start_ms':0,'end_ms':WINDOW}]
    for order in s['orders']:order['due_at_ms']=WINDOW
    cal=copy.deepcopy(next(c for c in p['calendars'] if c['id']=='PF-ALL'))
    cal['intervals']=[{'start_ms':0,'end_ms':WINDOW}];changed['calendar'][cal['id']]=cal
    transport_seed=next(d['payload']['data'] for d in read(ROOT/'packages/commands/tests/fixtures/production/p202-identity.json')['definitions'] if d['payload']['category']=='transport')
    modules=[];transfers=[];coverage=[];expected_orders={o['id']:amount(o['quantity']) for o in s['orders']}

    def stock_at(device,role,direction):
        port=device['role_bindings']['ports'][role]['port_id']
        for c in p['connections']:
            e=c['data']['destination' if direction=='input' else 'source']
            if e['kind']=='port' and e['data']['instance_id']==device['id'] and e['data']['port_id']==port:
                return locations[c['data']['destination_store']]
        raise AssertionError((device['id'],role,direction))

    def transfer(identity,origin,destination,type_id,count):
        assert count>0
        a=origin['position']['translation'];b=destination['position']['translation']
        road=lambda y: 16 if y<42 else 40 if y<66 else 64
        points=[dict(a),{'x':a['x'],'y':road(a['y']),'z':.9},{'x':52,'y':road(a['y']),'z':.9},
                {'x':52,'y':road(b['y']),'z':.9},{'x':b['x'],'y':road(b['y']),'z':.9},dict(b)]
        points=[v for i,v in enumerate(points) if i==0 or v!=points[i-1]]
        distance=sum(math.dist([a[k] for k in ['x','y','z']],[b[k] for k in ['x','y','z']]) for a,b in zip(points,points[1:]))
        transport=copy.deepcopy(transport_seed);transport['batch_capacity']=quantity(1)
        transport['loading_resources']=[{'role':'dispatch_gate','quantity':quantity(1,'slot'),'qualifications':[]}]
        for phase,t in [('loading',1000),('outbound',math.ceil(distance/1.2*1000)),('unloading',1000),('returning',math.ceil(distance/1.2*1000))]:
            transport['times'][phase]={'value':t,'unit':'millisecond','basis':'user_provided','source_ref':None}
        ref={'id':'sf.paperfactory.branches.transport.'+identity,'version':'1.0.0'}
        directory=output/'modules'/ref['id']
        definition=dict(ref,title='厂内配送 · '+origin['title']+' → '+destination['title'],definition_state='runtime_candidate',
                        contribution={'author':'Super Factory factory composition','license':'CC0-1.0','assets':[]},
                        parameters=[],attribute_schema=[],payload={'category':'transport','data':transport})
        write(directory/'module.json',{'schema_version':'1.0','data':{'definition':definition,'assets':[],'files':[],'dependencies':[],'rights':[],'issues':[]}})
        modules.append(dict(ref,directory=str(directory)))
        type_ref={'id':type_id,'revision':types[type_id]['revision']}
        supply={'id':identity,'source':origin['id'],'destination':destination['id'],'item_type_ref':type_ref,
                'target_quantity':quantity(count),'window':{'start_ms':0,'end_ms':WINDOW},'transport_ref':ref,
                'resources':{'carrier':'PF-CARRIERS','dispatch_gate':'PF-TRANSIT-GATE'},
                'departure_windows':[{'start_ms':0,'end_ms':WINDOW}],'allow_partial_load':True,
                'presentation_path':{'hall_id':'PF-H1','waypoints_m':points,'cargo_layout':{'assignment':'item_id_order',
                    'slots':[{'item_type_ref':type_ref,'anchor_role':None,'offset':pose(0,0,.08)}]}}}
        s['supplies'].append(supply);transfers.append({'id':identity,'count':count,'source':origin['id'],'destination':destination['id'],'item_type':type_id,'waypoints_m':points})

    for index,row in enumerate(spec['branches']):
        device=devices[row['device_id']];slug=row['device_id'].removeprefix('PF-')
        task=next(t for t in s['production_tasks'] if t['id']==row['task_id']);route=routes[row['route_id']];step=route['steps'][0]
        ref=step['operation_ref'];definition=definitions[(ref['id'],ref['version'])];op=definition['payload']['data']
        if definition['payload']['category']=='devices':
            behavior=op['behavior'];op=behavior['inline'] if behavior['selected']=='inline' else definitions[(behavior['operation_ref']['id'],behavior['operation_ref']['version'])]['payload']['data']
        incoming=stock_at(device,'input','input');outgoing=stock_at(device,'output','output')
        input_type=row['input_type'];output_type=row['output_type']
        raw=[i for i in s['initial_conditions']['items'] if i['holder']=={'kind':'location','id':incoming['id']} and i['item_type_ref']['id']==input_type]
        raw_count=sum(amount(i['quantity']) for i in raw);assert raw_count>0,(slug,'missing original feed')
        receiving=copy.deepcopy(incoming);receiving.update(id='PF-BRANCH-IN-'+slug,title='备料 · '+device['title'],
            position=pose(3+(index%8)*1.6,1.5+(index//8)*1.6,.9))
        changed['location'][receiving['id']]=receiving
        for item in raw:item['holder']={'kind':'location','id':receiving['id']}
        transfer('PF-BRANCH-FEED-'+slug,receiving,incoming,input_type,raw_count)
        primary=next(o for o in op['outputs'] if o['role']=='output')
        produced=amount(primary['quantity']);assert amount(task['planned_quantity'])==1
        dispatch=copy.deepcopy(outgoing);dispatch.update(id='PF-BRANCH-OUT-'+slug,title=('搬运交付 · ' if row['role']=='material_handling_service' else '成品 · ')+device['title'],purpose='output',
            position=pose(60+(index%8)*2,1.5+(index//8)*1.6,.9))
        if slug=='carton-counter-stacker':
            for level,slot in enumerate(dispatch['item_layout']['slots'][:6]):
                slot['offset']['translation']['z']=level*.013
        changed['location'][dispatch['id']]=dispatch
        transfer('PF-BRANCH-SHIP-'+slug,outgoing,dispatch,output_type,produced)
        bom_id='PF-BRANCH-BOM-'+slug;product_id='PF-BRANCH-PRODUCT-'+slug;order_id='PF-BRANCH-ORDER-'+slug
        changed['bom'][bom_id]={'id':bom_id,'revision':'r1','rows':[{'item_type_ref':{'id':step['item_bindings'][i['role']],'revision':types[step['item_bindings'][i['role']]]['revision']},'quantity':i['quantity'],'consume_at_operation':step['id']} for i in op['inputs']]}
        changed['product'][product_id]={'id':product_id,'revision':'r1','item_type_ref':{'id':output_type,'revision':types[output_type]['revision']},'bom_ref':{'id':bom_id,'revision':'r1'},'routing_ref':task['routing_ref'],'artifact_refs':[]}
        s['orders'].append({'id':order_id,'product_ref':{'id':product_id,'revision':'r1'},'quantity':quantity(produced),'release_at_ms':0,'due_at_ms':WINDOW,'destination':dispatch['id'],'acceptance_stage':'unloaded_and_released','allow_partial':True,'priority':0})
        task['planned_quantity']=quantity(produced);task['order_allocations']=[{'order_id':order_id,'quantity':quantity(produced)}]
        expected_orders[order_id]=produced
        coverage.append(dict(row,order_id=order_id,expected_output=produced,receiving=receiving['id'],dispatch=dispatch['id'],feed_item_ids=[i['id'] for i in raw]))
    # An independent expectation from the unchanged original tasks and declared
    # per-leg quantities; no Run results are read to construct the expected result.
    original_trips=sum(math.ceil(amount(x['target_quantity'])/amount(definitions[(x['transport_ref']['id'],x['transport_ref']['version'])]['payload']['data']['batch_capacity'])) for x in read(source/'actual-scenario.json')['supplies'])
    commands=[{'kind':'put_object','data':{'kind':kind,'data':x}} for kind,rows in changed.items() for x in rows.values()]
    commands.append({'kind':'put_object','data':{'kind':'scenario','data':s}})
    target=next(row for row in coverage if row['device_id']=='PF-adhesive-dispenser')
    checks=[];material_checks={}
    for suffix in ['MISSING-FEED','BLOCKED-RECEIVER']:
        check=copy.deepcopy(s);check['id']='PF-BRANCH-'+suffix;check['title']='配送边界 · '+('缺少来料' if suffix=='MISSING-FEED' else '待处理物料占满接收位')
        if suffix=='MISSING-FEED':
            check['initial_conditions']['items']=[i for i in check['initial_conditions']['items'] if i['id'] not in target['feed_item_ids']]
        else:
            blocked=copy.deepcopy(next(i for i in check['initial_conditions']['items'] if i['id'] in target['feed_item_ids']))
            assert not blocked['contents'] and not blocked['components']
            blocked.update(id='PF-BRANCH-REJECTED-BLOCKER',quality='rejected',holder={'kind':'location','id':stock_at(devices[target['device_id']],'input','input')['id']})
            check['initial_conditions']['items'].append(blocked)
        checks.append(check);material_checks[check['id']]={target['order_id']:0}
    write(output/'commands.json',{'base_revision':p['revision'],'commands':commands,'new_modules':modules,'scenario_id':s['id'],'quality_checks':checks,'material_checks':material_checks,
        'expected':{'manufacturing_jobs':162,'transport_trips':original_trips+sum(t['count'] for t in transfers),'accepted_orders':expected_orders,'integrated_devices':92,'independent_normal_cells':0}})
    write(output/'branch-coverage.json',{'branches':coverage,'retained_connected_devices':spec['existing_connected_devices'],'transfers':transfers,'verified':False})
    print('Staged',len(coverage),'branches,',len(transfers),'transport legs and',len(expected_orders),'orders; Host verification pending.')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--source',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    a=parser.parse_args();prepare(a.source,a.output)
