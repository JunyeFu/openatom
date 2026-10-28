"""Check DRF05 proposed shapes, finite counterexamples and current derived consumers.

Reads canonical/Schema/TS artifacts; never regenerates them or runs the product.
"""
from pathlib import Path
from fractions import Fraction
import json
import re

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[2]


def read(ref):
    return json.loads((ROOT / ref).read_text(encoding="utf-8-sig"))


def fields(source):
    return {name: dict(re.findall(r"pub (\w+): ([^\n]+),", body))
            for name, body in re.findall(r"pub struct (\w+)\s*\{(.*?)^\}", source, re.S | re.M)}


def decimal(value):
    return Fraction(value["coefficient"], 10 ** value["decimal_places"])


def main():
    delta = read("packages/reporting/public-delta.design.json")
    cases = read("docs/04_开发与验收/特征细化/DRF05_增量正反例.json")["cases"]
    domain = (ROOT / "packages/domain/types.design.rs").read_text(encoding="utf-8")
    api = (ROOT / "packages/commands/api.design.rs").read_text(encoding="utf-8")
    canonical = fields(domain)
    spec = read("docs/03_架构与契约/全层类型与工程格式.json")
    definitions = read(spec["derived_schema_ref"])["$defs"]
    ts_root = ROOT / "tools/type-contract-proof/output/typescript"
    type_pool = set(re.findall(r"pub (?:struct|enum|type) (\w+)", domain + api))
    type_pool |= set(re.findall(r"(?:string_id!|model!)\s*\(\s*(\w+)", domain + api))
    type_pool |= set(delta["records"]) | set(delta["enums"])
    type_pool |= {"InputChangeValue", "NumericDifference", "Vec", "Option", "Box", "String", "bool"}
    for record in delta["records"].values():
        for typ in record["fields"].values():
            assert set(re.findall(r"\b[A-Za-z_][A-Za-z_0-9]*\b", typ)) <= type_pool, typ
    for variant in delta["InputChangeValue"]["variants"].values():
        variants = [variant] if isinstance(variant, str) else list(variant.values()) if isinstance(variant, dict) else []
        for typ in variants:
            assert set(re.findall(r"\b[A-Za-z_][A-Za-z_0-9]*\b", typ)) <= type_pool, typ
    assert delta["CostBasis_review"]["existing_enum_sufficient_for_selected_scope"] is True
    observations = []

    def inspect(name, expected, removed=()):
        actual = canonical.get(name, {})
        mismatches = {f: {"expected": t, "actual": actual.get(f)}
                      for f, t in expected.items() if actual.get(f) != t}
        obsolete = [f for f in removed if f in actual]
        props = definitions.get(name, {}).get("properties", {})
        schema_missing = [f for f in expected if f not in props]
        ts_file = ts_root / (name + ".ts")
        ts = ts_file.read_text(encoding="utf-8") if ts_file.exists() else ""
        ts_missing = [f for f in expected if not re.search(r"\b" + re.escape(f) + r"\??\s*:", ts)]
        observations.append({"type": name, "canonical_mismatches": mismatches,
                             "canonical_obsolete_fields": obsolete, "schema_missing_fields": schema_missing,
                             "TS_missing_fields": ts_missing,
                             "status": "merged_canonical_schema_TS_fields" if not (mismatches or obsolete or schema_missing or ts_missing)
                             else "needs_owner_sync"})

    for name, record in delta["records"].items():
        inspect(name, record["fields"])
    for mutation in delta["mutations"]:
        inspect(mutation["target"], {**mutation["add"], **mutation["replace"]}, mutation["remove"])
    enum_checks = []
    for name in delta["enums"]:
        expected = delta["enums"][name]
        actual = definitions.get(name, {}).get("enum", [])
        assert actual == expected, (name, actual, expected)
        enum_checks.append({"type": name, "schema_values": actual, "status": "exact_current_values"})
    for name in ["InputChangeValue", "NumericDifference"]:
        expected = set(delta[name]["variants"])
        actual = {v["properties"]["kind"]["const"] for v in definitions[name]["oneOf"]}
        ts_file = ts_root / (name + ".ts")
        ts = ts_file.read_text(encoding="utf-8") if ts_file.exists() else ""
        ts_missing = [v for v in expected if ('"' + v + '"') not in ts]
        observations.append({"type": name, "schema_missing_variants": sorted(expected - actual),
                             "schema_extra_variants": sorted(actual - expected), "TS_missing_variants": ts_missing,
                             "status": "merged_canonical_schema_TS_fields" if actual == expected and not ts_missing else "needs_owner_sync"})
        enum_checks.append({"type": name, "schema_variants": sorted(actual), "status": "exact_current_tagged_variants" if actual == expected else "named_delta_not_yet_in_current_schema"})
    assert not re.search(r"(?:pub enum|model!\()\s*(?:MetricKey|MetricValue|RunMetric|MetricComparison|RunComparison)\b", api)
    assert "pub struct ScenarioComparison" not in domain
    assert len(cases) == 15 and all(c["actual_result"] is None for c in cases)
    by = {c["id"]: c for c in cases}
    verified = []
    for i in range(1, 16):
        case = by[f"DRF05-D{i:02}"]
        pos, neg, exp = case["positive"], case["negative"], case["independent_expected"]
        if i == 1:
            assert pos["cause"] == "resolved" and exp["positive_end_known"] is True
            assert neg["cause"] == "observation_end" and not neg["whole_window_computed"]
            assert exp["legacy_alias_rejected"] not in delta["enums"]["WaitEndCause"]
        elif i == 2:
            assert len(pos["intervals"]) == len(pos["interval_end_causes"])
            assert len(neg["intervals"]) != len(neg["interval_end_causes"])
        elif i == 3:
            assert [r["input_revision"] for r in pos["runs"]] == exp["input_revisions"]
            assert [r["run_id"] + "/" + r["order_id"] for r in pos["runs"]] == exp["row_keys"]
            assert [r["input_revision"] for r in neg["runs"]] != exp["input_revisions"]
        elif i == 4:
            assert pos["kind"] == "changed" and pos["before"] is not None and pos["after"] is not None
            assert pos["after"]["data"]["value"]["value"] - pos["before"]["data"]["value"]["value"] == exp["change_delta_ms"]
            assert neg["before"] is None
        elif i == 5:
            assert pos["stock_change_typed"] and not pos["same_initial"]
            assert not neg["stock_change_typed"] and exp["negative_status"] == "side_by_side_only"
        elif i == 6:
            assert pos["A"] == pos["B"] and neg["A"] != neg["B"]
            assert pos["known_values"][1] - pos["known_values"][0] == exp["positive_difference"]["data"]["magnitude"]["value"]["coefficient"]
            assert pos["known_values"][0] == 0 and exp["positive_percent"] is None
            assert exp["negative_difference"] is None
        elif i == 7:
            assert pos["B"] < pos["A"] and exp["difference"]["data"]["negative"]
            assert pos["A"] - pos["B"] == exp["difference"]["data"]["magnitude"]["value"]["coefficient"]
            assert neg["unit_A"] != neg["unit_B"] and exp["negative_difference"] is None
        elif i == 8:
            assert pos["open_unit_ms"] + pos["closed_unit_ms"] == pos["held_unit_ms"]
            assert Fraction(pos["open_unit_ms"], pos["available_unit_ms"]) == Fraction(str(exp["positive_utilization"]))
            assert neg["available_unit_ms"] is None and exp["negative_utilization"] is None
        elif i == 9:
            assert decimal(pos["rate"]) == 0 and exp["positive_amount"] == 0
            assert neg["rate"] is None and exp["negative_amount"] is None
        elif i == 10:
            assert pos["measured"]["unit"] == pos["rate_quantity"]["unit"]
            assert decimal(pos["rate"]) * decimal(pos["measured"]) / decimal(pos["rate_quantity"]) == Fraction(exp["positive_amount"])
            assert neg["measured"]["unit"] != neg["rate_quantity"]["unit"] and exp["negative_amount"] is None
        elif i == 11:
            total = sum(Fraction(r["amount"]) for r in pos["lines"])
            known = sum(Fraction(r["amount"]) for r in neg["lines"] if r["amount"] is not None)
            assert total == Fraction(exp["positive_complete_total"]) == Fraction(exp["positive_known_subtotal"])
            assert total / pos["accepted"] == Fraction(exp["positive_unit_cost"])
            assert known == Fraction(exp["negative_known_subtotal"]) and exp["negative_complete_total"] is None
        elif i == 12:
            assert {r["currency"]: r["amount"] for r in pos["lines"]} == exp["currency_subtotals"]
            assert exp["cross_currency_total"] is None and not neg["whole_window_computed"] and exp["prefix_complete_total"] is None
        elif i == 13:
            assert len(pos["bases"]) == 1 and set(neg["bases"]) == {"occupied_time", "paid_calendar"}
        elif i == 14:
            assert pos["stored_origin"] == "actual_software" and neg["stored_origin"] != "actual_software"
            assert neg["caller_claim"] == "actual_software" and exp["actual_result"] is None
        else:
            assert pos["queue"]["commit_order"] == pos["queue"]["admission_order"]
            assert neg["queue"]["commit_order"] != neg["queue"]["admission_order"]
            assert pos["queue"]["commit_order"][-1] == exp["queue_last_committed_revision"]
            assert exp["restore_success_task"] == exp["restore_failure_task"] == "failed"
            assert exp["actual_Windows_result"] is None
        verified.append(case["id"])
    pending = [o for o in observations if o["status"] != "merged_canonical_schema_TS_fields"]
    result = {"document_kind": "DRF05_current_source_consumer_recheck_and_independent_delta_examples",
              "date": "2026-10-14 Asia/Shanghai", "delta_revision": delta["revision"],
              "merge_owner": "复审窗口1", "status": "passed_owned_definitions_with_named_pending_owner_sync" if pending else "passed_current_source_schema_TS_consumer_check",
              "canonical_schema_TS_observations": observations, "enum_checks": enum_checks,
              "independently_checked_positive_negative_pairs": verified,
              "Schema_TS_read_only": True, "shared_codegen_or_compile_executed_here": False,
              "pending_owner_items": pending, "actual_product_runtime": None,
              "actual_exporter": None, "actual_Windows_save": None, "actual_cross_machine": None}
    (BASE / "DRF05_消费者复核.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"PASS:15 finite positive/negative pairs; exact end-cause schemas; {len(observations)} carrier checks, {len(pending)} named owner-sync items. No product/Windows/exporter execution.")


if __name__ == "__main__":
    main()
