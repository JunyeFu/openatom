"""DRF04 definition/Schema/finite math oracle. Does not call a Host or Engine."""
import ast
import copy
import json
import math
import re
import subprocess
import sys
from datetime import datetime, timezone, timedelta
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).with_name('DRF04_静态与数学核验.json')
sys.path.insert(0, str(ROOT / 'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads((ROOT / path).read_text(encoding='utf-8'))


def definition_result(case):
    """Independent rule/math model only; literal expectations are in finite-cases."""
    i, kind = case['input'], case['kind']
    if kind == 'sizing_math':
        return dict(ideal_takt_seconds=i['effective_seconds']/i['Q'],
                    machine_lower_bound=math.ceil(i['Q']*i['stage_t_seconds']/(i['effective_seconds']*i['accepted_per_cycle'])), production_proven=False)
    if kind == 'content_quantity_math':
        return dict(accepted_cartons=i['bundles']*i['cartons_per_bundle'], bundle_plus_content_double_count=False)
    if kind == 'unit_and_axis_math':
        return dict(hall_width_m=i['L_m'],hall_depth_m=i['W_m'],hall_height_m=i['H_m'],product_extent_m=i['product_mm']/1000)
    if kind == 'identity_remap':
        a,p,b,q=i['edge']
        return dict(edge=[i['ids'][a],p,i['ids'][b],q],module_ref_unchanged=True)
    if kind == 'default_seed':
        return dict(existing_stock=i['existing_stock'],new_seed_items=0,new_orders=0,explicit_new_scenario_seed=None)
    if kind == 'initial_success':
        assert i['checks_pass'] and i['whole_window_computed'] and i['same_unit_and_scope'] and i['accepted']>=i['target']
        return dict(stop_reason='initial_target_met',capacity_runs=1,repairs=0,compare_calls=0,revised_run_ids=[])
    if kind == 'no_change':
        assert i['proposal_before_ms']==i['proposal_after_ms'] and i['accepted']<i['target']
        return dict(stop_reason='unrepairable_shortfall',edit_calls=0,new_run_calls=0)
    if kind in ('field_permission','whole_object_permission'):
        assert not set(i['changed']).issubset(i['granted'])
        result=dict(error='PermissionDenied',committed=False)
        if kind=='whole_object_permission': result['partial_commit']=False
        return result
    if kind == 'resource_reuse':
        return dict(distinct_resources=len(set(i['role_resource_ids'])),capacity=i['existing_capacity'],new_workers=0,concurrent_jobs_still_contend=True)
    if kind == 'cancel_completion_race':
        assert i['state_before_cancel']=='completed'
        return dict(state='completed',terminal_won=True,preserve_output=i['output_exists'])
    if kind == 'cancel_request_pending':
        return dict(terminal=i['worker_terminal'],claim_cancelled_or_success=False)
    if kind == 'partial_goal':
        return dict(whole_goal_pass=i['whole_window_computed'] and i['state']=='completed' and i['accepted']>=i['target'],partial_scope_retained=True)
    if kind == 'timeline_clip_math':
        a,b=i['true_interval']; x,y=i['visible_window']
        return dict(job=i['job'],true_interval=[a,b],visible_interval=[max(a,x),min(b,y)],continued_left=a<x,continued_right=b>y,new_completion_event=False)
    if kind == 'endpoint_quantity_math':
        return dict(A_H=sum(t<=i['H_ms'] for t in i['acceptance_events_ms']),interior_display_events=sum(t<i['display_query_half_open_end'] for t in i['acceptance_events_ms']),boundary_event_separate=True,Events_page_events=sum(t<=i['display_query_half_open_end'] for t in i['acceptance_events_ms']))
    if kind == 'missing_phase_history':
        assert not i['execution_ref_present'] and not i['phase_intervals_present']
        return dict(guess_execution_device=False,reconstruct_nominal_phase=False,display='unknown_required_history')
    if kind == 'seed_order_separation':
        return dict(new_scenario_items=i['seed_items'] if i['supply_selected'] else 0,new_orders=i['template_demand'] if i['orders_selected'] else 0,mutate_active_Run=False)
    if kind == 'file_purpose_permission':
        assert i['Host_grant_purpose']!=i['request_purpose']
        return dict(error='PermissionDenied',files_written=0)
    if kind == 'lost_connection':
        assert i['run_admitted'] and i['connection_lost']
        return dict(task_owner='Host',automatic_run_resubmissions=0,second_Engine=False)
    if kind == 'three_repairs':
        assert len(i['repair_accepted'])==3 and max(i['repair_accepted'])<i['target']
        return dict(repair_rounds=3,capacity_runs=1+len(i['repair_accepted']),stop_reason='finite_budget_exhausted',fourth_repair_allowed=False)
    if kind == 'incomplete_goal_form':
        assert all(v is None for v in i.values())
        return dict(check_call=False,form_draft_saved=True,missing_inputs_asked_together=True)
    if kind == 'finite_supply_math':
        q=min(i['shell'],i['pcb'],i['fastener']//i['fastener_per_product'],i['carton'])
        return dict(ideal_max_accepted=q,minimum_shortfall=max(0,i['target']-q),invented_stock=0)
    if kind == 'authorized_change':
        assert set(i['changed']).issubset(i['granted'])
        return dict(effective_change=i['before_ms']!=i['after_ms'],repair_round_consumed_after_commit=1,fresh_check_required=True,actual_run_required_when_runnable=True)
    if kind == 'declared_straight_layout_math':
        x=i['anchor_x_m']; starts=[]
        for n,w in enumerate(i['station_widths_m']):
            if n: x+=i['declared_gap_m']
            starts.append(x); x+=w
        return dict(station_min_x_m=starts,occupied_end_x_m=x,fits_declared_x_bounds=x<=i['hall_length_m'],height_clearance_unproved=True)
    raise AssertionError('Undefined finite rule: '+kind)


def main():
    definition=read('docs/04_开发与验收/特征细化/DRF04_完整定义.json')
    gaps=read('docs/04_开发与验收/特征细化/DRF04_缺口与修复.json')
    handoff=read('docs/04_开发与验收/特征细化/DRF04_公共交接.json')
    contracts=read('modules/templates/instantiation.design.json')
    acceptance=read('scenarios/目标驱动建厂/acceptance.design.json')
    finite=read('scenarios/目标驱动建厂/finite-cases.design.json')
    examples=read('scenarios/目标驱动建厂/mcp-calls.design.json')
    catalog=read('packages/commands/command-catalog.design.json')
    assert definition['scope_preserved']==dict(SF=17,P=21,S=60,Q=64,product_coding=False,Figma_resumed=False,stage_credit_changed=False)
    for ref in definition['canonical_refs']:
        assert (ROOT/ref).is_file(), ref
    assert len(catalog['operation_catalog'])==18
    assert {c['id'] for c in acceptance['cases']}=={'GF-0'+str(i) for i in range(1,8)}
    assert definition['repair_contract']['max_repair_rounds']==3
    assert definition['repair_contract']['max_comparable_capacity_runs']==4
    assert all(c['actual_result'] is None for c in finite['cases']+acceptance['cases'])

    profile=read('modules/templates/carton-factory/generation-profile.design.json')
    mappings={x['input']:x['target'] for x in profile['canonical_bindings']}
    assert mappings['daily_accepted_quantity']=='FactoryGenerationGoal.accepted_quantity_per_day.value'
    assert mappings['quantity_unit']=='FactoryGenerationGoal.accepted_quantity_per_day.unit'
    for field in ('length','width','height'):
        assert mappings['site_'+field+'_m']=='FactoryGenerationGoal.site_envelope.'+field+'_m'
    assert 'unknown' in profile['parameter_sources'] and 'unknown_requires_input' not in profile['parameter_sources']
    assert profile['actual_result'] is None and not profile['ready_for_runtime']
    for folder in contracts['applies_to']:
        manifest=read('modules/templates/'+folder+'/manifest.json')
        project=read('modules/templates/'+folder+'/project.design.json')
        assert manifest['instantiation_contract_ref']==project['instantiation_contract_ref']=='modules/templates/instantiation.design.json'
        assert project['adoption_preview']['default_explicit_new_scenario_seed'] is None
        assert project['actual_result'] is None
        assert all(h.get('height_m') is None for h in project['halls'])
    generator=(ROOT/'modules/templates/生成三类模板设计.py').read_text(encoding='utf-8')
    ast.parse(generator)
    assert 'instantiation_contract_ref' in generator and 'height_m=None' in generator

    # Existing standard validator/library is read-only. No install or proof-generator run.
    validator_path=Path('C:/Users/fujunye/.codex/skills/.system/skill-creator/scripts/quick_validate.py')
    # Existing global Python has PyYAML; Schema's existing cp312 rpds uses the
    # bundled 3.12 runtime running this script. No new dependency is installed.
    skill_python=Path('C:/Users/fujunye/AppData/Local/Python/pythoncore-3.14-64/python.exe')
    skill_check=subprocess.run([str(skill_python),'-B','-X','utf8',str(validator_path),str(ROOT/'integrations/agent/skills/superfactory-workflow')],check=True,capture_output=True,text=True,encoding='utf-8')
    skill_message=skill_check.stdout.strip()

    schema_results=[]
    validators={}
    for call in examples['calls']:
        op=call['operation']; req=call['request']
        assert req['method']=='tools/call' and req['params']['name']=='superfactory_'+op
        schema=read('tools/type-contract-proof/output/api-schemas/'+op+'.request.schema.json')
        Draft202012Validator.check_schema(schema)
        v=Draft202012Validator(schema); validators[op]=v
        v.validate(req['params']['arguments'])
        schema_results.append(dict(operation=op,status='passed_request_schema',actual_result=None))
    negatives=[]
    for kind,op in [('caller_injection','get_context'),('missing_project_header','get_context'),('unknown_control','cancel_task')]:
        payload=copy.deepcopy(next(c['request']['params']['arguments'] for c in examples['calls'] if c['operation']==op))
        if kind=='caller_injection': payload['caller']={'origin':'ui'}
        if kind=='missing_project_header': del payload['project_id']
        if kind=='unknown_control': payload['params']['action']='force_success'
        assert list(validators[op].iter_errors(payload)),kind
        negatives.append(dict(kind=kind,expected='rejected',status='passed_schema_rejection_not_Host_permission'))

    results=[]
    for case in finite['cases']:
        computed=definition_result(case)
        assert computed==case['expected'],(case['id'],computed,case['expected'])
        results.append(dict(id=case['id'],kind=case['kind'],computed_definition_result=computed,status='passed_independent_definition_model',actual_result=None))
    markdowns=[ROOT/x for x in [
        'docs/03_架构与契约/目标驱动厂房生成.md','docs/03_架构与契约/Agent设备生成契约.md',
        'docs/03_架构与契约/通用Agent接入与专业软件连接器.md','apps/mcp-bridge/README.md','integrations/agent/clients/README.md',
        'docs/01_业务需求/三类工厂模板与接手.md','integrations/agent/README.md',
        'integrations/agent/skills/superfactory-workflow/SKILL.md','modules/templates/README.md',
        'modules/templates/carton-factory/README.md','scenarios/目标驱动建厂/README.md']]
    markdowns+=list(Path(__file__).parent.glob('DRF04_*.md'))
    checked_links=0
    for path in markdowns:
        for target in re.findall(r'\[[^\]]+\]\(([^)]+)\)',path.read_text(encoding='utf-8')):
            if target.startswith(('https:','http:','#')): continue
            rel=unquote(target.split('#')[0])
            resolved=(path.parent/rel).resolve()
            assert resolved.exists() or resolved==OUT,(str(path),target)
            checked_links+=1
    domain=(ROOT/'packages/domain/types.design.rs').read_text(encoding='utf-8')
    api=(ROOT/'packages/commands/api.design.rs').read_text(encoding='utf-8')
    job_body=re.search(r'pub struct JobState \{(.*?)\n\}',domain,re.S).group(1)
    run_body=re.search(r'pub struct RunResult \{(.*?)\n\}',domain,re.S).group(1)
    host_body=re.search(r'pub struct HostCallContext \{(.*?)\n\}',api,re.S).group(1)
    assert 'edit_grants' in host_body
    assert 'execution_ref' in job_body and 'phase_work_intervals' in run_body and 'computed_until_ms' in run_body
    assert 'HostEditGrant' not in read('tools/type-contract-proof/output/api-schemas/get_context.request.schema.json')['$defs']
    ui=read('docs/04_开发与验收/特征细化/DRF04_H04-04界面绑定.json')
    forms=read('docs/02_交互设计/首版表单与反馈.json')
    field_ids={x['ui_field_id'] for f in forms['forms'] for x in f['fields']}
    assert len(ui['bindings'])==32 and len(ui['state_expectations'])==6
    assert all(x['existing_ui_field_id'] in field_ids for x in ui['bindings'])
    assert all(x['actual_result'] is None for x in ui['state_expectations'])
    connector=read('integrations/agent/stdio-connector.design.json')
    assert connector['actual_product_connector'] is None and connector['actual_Codex_interop'] is None
    native=read(connector['actual_trials']['native_stdio_pipe_ref'])
    asset=read(connector['actual_trials']['readonly_asset_handle_ref'])
    assert native['mcp_initialized'] and native['status']=='passed_actual_native_subset'
    assert native['actual_product_runtime'] is None and native['product_operations_available']==0
    assert asset['status']=='passed_native_readonly_asset_subprocess_and_scope_negative'
    assert asset['cases'][0]['write_error']==5 and asset['cases'][1]['returned_bytes']==0
    assert asset['actual_renderer'] is None and asset['actual_product_runtime'] is None
    import tomllib
    config=tomllib.loads((ROOT/'integrations/agent/clients/codex-stdio.design.toml').read_text(encoding='utf-8'))
    assert config['mcp_servers']['superfactory']['enabled'] is False
    authority=read('docs/04_开发与验收/特征细化/DRF04_授权差异核验.json')
    assert authority['branches']==15 and len(authority['cases'])==40
    output={
        'document_kind':'actual_static_and_finite_definition_validation_not_product_acceptance',
        'date':datetime.now(timezone(timedelta(hours=8))).isoformat(),
        'status':'passed_static_schema_and_independent_definition_models',
        'command':str(Path(sys.executable))+' -B -X utf8 docs/04_开发与验收/特征细化/DRF04_验证定义.py',
        'runtime':{'schema_python':sys.version.split()[0],'schema_libraries':'existing tools/type-contract-proof/python-libs (rpds cp312)','skill_python':str(skill_python),'dependencies_installed':False},
        'six_views':len(definition['six_views']),'field_bindings':len(definition['field_bindings']),
        'owned_gap_records':len(gaps['gaps']),'public_handoff_requests':len(handoff['requests']),
        'templates_checked':4,'GF_cases_preserved':7,'finite_definition_cases':len(results),
        'math_cases':sum(c['kind'].endswith('_math') for c in finite['cases']),
        'finite_decision_cases':sum(not c['kind'].endswith('_math') for c in finite['cases']),
        'request_schema_examples':schema_results,'negative_schema_cases':negatives,
        'skill_structure':{'status':'passed','validator':str(validator_path),'message':skill_message},
        'template_emitter':{'syntax':'passed_ast_parse','generator_executed':False},
        'checked_local_markdown_links':checked_links,'case_results':results,
        'public_gap_snapshot':{'TC01_execution_ref_present':'execution_ref' in job_body,'TC02_phase_work_intervals_present':'phase_work_intervals' in run_body,'Host_edit_grants_present':'edit_grants' in host_body},
        'current_consumer_checks':{'UI_bindings':32,'UI_state_expectations':6,'all_UI_group_ids_exist':True,'authorization_model_cases':40,'authority_wire_not_exposed':True,'Events_end_point':'closed point, duration bands half-open','Codex_design_config_installed_or_enabled':False},
        'native_subset_evidence':{'official_rmcp_stdio_pipe_fixture':connector['actual_trials']['native_stdio_pipe_ref'],'readonly_handle_subprocess':connector['actual_trials']['readonly_asset_handle_ref'],'actual_scope':'transport/fixture and Win32 handles only; canonical product Host/IPC renderer/Codex not executed'},
        'actual_agent_execution':None,'real_Host_authorization':None,'actual_product_runtime':None,
        'real_prototype':None,'human_walkthrough':None,'actual_result':None,
        'scope_limit':'definition/schema/math/decision checker plus read-only references to actual native fixture/handle trials. None of 18 product operations dispatched; no actual product Host/renderer/Engine/Codex/walkthrough; shared source only read'
    }
    OUT.write_text(json.dumps(output,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:output[k] for k in ['status','templates_checked','GF_cases_preserved','finite_definition_cases','math_cases','finite_decision_cases','checked_local_markdown_links','public_gap_snapshot']},ensure_ascii=False))


if __name__=='__main__':
    main()
