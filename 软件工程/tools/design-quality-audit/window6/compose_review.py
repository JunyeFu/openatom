"""Compose only window6 review, preserving global audit and all repair owners' sources."""
import json
from datetime import datetime
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
TARGET=ROOT/"docs/04_开发与验收/特征细化/DQ_跨层独立复核.json"

def read(p):
    return json.loads((ROOT/p).read_text(encoding="utf-8-sig"))

audit=read("docs/04_开发与验收/设计阶段全量质量检查.json")
s04=read("tools/design-quality-audit/window6/s04-independent-result.json")
save=read("tools/design-quality-audit/window6/save-independent-result.json")
nfr=read("tools/design-quality-audit/window6/nfr-alignment-result.json")
cross=read("tools/design-quality-audit/window6/cross-layer-result.json")
w3=read("tools/design-quality-audit/window3/dq05-result.json")
w4=read("tools/design-quality-audit/window4/output/probe-result.json")
final_dq05=read("tools/design-quality-audit/window6/s04-final-dq05-result.json")
w2=read("docs/04_开发与验收/特征细化/DQ_S04修复验收.json")
browser=read(w2["actual_browser_regression"]["DQ_ref"])
browser_ok=(w2["source_stable"] is True and w2["actual_browser_regression"]["DQ_passed"] is True
            and browser["completed"] is True and len(browser["results"])==16
            and all(r["passed"] for r in browser["results"]) and not browser["page_errors"])
final_ms=datetime.fromisoformat(final_dq05["source_modified_utc"].replace("Z","+00:00")).timestamp()*1000
final_source_receipts_match=(w4["source_last_modified"]==final_dq05["source_modified_utc"]
                             and abs(w3["source_observation"]["mtime_ms"]-final_ms)<1)
gate_now=read("docs/04_开发与验收/完整设计与执行准入.json")["execution_entry"]
selection_now=read("docs/00_项目基线/首版任务选择.json")["selected"]
entry_preserved=(len(selection_now)==21 and all(t["status"]=="planned" for t in selection_now)
                 and gate_now["status"]=="not_ready" and gate_now["actual_entry"] is None)
own_fail=[{"id":c["id"],"meaning":c["meaning"],"actual":c.get("actual"),
           "error":c.get("error"),"source_ref":s04["source_ref"]} for c in s04["cases"] if not c["matched"]]
if own_fail and not (HERE/"observed-followup-counterexamples.json").exists():
    (HERE/"observed-followup-counterexamples.json").write_text(
        json.dumps({"source_modified_utc":s04["source_modified_utc"],
                    "observations":own_fail,"not_new_product_tasks":True},ensure_ascii=False,indent=2)+"\n",
        encoding="utf-8")
source_moved=s04.get("source_changed_during_execution",False)
owners_closed=w3["failed"]==0 and w4["cases_passed"]==w4["cases_total"] and not w4["blockers"]
local_ok=not own_fail and not save["failed_ids"] and not nfr["failed_ids"] and not cross["failed_ids"] and not source_moved
received=(local_ok and owners_closed and not final_dq05["failed_ids"]
          and not final_dq05["source_changed_during_execution"] and final_source_receipts_match
          and browser_ok and entry_preserved)
scope_ids={c["id"] for c in audit["coverage"]}
expected_ids={f"S0{s}-{i:02}" for s in range(1,7) for i in range(1,6)}
assert scope_ids==expected_ids
coverage=[]
for row in audit["coverage"]:
    affected=bool(row["finding_ids"])
    coverage.append({"id":row["id"],"source":row["source"],"consumer":row["consumer"],
      "repair_ids":row["finding_ids"],
      "review_kind":"repair_impact_semantic_recheck" if affected else "unchanged_scope_and_acceptance_guard",
      "evidence_ref":("tools/design-quality-audit/window6/nfr-alignment-result.json" if "DQ-02" in row["finding_ids"]
                      else "tools/design-quality-audit/window6/s04-independent-result.json" if row["id"].startswith("S04")
                      else "tools/design-quality-audit/window6/cross-layer-result.json"),
      "limit":"Repair-impact scope; prior external/source/visual/architecture evidence is retained, not newly executed." if not affected else None})
decisions=[
 {"id":"DQ-01","verdict":"received" if all(c["matched"] for c in cross["checks"] if c["id"].startswith("DQ01")) else "blocked",
  "evidence":"Current MD paragraphs agree with gate30, next S07, delivered25-page report,115-189 human/261-456 window budgets, MS01-04 accepted as authorized AI design only.",
  "refs":["tools/design-quality-audit/window6/cross-layer-result.json#/checks"]},
 {"id":"DQ-02","verdict":"received" if not nfr["failed_ids"] and not save["failed_ids"] else "blocked",
  "evidence":"All public proposed operations equal current JSON; MD four exits match. F01-03 require actual old-root full-open; F04 retained complete content and original-root health judged separately. Existing finite save consumer16 cases; window5 complete fixture/no Windows injection read.",
  "refs":["tools/design-quality-audit/window6/nfr-alignment-result.json","tools/design-quality-audit/window6/save-independent-result.json"]},
 {"id":"DQ-03","verdict":"received" if all(c["matched"] for c in s04["cases"] if c["id"].startswith("X3-")) else "blocked",
  "evidence":"Actual source apply/discard/stay, conflict and missing-field sequences; no switch or field loss on failed apply.",
  "refs":["tools/design-quality-audit/window6/s04-independent-result.json"]},
 {"id":"DQ-04","verdict":"received" if all(c["matched"] for c in s04["cases"] if c["id"].startswith("X4-")) and not w4["blockers"] else "pending_independent_followup",
  "evidence":"Old selectedUnit/definitionDraft/late-confirm counterexamples remain mandatory; current source and window4 original-function results must both close them.",
  "refs":["tools/design-quality-audit/window6/s04-independent-result.json","tools/design-quality-audit/window4/output/probe-result.json"]},
 {"id":"DQ-05","verdict":"received" if not final_dq05["failed_ids"] and w3["failed"]==0 else "blocked_current_effective_binding_sequence",
  "evidence":"Earlier geometry/preview cases retained. Current final3 source sequences prove that canceling purpose followed by old clean-form preview/same-width successful Apply cannot restore stale ack/bound; only explicit current acknowledgment permits rebind. UI flags remain distinct from irreversible historical HumanAcknowledgment.",
  "refs":["tools/design-quality-audit/window6/s04-final-dq05-result.json","tools/design-quality-audit/window3/dq05-result.json","docs/04_开发与验收/特征细化/DQ_S04修复验收.json"]}]
result={
 "document_kind":"DQ01_to_DQ05_window6_cross_layer_independent_repair_review",
 "reviewed_on":"2026-10-15 Asia/Shanghai","owner":"复审窗口6",
 "authority_ref":"docs/04_开发与验收/设计阶段全量质量检查.json#/repair_round",
 "status":"received" if received else "in_progress_waiting_current_source_and_independent_followup",
 "verdict":"received" if received else "blocked_until_observed_followup_sequences_close",
 "scope":{"stages":[f"S0{i}" for i in range(1,7)],"checkpoint_set_count":30,
          "mode":"DQ repair impact across stages and original production/timeline/admission boundaries, not a fresh full audit of every unchanged external or historical source.",
          "inventory_ref":"tools/design-quality-audit/window6/source-inventory.json",
          "write_scope":["docs/04_开发与验收/特征细化/DQ_跨层独立复核.json","tools/design-quality-audit/window6/"],
          "other_source_or_public_progress_modified":False},
 "decisions":decisions,"coverage":coverage,
 "independent_execution":{"prototype_cases":len(s04["cases"]),"prototype_failed_ids":s04["failed_ids"],
   "prototype_cases_scope":"Prior27 retained, not rerun for final delta; current final3 are in a separate result.",
   "final_delta_cases":len(final_dq05["cases"]),"final_delta_failed_ids":final_dq05["failed_ids"],
   "final_delta_ref":"tools/design-quality-audit/window6/s04-final-dq05-result.json",
   "save_reference_cases":len(save["cases"]),"save_reference_failed_ids":save["failed_ids"],
   "NFR_alignment_checks":len(nfr["checks"]),"NFR_alignment_failed_ids":nfr["failed_ids"],
   "cross_layer_guards":len(cross["checks"]),"cross_layer_failed_ids":cross["failed_ids"],
   "source_changed_during_prototype_probe":source_moved,
   "commands":["node tools/design-quality-audit/window6/s04_independent.cjs",
               "python tools/design-quality-audit/window6/save_independent.py",
               "python tools/design-quality-audit/window6/nfr_alignment.py",
               "python tools/design-quality-audit/window6/cross_layer_probe.py"],
   "public_synchronizers_or_old35_25_20_called":False},
 "final_current_source_closure":{"source_modified_utc":final_dq05["source_modified_utc"],
   "window3_and_window4_current_source_receipts_match":final_source_receipts_match,
   "window3_passed":w3["passed"],"window3_total":w3["case_count"],
   "window4_passed":w4["cases_passed"],"window4_total":w4["cases_total"],
   "window2_browser_delivery_ref":"docs/04_开发与验收/特征细化/DQ_S04修复验收.json",
   "window2_actual_Chrome_DQ_cases":len(browser["results"]),"window2_actual_Chrome_DQ_passed":browser_ok,
   "actual_browser_executed_by_window6":False,
   "original_product_entry_preserved":entry_preserved,
   "final_delta_command":"node tools/design-quality-audit/window6/s04_independent.cjs --final-dq05",
   "unchanged_save_NFR_and_broad_guards":"Existing16/7/11 evidence retained; not rerun in final delta.",
   "independent_Figma_batch_completion_required":False},
 "blocking_counterexamples":own_fail,
 "known_followup_closure":[
   {"finding":"DQ-04","prior_window4_case":"D04-13","counterexample":"Old Run selectedUnit005 survived opening C with no Run",
    "current_own_case":"X4-old-unit","current_window4_total":w4["cases_total"],"current_window4_blockers":len(w4["blockers"])},
   {"finding":"DQ-04","prior_window4_case":"D04-15","counterexample":"Stale A definitionDraft confirmed into C after project replacement",
    "current_own_case":"X4-stale-definition","current_window4_blockers":len(w4["blockers"])},
   {"finding":"DQ-05","prior_window3_case":"DQ05-revoke-current-use","counterexample":"Current purpose unchecked gave ack=false but bound=true",
    "current_own_case":"X5-purpose-uncheck","current_window3_failures":w3["failed"],
    "own_original_observation_ref":"tools/design-quality-audit/window6/observed-followup-counterexamples.json"}
 ],
 "other_independent_current_receipts":{"window3":{"passed":w3["passed"],"total":w3["case_count"],"failed":w3["failed"],"status":w3["status"]},
                                        "window4":{"passed":w4["cases_passed"],"total":w4["cases_total"],"blockers":w4["blockers"]}},
 "production_and_professional_boundary":{
   "single_truth":"simulation owns real material/quantity/holder/claims and Run time. OS16 view_time/media and UI axis changes do not create work, inventory or another simulation clock; actual source axis/LOD case keeps frozen Run/project unchanged.",
   "effective_binding":"Prototype paper.ack/paper.bound express only current sample effective UI state. Input or current-purpose mismatch must disable it.",
   "irreversible_history":"OS10/OS11/OS12 and professional_version_effects preserve actual acknowledgment/artifact/version history and old Run inputs. Local draft undo cannot revoke, create or replay historical HumanAcknowledgment; prototype has no such actual ledger or product API call.",
   "save_recovery":"Fault handling pass never implies successful new-R save or original target healthy. Real file/complete-reference content proof and normal target health remain separate.",
   "scope_preserved":"21 tasks planned /21 packages draft_not_dispatchable /17SF+11NFR+3IX identities retained; S07 actual reviews pending, execution_entry not_ready.",
   "Figma":"Latest separately authorized Figma design batch remains owner work; its completion is not an added DQ prerequisite. Product coding is still unauthorized."},
 "later_gates_preserved":["Actual Windows1176/1177 recovery and product Save/Open",
   "Actual Engine/Host18 operations and full21 task/17SF/11NFR/3IX acceptance",
   "Real professional solver, immutable acknowledgment and full external Job evidence",
   "Second machine/third-party handoff/real target-user evidence and release/submission"],
 "actual_browser_executed_here":False,"actual_product_runtime":None,
 "global_DQ_status_written":False,"original_stage_or_task_status_changed":False
}
TARGET.write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
print(json.dumps({"verdict":result["verdict"],"own_failures":[f["id"] for f in own_fail],
                  "window3":w3["failed"],"window4_blockers":len(w4["blockers"])}))

