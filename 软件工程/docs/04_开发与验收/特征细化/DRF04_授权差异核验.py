"""Independent Host-private candidate-effect policy model, never product dispatch."""
import copy
import json
import re
import sys
from datetime import datetime, timezone, timedelta
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).parent
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator


class ModelDenied(Exception):
    pass


def apply_model(state,grant,batch,origin,scope):
    """Inputs are trusted fixture projections, NOT client-normalized effects."""
    candidate=copy.deepcopy(state)
    if grant['scope']!=scope: raise ModelDenied('scope')
    for command in batch:
        branch=command['branch']
        if branch not in grant['branches']: raise ModelDenied('branch')
        if branch in ('create_project','record_artifact_acknowledgment') and origin!='interactive_ui':
            raise ModelDenied('trusted UI required')
        if branch=='adopt_imported_draft' and scope.startswith('workspace:') and origin!='interactive_ui':
            raise ModelDenied('workspace adoption UI required')
        if branch in ('undo','redo') and not command.get('history_from_Host'):
            raise ModelDenied('not Host history')
        for effect in command['effects']:
            target=effect['target']; kind=effect['kind']
            if 'source_version' in effect and effect['source_version'] not in grant['source_versions']:
                raise ModelDenied('exact selected source version')
            if kind=='reuse':
                if target not in grant['reusable_resources']: raise ModelDenied('resource reuse')
                continue
            if kind=='create':
                object_kind=target.split(':',1)[0]
                allowance=grant['creates'].get(object_kind)
                if allowance is None: raise ModelDenied('create kind')
                restore=effect.get('restore_from_history',False)
                already_created=target in candidate['created_objects']
                if already_created and not (branch=='redo' and restore):
                    raise ModelDenied('fresh ID required')
                n=sum(x.startswith(object_kind+':') for x in candidate['created_objects'])
                if not already_created and n>=allowance['max_new_objects']:
                    raise ModelDenied('cumulative create budget')
                fields=allowance['fields']
                if target in candidate['objects']: raise AssertionError('fixture creates existing ID')
            else:
                target_grant=grant['objects'].get(target)
                if target_grant is None: raise ModelDenied('target')
                if kind=='remove':
                    if not target_grant['can_remove']: raise ModelDenied('remove')
                    del candidate['objects'][target]
                    continue
                assert kind=='set' and target in candidate['objects']
                if candidate['objects'][target].get(effect['pointer'])==effect['value']:
                    continue  # normalized no-change is not a hidden leaf mutation
                fields=target_grant['fields']
            if not any(f['pointer']==effect['pointer'] and f['element_id']==effect['element_id'] for f in fields):
                raise ModelDenied('leaf/array identity')
            if kind=='create':
                candidate['objects'][target]={}
                if not already_created: candidate['created_objects'].append(target)
            candidate['objects'][target][effect['pointer']]=effect['value']
    if candidate['objects']!=state['objects']:
        candidate['revision']+=1
    return candidate


def main():
    read=lambda p:json.loads(p.read_text(encoding='utf-8'))
    cases=read(HERE/'DRF04_授权有限案例.json')
    specification=read(HERE/'DRF04_Host授权合入.json')
    api=(ROOT/'packages/commands/api.design.rs').read_text(encoding='utf-8')
    block=re.search(r'pub enum EditCommand \{(.*?)\n\}',api,re.S).group(1)
    names=re.findall(r'^    ([A-Z][A-Za-z]+)(?:\(|\s*\{|,)',block,re.M)
    snake=lambda s:re.sub(r'(?<!^)(?=[A-Z])','_',s).lower()
    branches={snake(x) for x in names}
    assert len(branches)==15
    assert branches=={x['branch'] for x in specification['all_branches']}
    positives={c['branch'] for c in cases['cases'] if c.get('branch') and c['id'].endswith('A')}
    negatives={c['branch'] for c in cases['cases'] if c.get('branch') and c['id'].endswith('D')}
    assert positives==negatives==branches
    results=[]
    for case in cases['cases']:
        state=copy.deepcopy(cases['state']);statuses=[];atomic_failures=0
        grant=copy.deepcopy(cases['base_grant'])
        # Compact fixture data only: the Host authority is prepared before the
        # projected command. These changes are not an API editing a grant.
        for change in case['grant_changes']:
            keys=[x.replace('~1','/').replace('~0','~') for x in change['pointer'][1:].split('/')]
            target=grant
            for key in keys[:-1]:target=target[key]
            target[keys[-1]]=copy.deepcopy(change['value'])
        for batch in case['batches']:
            before=copy.deepcopy(state)
            try:
                state=apply_model(state,grant,batch,case['origin'],case['scope'])
                statuses.append('ok')
            except ModelDenied:
                statuses.append('PermissionDenied')
                assert state==before,case['id']  # includes revision and create ledger
                atomic_failures+=1
        assert statuses==case['expected_statuses'],(case['id'],statuses)
        assert state['revision']==case['expected_revision'],case['id']
        results.append({'id':case['id'],'statuses':statuses,'revision':state['revision'],'atomic_denials':atomic_failures,'status':'passed_fixture_authorization_model','actual_result':None})
    context_schema=read(ROOT/'tools/type-contract-proof/output/api-schemas/get_context.request.schema.json')
    raw=read(ROOT/'tools/type-contract-proof/api-fixtures/get_context.request.json')
    raw['edit_grants']=[{'origin':'interactive_ui','objects':['all']}]
    assert list(Draft202012Validator(context_schema).iter_errors(raw))
    record={
        'document_kind':'actual_40_case_policy_model_and_standard_schema_not_real_Host',
        'date':datetime.now(timezone(timedelta(hours=8))).isoformat(),
        'status':'passed_all_15_branch_pairs_and_10_cross_cases',
        'branches':15,'allow_cases':15,'deny_cases':15,'cross_cases':10,
        'client_grant_injection':'rejected_by_current_standard_request_schema',
        'cases':results,
        'native_transport_ref':'docs/04_开发与验收/特征细化/DRF04_stdio-proof/output/native-trial-result.json',
        'authority_limits':'trusted candidate effects were handcrafted fixture projections; future Host normalization/compiler/permissions were not executed',
        'real_Host_authorization':None,'actual_agent_execution':None,
        'actual_product_runtime':None,'actual_result':None,
    }
    (HERE/'DRF04_授权差异核验.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:record[k] for k in ('status','branches','allow_cases','deny_cases','cross_cases','client_grant_injection')},ensure_ascii=False))


if __name__=='__main__':main()
