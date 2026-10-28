"""Derive nonsecret fixed deployment mappings from actual Host-frozen input IDs."""
import argparse,json
from pathlib import Path

def build(frozen,cloud_project_id,owner_scope_ref,telemetry_dataset_id,load_dataset_id,source_mode):
    assert frozen['document_kind']=='execution_frozen_inputs' and frozen['contract_version']=='sf.execution/2.0-draft11'
    assert frozen['identity']['project_id']==frozen['project']['id']==frozen['project_input_ref']['project_id']
    project=dict(cloud_project_id=cloud_project_id,local_project_id=frozen['project']['id'],owner_scope_ref=owner_scope_ref)
    devices={d['id']:d for d in frozen['project']['devices']}
    definitions={(d['id'],d['version']) for d in frozen['execution_definitions']}
    definitions.update((m['definition']['id'],m['definition']['version']) for m in frozen['modules'])
    bindings=[];seen_bindings=set();groups={}
    for b in frozen['candidate_bindings']:
        assert b['binding_id'] not in seen_bindings
        seen_bindings.add(b['binding_id'])
        if b.get('transport_supply_id'):
            supply=next(s for s in frozen['scenario']['supplies'] if s['id']==b['transport_supply_id'])
            assert supply['resources']['carrier']==b['device_id'] and supply['transport_ref']==b['operation_ref']
        else:
            assert devices[b['device_id']]['module_ref']==b['device_module_ref']
        assert all((b[k]['id'],b[k]['version']) in definitions for k in ['device_module_ref','operation_ref'])
        groups.setdefault(b['device_id'],[]).append(b)
    for device,stages in groups.items():
        b=stages[0]
        action='transport_cargo' if b.get('transport_supply_id') else 'prepare_equipment' if b.get('setup_transition') else 'process_unit'
        capability={'stage_bindings':stages} if len(stages)>1 else dict(action=action,operation_ref=b['operation_ref'],execution_capacity_ref=b['execution_capacity_ref'])
        bindings.append(dict(binding_id=b['binding_id'],cloud_project_id=cloud_project_id,device_id=device,principal_scope_ref='device:'+device,definition_ref=b['device_module_ref'],capability_map=capability))
    assert telemetry_dataset_id!=load_dataset_id
    return dict(projects=[project],device_bindings=bindings,datasets=[dict(dataset_id=telemetry_dataset_id,cloud_project_id=cloud_project_id,source_mode=source_mode,purpose='telemetry_only'),dict(dataset_id=load_dataset_id,cloud_project_id=cloud_project_id,source_mode='generated_load',purpose='generated_load')])

def main():
    p=argparse.ArgumentParser();p.add_argument('frozen_inputs',type=Path);p.add_argument('output',type=Path);p.add_argument('--cloud-project-id',required=True);p.add_argument('--owner-scope-ref',required=True);p.add_argument('--telemetry-dataset-id',required=True);p.add_argument('--load-dataset-id',required=True);p.add_argument('--source-mode',choices=['software_in_loop','physical_device'],required=True);a=p.parse_args()
    v=build(json.loads(a.frozen_inputs.read_text(encoding='utf-8-sig')),a.cloud_project_id,a.owner_scope_ref,a.telemetry_dataset_id,a.load_dataset_id,a.source_mode)
    a.output.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('fixed project/device/exact definition mappings written; credentials are configured separately')

if __name__=='__main__':main()
