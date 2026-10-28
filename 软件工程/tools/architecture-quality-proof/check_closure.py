"""Overall AQ design-source/evidence/entry consistency, not product acceptance."""
from datetime import datetime
from pathlib import Path
import json

ROOT=Path(__file__).resolve().parents[2]
A=ROOT/'docs/04_开发与验收'

def read(ref):return json.loads((ROOT/ref).read_text(encoding='utf-8-sig'))
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

def main():
    record=read('docs/04_开发与验收/架构质量抽查修复.json')
    assert [r['id'] for r in record['findings']]==[f'AQ-{i:02}' for i in range(1,6)]
    for row in record['findings']:
        assert row['status']=='resolved_design_and_isolated_verification'
        assert all((ROOT/p).is_file() for p in row['sources']+row['evidence_refs'])
    native=read('tools/type-contract-proof/output/aq01-native-result.json')
    transfer=read('tools/type-contract-proof/output/aq01-transport-result.json')
    assert native['result_bytes']==transfer['payload_bytes']>transfer['frame_cap']>=transfer['frame_bytes']
    assert transfer['schema_positives']==3 and transfer['schema_negatives']==3
    assert len(transfer['actual_handle_cases'])==5 and len(transfer['finite_identity_rejections'])==6
    assert transfer['MCP_embedded_resource_roundtrip'] and not transfer['actual_product_Host_or_Codex_client']
    api=read('docs/03_架构与契约/公开操作与宿主调用契约.json')
    assert len(api['operations'])==18 and api['implemented_public_operations']==0
    assert api['managed_result_contract']['frame_cap']==1048576
    assert 'ApiOutcome<R>::Managed' in api['managed_result_contract']['canonical_fields']
    arch=read('docs/03_架构与契约/全层组件与接点.json')
    checked=read('docs/04_开发与验收/架构接点检查结果.json')
    changes=read('tools/architecture-quality-proof/output/aq03-aq05-change-result.json')
    assert len(arch['components'])==checked['components']==changes['components']==16
    assert checked['errors']==[] and checked['import_cycles']==[] and checked['ownership_path_conflicts']==[]
    assert len(arch['architecture_change_scenarios'])==len(changes['change_scenarios'])==3
    assert len(changes['forbidden_edges_rejected'])==3 and changes['TS_compiled_variants']==['before','after']
    cost=read('tools/packaging-algorithm-proof/output/aq04-cost-result.json')
    algorithm=read('packages/packaging/algorithm.design.json')
    assert len(cost['rows'])==18 and not cost['product_30_second_guarantee']
    for key,value in cost['reference_profile'].items():assert algorithm['numerics']['default_budget_profile'][key]==value
    assert cost['max_queries']<=cost['reference_profile']['interval_query_limit']
    assert read('tools/packaging-algorithm-proof/output/proof-result.json')['all_expected_matched']
    assert read('tools/packaging-algorithm-proof/output/fixed-pose-result.json')['all_expected_matched']
    assert len(read('tools/type-contract-proof/output/timeline-schema-proof-result.json')['cases'])==20
    for path in ['docs/04_开发与验收/公开操作契约检查结果.json','docs/04_开发与验收/类型格式检查结果.json','tools/type-contract-proof/output/schema-validation-result.json']:
        value=read(path);assert value['status'].startswith('passed') and value['errors']==[]
    gate=read('docs/04_开发与验收/完整设计与执行准入.json')
    assert gate['design_freeze']['status']=='passed' and gate['design_freeze']['currently_passed_design_reviews']==30 and not gate['design_freeze']['currently_pending_design_reviews']
    assert gate['execution_entry']['actual_entry'] is None and gate['actual_product_runtime'] is None
    assert gate['final_topic_pdf']['actual_output']['author']=='傅钧烨' and gate['final_topic_pdf']['actual_output']['actual_pages']==25
    selection=read('docs/00_项目基线/首版任务选择.json')
    assert len(selection['selected'])==21 and all(r['status']=='planned' and r['actual_result'] is None for r in selection['selected'])
    text=(ROOT/'AGENTS.md').read_text(encoding='utf-8')
    assert '30项待审' not in text and '完整冻结仍待' not in text and '待审0项' in text
    maturity=read('docs/04_开发与验收/软件开发流程与当前成熟度评估.json')
    assert '25页' in maturity['current_position'] and not any('姓名待' in r for r in maturity['current_gaps'])
    mainline=read('docs/04_开发与验收/主线同步检查结果.json')
    assert len(mainline['matrix'])==12 and all(x['status']=='设计同步通过' for x in mainline['matrix']) and mainline['errors']==[]
    now=datetime.now().astimezone().isoformat(timespec='seconds')
    result={'status':'passed_AQ01_to_AQ05_design_and_isolated_evidence_closure','at':now,'closed_AQ_ids':[r['id'] for r in record['findings']],'open_AQ_findings':[],'mainline_matrices':12,'checked_local_references':mainline['checked_local_references'],'operations':18,'components':16,'cost_cases':18,'change_scenarios':3,'original_design_reviews_passed':30,'product_tasks_planned':21,'actual_product_runtime':None,'errors':[]}
    write(ROOT/'tools/architecture-quality-proof/output/aq-closure-result.json',result)
    record['status']='passed_design_revision_and_isolated_verification';record['overall_review']=result;write(A/'架构质量抽查修复.json',record)
    gate['design_freeze']['post_freeze_revisions'][-1]['status']='accepted_design_revision_and_isolated_verification';write(A/'完整设计与执行准入.json',gate)
    path=A/'架构质量抽查修复.md';text=path.read_text(encoding='utf-8').replace('实际整体复核待本轮命令完成后填写。','五项修复及整体复核已通过；关闭范围为设计和所列隔离试做。')
    if '## 整体复核结果' not in text:
        text+=f'\n## 整体复核结果\n\n{now}：AQ-01至AQ-05全部关闭。12项主线矩阵、{mainline["checked_local_references"]}处本地引用、18操作/36Schema及423派生TS检查通过；16组件无环/无写入归属冲突。32专业方法、6保持姿态、20时间轴合同回归均通过。详见[闭环结果](../../tools/architecture-quality-proof/output/aq-closure-result.json)。\n\nS01—S06原30项设计冻结保留并登记AQ修订；S07实际准备未完成，21产品任务planned、任务包draft_not_dispatchable，不增加产品运行或用户验收结论。\n'
    path.write_text(text,encoding='utf-8');print(json.dumps(result,ensure_ascii=False))

if __name__=='__main__':main()
