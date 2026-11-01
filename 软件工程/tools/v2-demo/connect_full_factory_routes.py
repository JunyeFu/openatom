"""Prepare connected paper-product routes for the current actual factory.

Uses existing equipment definitions unchanged, declared compatible product
specifications, real supply/transport modules and explicit order acceptance.
The actual Host consumer applies these commands and computes all results.
"""
import argparse
import copy
import math
import shutil
from pathlib import Path

from prepare_full_factory import ROOT, DESIGN, read, write, asset_bounds
from case_material_media import prepare_case_media

WINDOW=1800000  # Declared 30-minute simulated work window, including return trips.
COUNT=3


def quantity(n=1,unit='piece'):
    return {'value':{'coefficient':n,'decimal_places':0},'unit':unit}


def pose(x,y,z=0):
    return {'translation':{'x':x,'y':y,'z':z},'rotation':{'x':0,'y':0,'z':0,'w':1},'scale':{'x':1,'y':1,'z':1},'unit':'meter','coordinate_frame':'right_handed_z_up'}


def main(project_path,scenario_path,output):
    p=read(project_path);s=read(scenario_path)['restore_scenario'];layout=read(DESIGN/'layout.json')
    original=copy.deepcopy(p)
    devices={x['id']:x for x in p['devices']};locations={x['id']:x for x in p['locations']};types={x['id']:x for x in p['item_types']}
    cell_routes={r['steps'][0]['device_id']:r for r in p['routes'] if len(r['steps'])==1}
    blank=['sheet-feeder','platen-creaser','folder-gluer']
    packing=['manual-packing','tape-case-sealer','label-applicator','strapper','vision-carton-inspector']
    selected={'PF-'+x for x in blank+packing}
    changed={k:{} for k in ['item_type','device','location','connection','route','bom','product','resource','calendar']}
    new_modules=[];transport_paths=[]
    s=copy.deepcopy(s);s.update(id='PF-PRODUCT-FLOW',title='全厂协同 · 盒坯与装箱 · 实际运输和成品放行',revision=1,project_revision=p['revision'],window={'start_ms':0,'end_ms':WINDOW},scope=[])
    s['orders']=[];s['supplies']=[];s['parameter_overrides']=[];s['acceptance_goal']=None
    s['production_tasks']=[t for t in s['production_tasks'] if not any(st['device_id'] in selected for st in next(r for r in p['routes'] if r['id']==t['routing_ref']['id'])['steps'])]
    selected_locations=set()
    for c in p['connections']:
        for endpoint in [c['data']['source'],c['data']['destination']]:
            if endpoint['kind']=='port' and endpoint['data']['instance_id'] in selected:
                selected_locations.add(c['data']['destination_store'])
    all_items={x['id']:x for x in s['initial_conditions']['items']}
    remove={x['id'] for x in all_items.values() if x['holder']['kind']=='location' and x['holder']['id'] in selected_locations}
    while True:
        prior=set(remove)
        for identity in list(remove):remove.update(all_items[identity]['contents']);remove.update(all_items[identity]['components'])
        if prior==remove:break
    s['initial_conditions']['items']=[x for x in all_items.values() if x['id'] not in remove]
    s['quality_presets']=[x for x in s['quality_presets'] if x['inspection_module']['id']!=devices['PF-vision-carton-inspector']['module_ref']['id']]
    s['initial_conditions']['declared_quality_decisions']=[]
    calendar=next(x for x in p['calendars'] if x['id']=='PF-ALL');calendar['intervals']=[{'start_ms':0,'end_ms':WINDOW}];changed['calendar'][calendar['id']]=calendar

    # These two specifications are explicit demonstration products, with no
    # inferred machine ratings. They share dimensions with the existing models.
    def combined_schema(names):
        fields={}
        for name in names:
            for field in types[name]['attribute_schema']:
                if field['name'] not in fields:fields[field['name']]=copy.deepcopy(field)
                else:
                    assert fields[field['name']]['value_type']==field['value_type']
                    fields[field['name']]['options']=list(dict.fromkeys(fields[field['name']]['options']+field['options']))
        return list(fields.values())
    flat=copy.deepcopy(types['folder-gluer-input']);flat.update(id='PF-flat-box-blank',title='420×400mm预切盒坯（演示）')
    flat['attribute_schema']=combined_schema(['folder-gluer-input','platen-creaser-input','sheet-feeder-input'])
    flat['attributes']['presentation_state']={'kind':'enum','value':'flat'}
    flat['attributes']['sheet_count']={'kind':'number','value':1}
    flat['module_ref']={'id':'sf.paperfactory.items.box-blank','version':'1.0.1'}
    types[flat['id']]=flat;changed['item_type'][flat['id']]=flat
    packed=copy.deepcopy(types['manual-packing-output']);packed.update(id='PF-packed-case',title='六件装纸箱（320×240×240mm演示规格）')
    packed['attribute_schema']=combined_schema(['manual-packing-output','tape-case-sealer-input','label-applicator-input','strapper-input','vision-carton-inspector-input'])
    packed['attributes'].update({k:{'kind':'enum','value':v} for k,v in [('seal_state','open'),('label_state','unlabelled'),('band_state','unstrapped')]})
    packed['attributes']['label_code']={'kind':'enum','value':''}
    packed['module_ref']={'id':'sf.paperfactory.items.packed-case','version':'1.0.1'}
    types[packed['id']]=packed;changed['item_type'][packed['id']]=packed

    def rename_type(v,mapping):
        if isinstance(v,str):return mapping.get(v,v)
        if isinstance(v,list):return [rename_type(x,mapping) for x in v]
        if isinstance(v,dict):return {k:rename_type(x,mapping) for k,x in v.items()}
        return v

    mappings={
      'PF-sheet-feeder':{'sheet-feeder-input':flat['id']},
      'PF-platen-creaser':{'platen-creaser-input':flat['id']},
      'PF-folder-gluer':{'folder-gluer-input':flat['id']},
      'PF-manual-packing':{'manual-packing-output':packed['id']},
      'PF-tape-case-sealer':{'tape-case-sealer-input':packed['id']},
      'PF-label-applicator':{'label-applicator-input':packed['id']},
      'PF-strapper':{'strapper-input':packed['id']},
      'PF-vision-carton-inspector':{'vision-carton-inspector-input':packed['id']},
    }
    for identity,mapping in mappings.items():
        devices[identity]=rename_type(devices[identity],mapping);changed['device'][identity]=devices[identity]
        for c in p['connections']:
            if any(e['kind']=='port' and e['data']['instance_id']==identity for e in [c['data']['source'],c['data']['destination']]):
                c=rename_type(c,mapping);changed['connection'][c['data']['id']]=c
                stock=c['data']['destination_store'];locations[stock]=rename_type(locations[stock],mapping);changed['location'][stock]=locations[stock]

    def stock(identity,title,type_ids,where,capacity):
        loc=copy.deepcopy(next(iter(locations.values())));loc.update(id=identity,title=title,hall_id='PF-H1',position=where,purpose='buffer',ordinary_issue_enabled=True,accepted_item_types=type_ids,capacity=quantity(capacity))
        loc['item_layout']={'assignment':'item_id_order','slots':[{'item_type_ref':{'id':t,'revision':types[t]['revision']},'anchor_role':None,'offset':pose(((type_index*capacity+i)%5)*.55,((type_index*capacity+i)//5)*.55,0)} for type_index,t in enumerate(type_ids) for i in range(capacity)]}
        locations[identity]=loc;changed['location'][identity]=loc
        return loc

    source=stock('PF-BLANK-RECEIVING','预切盒坯 · 收货区',[flat['id']],pose(18,9,.9),COUNT)
    shipping=stock('PF-SHIPPING','合格成品 · 卸货放行',[packed['id'],'folder-gluer-output'],pose(86,9,.9),COUNT*2)
    shipping['purpose']='output'

    def initial(identity,type_id,holder):
        s['initial_conditions']['items'].append({'id':identity,'item_type_ref':{'id':type_id,'revision':types[type_id]['revision']},'quantity':quantity(),'holder':{'kind':'location','id':holder},'quality':'released','components':[],'contents':[],'source':'user_provided'})

    for i in range(COUNT):initial(f'PF-blank-{i}',flat['id'],source['id'])

    def port_stock(device,role,direction):
        port=devices[device]['role_bindings']['ports'][role]['port_id']
        connections={c['data']['id']:c for c in p['connections']};connections.update(changed['connection'])
        for c in connections.values():
            e=c['data']['destination' if direction=='input' else 'source']
            if e['kind']=='port' and e['data']['instance_id']==device and e['data']['port_id']==port:return c['data']['destination_store']
        raise AssertionError((device,role,direction))

    def prepare_doses(device,role,count):
        type_id=devices[device]['role_bindings']['item_types'][role];loc=port_stock(device,role,'input')
        current=locations[loc];stock(loc,current['title'],[type_id],current['position'],count)
        for i in range(count):initial(f'{device}-{role}-dose-{i}',type_id,loc)

    for device in selected:
        roles=devices[device]['role_bindings']['item_types']
        for role in roles:
            if role.startswith('auxiliary'):prepare_doses(device,role,COUNT)
    # Assembly consumes six individual products and one distinct empty carton.
    manual='PF-manual-packing'
    for role,count in [('input',6*COUNT),('container',COUNT)]:
        t=devices[manual]['role_bindings']['item_types'][role];loc=port_stock(manual,role,'input');old=locations[loc]
        stock(loc,old['title'],[t],old['position'],count)
        for i in range(count):initial(f'PF-pack-{role}-{i:02d}',t,loc)

    # A real pooled transport resource; no additional manufacturing machine is
    # counted among the 92. Each trip consumes one of three declared vehicles.
    carrier_module='sf.paperfactory.carrier'
    carrier_definition={'kind':'vehicle','role':'carrier','capacity':quantity(3,'vehicle'),'qualifications':[],'default_calendar_ms':[{'start_ms':0,'end_ms':WINDOW}],'presentation':None}
    def module(identity,title,category,data,assets=None,files=None,rights=None,dependencies=None,version='1.0.0'):
        definition={'id':identity,'version':version,'title':title,'definition_state':'runtime_candidate','contribution':{'author':'Super Factory demo factory design','license':'CC0-1.0','assets':rights or []},'parameters':[],'attribute_schema':[],'payload':{'category':category,'data':data}}
        package={'schema_version':'1.0','data':{'definition':definition,'assets':assets or [],'files':files or [],'dependencies':dependencies or [],'rights':rights or [],'issues':[]}}
        directory=output/'modules'/identity;write(directory/'module.json',package);new_modules.append({'id':identity,'version':version,'directory':str(directory)})
    # The derived 420x400 blank needs both flat and creased clips. The original
    # folder-gluer input only declares precreased_blank and cannot show flat.
    saved=project_path.parent/'project';manifest=read(saved/'project.json')['data']
    def saved_module(reference):
        entry=next(x for x in manifest['documents'] if x.get('module_ref')==reference)
        return read(saved/entry['file_ref']['path'])['data']
    source_item=saved_module(types['platen-creaser-input']['module_ref'])
    wrapper=copy.deepcopy(source_item['definition']['payload']['data']['presentation_or_media_ref'])
    source_media_ref=wrapper['source']['data'];source_media=saved_module(source_media_ref)
    media=copy.deepcopy(source_media['definition']['payload']['data']);old_asset=media['source']['data']
    new_asset={'id':'sf.paperfactory.box-blank.glb','path':'box-blank.glb','kind':'glb'}
    media=rename_type(media,{old_asset['id']:new_asset['id'],old_asset['path']:new_asset['path']})
    creaser=cell_routes['PF-platen-creaser']['steps'][0]['operation_ref']
    for clip in media['timeline']:
        if clip['phase_ref'].startswith('flat:'):
            state,phase=clip['phase_ref'].split(':',1)
            clip['phase_ref']=f"{state}:{creaser['id']}:{creaser['version']}:{phase}"
    bounds=asset_bounds(saved/old_asset['path'])
    media['scale']['x']=.42/(bounds['maximum'][0]-bounds['minimum'][0])
    media['scale']['z']=.4/(bounds['maximum'][2]-bounds['minimum'][2])
    media_ref={'id':'sf.paperfactory.media.box-blank','version':'1.0.1'}
    rights=copy.deepcopy(source_media['rights'])
    for row in rights:row['asset_ref']=new_asset['id']
    module(media_ref['id'],'420×400mm盒坯（平展与压痕）','media',media,assets=[new_asset],files=[{'path':new_asset['path'],'kind':'asset'}],rights=rights,version=media_ref['version'])
    shutil.copyfile(saved/old_asset['path'],output/'modules'/media_ref['id']/new_asset['path'])
    def rebind(value):
        if value==source_media_ref:return copy.deepcopy(media_ref)
        if isinstance(value,dict):return {k:rebind(v) for k,v in value.items()}
        if isinstance(value,list):return [rebind(x) for x in value]
        return value
    wrapper=rebind(wrapper)
    module(flat['module_ref']['id'],flat['title'],'items',{'presentation_or_media_ref':wrapper,'unit':flat['unit'],'tracking':flat['tracking'],'spec_ref':flat['spec_ref'],'attributes':flat['attributes'],'attribute_schema':flat['attribute_schema']},dependencies=[media_ref],version=flat['module_ref']['version'])
    inspection_ref={'id':'sf.paperfactory.operation.final-case-inspection','version':'1.0.0'}
    prepare_case_media(saved,saved_module,module,types,packed,output,{
        'packed_folded_case':cell_routes['PF-tape-case-sealer']['steps'][0]['operation_ref'],
        'taped_top_pack':cell_routes['PF-label-applicator']['steps'][0]['operation_ref'],
        'sealed_labelled_pack':cell_routes['PF-strapper']['steps'][0]['operation_ref'],
        'two_band_pack':inspection_ref,
    })
    # A final inspection is distinct from intermediate release for transport.
    # Reuse the actual inspector's phases and PASS/FAIL/pending rule, while
    # explicitly accepting intermediate released products as inspection inputs.
    inspector='PF-vision-carton-inspector'
    inspection=copy.deepcopy(read(ROOT/'modules/media/native-equipment-500/reviews/B01-Q18/devices/vision-carton-inspector/module.json')['data']['definition']['payload']['data']['behavior']['inline'])
    inspection['inputs'][0]['required_quality']=['unchecked','released']
    module(inspection_ref['id'],'纸箱最终检验（含已获工序放行的来料）','operations',inspection)
    devices[inspector]['route_operation_refs']=[*devices[inspector].get('route_operation_refs',[]),inspection_ref]
    changed['device'][inspector]=devices[inspector]
    module(carrier_module,'厂内转运车资源池（3台演示）','resources',carrier_definition)
    changed['resource']['PF-CARRIERS']={'id':'PF-CARRIERS','definition_ref':{'id':carrier_module,'version':'1.0.0'},'title':'3台厂内转运车（演示资源池）','kind':'vehicle','role':'carrier','capacity':quantity(3,'vehicle'),'qualifications':[],'calendar_id':'PF-ALL','home_location':source['id'],'presentation':None}
    gate_module='sf.paperfactory.dispatch-gate'
    module(gate_module,'车辆放行控制器（演示）','resources',{'kind':'fixture','role':'dispatch_gate','capacity':quantity(1,'slot'),'qualifications':[],'default_calendar_ms':[{'start_ms':0,'end_ms':WINDOW}],'presentation':None})
    changed['resource']['PF-TRANSIT-GATE']={'id':'PF-TRANSIT-GATE','definition_ref':{'id':gate_module,'version':'1.0.0'},'title':'装载放行间隔1秒（演示）','kind':'fixture','role':'dispatch_gate','capacity':quantity(1,'slot'),'qualifications':[],'calendar_id':'PF-ALL','home_location':source['id'],'presentation':None}

    def road(y):
        if y<18:return 16
        if y<42:return 16
        if y<66:return 40
        return 64

    def transfer(identity,from_id,to_id,type_id):
        a=locations[from_id]['position']['translation'];b=locations[to_id]['position']['translation']
        points=[dict(a),{'x':a['x'],'y':road(a['y']),'z':.9},{'x':52,'y':road(a['y']),'z':.9},{'x':52,'y':road(b['y']),'z':.9},{'x':b['x'],'y':road(b['y']),'z':.9},dict(b)]
        points=[point for i,point in enumerate(points) if i==0 or point!=points[i-1]]
        distance=sum(math.dist([a[k] for k in ['x','y','z']],[b[k] for k in ['x','y','z']]) for a,b in zip(points,points[1:]))
        travel=math.ceil(distance/1.2*1000)
        template=read(ROOT/'packages/commands/tests/fixtures/production/p202-identity.json')
        transport=copy.deepcopy(next(x['payload']['data'] for x in template['definitions'] if x['payload']['category']=='transport'))
        transport['loading_resources']=[{'role':'dispatch_gate','quantity':quantity(1,'slot'),'qualifications':[]}]
        for key,time in [('loading',1000),('outbound',travel),('unloading',1000),('returning',travel)]:transport['times'][key]={'value':time,'unit':'millisecond','basis':'user_provided','source_ref':None}
        transport_id='sf.paperfactory.transport.'+identity
        module(transport_id,'演示转运 · '+identity,'transport',transport)
        supply={'id':identity,'source':from_id,'destination':to_id,'item_type_ref':{'id':type_id,'revision':types[type_id]['revision']},'target_quantity':quantity(COUNT),'window':{'start_ms':0,'end_ms':WINDOW},'transport_ref':{'id':transport_id,'version':'1.0.0'},'resources':{'carrier':'PF-CARRIERS'},'departure_windows':[{'start_ms':0,'end_ms':WINDOW}],'allow_partial_load':True}
        supply['presentation_path']={'hall_id':'PF-H1','waypoints_m':points,'cargo_layout':{'assignment':'item_id_order','slots':[{'item_type_ref':supply['item_type_ref'],'anchor_role':None,'offset':pose(0,0,.08)}]}}
        supply['resources']['dispatch_gate']='PF-TRANSIT-GATE'
        s['supplies'].append(supply)
        transport_paths.append({'supply_id':identity,'source':from_id,'destination':to_id,'waypoints_m':points,'distance_m':round(distance,4),'outbound_ms':travel,'basis':'Explicit demo layout route at1.2m/s; actual trip timing remains from Run','presentation_state':'to_be_consumed_by_transport_view'})

    def route(identity,title,names,output_type,bom_rows):
        steps=[]
        for i,name in enumerate(names):
            device='PF-'+name;step=copy.deepcopy(cell_routes[device]['steps'][0]);step['id']=f'{identity}-step-{i+1}';step['predecessors']=[steps[-1]['id']] if steps else []
            if device==inspector:step['operation_ref']=inspection_ref
            step['item_bindings']=devices[device]['role_bindings']['item_types'];steps.append(step)
        route_id='PF-ROUTE-'+identity;order_id='PF-ORDER-'+identity;product_id='PF-PRODUCT-'+identity;bom_id='PF-BOM-'+identity
        changed['route'][route_id]={'id':route_id,'revision':'r1','steps':steps}
        changed['bom'][bom_id]={'id':bom_id,'revision':'r1','rows':[{'item_type_ref':{'id':type_id,'revision':types[type_id]['revision']},'quantity':quantity(count),'consume_at_operation':steps[index]['id']} for type_id,count,index in bom_rows]}
        changed['product'][product_id]={'id':product_id,'revision':'r1','item_type_ref':{'id':output_type,'revision':types[output_type]['revision']},'bom_ref':{'id':bom_id,'revision':'r1'},'routing_ref':{'id':route_id,'revision':'r1'},'artifact_refs':[]}
        s['orders'].append({'id':order_id,'product_ref':{'id':product_id,'revision':'r1'},'quantity':quantity(COUNT),'release_at_ms':0,'due_at_ms':WINDOW,'destination':shipping['id'],'acceptance_stage':'unloaded_and_released','allow_partial':True,'priority':10})
        s['production_tasks'].append({'id':'PF-TASK-'+identity,'routing_ref':{'id':route_id,'revision':'r1'},'order_allocations':[{'order_id':order_id,'quantity':quantity(COUNT)}],'planned_quantity':quantity(COUNT),'release_at_ms':0,'priority':10})
        for a,b in zip(names,names[1:]):
            ad,bd='PF-'+a,'PF-'+b;out_type=devices[ad]['role_bindings']['item_types']['output'];in_type=devices[bd]['role_bindings']['item_types']['input'];assert out_type==in_type,(a,b,out_type,in_type)
            transfer('PF-TRANSFER-'+a+'-'+b,port_stock(ad,'output','output'),port_stock(bd,'input','input'),out_type)
        last='PF-'+names[-1];transfer('PF-SHIP-'+identity,port_stock(last,'output','output'),shipping['id'],output_type)

    transfer('PF-FEED-BOX-BLANK',source['id'],port_stock('PF-sheet-feeder','input','input'),flat['id'])
    route('SLEEVE','折糊纸套',blank,'folder-gluer-output',[(flat['id'],1,0),('folder-gluer-auxiliary',1,2)])
    route('CASE','六件装封箱',packing,packed['id'],[('manual-packing-input',6,0),('manual-packing-container',1,0),('tape-case-sealer-auxiliary',1,1),('label-applicator-auxiliary',1,2),('strapper-auxiliary',1,3)])
    for i in range(COUNT):
        s['quality_presets'].append({'id':f'PF-QC-PASS-{i}','item_or_ancestor':f'PF-pack-container-{i:02d}','match_kind':'ancestor','inspection_module':inspection_ref,'results':[{'kind':'string','value':'PASS'}]})
    checks=[]
    for name,result in [('FAIL','FAIL'),('PENDING',None)]:
        check=copy.deepcopy(s);check['id']='PF-QC-'+name;check['title']='最终检验边界 · '+name
        first=next(x for x in check['quality_presets'] if x['id']=='PF-QC-PASS-0')
        first['results']=[] if result is None else [{'kind':'string','value':result}]
        checks.append(check)
    commands=[]
    for group,rows in changed.items():
        for row in rows.values():commands.append({'kind':'put_object','data':{'kind':group,'data':row}})
    commands.append({'kind':'put_object','data':{'kind':'scenario','data':s}})
    write(output/'commands.json',{'base_revision':original['revision'],'commands':commands,'new_modules':new_modules,'scenario_id':s['id'],'quality_checks':checks,'expected':{'orders':2,'accepted_per_order':COUNT,'manufacturing_jobs':108,'connected_route_devices':len(selected),'all_factory_devices':92,'physical_device_claim':False},'original_project':str(project_path)})
    write(output/'transport-paths.json',{'coordinate_frame':'right_handed_z_up','unit':'meter','routes':transport_paths})
    print('Staged connected sleeve/packing routes, explicit transport legs and order acceptance:',output)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--project',type=Path,required=True);parser.add_argument('--scenario-source',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);a=parser.parse_args();a.output.mkdir(parents=True,exist_ok=True);main(a.project,a.scenario_source,a.output)
