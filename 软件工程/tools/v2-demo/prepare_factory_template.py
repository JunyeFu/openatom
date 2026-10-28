"""Author an adoptable factory from actual frozen inputs, reusing canonical assets.

The source remains a declared demonstration: connected product routes and the
remaining independent normal-use cells are recorded separately by acceptance.
"""
import argparse
import copy
import json
import shutil
from pathlib import Path

from prepare_full_factory import ROOT, read, write

BASE=ROOT/'modules/templates/paper-packaging-factory'
REFERENCE={'id':'community.template-paper-packaging-factory','version':'1.6.0'}


def prepare(proof,staged, *, resource_root=None, output_root=None, version=None, runtime_profile=False):
    catalog_root=Path(resource_root) if resource_root else ROOT
    base=(Path(output_root) if output_root else catalog_root)/'modules/templates/paper-packaging-factory'
    reference=dict(REFERENCE,version=version or REFERENCE['version'])
    if runtime_profile and version is None:
        raise ValueError('A specialized runtime profile requires an explicit exact version')
    project=read(proof/'actual-frozen-project.json')
    scenario=read(proof/'actual-scenario.json')
    packages=read(proof/'actual-frozen-modules.json')
    by={(m['definition']['id'],m['definition']['version']):m for m in packages}
    used=set()
    def references(value):
        if isinstance(value,dict):
            key=(value.get('id'),value.get('version'))
            if key in by:used.add(key)
            for row in value.values():references(row)
        elif isinstance(value,list):
            for row in value:references(row)
    references({k:v for k,v in project.items() if k not in ({'module_refs','template_adoptions'} if runtime_profile else {'module_refs'})});references(scenario)
    while True:
        before=set(used)
        for key in list(used):references(by[key])
        if used==before:break
    adoption=read(staged/'adoption.json');index={}
    for registry in [adoption['catalog']]+adoption['item_registrations']:
        for row in read(catalog_root/registry)['register_order']:
            package=read(catalog_root/row['directory']/'module.json')['data']
            index[(package['definition']['id'],package['definition']['version'])]=(row['directory'],package)
    def original(value,prefix):
        if isinstance(value,str):return value[len(prefix):] if value.startswith(prefix) else value
        if isinstance(value,dict):return {k:original(v,prefix) for k,v in value.items()}
        if isinstance(value,list):return [original(v,prefix) for v in value]
        return value
    def optional_fields(value):
        # Rust serializes absent optional fields as null in the frozen archive.
        if isinstance(value,dict):return {k:optional_fields(v) for k,v in value.items() if v is not None}
        if isinstance(value,list):return [optional_fields(v) for v in value]
        return value
    paths={};canonical={};new_bytes=0
    for identity,version in sorted(used):
        key=(identity,version);package=by[key]
        prefix=f"bundled/{identity.encode().hex()}/{version.encode().hex()}/"
        package=original(package,prefix);canonical[key]=package
        if key in index:
            directory,expected=index[key]
            assert optional_fields(package)==optional_fields(expected),('Canonical exact version differs from actual frozen source',key)
            paths[key]=directory.removeprefix('modules/')
        else:
            assert '/' not in identity and '\\' not in identity
            directory=base/'support'/identity
            write(directory/'module.json',{'schema_version':'1.0','data':package})
            for entry in package['files']:
                source=(proof/'project'/prefix/entry['path']).resolve()
                assert source.is_relative_to((proof/'project').resolve())
                target=directory/entry['path'];target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(source,target);new_bytes+=source.stat().st_size
            paths[key]=str(directory.relative_to(base.parent.parent)).replace('\\','/')
    # Preserve declared dependency order, including exact references in media.
    ordered=[];visiting=set();done=set()
    def visit(key):
        if key in done:return
        assert key not in visiting,('Cyclic exact module dependency',key)
        visiting.add(key)
        for dep in canonical[key]['dependencies']:visit((dep['id'],dep['version']))
        visiting.remove(key);done.add(key);ordered.append(key)
    for key in sorted(used):visit(key)
    refs=[{'id':i,'version':v} for i,v in ordered]
    project=copy.deepcopy(project)
    # Authored dispatch clearance: the rendered flat carton is 12.1 mm thick.
    # Keep the frozen business proof unchanged; Native verifies this template.
    for location in project['locations']:
        if location['id']=='PF-BRANCH-OUT-carton-counter-stacker':
            for level,slot in enumerate(location['item_layout']['slots'][:6]):
                slot['offset']['translation']['z']=level*.013
    project.update(id='PAPER-FACTORY-SOURCE',title='纸包装多产品演示工厂',revision=1,module_refs=refs,scenario_refs=[scenario['id']])
    scenario=copy.deepcopy(scenario);scenario.update(revision=1,project_revision=1)
    resources=[]
    for resource in project['resources']:
        calendar=next(c for c in project['calendars'] if c['id']==resource['calendar_id'])
        resources.append({k:copy.deepcopy(resource[k]) for k in ['kind','role','capacity','qualifications','presentation']})
        resources[-1]['default_calendar_ms']=copy.deepcopy(calendar['intervals'])
    presets=[]
    for preset in scenario['quality_presets']:
        consumers={step['device_id'] for route in project['routes'] for step in route['steps'] if step['operation_ref']==preset['inspection_module']}
        assert len(consumers)==1,('Select one actual inspection consumer',preset['id'])
        presets.append(dict(copy.deepcopy(preset),consumer_device_id=next(iter(consumers))))
    adoption={'id':'paper-factory-adoption','template_ref':reference,'source_archive':{'path':'example-project.json','kind':'asset'},
        'resource_choices':[{'source_id':r['id'],'requirement_index':i,'resource_id':None} for i,r in enumerate(project['resources'])],
        'preparation_bindings':[],'imported_claims':[],'inspection_presets':presets,'deferred_seed':None,'missing_inputs':[]}
    project['template_adoptions']=[adoption]
    subset={k:copy.deepcopy(project[k]) for k in ['halls','lines','devices','connections','module_refs','item_types','locations']}
    subset.update(layout_scope='hall',resource_requirements=resources,role_requirements=[],
        suggested_role_bindings={'item_types':{},'resources':{},'targets':{},'ports':{}},
        seed={k:copy.deepcopy(scenario[k]) for k in ['initial_conditions','orders','arrivals']},adoption=adoption)
    candidate=base/'candidate'
    if (candidate/'module.json').exists():
        previous=read(candidate/'module.json')['data']['definition']
        if previous['version']!=reference['version']:
            archive=base/'versions'/previous['version']
            if not archive.exists():
                shutil.copytree(candidate,archive/'candidate')
                for name in ['registration.json','template-build.json']:shutil.copyfile(base/name,archive/name)
    for name,value in [('template.json',subset),('example-project.json',project),('source-scenario.json',scenario)]:
        if (candidate/'module.json').exists() and previous['version']==reference['version']:
            assert read(candidate/name)=={'schema_version':'1.0','data':value},('Changed template requires a new exact version',name)
        write(candidate/name,{'schema_version':'1.0','data':value})
    definition=dict(reference,title='纸包装多产品工厂 · 92台设备',definition_state='design',
        contribution={'author':'Super Factory','license':'CC0-1.0','assets':[]},parameters=[],attribute_schema=[],
        payload={'category':'templates','data':{'project':'template.json','generation_profile':None}})
    write(candidate/'module.json',{'schema_version':'1.0','data':{'definition':definition,'assets':[],
        'files':[{'path':name,'kind':kind} for name,kind in [('template.json','template'),('example-project.json','asset'),('source-scenario.json','scenario')]],
        'dependencies':refs,'rights':[],'issues':[]}})
    accepted=read(catalog_root/'modules/media/native-equipment-500/accepted-registration.json')
    already={(row['id'],row['version']) for row in accepted['register_order']}
    rows=[dict(id=i,version=v,directory=paths[(i,v)]) for i,v in ordered]
    write(base/'registration.json',{'template_ref':reference,'template_directory':'templates/paper-packaging-factory/candidate',
        'register_order':rows,'additional_register_order':[row for row in rows if (row['id'],row['version']) not in already]})
    routes={r['id']:r for r in project['routes']}
    ordered_products={o['product_ref']['id'] for o in scenario['orders']}
    ordered_routes={p['routing_ref']['id'] for p in project['products'] if p['id'] in ordered_products}
    product_routes={p['routing_ref']['id'] for p in project['products']}
    active_routes={t['routing_ref']['id'] for t in scenario['production_tasks']}
    connected={step['device_id'] for route_id in active_routes & product_routes for step in routes[route_id]['steps']}
    independent=len(project['devices'])-len(connected)
    result={'devices':len(project['devices']),'exact_dependency_modules':len(refs),'reused_canonical_modules':sum(k in index for k in used),
        'support_modules':sum(k not in index for k in used),'new_asset_bytes':new_bytes,'factory_accepted':False,
        'source':'Actual frozen project/scenario/modules; new authored template identities; existing geometry bytes reused',
        'template_ref':reference,'connected_product_routes':len(active_routes & ordered_routes),
        'component_preparation_routes':len((active_routes & product_routes)-ordered_routes),
        'connected_devices':len(connected),'independent_normal_cells':independent,
        'remaining':f'Ordinary Native template adoption and fresh reopen pending; {independent} independent normal-use cells.'}
    if runtime_profile:
        required={(row['id'],row['version']) for row in accepted['register_order']}
        assert required.issubset(used), 'All92 accepted devices and both LODs must remain available'
        profile={'schema_version':'1.0','profile':'paper-packaging-runtime','template_ref':reference,
            'register_order':[{'module_ref':{'id':row['id'],'version':row['version']},'directory':row['directory'],'kind':'module'} for row in rows]
                +[{'module_ref':reference,'directory':'templates/paper-packaging-factory/candidate','kind':'template'}]}
        write((Path(output_root) if output_root else catalog_root)/'modules/startup-library.json',profile)
        result['runtime_profile']='paper-packaging-runtime'
        result['legacy_startup_directories_required']=False
    write(base/'template-build.json',result)
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--proof',type=Path,required=True);parser.add_argument('--staged',type=Path,required=True)
    parser.add_argument('--resource-root',type=Path);parser.add_argument('--output-root',type=Path)
    parser.add_argument('--version');parser.add_argument('--runtime-profile',action='store_true')
    args=parser.parse_args();print(json.dumps(prepare(args.proof,args.staged,resource_root=args.resource_root,output_root=args.output_root,version=args.version,runtime_profile=args.runtime_profile),ensure_ascii=False))
