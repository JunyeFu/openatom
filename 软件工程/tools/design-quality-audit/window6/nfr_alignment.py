"""Independent contract/consumer alignment; read-only including other window outputs."""
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent

def read(p):
    return json.loads((ROOT/p).read_text(encoding="utf-8-sig"))

def text(p):
    return (ROOT/p).read_text(encoding="utf-8-sig")

def pointer(doc,p):
    for part in p.strip("/").split("/"):
        part=part.replace("~1","/").replace("~0","~")
        doc=doc[int(part)] if isinstance(doc,list) else doc[part]
    return doc

nfr=read("docs/01_业务需求/非功能需求与测量验收.json")
six=next(r for r in nfr["requirements"] if r["id"]=="NFR-06")
criteria=six["failure_exit_criteria"]
proposal=read("docs/04_开发与验收/特征细化/DQ02_保存修复交接.json")["public_json_proposal"]
checks=[]
def check(id,matched,evidence):
    checks.append({"id":id,"matched":bool(matched),"evidence":evidence})
merged=[{"op":o["op"],"path":o["path"],"matched":pointer(nfr,o["path"])==o["value"]}
        for o in proposal["operations"]]
check("NFR-actual-merge",all(o["matched"] for o in merged),merged)
groups=nfr["test_execution_contract"]["required_check_groups"]
count=sum(len(v) for v in groups.values())
check("NFR-preserved-acceptance",len(nfr["requirements"])==11 and count==38
      and six["actual_result"] is None and six["runtime_status"]=="not_executed",
      {"requirements":len(nfr["requirements"]),"required_groups":count,
       "actual_result":six["actual_result"],"runtime_status":six["runtime_status"]})
md=text("docs/01_业务需求/非功能需求与测量验收.md")
handoff=text("docs/03_架构与契约/工程交接与打开.md")
ids=[f"NFR06-F0{n}" for n in range(1,5)]
check("NFR-four-outlets",criteria["branch_ids"]==ids
      and [b["id"] for b in criteria["branches"]]==ids
      and all(id in md and id in handoff for id in ids)
      and criteria["branches"][0]["group"]=="write_failure_retention"
      and all(b["group"]=="commit_failure_retention" for b in criteria["branches"][1:]),
      {"branch_ids":ids,"new_required_groups":0,"common_terminal":criteria["common_terminal"]})
check("NFR-no-new-wire",criteria["record_binding"]["runtime_wire_fields_added"]==0
      and "现有string/null" in criteria["record_binding"]["existing_fields"]["actual_outcome"]
      and "actual_outcome" in criteria["record_binding"]["existing_fields"],
      criteria["record_binding"])
storage=read("docs/03_架构与契约/全层类型与工程格式.json")["storage"]["replacement_recovery"]
states=read("docs/03_架构与契约/共享状态与所有权.json")
api=read("docs/03_架构与契约/公开操作与宿主调用契约.json")
saveop=next(x for x in api["operations"] if x["operation"]=="save_project")
check("NFR-state-API-format-chain",
      states["file_commit_and_recovery_ref"].endswith("#/storage/replacement_recovery")
      and storage["NFR_failure_criteria_ref"].endswith("/failure_exit_criteria")
      and "no false intact/success claim" in saveop["conditions"]
      and "dirty" in saveop["conditions"]
      and "恢复失败" in "\n".join(storage["existing_target_steps"]),
      {"OS17_ref":states["file_commit_and_recovery_ref"],
       "storage_criteria_ref":storage["NFR_failure_criteria_ref"],"API_conditions":saveop["conditions"]})
io=read("tools/design-quality-audit/window5/output/dq02-result.json")
branches={b["branch_id"]:b for b in io["branches"]}
all_old=all(branches[id]["original_root_open"]["status"]=="pass" for id in ids[:3])
f4=branches["NFR06-F04"]
negative=io["backup_exists_but_reopen_rejected"]
check("NFR-actual-file-consumer",set(branches)==set(ids) and all_old
      and f4["original_root_open"]["status"]=="fail"
      and f4["retained_root_open"]["status"]=="pass"
      and f4["judgments"]["project_health_status"]=="fail"
      and f4["judgments"]["old_content_retention_status"]=="pass"
      and all(b["judgments"]["normal_save_succeeded"] is False for b in branches.values())
      and len(negative)==3 and all(n["open_evidence"]["status"]=="fail" for n in negative),
      {"fixture_scope":io["fixture_scope"],
       "old_roots_F01_F03":all_old,
       "F04_original":f4["original_root_open"],"F04_retained":f4["retained_root_open"],
       "F04_judgments":f4["judgments"],
       "negative_cases":[{"case":n["case"],"backup_exists":n["backup_exists"],"status":n["open_evidence"]["status"]} for n in negative],
       "Windows_1176_1177_injected":io["Windows_1176_1177_injected"],
       "actual_product_Save_Open_or_Host":io["product_Save_Open_or_Host_executed"]})
check("NFR-no-false-negative-pass",
      io["judgment_negative_cases"]["missing_old_content"]["branch_acceptance_status"]=="fail"
      and io["judgment_negative_cases"]["unproven_old_content"]["branch_acceptance_status"]=="not_evaluable"
      and io["judgment_negative_cases"]["unexpected_restore_failure"]["branch_acceptance_status"]=="fail",
      io["judgment_negative_cases"])
result={"document_kind":"window6_NFR06_four_outlet_contract_and_consumer_independent_alignment",
        "checks":checks,"failed_ids":[c["id"] for c in checks if not c["matched"]],
        "independent_reference_probe_ref":"tools/design-quality-audit/window6/save-independent-result.json",
        "other_window_scripts_called":False,"public_result_written":False,
        "actual_Windows_recovery":None,"actual_product_runtime":None}
(HERE/"nfr-alignment-result.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print(json.dumps({"checks":len(checks),"failed":result["failed_ids"]}))

