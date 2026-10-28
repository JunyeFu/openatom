"""Finite delivery closure, profile and secret-key checks over the actual archive."""
import argparse,json
from pathlib import Path

def load(path):return json.loads(path.read_text(encoding='utf-8-sig'))
def audit(root):
    project_dir=root/'project';manifest=load(project_dir/'project.json');assert manifest['schema_version']=='1.0';manifest=manifest['data']
    docs={e['role']:e for e in manifest['documents'] if e['role'] in ['active_project','scenario_input']}
    project=load(project_dir/docs['active_project']['file_ref']['path'])['data'];scenario=load(project_dir/docs['scenario_input']['file_ref']['path'])['data']
    assert project['id']=='Task8-two-device-six-units' and scenario['id']=='Task8-six-units'
    assert len(project['lines'])==1 and len(project['devices'])==2
    initial=scenario['initial_conditions']['items'];assert len(initial)==6 and len({i['id'] for i in initial})==6
    assert all(i['quantity']=={'value':{'coefficient':1,'decimal_places':0},'unit':'piece'} and i['holder']=={'kind':'location','id':'L1-SOURCE'} for i in initial)
    assert scenario['production_tasks'][0]['planned_quantity']=={'value':{'coefficient':6,'decimal_places':0},'unit':'piece'}
    modules={};assets=[];run=None
    for entry in manifest['documents']:
        path=project_dir/entry['file_ref']['path'];assert path.is_file() or not entry['required']
        if entry['file_ref']['kind']=='module':
            module=load(path)['data'];definition=module['definition'];modules[(definition['id'],definition['version'])]=module
            original=root/'source'/entry['file_ref']['path'];assert path.read_bytes()==original.read_bytes()
            assets.extend(module['assets'])
        elif entry['file_ref']['kind']=='run_result':run=load(path)['data']
    for module in modules.values():
        for dep in module['dependencies']:assert (dep['id'],dep['version']) in modules
    for asset in assets:assert (project_dir/asset['path']).read_bytes()==(root/'source'/asset['path']).read_bytes()
    assert run and run['computation_status']=='completed' and len(run['acceptance_events'])==6
    assert all(j['execution_ref']['data']['device_id']=='L1-D01' for j in run['jobs'])
    bindings=load(root/'candidate-bindings.json');profiles=load(root/'profile-bindings.json');assert len(bindings)==len(profiles['execution_profiles'])==2
    original_profiles=load(Path(__file__).resolve().parents[2]/'resources/v2-devices/scripts/devices.demo.json')['execution_profiles'];original_profiles={p['profile_id']:p['processing_time_ms'] for p in original_profiles}
    for assignment in profiles['execution_profiles']:
        device=next(d for d in project['devices'] if d['id']==assignment['device_id']);binding=next(b for b in bindings if b['binding_id']==assignment['binding_id'])
        assert binding['device_id']==device['id'] and binding['nominal_work_ms']==device['configuration']['processing_sim_seconds']['value']==assignment['processing_time_ms']==original_profiles[assignment['profile_id']]
        source=next(s for s in project['parameter_sources'] if s['target']=={'kind':'device','id':device['id']} and s['field_path']=='/configuration/processing_sim_seconds/value')
        assert source['value']==device['configuration']['processing_sim_seconds'] and source['basis']=='user_provided'
    forbidden={'token','authorization','database_url','connection_string','execution_epoch_id','device_boot_id','session_id'}
    def visit(v):
        if isinstance(v,dict):
            assert not forbidden.intersection(k.lower() for k in v)
            for x in v.values():visit(x)
        elif isinstance(v,list):
            for x in v:visit(x)
    files=list(root.rglob('*'))
    for file in files:
        if file.is_file():
            assert file.suffix.lower() not in ['.sqlite','.db','.pyc'] and not any(part.lower() in ['target','__pycache__','journal','node_modules'] for part in file.parts)
            if file.suffix.lower()=='.json':visit(load(file))
    return dict(passed=True,project_id=project['id'],scenario_id=scenario['id'],prediction_run_id=run['id'],required_documents=sum(e['required'] for e in manifest['documents']),exact_modules=[{'id':k[0],'version':k[1]} for k in modules],required_external_assets=len(assets),input_items=6,accepted_quantity=6,profile_parameters_and_sources_match=True,original_module_and_asset_bytes_preserved=True,no_secret_or_runtime_journal_keys=True)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('bundle',type=Path);p.add_argument('receipt',type=Path);a=p.parse_args();r=audit(a.bundle.resolve());a.receipt.write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(json.dumps(r,ensure_ascii=False))
