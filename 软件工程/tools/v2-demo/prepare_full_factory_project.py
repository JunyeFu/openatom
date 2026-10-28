"""Stage the real 92-device factory from current source producers and layout.

The first build is a layout + per-device normal-use scene. Continuous product
routes are a separate, explicit next construction step, never inferred here.
"""
import argparse
import copy
import json
import subprocess
import sys
from pathlib import Path

from prepare_full_factory import ROOT, DESIGN, ACCEPTED, CATALOG, read, write

PRODUCERS = {"B02-Q01-R1":"prepare-paper.py", "B02-Q02":"q02/prepare.py",
             "B02-Q03":"q03/prepare.py", "B02-Q03-R1":"q03r1/prepare.py", "B02-Q04":"q04/prepare.py"}
LISTS = ["item_types","locations","halls","lines","devices","resources","calendars","products","boms","routes","connections","module_refs","artifact_refs","acknowledgments","bindings","sources","issues","template_adoptions","setup_policies"]


def rewrite(v, mapping):
    if isinstance(v, str):return mapping.get(v,v)
    if isinstance(v,list):return [rewrite(x,mapping) for x in v]
    if isinstance(v,dict):return {mapping.get(k,k):rewrite(x,mapping) for k,x in v.items()}
    return v


def prepare(output):
    layout=read(DESIGN/'layout.json');accepted=read(ACCEPTED)['per_device']
    by_equipment={x['equipment_id']:x for x in layout['devices']}
    batches={x['review_directory'].split('/')[-1] for x in accepted}
    source={}
    for batch in sorted(batches):
        path=ROOT/'tools/3d-proof/native500'/PRODUCERS.get(batch,batch.lower().replace('-','')+'/prepare.py')
        target=output/'producer-inputs'/batch;target.mkdir(parents=True,exist_ok=True)
        review=Path('modules/media/native-equipment-500/reviews')/batch
        if not (target/'paper-input.json').exists():
            args=[sys.executable,str(path),str(review),str(target)]
            if batch in ['B02-Q01-R1','B02-Q02']:args.append('--input-only')
            run=subprocess.run(args,cwd=ROOT,capture_output=True,text=True,encoding='utf-8')
            (target/'producer.log').write_text(run.stdout+run.stderr,encoding='utf-8')
            assert run.returncode==0,(batch,run.stderr)
        source[batch]=read(target/'paper-input.json')

    base=copy.deepcopy(next(iter(source.values())))
    project=base['project'];scenario=base['scenario']
    for k in LISTS:project[k]=[]
    project.update(id='P1',title='纸包装多产品工厂 · 92设备布局与单机联动',revision=1,scenario_refs=['PF-ALL-NORMAL'],generation_context=None)
    scenario.update(id='PF-ALL-NORMAL',title='92设备正常工况 · 各产品独立投料',revision=1,window={'start_ms':0,'end_ms':300000},orders=[],production_tasks=[],arrivals=[],supplies=[],scope=[],parameter_overrides=[],quality_presets=[])
    scenario['initial_conditions']={'items':[],'initial_setup_families':{},'declared_quality_decisions':[],'material_allocations':[]}
    hall=copy.deepcopy(next(iter(source.values()))['project']['halls'][0]);hall.update(id='PF-H1',title='纸包装多产品演示厂房',width_m=104,depth_m=90,height_m=9,input_locations=[],output_locations=[])
    hall['origin']=copy.deepcopy(layout['devices'][0]['pose']);hall['origin']['translation']={'x':0,'y':0,'z':0};project['halls']=[hall]
    project['views']['selected_hall']='PF-H1'
    project['views']['selected_objects']=[]
    for view in ['line_camera','multi_hall_camera']:
        project['views'][view].update(eye={'x':150,'y':110,'z':70},target={'x':52,'y':0,'z':-45},zoom=1)
    project['time_reference']['label']='演示运行相对时间'
    scenario['time_reference']=copy.deepcopy(project['time_reference'])
    for z in layout['zones']:
        project['lines'].append({'id':'PF-L'+z['id'][1:],'title':z['id']+' · '+z['title'],'hall_id':'PF-H1','transform':copy.deepcopy(hall['origin']),'members':z['device_ids'],'boundary_ports':[],
          'region_polygon_m':[{'x':z['x_m'],'y':z['y_m']},{'x':z['x_m']+z['width_m'],'y':z['y_m']},{'x':z['x_m']+z['width_m'],'y':z['y_m']+z['depth_m']},{'x':z['x_m'],'y':z['y_m']+z['depth_m']}]})
    project['calendars']=[{'id':'PF-ALL','title':'演示全时段班次','intervals':[{'start_ms':0,'end_ms':300000}],'source':'user_provided'}]
    definitions={};objects={k:{} for k in ['item_types','locations','resources','products','boms','routes','connections']};modules=[];adoptions=[];restores=[];item_adoptions={}
    for a in accepted:
        name=a['id'];design=by_equipment[name];batch=a['review_directory'].split('/')[-1];s=source[batch];p=s['project'];sc=s['scenario'];old=name+'--normal'
        dev=next(x for x in p['devices'] if x['id']==old)
        old_hall=dev['hall_id'];old_pose=dev['pose']['translation'];new_pose=design['pose']['translation']
        mapping={old:design['id'],old_hall:'PF-H1','ALL':'PF-ALL',**{k:'PF-'+batch.lower()+'-'+k for k in ['blocker','content-unit','load-carton']}}
        # Original normal-cell entities are already uniquely equipment-prefixed.
        # Calendar names other than ALL are only used by non-normal conditions.
        selected_resources=set(dev['role_bindings']['resources'].values())
        selected_locations={x['id'] for x in p['locations'] if x['hall_id']==old_hall}
        selected_routes=[x for x in p['routes'] if any(st.get('device_id')==old for st in x['steps'])]
        route_ids={x['id'] for x in selected_routes}
        tasks=[x for x in sc['production_tasks'] if x['routing_ref']['id'] in route_ids]
        item_ids={v for v in dev['role_bindings']['item_types'].values()}
        initial={x['id']:x for x in sc['initial_conditions']['items']}
        normal_item_ids={x['id'] for x in initial.values() if x['holder']['kind']=='location' and x['holder']['id'] in selected_locations}
        while True:
            before=set(normal_item_ids)
            for identity in list(normal_item_ids):
                normal_item_ids.update(initial[identity]['contents'])
                normal_item_ids.update(initial[identity]['components'])
            if normal_item_ids==before:break
        normal_items=[x for x in initial.values() if x['id'] in normal_item_ids]
        item_ids.update(x['item_type_ref']['id'] for x in normal_items)
        for location in p['locations']:
            if location['id'] in selected_locations and location.get('item_layout'):
                item_ids.update(slot['item_type_ref']['id'] for slot in location['item_layout']['slots'])
        if dev.get('item_layout'):
            item_ids.update(slot['item_type_ref']['id'] for slot in dev['item_layout']['slots'])
        for task in tasks:
            for st in next(x for x in selected_routes if x['id']==task['routing_ref']['id'])['steps']:item_ids.update(st['item_bindings'].values())
        # Containers retain the exact types used by their nested display slots.
        # They do not create inventory, but their declarations must resolve.
        while True:
            before=set(item_ids)
            for item_type in p['item_types']:
                if item_type['id'] in item_ids and item_type.get('contents_layout'):
                    item_ids.update(slot['item_type_ref']['id'] for slot in item_type['contents_layout']['slots'])
            if item_ids==before:break
        seed_device=rewrite(copy.deepcopy(dev),mapping);seed_device.update(title=design['title'],hall_id='PF-H1',line_id=design['line_id'],pose=design['pose'],height_m=design['height_m'])
        b=design['asset_bounds'];seed_device['footprint_m']=[{'x':b['minimum'][0],'y':-b['maximum'][2]},{'x':b['maximum'][0],'y':-b['maximum'][2]},{'x':b['maximum'][0],'y':-b['minimum'][2]},{'x':b['minimum'][0],'y':-b['minimum'][2]}]
        project['devices'].append(seed_device)
        stub=dev['module_ref'];definition=next(x for x in s['definitions'] if x['id']==stub['id'])
        definitions[(definition['id'],definition['version'])]=definition
        resource_refs={x['definition_ref']['id'] for x in p['resources'] if x['id'] in selected_resources}
        for definition in s['definitions']:
            if definition['id'] in resource_refs:definitions[(definition['id'],definition['version'])]=definition
        for kind,rows in [('resources',[x for x in p['resources'] if x['id'] in selected_resources]),('item_types',[x for x in p['item_types'] if x['id'] in item_ids]),('routes',selected_routes),('locations',[x for x in p['locations'] if x['id'] in selected_locations]),('connections',[x for x in p['connections'] if x['data']['id'].startswith(old+'-')])]:
            for obj in rows:
                obj=rewrite(copy.deepcopy(obj),mapping)
                if kind=='locations':
                    t=obj['position']['translation'];t.update(x=round(new_pose['x']+t['x']-old_pose['x'],4),y=round(new_pose['y']+t['y']-old_pose['y'],4))
                    obj['title']=design['title'].split('（')[0]+' · '+('出料' if obj['id'].endswith('-out') else '投料/暂存')
                identity=obj['data']['id'] if kind=='connections' else obj['id']
                if identity in objects[kind]:assert objects[kind][identity]==obj,(kind,identity)
                objects[kind][identity]=obj
        scenario['initial_conditions']['items'].extend(rewrite(normal_items,mapping))
        scenario['production_tasks'].extend(rewrite(tasks,mapping))
        scenario['scope'].extend([{'kind':'device','id':design['id']}])
        project['setup_policies'].extend(rewrite([x for x in p['setup_policies'] if x['device_id']==old],mapping))
        if old in sc['initial_conditions']['initial_setup_families']:
            scenario['initial_conditions']['initial_setup_families'][design['id']]=sc['initial_conditions']['initial_setup_families'][old]
        selected_item_ids={x['id'] for x in scenario['initial_conditions']['items']}
        for decision in sc['initial_conditions']['declared_quality_decisions']:
            if decision['item_ref'] in selected_item_ids:scenario['initial_conditions']['declared_quality_decisions'].append(rewrite(decision,mapping))
        for preset in sc['quality_presets']:
            if old in json.dumps(preset):scenario['quality_presets'].append(rewrite(preset,mapping))
        modules.append(a['module_ref'])
        adoptions.append({'module_ref':a['module_ref'],'target_id':design['id']})
        actual=rewrite(copy.deepcopy(seed_device),{stub['id']:a['module_ref']['id']})
        def exact(v):
            if isinstance(v,dict):
                if v.get('id')==a['module_ref']['id'] and 'version' in v:v['version']=a['module_ref']['version']
                for val in v.values():exact(val)
            elif isinstance(v,list):
                for val in v:exact(val)
        exact(actual);restores.append(actual)
        item_registry=read(ROOT/'modules/media/native-equipment-500'/a['review_directory']/'items/registration.json')
        for item in item_registry['register_order']:
            if item.get('category')=='items' and item.get('item_type') in item_ids:
                item=copy.deepcopy(item);item['item_type']=mapping.get(item['item_type'],item['item_type']);item_adoptions[item['item_type']]=item
    for kind,rows in objects.items():project[kind]=list(rows.values())
    project['module_refs']=[{'id':i,'version':v} for i,v in definitions]
    base={'project':project,'scenario':scenario,'definitions':list(definitions.values())}
    write(output/'seed.json',base)
    actual_refs={'qa.seed.'+a['id']:a['module_ref'] for a in accepted}
    actual_project=copy.deepcopy(project);actual_scenario=copy.deepcopy(scenario)
    def exact_modules(v):
        if isinstance(v,dict):
            if v.get('id') in actual_refs and 'version' in v:v.update(actual_refs[v['id']])
            for value in v.values():exact_modules(value)
        elif isinstance(v,list):
            for value in v:exact_modules(value)
    exact_modules(actual_project);exact_modules(actual_scenario)
    write(output/'adoption.json',{'devices':adoptions,'restore_devices':restores,'restore_routes':actual_project['routes'],'restore_connections':actual_project['connections'],'restore_scenario':actual_scenario,'items':list(item_adoptions.values()),'item_registrations':['modules/media/native-equipment-500/reviews/'+b+'/items/registration.json' for b in sorted(batches)],'catalog':str(CATALOG.relative_to(ROOT)),
      'boundary':'Layout plus independent normal-use product cells; connected multioperation routes still pending; do not claim whole-factory acceptance.'})
    return {'devices':len(project['devices']),'tasks':len(scenario['production_tasks']),'initial_items':len(scenario['initial_conditions']['items']),'definitions':len(definitions),'source_batches':len(batches),'state':'staged_for_actual_Host','connected_factory_accepted':False}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    result=prepare(args.output);write(args.output/'preparation.json',result);print(json.dumps(result,ensure_ascii=False))
