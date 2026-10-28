"""Isolated type_fixture projection, using existing runtime libraries.

No Engine, Host, Commands, product exporter or actual production Run is called.
"""
from pathlib import Path
from copy import deepcopy
from fractions import Fraction
from html import escape
from html.parser import HTMLParser
import csv
import json
import re
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
LAB = ROOT / "docs/04_开发与验收/特征细化/DRF05_四格式试做"
OUT = LAB / "output"
sys.path.insert(0, str(ROOT / "tools/type-contract-proof/python-libs"))
from jsonschema import Draft202012Validator
from pypdf import PdfReader
import pypdfium2 as pdfium
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.platypus import SimpleDocTemplate, Paragraph, Table, TableStyle, Spacer, PageBreak

LABEL = "TYPE_FIXTURE - 人工构造类型样例，非生产Run，非产品导出器"
COLS = ["row_id", "run", "field", "value", "unit", "state", "reason"]


def quantity(value, unit="piece"):
    return {"value": {"coefficient": value, "decimal_places": 0}, "unit": unit}


def issue(code, message, obj=None):
    return {"code": code, "object_ref": obj, "field_path": None,
            "message": message, "blocks": ["complete_conclusion"]}


def scope(stage):
    return {"product_ref": {"id": "TF-PRODUCT", "revision": "r1"},
            "unit": "piece", "destination": "TF-STORE", "stage": stage}


def source(scenario, path, value):
    return {"target": {"kind": "scenario", "id": scenario}, "field_path": path,
            "value": {"kind": "number", "value": value},
            "basis": "template_declared_example", "source_ref": None}


def build_fixture():
    product_scope = scope("received")
    missing_price = issue("TF_MISSING_PRICE", "类型样例：运输费率未给出")
    runs = []
    for letter, revision, accepted, rate, prefix in [("A", 3, 0, 36, False),
                                                    ("B", 4, 2, 72, False),
                                                    ("C", 5, 3, 72, True)]:
        scenario = "TF-SC-" + letter
        rid = "TF-" + letter
        frontier = 50000 if prefix else 100000
        held = frontier + 35000
        accepted_scope = scope("unloaded_and_released" if prefix else "received")
        cost_inputs = []
        cost_lines = []
        for component, basis, price, currency, denominator, measured in [
                ("LAB", "occupied_time", rate, "CNY", quantity(3600000, "person_ms"), quantity(held, "person_ms")),
                ("ZERO", "explicit_disposal", 0, "CNY", quantity(1), quantity(1)),
                ("TRP", "batch_transport", None, "CNY", quantity(1, "batch"), quantity(1, "batch")),
                ("MAT", "consumed_material", 5, "USD", quantity(1), quantity(2))]:
            ident = "TF-" + component + "-" + letter
            obj = {"kind": "resource", "id": "TF-W"} if component == "LAB" else {"kind": "device", "id": "TF-D"}
            cost_inputs.append({"id": ident, "object_ref": obj, "component": component.lower(), "basis": basis,
                                "rate": None if price is None else {"coefficient": price, "decimal_places": 0},
                                "currency": currency, "rate_quantity": denominator,
                                "includes": [], "source": source(scenario, "/cost_inputs/" + str(len(cost_inputs)) + "/rate", price) if price is not None
                                else {**source(scenario, "/cost_inputs/2/rate", 0), "value": None, "basis": "unknown"}})
            amount = None if price is None else float(Fraction(price * measured["value"]["coefficient"], denominator["value"]["coefficient"]))
            cost_lines.append({"cost_input_id": ident, "object_ref": obj, "basis": basis,
                               "measured_quantity": measured, "amount": amount, "currency": currency,
                               "missing_conditions": [missing_price] if price is None else []})
        labor_amount = cost_lines[0]["amount"]
        limitations = [issue("TYPE_FIXTURE_NOT_RUN", LABEL)]
        if prefix:
            limitations.append(issue("TF_PREFIX", "类型样例：只计算到50000ms，不能当完整100000ms观察窗"))
        execution = {"kind": "project_device", "data": {"device_id": "TF-D", "route_step_id": "TF-STEP", "execution_capacity_ref": {"kind": "device", "id": "TF-D"}}}
        job = {"id": "TF-J-" + letter, "task_ref": "TF-TASK", "operation_ref": {"id": "TF-OP", "version": "0.1.0"},
               "execution_ref": execution, "input_refs": [], "job_resources": ["TF-W"], "phase_ref": "TF-PHASE",
               "phase_resources": [], "effective_progress_ms": 25000, "started_at_ms": 0,
               "finished_at_ms": None, "state": "waiting_output", "pending_outputs": []}
        causes = ["computed_boundary" if prefix else "observation_end"]
        claim = {"id": "TF-CLAIM-" + letter, "resource_id": "TF-W", "job_id": job["id"], "units": 1,
                 "intervals": [{"start_ms": 0, "end_ms": frontier}], "kind": "job", "phase_ref": None,
                 "interval_end_causes": causes}
        claim2 = {**deepcopy(claim), "id": "TF-CLAIM2-" + letter,
                  "job_id": "TF-J2-" + letter, "kind": "phase", "phase_ref": "TF-PHASE2",
                  "intervals": [{"start_ms": 0, "end_ms": 35000}], "interval_end_causes": ["released"]}
        job2 = {**deepcopy(job), "id": claim2["job_id"], "effective_progress_ms": 35000,
                "execution_ref": {"kind": "project_device", "data": {"device_id": "TF-D2", "route_step_id": "TF-STEP2", "execution_capacity_ref": {"kind": "device", "id": "TF-D2"}}},
                "phase_ref": "TF-PHASE2", "state": "completed", "finished_at_ms": 35000}
        waiting = {"object_ref": {"kind": "job", "id": job["id"]}, "execution_ref": execution,
                   "phase_ref": "TF-PHASE", "interval": {"start_ms": 25000, "end_ms": frontier},
                   "reasons": [{"kind": "output_capacity", "blocking_object": {"kind": "location", "id": "TF-STORE"},
                                "required": "类型样例接收容量", "next_known_boundary_ms": None}], "end_cause": causes[0]}
        runs.append({"run_id": rid, "input_ref": {"project_id": "TF-PROJECT", "project_revision": revision,
                                                  "scenario_id": scenario, "scenario_revision": 1},
                     "evidence_origin": "type_fixture", "scope": [{"kind": "device", "id": "TF-D"}],
                     "time_reference": {"label": "类型夹具相对毫秒", "origin_iso8601": None, "timezone": None},
                     "requested_window": {"start_ms": 0, "end_ms": 100000}, "computed_until_ms": frontier,
                     "computation_status": "paused" if prefix else "completed", "whole_window_computed": not prefix, "goal_met": None,
                     "acceptance_scope": [accepted_scope],
                     "order_metrics": [{"order_id": "TF-ORDER", "demand": quantity(3), "accepted": quantity(accepted), "outstanding": quantity(3 - accepted),
                                        "completion_ms": 50000 if accepted == 3 else None, "due_at_ms": 90000,
                                        "tardiness_ms": 0 if accepted == 3 else None}],
                     "resource_metrics": [{"resource_id": "TF-W", "any_busy_ms": frontier, "occupied_capacity_ms": held,
                                           "available_capacity_ms": frontier * 2, "utilization": held / (frontier * 2),
                                           "effective_work_ms": 35000, "waiting_ms": frontier - 25000, "capacity_unit": "person",
                                           "open_calendar_occupied_capacity_ms": held, "closed_calendar_held_capacity_ms": 0, "missing_conditions": []}],
                     "costs": {"inputs": cost_inputs, "lines": cost_lines, "currencies": [
                         {"currency": currency, "cost_input_ids": [i["id"] for i in cost_inputs if i["currency"] == currency],
                          "known_subtotal": labor_amount if currency == "CNY" else 10,
                          "complete_total": None, "accepted_scope": accepted_scope, "accepted_quantity": quantity(accepted),
                          "unit_cost": None, "missing_conditions": [missing_price] + (limitations[1:] if prefix else [])}
                         for currency in ["CNY", "USD"]], "unassigned_cost_input_ids": [], "missing_conditions": [missing_price]},
                     "events": [], "acceptance_events": [] if accepted == 0 else [{"order_id": "TF-ORDER", "item_refs": [], "quantity": quantity(accepted),
                                                                                   "destination": "TF-STORE", "acceptance_stage": accepted_scope["stage"], "at_ms": frontier}],
                     "resource_claims": [claim, claim2], "waits": [waiting],
                     "phase_work_intervals": [{"job_id": job["id"], "phase_id": "TF-PHASE", "interval": {"start_ms": 0, "end_ms": 25000},
                                                "effective_progress_start_ms": 0, "effective_progress_end_ms": 25000, "end_cause": "phase_completed"},
                                               {"job_id": job2["id"], "phase_id": "TF-PHASE2", "interval": {"start_ms": 0, "end_ms": 35000},
                                                "effective_progress_start_ms": 0, "effective_progress_end_ms": 35000, "end_cause": "phase_completed"}],
                     "limitations": limitations, "module_versions": [{"id": "TF-OP", "version": "0.1.0"}],
                     "input_assumptions": [i["source"] for i in cost_inputs], "jobs": [job, job2], "transport": []})
    pairs = []
    keys = ["time_reference", "observation_window", "acceptance_scope", "demand", "selected_scope", "initial_conditions", "supported_capabilities", "explained_changes"]
    for candidate in ["TF-B", "TF-C"]:
        mismatch = candidate == "TF-C"
        pairs.append({"baseline_run_id": "TF-A", "candidate_run_id": candidate,
                      "status": "side_by_side_only" if mismatch else "controlled_changes",
                      "conditions": [{"key": key, "matches": not (mismatch and key in ["observation_window", "acceptance_scope"]),
                                      "issues": [issue("TF_SCOPE_MISMATCH", "类型样例：接受阶段/已知计算范围不同，仅并列")] if mismatch and key in ["observation_window", "acceptance_scope"] else []} for key in keys],
                      "input_changes": [] if mismatch else [{"target": {"kind": "scenario", "id": "TF-SC-B"}, "path": "/cost_inputs/0/rate", "kind": "changed",
                                                             "before": {"kind": "scalar", "data": {"value": {"kind": "number", "value": 36}, "unit": "CNY_per_person_hour"}},
                                                             "after": {"kind": "scalar", "data": {"value": {"kind": "number", "value": 72}, "unit": "CNY_per_person_hour"}},
                                                             "basis": "template_declared_example", "reason": "类型样例费率变化；不作产量因果归因"}]})
    pairs[0]["input_changes"].append({"target": {"kind": "location", "id": "TF-STORE"}, "path": "/capacity", "kind": "changed",
                                      "before": {"kind": "scalar", "data": {"value": {"kind": "quantity", "value": quantity(0)}, "unit": "piece"}},
                                      "after": {"kind": "scalar", "data": {"value": {"kind": "quantity", "value": quantity(2)}, "unit": "piece"}},
                                      "basis": "template_declared_example", "reason": "类型样例接收容量变化，不是Engine实算因果证据"})
    comparison = {"run_ids": [r["run_id"] for r in runs], "pairs": pairs,
                  "metrics": [{"metric": "accepted_quantity", "object_ref": None, "acceptance_scope": product_scope, "currency": None,
                               "values": [{"run_id": r["run_id"], "metric": "accepted_quantity", "value": {"kind": "quantity", "data": r["order_metrics"][0]["accepted"]}} for r in runs],
                               "pair_differences": [{"baseline_run_id": "TF-A", "candidate_run_id": "TF-B", "comparable": True,
                                                     "difference": {"kind": "quantity", "data": {"negative": False, "magnitude": quantity(2)}}, "change_percent": None,
                                                     "conditions": [issue("TF_ZERO_BASE", "类型样例基值为0，不可算百分比")]},
                                                    {"baseline_run_id": "TF-A", "candidate_run_id": "TF-C", "comparable": False,
                                                     "difference": None, "change_percent": None, "conditions": [issue("TF_SIDE_BY_SIDE", "仅并列；不形成改善结论")]}]}],
                  "conclusions": ["人工type_fixture数值仅用于投影，不能作为生产改善或成本结论"], "limitations": [issue("TYPE_FIXTURE_NOT_RUN", LABEL)]}
    return {"schema_version": "1.0", "data": {"id": "TF-REPORT-DRF05", "mode": "simulation_evaluation", "project_id": "TF-PROJECT", "project_revision": 8,
            "scenario_refs": [{"id": r["input_ref"]["scenario_id"], "revision": 1} for r in runs], "run_refs": [r["run_id"] for r in runs],
            "time_reference": runs[0]["time_reference"], "observation_window": runs[0]["requested_window"], "acceptance_scope": [product_scope, runs[2]["acceptance_scope"][0]],
            "question_and_scope": LABEL, "input_assumptions": [], "module_versions": [{"id": "TF-OP", "version": "0.1.0"}], "layout_and_schedule": [],
            "scenario_comparison": comparison, "conclusions": [LABEL], "missing_conditions": [missing_price],
            "supported_scope": ["isolated serialization and presentation of a type_fixture; no production tool calls"], "limitations": [issue("TYPE_FIXTURE_NOT_RUN", LABEL)],
            "editable_project_ref": None, "asset_rights": [], "generation_context": None, "run_evaluations": runs}}


def project_rows(report):
    rows = []
    def add(run, field, value, unit="", state="known", reason=""):
        rows.append({"row_id": "R" + str(len(rows) + 1).zfill(3), "run": run, "field": field,
                     "value": "" if value is None else str(value).lower() if isinstance(value, bool) else repr(value) if isinstance(value, float) else str(value),
                     "unit": unit, "state": state, "reason": reason})
    for run in report["data"]["run_evaluations"]:
        rid = run["run_id"]; cost = run["costs"]; order = run["order_metrics"][0]; prefix = not run["whole_window_computed"]
        add(rid, "input.project_revision", run["input_ref"]["project_revision"])
        add(rid, "input.scenario_revision", run["input_ref"]["scenario_revision"])
        add(rid, "computed_until_ms", run["computed_until_ms"], "ms")
        add(rid, "whole_window_computed", run["whole_window_computed"])
        add(rid, "acceptance.stage", run["acceptance_scope"][0]["stage"])
        add(rid, "accepted.known", order["accepted"]["value"]["coefficient"], "piece", "known_prefix" if prefix else "known")
        add(rid, "accepted.whole_window", None if prefix else order["accepted"]["value"]["coefficient"], "piece", "unknown" if prefix else "known", "前缀不是全窗" if prefix else "")
        add(rid, "completion_ms", order["completion_ms"], "ms", "unknown" if order["completion_ms"] is None else "known", "未完整接受" if order["completion_ms"] is None else "")
        add(rid, "resource.occupied", run["resource_metrics"][0]["occupied_capacity_ms"], "person_ms")
        add(rid, "labor.rate", cost["inputs"][0]["rate"]["coefficient"], cost["inputs"][0]["currency"])
        add(rid, "labor.rate_quantity", cost["inputs"][0]["rate_quantity"]["value"]["coefficient"], cost["inputs"][0]["rate_quantity"]["unit"])
        add(rid, "labor.amount", cost["lines"][0]["amount"], cost["lines"][0]["currency"])
        add(rid, "zero.amount", cost["lines"][1]["amount"], "CNY", "known", "明确零费率")
        transport_rate = cost["inputs"][2]["rate"]
        add(rid, "transport.rate", None if transport_rate is None else transport_rate["coefficient"], cost["inputs"][2]["currency"], "unknown" if transport_rate is None else "known", "未给费率" if transport_rate is None else "")
        add(rid, "transport.amount", cost["lines"][2]["amount"], "CNY", "unknown", "未给费率")
        for group in cost["currencies"]:
            add(rid, group["currency"] + ".known_subtotal", group["known_subtotal"], group["currency"], "known_prefix" if prefix else "known")
        add(rid, "CNY.complete_total", cost["currencies"][0]["complete_total"], "CNY", "unknown", "所选费用缺项" + ("/前缀" if prefix else ""))
        add(rid, "CNY.unit_cost", cost["currencies"][0]["unit_cost"], "CNY/piece", "unknown", "总额未知")
        add(rid, "wait.end_cause", run["waits"][0]["end_cause"], "", "known", "截断不表示解除")
        add(rid, "claim.end_cause", run["resource_claims"][0]["interval_end_causes"][0], "", "known", "截断不表示释放")
    for pair, diff in zip(report["data"]["scenario_comparison"]["pairs"], report["data"]["scenario_comparison"]["metrics"][0]["pair_differences"]):
        key = pair["baseline_run_id"] + "/" + pair["candidate_run_id"]
        add(key, "comparison.status", pair["status"])
        magnitude = None if diff["difference"] is None else diff["difference"]["data"]["magnitude"]
        value = None if magnitude is None else magnitude["value"]["coefficient"] * (-1 if diff["difference"]["data"]["negative"] else 1)
        add(key, "quantity.difference", value, "piece" if magnitude is None else magnitude["unit"], "not_applicable" if diff["difference"] is None else "known", "仅并列不可比" if diff["difference"] is None else "")
        add(key, "change_percent", diff["change_percent"], "%", "not_applicable", "仅并列不可比" if not diff["comparable"] else "基值为0")
    return rows


class HtmlRows(HTMLParser):
    def __init__(self):
        super().__init__(); self.rows = []; self.row = []; self.cell = ""; self.active = False
    def handle_starttag(self, tag, attrs):
        if tag == "tr": self.row = []
        if tag == "td": self.cell = ""; self.active = True
    def handle_data(self, data):
        if self.active: self.cell += data
    def handle_endtag(self, tag):
        if tag == "td": self.row.append(self.cell); self.active = False
        if tag == "tr" and self.row: self.rows.append(self.row)


def main():
    report = build_fixture()
    schema = json.loads((ROOT / "tools/type-contract-proof/output/schemas/evaluation.schema.json").read_text(encoding="utf-8"))
    Draft202012Validator(schema).validate(report)
    assert all(r["evidence_origin"] == "type_fixture" for r in report["data"]["run_evaluations"])
    rows = project_rows(report)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "evaluation.type_fixture.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    with (OUT / "evaluation.type_fixture.csv").open("w", encoding="utf-8-sig", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=COLS, lineterminator="\r\n"); writer.writeheader(); writer.writerows(rows)
    head = """<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>DRF05 TYPE_FIXTURE</title><style>
body{font-family:Arial,"Microsoft YaHei",sans-serif;margin:28px;color:#21333a;background:#f7f9fa}h1{font-size:23px}p{line-height:1.6}.banner{background:#e6f0f0;border-left:5px solid #0e6070;padding:12px}.table-scroll{max-width:100%;max-height:calc(100vh - 210px);overflow:auto;overscroll-behavior:contain;border:1px solid #c9d5d9}table{border-collapse:separate;border-spacing:0;table-layout:fixed;background:white;width:1160px;font-size:13px}th,td{box-sizing:border-box;padding:7px;border-right:1px solid #c9d5d9;border-bottom:1px solid #c9d5d9;text-align:left}th{position:sticky;top:0;z-index:3;background:#0e6070;color:white}td{background:white;overflow-wrap:anywhere}tbody tr:nth-child(even) td{background:#f0f5f6}th:nth-child(1),td:nth-child(1){position:sticky;left:0;z-index:2}th:nth-child(2),td:nth-child(2){position:sticky;left:64px;z-index:2}th:nth-child(1),th:nth-child(2){z-index:4}td:nth-child(1),td:nth-child(2),td:nth-child(4),td:nth-child(5),td:nth-child(6){white-space:nowrap;overflow-wrap:normal}</style>"""
    intro = "<h1>DRF05 四格式同源投影试做</h1><p class='banner'>" + escape(LABEL) + "</p><p>导出工程修订8；样例输入修订3/4/5。JSON unknown=null，表格未知值为空并有状态/原因；PDF/HTML显示null。币种不合并，基数0不算百分比，仅并列不形成改善结论。</p>"
    table = """<div class="table-scroll" role="region" aria-label="类型样例数据表" tabindex="0"><table><colgroup><col style="width:64px"><col style="width:120px"><col style="width:240px"><col style="width:190px"><col style="width:120px"><col style="width:160px"><col style="width:266px"></colgroup><thead><tr>""" + "".join("<th>" + c + "</th>" for c in COLS) + "</tr></thead><tbody>"
    table += "".join("<tr>" + "".join("<td>" + escape("null" if c == "value" and row[c] == "" else row[c]) + "</td>" for c in COLS) + "</tr>" for row in rows)
    (OUT / "evaluation.type_fixture.html").write_text(head + intro + table + "</tbody></table></div></html>", encoding="utf-8")
    font_path = Path("C:/Windows/Fonts/simhei.ttf")
    data = font_path.read_bytes(); table_count = struct.unpack_from(">H", data, 4)[0]
    offsets = {data[12 + i * 16:16 + i * 16].decode(): struct.unpack_from(">III", data, 16 + i * 16)[1] for i in range(table_count)}
    fs_type = struct.unpack_from(">H", data, offsets["OS/2"] + 8)[0]
    assert not fs_type & (2 | 256 | 512), "font embedding/subsetting restriction"
    pdfmetrics.registerFont(TTFont("ProbeCN", str(font_path)))
    pdf_dir = OUT / "pdf"; pdf_dir.mkdir(exist_ok=True)
    pdf_path = pdf_dir / "evaluation.type_fixture.pdf"
    style = ParagraphStyle("body", fontName="ProbeCN", fontSize=9, leading=12, wordWrap="CJK")
    title = ParagraphStyle("title", fontName="ProbeCN", fontSize=18, leading=23, textColor=colors.HexColor("#0e6070"))
    story = []
    for page_group in ["TF-A", "TF-B", "TF-C", "comparison"]:
        subset = [r for r in rows if ("/" in r["run"]) == (page_group == "comparison") and (page_group == "comparison" or r["run"] == page_group)]
        story += [Paragraph("DRF05 四格式同源投影", title), Spacer(1, 10), Paragraph(escape(LABEL), style), Spacer(1, 8),
                  Paragraph("导出工程修订8。输入身份、原费率与计价分母来自同一type_fixture报告；未知值显示null，明确零值显示0。", style), Spacer(1, 9)]
        cells = [[Paragraph(escape(c), style) for c in COLS]]
        cells += [[Paragraph(escape("null" if c == "value" and row[c] == "" else row[c]), style) for c in COLS] for row in subset]
        t = Table(cells, colWidths=[30, 44, 104, 45, 56, 55, 146], repeatRows=1)
        t.setStyle(TableStyle([("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e1eeee")),
                              ("GRID", (0, 0), (-1, -1), .35, colors.HexColor("#b5c9ce")),
                              ("VALIGN", (0, 0), (-1, -1), "TOP"), ("TOPPADDING", (0, 0), (-1, -1), 4),
                              ("BOTTOMPADDING", (0, 0), (-1, -1), 4)]))
        story += [t]
        if page_group != "comparison": story.append(PageBreak())
    def footer(canvas, doc):
        canvas.setFont("ProbeCN", 8); canvas.drawString(56, 30, "TYPE_FIXTURE / 非生产Run / 非产品导出器")
        canvas.drawRightString(A4[0] - 56, 30, str(doc.page))
    SimpleDocTemplate(str(pdf_path), pagesize=A4, leftMargin=56, rightMargin=56, topMargin=50, bottomMargin=48).build(story, onFirstPage=footer, onLaterPages=footer)
    # Actual reopen, not serializer success alone.
    reopened = json.loads((OUT / "evaluation.type_fixture.json").read_text(encoding="utf-8")); assert reopened == report
    with (OUT / "evaluation.type_fixture.csv").open(encoding="utf-8-sig", newline="") as f: assert list(csv.DictReader(f)) == rows
    parsed = HtmlRows(); parsed.feed((OUT / "evaluation.type_fixture.html").read_text(encoding="utf-8")); assert parsed.rows == [["null" if c == "value" and row[c] == "" else row[c] for c in COLS] for row in rows]
    pdf = PdfReader(pdf_path); texts = [p.extract_text() for p in pdf.pages]; joined = "\n".join(texts)
    for row in rows:
        assert joined.count(row["row_id"]) == 1, row["row_id"]
    for text in texts: assert "TYPE_FIXTURE" in text and "非生产Run" in text
    parts = re.split(r"(R\d{3})", joined)
    by_id = {r["row_id"]: r for r in rows}
    for i in range(1, len(parts), 2):
        row = by_id[parts[i]]; segment = re.sub(r"\s+", "", parts[i + 1])
        for c in ["run", "field", "value", "unit", "state", "reason"]:
            value = "null" if c == "value" and row[c] == "" else row[c]
            assert re.sub(r"\s+", "", value) in segment, (row["row_id"], c, value)
    render_dir = LAB / "tmp/pdfs"; render_dir.mkdir(parents=True, exist_ok=True)
    native_pdf = pdfium.PdfDocument(str(pdf_path))
    images = []
    for i, page in enumerate(native_pdf):
        path = render_dir / ("page-" + str(i + 1) + ".png"); page.render(scale=1.8).to_pil().save(path); images.append(str(path))
    result = {"document_kind": "actual_four_format_type_fixture_projection_not_product_exporter", "date": "2026-10-14 Asia/Shanghai",
              "fixture_origin": "type_fixture", "source_report_schema_valid": True, "source_report_count": 1,
              "shared_projection_row_count": len(rows), "JSON_CSV_HTML_PDF_actual_written_and_reopened": True,
              "PDF_text_field_rows_matched": len(rows), "PDF_page_count": len(pdf.pages), "PDF_native_render_images": images,
              "PDF_renderer": "bundled ReportLab reference; not selected printpdf product path", "font": {"source": str(font_path), "fsType": fs_type, "original_font_copied": False},
              "visual_review": "pending_manual_image_inspection", "HTML_browser_open": "pending_browser_inspection",
              "actual_production_Run_created": False, "Engine_Host_Commands_called": False, "product_exporter_completed": False,
              "actual_cross_machine": None, "actual_new_Windows_replace_failure": None}
    (LAB / "projection-result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"written_and_reopened_formats": 4, "row_count": len(rows), "pdf_pages": len(pdf.pages), "origin": "type_fixture", "actual_production_Run_created": False}))


if __name__ == "__main__":
    main()
