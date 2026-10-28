"""Read-only cross-layer guards for DQ repair; outputs stay in window6."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent


def read(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8-sig"))


def text(path):
    return (ROOT / path).read_text(encoding="utf-8-sig")


def dag(rows, field):
    graph = {r["id"]: set(r[field]) for r in rows}
    done = set()
    while True:
        ready = {key for key, deps in graph.items() if deps <= done}
        new = ready - done
        if not new:
            break
        done |= new
    return graph, done


def main():
    checks = []
    def check(id, meaning, holds, source_refs, actual):
        checks.append({"id": id, "meaning": meaning, "matched": bool(holds),
                       "source_refs": source_refs, "actual": actual})

    base = "docs/04_开发与验收/"
    selection = read("docs/00_项目基线/首版任务选择.json")["selected"]
    expected = ["P0-01","P0-02","P0-03"] + [f"P1-{n:02}" for n in range(1,6)] + [
        f"P2-{n:02}" for n in range(1,7)] + [f"P3-{n:02}" for n in range(1,5)] + ["P4-01","P4-02","P5-01"]
    trace = read(base + "首版需求验收追踪.json")
    packages = read(base + "编码任务包/任务包索引.json")["packages"]
    check("G21", "21 original task identities remain planned and packages non-dispatchable",
          [r["id"] for r in selection] == expected
          and all(r["status"] == "planned" for r in selection)
          and {r["task_id"] for r in packages} == set(expected)
          and all(r["status"] == "draft_not_dispatchable" and r["actual_dispatch"] is None for r in packages),
          ["docs/00_项目基线/首版任务选择.json",base+"编码任务包/任务包索引.json"],
          {"selected": [r["id"] for r in selection], "task_statuses": sorted({r["status"] for r in selection}),
           "package_statuses": sorted({r["status"] for r in packages})})
    for key,prefix,count in [("functional_requirements","SF",17),("nonfunctional_requirements","NFR",11),
                             ("integration_checks","IX",3)]:
        rows=trace[key]
        ids={r["id"] for r in rows}
        expected_ids={f"{prefix}-{n:02}" for n in range(1,count+1)}
        check(prefix,"Original requirement identities preserved; no runtime result inferred",
              ids==expected_ids and all(r.get("actual_result") is None for r in rows),
              [base+"首版需求验收追踪.json#/"+key],
              {"ids":sorted(ids),"actual_result_nonnull":[r["id"] for r in rows if r.get("actual_result") is not None]})
    dag_data={}
    for field in ("implementation_dependencies","release_dependencies"):
        graph,done=dag(selection,field)
        dag_data[field]={"acyclic":len(done)==21,"unknown":sorted(set().union(*graph.values())-set(expected))}
    graph,_=dag(selection,"release_dependencies")
    closure=set(graph["P5-01"])
    while True:
        expanded=closure|set().union(*(graph[x] for x in closure))
        if expanded==closure:break
        closure=expanded
    check("DAG","Startup and complete-acceptance DAGs retain correct P5 closure",
          all(v["acyclic"] and not v["unknown"] for v in dag_data.values())
          and closure==set(expected)-{"P5-01"},
          ["docs/00_项目基线/首版任务选择.json#/selected"],
          {**dag_data,"P5_release_closure":sorted(closure)})
    gate=read(base+"完整设计与执行准入.json")
    maturity=read(base+"软件开发流程与当前成熟度评估.json")
    pre=[c for p in maturity["phases"] if p["id"]=="S07" for c in p["checkpoints"]]
    design=[c for p in maturity["phases"] if p["id"] in [f"S0{n}" for n in range(1,7)] for c in p["checkpoints"]]
    check("S07","Historical design freeze does not complete actual S07 or authorize product coding",
          gate["design_freeze"]["currently_passed_design_reviews"]==30
          and len(design)==30 and len(pre)==5 and all(c["review"]["status"]=="pending" for c in pre)
          and gate["execution_entry"]["status"]=="not_ready" and gate["execution_entry"]["actual_entry"] is None,
          [base+"完整设计与执行准入.json#/execution_entry",base+"软件开发流程与当前成熟度评估.json#/phases"],
          {"design_count":len(design),"S07":[{"id":c["id"],"review":c["review"]["status"]} for c in pre],
           "execution_entry":gate["execution_entry"]})
    efforts=read(base+"任务框架与工量里程碑.json")
    sums=[sum(t["human_effort_hours"][i] for t in efforts["tasks"])+sum(w["human_effort_hours"][i] for w in efforts["shared_work"])
          for i in range(2)]
    shared={w["id"]:w for w in efforts["shared_work"]}
    milestones={m["id"]:m for m in efforts["milestones"]}
    check("DQ01-machine","Current budget and accepted design milestones remain source-consistent",
          sums==efforts["totals"]["remaining_human_hours"]==[115,189]
          and all(shared[k]["human_effort_hours"]==[0,0] and shared[k]["actual_human_hours"] is None
                  for k in ("E-DESIGN","E-TOPIC-PDF"))
          and all(milestones[f"MS-0{n}"]["status"]=="accepted_design_milestone" for n in range(1,5)),
          [base+"任务框架与工量里程碑.json#/totals",base+"阶段实现计划.json"],
          {"remaining_human_hours":sums,"MS01_04":{k:milestones[k]["status"] for k in ("MS-01","MS-02","MS-03","MS-04")},
           "actual_human_hours":efforts["totals"]["actual_human_hours"]})
    md_refs=[base+"本机并行窗口计划.md","docs/00_项目基线/技术与经济可行性.md",base+"阶段实现计划.md"]
    lines={}
    patterns=["本项窗口规格及信用","现行剩余工量只取","当前30项设计文档规格"]
    for p, pattern in zip(md_refs,patterns):
        hits=[{"line":i+1,"text":line} for i,line in enumerate(text(p).splitlines()) if pattern in line]
        lines[p]=hits
    joined="\n".join(v["text"] for values in lines.values() for v in values)
    check("DQ01-current-MD","Same current-entry questions yield next S07, delivered report and no repeated design budget",
          all(len(v)==1 for v in lines.values()) and "S07-01" in joined and "25页" in joined
          and "115—189" in joined and "E-DESIGN" in joined and "E-TOPIC-PDF" in joined
          and "MS-01至MS-04" in joined and "13项通过/47项待" not in joined
          and "余17项" not in joined and "余15项" not in joined,
          md_refs,lines)
    boundary=read("scenarios/真实业务设计/business-rule-boundaries.design.json")
    manual=read(base+"特征细化/DRF06_时间轴独立预期.json")
    legacy=read("scenarios/真实业务设计/expected-business.design.json")
    c04=read(base+"特征细化/DRF06_C04_显式绑定收口.json")
    rules=read("docs/03_架构与契约/调度与状态转换算法.json")
    types=text("packages/domain/types.design.rs")
    check("S06-boundary","Original production/time-axis cases and canonical claim/wait identities remain intact",
          [r["id"] for r in boundary["cases"]]==[f"B{n:02}" for n in range(1,19)]
          and len(manual["fixtures"])==8 and len(legacy["cases"])==6
          and len(rules["rules"])==14 and not c04["required_still_missing"] and c04["actual_Engine"] is None
          and "pub interval_end_causes: Vec<ClaimIntervalEndCause>" in types
          and "pub execution_ref: JobExecutionRef" in types
          and "pub computed_until_ms: Option<SimTimeMs>" in types,
          ["scenarios/真实业务设计/business-rule-boundaries.design.json",
           base+"特征细化/DRF06_时间轴独立预期.json","packages/domain/types.design.rs"],
          {"B_cases":len(boundary["cases"]),"manual_expected":len(manual["fixtures"]),
           "legacy_cases":len(legacy["cases"]),"algorithms":len(rules["rules"]),
           "C04_actual_Engine":c04["actual_Engine"],"old_finite_scripts_called":False})
    s04=read("tools/design-quality-audit/window6/s04-independent-result.json")
    save=read("tools/design-quality-audit/window6/save-independent-result.json")
    check("DQ03-05-source","Actual prototype source sequences cover fields, project history and professional identity",
          not s04["failed_ids"],[s04["source_ref"],"tools/design-quality-audit/window6/s04-independent-result.json"],
          {"cases":len(s04["cases"]),"failed":s04["failed_ids"],"browser":False})
    check("DQ02-consumer","Save reference rejects false success and distinguishes retained-content vs root health",
          not save["failed_ids"],[save["source_ref"],"tools/design-quality-audit/window6/save-independent-result.json"],
          {"cases":len(save["cases"]),"failed":save["failed_ids"],"Windows_IO_called":False})
    result={"document_kind":"window6_DQ_repair_cross_layer_readonly_guards","checks":checks,
            "failed_ids":[c["id"] for c in checks if not c["matched"]],
            "scope":"Repair-impact review across S01-S06 and original acceptance boundaries, not a fresh external-source or whole-product acceptance.",
            "public_synchronizers_called":False,"actual_product_runtime":None}
    (HERE/"cross-layer-result.json").write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({"checks":len(checks),"failed":result["failed_ids"]}))


if __name__=="__main__":
    main()

