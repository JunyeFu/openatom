"""Validate only new round3 artifacts; do not rerun credited prior trials/tests."""
import json
import re
from pathlib import Path
from urllib.parse import unquote

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).parent


def main():
    read=lambda p:json.loads(p.read_text(encoding='utf-8'))
    attachment=read(HERE/'DRF04_stdio-proof/output/attachment-lifecycle-result.json')
    assert len(attachment['cases'])==14 and attachment['old_trials_replayed'] is False
    assert attachment['actual_Engine_processes_started']==0 and attachment['actual_product_runtime'] is None
    rows={x['id']:x for x in attachment['cases']}
    assert rows['AL01']['native_result']['requested_revision']==7
    assert rows['AL02']['native_result']['old_revision_result'] is True
    for key,row in rows.items():
        result=row['native_result']
        if key not in ('AL01','AL02'):assert result['returned_bytes']==0
        if key!='AL11':assert result['owned_handle_closed'] and result['after_close_error']==6
        assert row['business_model_after']=={'state':'running','admitted':True,'cancel_calls':0,'resubmissions':0,'engine_instances':1}
    assert rows['AL11']['native_result']['handle_transferred'] is False
    assert rows['AL12']['native_result']['status']=='disconnected'
    c02=read(HERE/'DRF04_C02生成分支核验.json')
    assert len(c02['cases'])==11 and c02['generator_main_executed'] is False
    assert c02['three_template_files_unchanged'] and c02['catalog_or_devices_written'] is False
    assert c02['actual_product_runtime'] is None
    ui=read(HERE/'DRF04_H04-04界面绑定.json')
    assert len(ui['bindings'])==32 and len(ui['state_expectations'])==6
    assert all(x['delivery_status']=='definition_delivered' and x['prototype_verification']=='pending_actual_S04_04' and x['actual_prototype_result'] is None for x in ui['bindings'])
    assert all(x['actual_result'] is None for x in ui['state_expectations'])
    s05=read(HERE/'DRF04_S05接点准备.json')
    assert len(s05['contacts'])==6 and len(s05['readonly_tool_snapshot'])==8
    assert all(x['actual_result'] is None for x in s05['contacts'])
    assert s05['actual_Tauri_trial'] is None and s05['old_trials_replayed'] is False
    api=(ROOT/'packages/commands/api.design.rs').read_text(encoding='utf-8')
    assert 'model!(IpcManagedAssetAttachment' in api and 'managed_assets: Option<Vec<IpcManagedAssetAttachment>>' in api and 'ipc_task_response: IpcResponseFrame<TaskState>' in api
    connector=read(ROOT/'integrations/agent/stdio-connector.design.json')
    assert connector['actual_product_connector'] is None and connector['actual_Codex_interop'] is None
    docs=[HERE/x for x in ('DRF04_S05接点准备.md','DRF04_C02生成接入.md','DRF04_H04-04界面绑定.md')]
    links=0
    for p in docs:
        for target in re.findall(r'\[[^\]]+\]\(([^)]+)\)',p.read_text(encoding='utf-8')):
            if target.startswith(('http:','https:','#')):continue
            assert (p.parent/unquote(target.split('#')[0])).resolve().is_file(),(p,target)
            links+=1
    out={'document_kind':'new_round3_artifact_validation_not_repeat_of_prior_tests',
         'status':'passed_new_attachment_generator_S04_delivery_and_S05_preparation',
         'new_attachment_cases':14,'new_C02_cases':11,'S04_delivered_fields':32,'S04_pending_prototype_state_cases':6,'S05_prepared_contacts':6,'readonly_tools':8,'new_links_checked':links,
         'credited_prior_trials_or_40_24_7_tests_replayed':False,
         'canonical_IPC_carrier_present':True,'actual_product_Host':None,'actual_Codex':None,'actual_Tauri_trial':None,'actual_prototype':None,'human_walkthrough':None,'actual_result':None}
    (HERE/'DRF04_本轮新增核验.json').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(out,ensure_ascii=False))


if __name__=='__main__':main()
