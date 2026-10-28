"""Actual stdio consumers of the repository finite Agent policy."""
import copy,importlib.util,json
from pathlib import Path
def execute(n):
    root=n['ROOT'];out=n['OUT'];data=n['INPUT'];mode=n['MODE'];check=n['check'];ok=n['ok']
    spec=importlib.util.spec_from_file_location('sf_workflow',root/'integrations/agent/workflow.py');w=importlib.util.module_from_spec(spec);spec.loader.exec_module(w)
    client=n['Client']();p=data['project']['id'];images=[];n['RENDER_IMAGES']=images;render_race={'edited':False}
    def call(op,params):
        reply=client.call(op,params,project=p)
        if op=='get_task' and data.get('require_render'):
            from cu_render_cases import record_image
            record_image(client,reply,out,images,check)
            if data.get('concurrent_after_render') and images and not render_race['edited']:
                render_race['edited']=True
                current=w.object_at_draft(call,'project',p);settings={k:copy.deepcopy(current[k]) for k in ['title','time_reference','generation_context','sources','setup_policies']};settings['title']='Concurrent edit after actual product frame retained'
                ok(n['admin']('ui',request=n['canonical']('apply_commands',{'base_revision':current['revision'],'commands':[{'kind':'set_project_settings','data':settings}]},project=p)))
        if op=='render_view' and data.get('cancel_render') and reply['outcome']['status']=='ok':
            control=client.call('cancel_task',{'task_id':reply['outcome']['data']['id'],'action':'cancel'},project=p)
            (out/'render-cancel-request.json').write_text(json.dumps(control,ensure_ascii=False,indent=2),encoding='utf-8')
        return reply
    reply=call('get_context',{'selection':[]});client.session=reply['context']['session_id'];context=ok(reply)
    check('actual_same_project_identity',context['project']['id']==p)
    tools=[t['name'].removeprefix('superfactory_') for t in client.rpc('tools/list',{})['result']['tools']]
    (out/'actual-tools.json').write_text(json.dumps(tools,ensure_ascii=False,indent=2),encoding='utf-8')
    if mode=='workflow-catalog':
        before=w.object_at_draft(call,'project',p)
        selection=w.template_for_goal(call,data['goal'])
        check('catalog_has_no_evaluated_applicable_template',selection['status']=='missing_conditions' and selection['applicable_candidates']==[])
        check('inapplicable_exact_private_example_not_autonomously_selected',any(c['module_ref']==data['template_ref'] and not c['applicable'] and c['missing'] for c in selection['actual_library']['candidates']))
        check('rejected_autonomous_selection_preserves_entire_draft',w.object_at_draft(call,'project',p)==before)
        (out/'applicability.json').write_text(json.dumps(selection,ensure_ascii=False,indent=2),encoding='utf-8')
        client.close();return
    if mode=='workflow-no-match':
        before=w.object_at_draft(call,'project',p)
        selection=w.template_for_goal(call,data['goal'])
        check('GF03_no_matching_declared_profile',selection['status']=='missing_conditions' and selection['applicable_candidates']==[] and not any(c['module_ref']==data['template_ref'] for c in selection['actual_library']['candidates']) and all(not c['applicable'] for c in selection['actual_library']['candidates']) and selection['actual_library']['missing'])
        check('GF03_no_fake_factory_or_Run',w.object_at_draft(call,'project',p)==before and before['devices']==[] and before['scenario_refs']==[] and not any(q['operation']=='run_scenario' for q in n['REQUESTS']))
        selected=n['selected'](client,'save_project','missing-template-draft')
        saved=w.complete(call,'save_project',{'revision':before['revision'],'selected_directory_ref':selected['token']})
        check('GF03_actual_missing_scope_draft_saved',saved['task']['state']=='completed' and Path(selected['path'],'project.json').is_file())
        (out/'missing-template-result.json').write_text(json.dumps({'selection':selection,'save':saved},ensure_ascii=False,indent=2),encoding='utf-8')
        client.close();return
    if mode=='workflow-history':
        observations=[]
        for identity,original in data['workflow']['frozen_runs'].items():
            events=ok(call('get_run',{'run_id':identity,'query':{'kind':'events','data':{'interval':None,'page':{'after_index':None,'limit':1000}}}}))['data']
            check('actual_event_stream_'+identity,events['run_id']==identity and events['events']==original['events'])
            moments={};end=original['computed_until_ms']
            for at in [0,min(45000,end),min(65000,end),end,min(65000,end),0]:
                frame=ok(call('get_run',{'run_id':identity,'query':{'kind':'objects_at','data':{'at_ms':at,'objects':[],'page':{'after_index':None,'limit':1000}}}}))['data']
                check('actual_same_Run_time_'+identity+'_'+str(at),frame['run_id']==identity and frame['at_ms']==at)
                if at in moments:check('reverse_history_exact_'+identity+'_'+str(at),moments[at]==frame)
                moments[at]=frame
            check('history_reads_do_not_mutate_or_rerun_'+identity,ok(call('get_run',{'run_id':identity,'query':{'kind':'full_result'}}))['data']==original)
            observations.append({'run_id':identity,'frames':list(moments.values())})
        (out/'history.json').write_text(json.dumps(observations,ensure_ascii=False,indent=2),encoding='utf-8')
        client.close();return
    if mode=='workflow-reopen':
        old=data['workflow']['frozen_runs']
        for identity,expected in old.items():check('relocated_fresh_process_exact_'+identity,ok(call('get_run',{'run_id':identity,'query':{'kind':'full_result'}}))['data']==expected)
        scenario=w.object_at_draft(call,'scenario',data['scenario']['id']);before=copy.deepcopy(scenario)
        scenario['production_tasks'][0]['release_at_ms']+=1000
        project=w.object_at_draft(call,'project',p)
        commit=ok(call('apply_commands',{'base_revision':project['revision'],'commands':[{'kind':'put_object','data':{'kind':'scenario','data':scenario}}]}))
        check('actual_relocated_project_is_editable',commit['new_revision']==project['revision']+1)
        current=w.object_at_draft(call,'project',p)
        state=w.complete(call,'run_scenario',{'scenario_id':scenario['id'],'input_revision':current['revision'],'observation_window':scenario['window'],'intent':{'kind':'project_scenario'}})
        check('relocated_edited_scenario_really_reruns',state['task']['state']=='completed' and state['result']['data']['run_id'] not in old)
        for identity,expected in old.items():check('old_Run_after_handoff_edit_'+identity,ok(call('get_run',{'run_id':identity,'query':{'kind':'full_result'}}))['data']==expected)
        check('handoff_changes_only_release_not_supply_or_goal',w.object_at_draft(call,'scenario',scenario['id'])['initial_conditions']==before['initial_conditions'] and scenario['acceptance_goal']==before['acceptance_goal'])
        selected=n['selected'](client,'save_project','edited-after-relocation')
        save=w.complete(call,'save_project',{'revision':current['revision'],'selected_directory_ref':selected['token']})
        check('relocated_edits_saved_by_real_Host',save['task']['state']=='completed')
        if data.get('require_render'):
            for identity,expected in old.items():
                request=copy.deepcopy(data['render_request']);request['revision']=current['revision'];request['view']['project_revision']=expected['input_ref']['project_revision'];request['view']['run_id']=identity;request['view']['view_time_ms']=min(45000,expected['computed_until_ms'])
                rendered=w.complete(call,'render_view',request)
                check('relocated_actual_product_renderer_uses_exact_frozen_Run',rendered['task']['state']=='completed' and rendered['result']['data']['scene']['scene_revision']==current['revision'] and rendered['result']['data']['scene']['view']==request['view'])
                check('relocated_render_never_changes_saved_old_Run',ok(call('get_run',{'run_id':identity,'query':{'kind':'full_result'}}))['data']==expected)
            (out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8')
        client.close();return
    library=ok(call('query_module_library',{'category_filters':['templates'],'industry':data['goal']['industry'],'product_family':data['goal']['product_family'],
        'site_envelope':data['goal']['site_envelope'],'source_scope':['bundled','project_packages']}))
    if data.get('require_applicable'):
        selection=w.template_for_goal(call,data['goal'])
        check('real_catalog_declared_model_is_applicable',selection['status']=='available' and any(c['module_ref']==data['template_ref'] for c in selection['applicable_candidates']))
        check('catalog_preserves_unmeasured_and_Unknown_source_limits',any(c['module_ref']==data['template_ref'] and c['missing'] and any(s['basis']=='unknown' for s in c['template_profile']['parameter_sources']) for c in selection['applicable_candidates']))
        (out/'applicability.json').write_text(json.dumps(selection,ensure_ascii=False,indent=2),encoding='utf-8')
    check('exact_actual_template_candidate_selected',any(c['module_ref']==data['template_ref'] for c in library['candidates']))
    missing=ok(call('query_module_library',{'category_filters':['templates'],'industry':'explicit-unsupported-cu-industry','product_family':'explicit-no-template',
        'site_envelope':None,'source_scope':['bundled']}))
    check('GF03_no_fake_applicable_template',all(not c['applicable'] and c['missing'] for c in missing['candidates']))
    before=w.object_at_draft(call,'project',p)
    preview=w.instantiate(call,p,data['template_ref'],data['preview']['choices'],None)
    check('preview_uses_frozen_public_Host_producer',preview==data['preview']['unseeded']['data'])
    check('generation_preview_never_adopts_or_seeds',w.object_at_draft(call,'project',p)==before and preview['command']['explicit_new_scenario_seed'] is None)
    w.adopt_template(call,preview)
    adopted=w.object_at_draft(call,'project',p)
    check('explicit_adoption_has_no_automatic_inventory_or_orders',adopted['scenario_refs']==[])
    # Preserve the original template's missing-geometry result separately.
    raw_missing=ok(call('check_factory_draft',{'revision':adopted['revision'],'generation_goal':data['goal'],'check_scope':{'objects':[{'kind':'project','id':p}],'check_references':True,'check_business':True,'check_static_layout':True}}))
    check('unseeded_factory_missing_conditions_are_not_faked',any(r['outcome']=='undetermined' for r in raw_missing['checks']))
    (out/'original-template-missing.json').write_text(json.dumps(raw_missing,ensure_ascii=False,indent=2),encoding='utf-8')
    scenario=copy.deepcopy(data['scenario']);scenario['project_revision']=adopted['revision']
    commands=[{'kind':'put_object','data':{'kind':'scenario','data':scenario}}]
    ok(call('apply_commands',{'base_revision':adopted['revision'],'commands':commands}))
    project=w.object_at_draft(call,'project',p)
    static={'revision':project['revision'],'generation_goal':data['goal'],'check_scope':{'objects':[{'kind':'project','id':p}],'check_references':True,'check_business':True,'check_static_layout':True}}
    missing=copy.deepcopy(static);missing['generation_goal']['production_calendar_ref']=None
    unknown=ok(call('check_factory_draft',missing));check('GF02_missing_calendar_is_named_undetermined',any(r['outcome']=='undetermined' for r in unknown['checks']))
    conflict=copy.deepcopy(static);conflict['generation_goal']['site_envelope']['length_m']=.1
    conflict['generation_goal']['site_envelope']['regions'][0]['polygon_m']=[{'x':0,'y':0},{'x':.1,'y':0},{'x':.1,'y':32},{'x':0,'y':32}]
    conflicted=ok(call('check_factory_draft',conflict))
    (out/'negative-static.json').write_text(json.dumps({'missing_calendar':unknown,'site_conflict':conflicted},ensure_ascii=False,indent=2),encoding='utf-8')
    check('GF04_actual_geometry_conflict_not_scaled_away',any(r['outcome']=='fail' and r['objects'] for r in conflicted['checks']))
    check('negative_static_queries_preserve_actual_draft',w.object_at_draft(call,'project',p)==project)
    (out/'negative-static.json').write_text(json.dumps({'missing_calendar':unknown,'site_conflict':conflicted},ensure_ascii=False,indent=2),encoding='utf-8')
    start=len(n['REQUESTS'])
    def propose(state):
        if data.get('require_render'):
            rendered=state['render']
            check('actual_render_feedback_reaches_repair_policy',rendered['task']['state']=='completed' and rendered['result']['data']['scene']['scene_revision']==state['project']['revision'] and any(image['task']['id']==rendered['task']['id'] for image in images))
        if mode=='workflow-budget':
            candidate=copy.deepcopy(state['scenario']);candidate['production_tasks'][0]['release_at_ms']=[50000,30000,20000][state['committed_repair_rounds']]
            return {'objects':[{'kind':'scenario','data':candidate}],'reason':'A fixed authorized gradual release advance; preserve goal, supply, phase rules and declared80s window.'}
        if mode=='workflow-shortfall':
            return {'objects':[{'kind':'scenario','data':copy.deepcopy(state['scenario'])}],
                'reason':'Two declared supplied pieces cannot meet three; no authorized new stock or production-rule change. An identical proposal is not committed or rerun.'}
        if state['committed_repair_rounds']:
            return {'objects':[{'kind':'scenario','data':copy.deepcopy(state['scenario'])}],
                'reason':'The release-only repair did not add accepted output; two declared supplied pieces cannot meet three. No authority to alter quality, BOM, stock or cycle.'}
        candidate=copy.deepcopy(state['scenario']);candidate['production_tasks'][0]['release_at_ms']=0 if mode=='workflow-repair' else candidate['production_tasks'][0]['release_at_ms']+1000
        if mode=='workflow-conflict':
            settings={k:copy.deepcopy(state['project'][k]) for k in ['title','time_reference','generation_context','sources','setup_policies']};settings['title']='Concurrent owned UI edit retained'
            ok(n['admin']('ui',request=n['canonical']('apply_commands',{'base_revision':state['project']['revision'],'commands':[{'kind':'set_project_settings','data':settings}]},project=p)))
        return {'objects':[{'kind':'scenario','data':candidate}],'reason':'One explicitly authorized release-only candidate derived from actual schedule; supplies and production rules unchanged.'}
    result=w.run_factory(call,p,scenario['id'],data['goal'],lambda purpose,name:n['selected'](client,purpose,name),propose,tools,render_request=data.get('render_request'))
    (out/'workflow-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    segment=n['REQUESTS'][start:];expected=data['expected']
    if data.get('render_expected_stop'):
        check('real_render_boundary_stops_before_production',result['status']==data['render_expected_stop'] and result['run_ids']==[] and result['committed_repair_rounds']==0)
        check('stopped_render_policy_preserves_actual_editable_draft',result['save']['task']['state']=='completed' and Path(result['saved_directory'],'project.json').is_file() and 'report' not in result)
        check('render_stop_never_replays_or_fakes_Run',not any(q['operation'] in ['run_scenario','compare_runs'] for q in segment) if data['render_expected_stop']!='run_rejected' else sum(q['operation']=='run_scenario' for q in segment)==1)
        if data.get('concurrent_after_render'):check('render_run_conflict_keeps_concurrent_UI_title',w.object_at_draft(call,'project',p)['title']=='Concurrent edit after actual product frame retained')
        (out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8');client.close();return
    if data.get('require_render'):
        check('every_checked_revision_has_actual_product_feedback',result['render_complete'] and len(images)==len(result['checks']) and all(rendered['result']['data']['scene']['scene_revision']==checked['project_revision'] and rendered['result']['data']['scene']['view']['project_revision']==checked['project_revision'] for rendered,checked in zip(result['renders'],result['checks'])))
        (out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8')
    if mode=='workflow-missing':
        check('GF02_policy_preserves_missing_calendar_without_Run',result['status']=='missing_conditions' and result['run_ids']==[] and result['committed_repair_rounds']==0)
        check('GF02_no_fake_report_or_production',not any(q['operation'] in ['run_scenario','compare_runs','export_evaluation','apply_commands'] for q in segment))
        check('GF02_actual_editable_draft_saved',result['save']['task']['state']=='completed' and Path(result['saved_directory'],'project.json').is_file())
        client.close();return
    check('fixed_actual_acceptance_not_target_assigned',[len(r['acceptance_events']) for r in result['frozen_runs'].values()]==expected['acceptance_counts'])
    original=next(iter(result['frozen_runs'].values()));check('independent_original_acceptance_times',[e['at_ms'] for e in original['acceptance_events']]==expected['initial_acceptance_times'])
    if mode=='workflow-success':
        check('GF07_direct_one_Run_zero_repairs',result['status']=='initial_target_met' and result['committed_repair_rounds']==0 and len(result['run_ids'])==1)
        check('GF07_no_forced_apply_or_compare_after_initial_Run',not any(q['operation'] in ['apply_commands','compare_runs'] for q in segment))
    elif mode=='workflow-shortfall':
        check('GF05_identical_or_unsupported_proposal_stops_before_commit',result['status']=='unrepairable_shortfall' and result['committed_repair_rounds']==0 and len(result['run_ids'])==1 and len(result['comparisons'])==0)
        check('GF05_goal_never_reduced',all(r['goal_met'] is False for r in result['frozen_runs'].values()) and w.object_at_draft(call,'scenario',scenario['id'])['acceptance_goal']==data['goal'])
    elif mode=='workflow-repair':
        check('GF05_actual_supported_release_repair_reaches_target',result['status']=='repaired_target_met' and result['committed_repair_rounds']==1 and len(result['run_ids'])==2 and len(result['comparisons'])==1)
        repaired=list(result['frozen_runs'].values())[-1]
        check('GF05_independent_repaired_acceptance_times',[e['at_ms'] for e in repaired['acceptance_events']]==[45000,65000] and original['goal_met'] is False and repaired['goal_met'] is True)
    elif mode=='workflow-budget':
        check('GF05_three_committed_repairs_are_the_hard_limit',result['status']=='finite_budget_exhausted' and result['committed_repair_rounds']==3 and len(result['run_ids'])==4 and len(result['comparisons'])==3)
        check('GF05_budget_never_commits_a_fourth_change',sum(q['operation']=='apply_commands' for q in segment)==3 and sum(q['operation']=='run_scenario' for q in segment)==4)
        check('GF05_fixed_intermediate_acceptance_history',[[e['at_ms'] for e in r['acceptance_events']] for r in result['frozen_runs'].values()]==[[],[],[75000],[65000]])
    else:
        check('revision_conflict_retains_proposal_and_never_replays',result['status']=='edit_rejected' and result['rejected_proposal']['response']['outcome']['data']['code']=='RevisionConflict' and result['committed_repair_rounds']==0 and len(result['run_ids'])==1)
        check('concurrent_UI_title_is_preserved',w.object_at_draft(call,'project',p)['title']=='Concurrent owned UI edit retained')
    check('actual_save_and_report_tasks_completed',result['save']['task']['state']=='completed' and result['report']['task']['state']=='completed')
    files=[Path(result['report_directory'])/f['path'] for f in result['report']['result']['data']['files']]
    check('four_real_formats_from_one_selected_report',all(any(f.suffix=='.'+kind and f.is_file() and f.stat().st_size for f in files) for kind in ['pdf','html','csv','json']))
    check('real_PDF_signature',next(f for f in files if f.suffix=='.pdf').read_bytes().startswith(b'%PDF-'))
    actual=json.loads(next(f for f in files if f.suffix=='.json').read_text(encoding='utf-8'))
    check('report_contains_each_selected_actual_Run',all(identity in json.dumps(actual,ensure_ascii=False) for identity in result['run_ids']))
    (out/'workflow-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    client.close()
