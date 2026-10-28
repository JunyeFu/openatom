use crate::evaluation::problem;
use crate::evaluation_rows::{cell, tables};
use sf_domain::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy)]
pub enum EvaluationOutputFormat {
    Pdf,
    Html,
    Csv,
    Json,
}
#[derive(Debug, Default)]
pub struct EvaluationResources {
    pub cjk_font: Option<Vec<u8>>,
    pub font_license: Option<Vec<u8>>,
    pub files: BTreeMap<String, Vec<u8>>,
}
#[derive(Debug)]
pub struct EvaluationFile {
    pub relative_path: String,
    pub media_type: String,
    pub bytes: Vec<u8>,
}
pub(crate) fn cancelled(flag: &AtomicBool) -> Result<(), Vec<Issue>> {
    if flag.load(Ordering::Acquire) {
        Err(vec![problem("cancelled", "导出已取消；没有返回可提交文件")])
    } else {
        Ok(())
    }
}
pub(crate) fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
/// Half-up rounding of the decimal round-trip value, only for human display.
/// The report and CSV/JSON keep their unrounded base numbers.
pub(crate) fn money(n: f64) -> String {
    let raw = n.to_string();
    let sign = if raw.starts_with('-') { "-" } else { "" };
    let raw = raw.trim_start_matches('-');
    let (mantissa, exponent) = raw
        .split_once('e')
        .or_else(|| raw.split_once('E'))
        .map_or((raw, 0), |(m, e)| {
            (m, e.parse::<i32>().expect("finite exponent"))
        });
    let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32 + exponent;
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let (whole, fraction) = if point <= 0 {
        ("0".to_owned(), "0".repeat((-point) as usize) + &digits)
    } else if point as usize >= digits.len() {
        (
            digits.clone() + &"0".repeat(point as usize - digits.len()),
            String::new(),
        )
    } else {
        (
            digits[..point as usize].to_owned(),
            digits[point as usize..].to_owned(),
        )
    };
    let fraction = fraction + "000";
    let mut cents = (whole + &fraction[..2]).into_bytes();
    if fraction.as_bytes()[2] >= b'5' {
        let mut carry = true;
        for d in cents.iter_mut().rev() {
            if *d == b'9' {
                *d = b'0';
            } else {
                *d += 1;
                carry = false;
                break;
            }
        }
        if carry {
            cents.insert(0, b'1');
        }
    }
    let s = String::from_utf8(cents).expect("decimal digits");
    let split = s.len() - 2;
    format!("{sign}{}.{}", &s[..split], &s[split..])
}
fn file(path: &str, kind: &str, bytes: Vec<u8>) -> EvaluationFile {
    EvaluationFile {
        relative_path: path.into(),
        media_type: kind.into(),
        bytes,
    }
}

/// Pure byte producer. Host owns selected paths, authorization, Task/FIFO and atomic commit.
pub fn render_evaluation(
    report: &Versioned<EvaluationReport>,
    formats: &[EvaluationOutputFormat],
    resources: &EvaluationResources,
    flag: &AtomicBool,
) -> Result<Vec<EvaluationFile>, Vec<Issue>> {
    cancelled(flag)?;
    if formats.is_empty() {
        return Err(vec![problem("invalid_context", "至少选择一种报告格式")]);
    }
    if matches!(report.data.mode, ReportMode::DesignDescription)
        && (!report.data.run_refs.is_empty()
            || !report.data.run_evaluations.is_empty()
            || report.data.scenario_comparison.is_some())
    {
        return Err(vec![problem("invalid_context", "设计说明不能携带实际结果")]);
    }
    let tables = tables(report);
    let mut outputs = BTreeMap::new();
    for image in &report.data.layout_and_schedule {
        let a = &image.scene_ref.file;
        let path = a.path.as_str();
        let allowed = report.data.asset_rights.iter().any(|r| {
            r.asset_ref == a.id
                && r.redistribution_confirmed
                && r.source.as_ref().is_some_and(|s| !s.is_empty())
                && r.license.as_ref().is_some_and(|s| !s.is_empty())
        });
        if !allowed {
            return Err(vec![problem(
                "missing_asset",
                format!("图像 {} 未获当前报告的来源/许可/分发确认", a.id.as_str()),
            )]);
        }
        let bytes = resources.files.get(path).ok_or_else(|| {
            vec![problem(
                "missing_asset",
                format!("缺少明确选择的图像 {path}"),
            )]
        })?;
        if path.contains('\\')
            || path.starts_with('/')
            || path.split('/').any(|s| s == ".." || s == ".")
            || path.contains(':')
            || ["report.pdf", "report.json", "index.html"].contains(&path)
            || path.starts_with("csv/")
            || path.starts_with("licenses/")
        {
            return Err(vec![problem(
                "invalid_context",
                "图像相对路径不安全或与报告文件冲突",
            )]);
        }
        outputs.insert(path.to_owned(), file(path, "image/png", bytes.clone()));
    }
    for format in formats {
        cancelled(flag)?;
        match format {
            EvaluationOutputFormat::Json => {
                let bytes = serde_json::to_vec_pretty(report)
                    .map_err(|e| vec![problem("format_serialization", e.to_string())])?;
                outputs.insert(
                    "report.json".into(),
                    file("report.json", "application/json", bytes),
                );
            }
            EvaluationOutputFormat::Csv => {
                for table in &tables {
                    let mut writer = csv::WriterBuilder::new()
                        .terminator(csv::Terminator::CRLF)
                        .from_writer(vec![]);
                    writer
                        .write_record(&table.columns)
                        .map_err(|e| vec![problem("format_serialization", e.to_string())])?;
                    for row in &table.rows {
                        cancelled(flag)?;
                        writer
                            .write_record(table.columns.iter().map(|c| cell(&row[c])))
                            .map_err(|e| vec![problem("format_serialization", e.to_string())])?;
                    }
                    let mut bytes = vec![0xEF, 0xBB, 0xBF];
                    bytes.extend(
                        writer
                            .into_inner()
                            .map_err(|e| vec![problem("format_serialization", e.to_string())])?,
                    );
                    let path = format!("csv/{}", table.name);
                    outputs.insert(path.clone(), file(&path, "text/csv;charset=utf-8", bytes));
                }
            }
            EvaluationOutputFormat::Html => {
                let mut html = String::from(
                    "<!doctype html><html lang=\"zh-CN\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>超级工厂工程评估</title><style>body{font:16px/1.7 system-ui,'Microsoft YaHei',sans-serif;color:#19334a;background:#f1f5f8;margin:0;overflow-wrap:anywhere}main{max-width:1280px;margin:auto;padding:36px}section,details{background:white;padding:20px;margin:16px 0;border:1px solid #d5e0e8;border-radius:8px}h1{color:#075a9a}h2{font-size:22px}small{color:#536b7e}.scroll{overflow:auto}table{border-collapse:collapse;font-size:13px;width:100%}th,td{border:1px solid #d5e0e8;padding:8px;text-align:left;vertical-align:top;min-width:100px;max-width:400px;overflow-wrap:anywhere}th{background:#e6f0f7;position:sticky;top:0}input{padding:9px;width:min(700px,90%)}pre{white-space:pre-wrap;overflow-wrap:anywhere}img{max-width:100%;height:auto}.note{border-left:4px solid #d69523}summary{cursor:pointer;font-weight:600}a{color:#075a9a}.reading{font-size:15px}.reading th{width:24%;min-width:140px;position:static}.reading td{max-width:none}</style><main><h1>超级工厂 · 工程评估报告</h1>",
                );
                html.push_str(&format!("<p>报告 {} · 工程 {} · 导出修订 {}</p><section><h2>问题与范围</h2><p>{}</p><p class=note>{}</p></section>",escape(report.data.id.as_str()),escape(report.data.project_id.as_str()),report.data.project_revision.0.value(),escape(&report.data.question_and_scope),if matches!(report.data.mode,ReportMode::DesignDescription){"设计说明：无实际运行结果。"}else{"实际软件Run在声明模型与输入下的模拟结果；不是现场生产观测，也不是软件性能测量。"}));
                html.push_str("<div id=\"report-reading\">");
                for section in crate::evaluation_reading::sections(report) {
                    html.push_str(&format!(
                        "<section><h2>{}</h2><table class=reading><tbody>",
                        escape(&section.title)
                    ));
                    for (label, value) in section.rows {
                        html.push_str(&format!(
                            "<tr><th>{}</th><td>{}</td></tr>",
                            escape(&label),
                            escape(&value).replace('\n', "<br>")
                        ));
                    }
                    html.push_str("</tbody></table></section>");
                }
                html.push_str("</div>");
                for image in &report.data.layout_and_schedule {
                    html.push_str(&format!("<section><h2>{}</h2><img src=\"{}\" alt=\"{}\"><p>场景修订 {} · 视图 {}</p></section>",escape(&image.title),escape(image.scene_ref.file.path.as_str()),escape(&image.title),image.scene_ref.scene_revision.0.value(),escape(&serde_json::to_string(&image.scene_ref.view).unwrap())));
                }
                html.push_str("<h2>完整数据明细</h2><label>筛选已保存的记录：<input id=\"filter\" type=\"search\" placeholder=\"输入Run、设备、物品或等待原因\"></label><p><small>以下15张原始数据表可展开；未知为空/null，并附状态与原因。所有筛选只读取本报告已保存的数据，不联网、不重新仿真。</small></p>");
                for table in &tables {
                    cancelled(flag)?;
                    html.push_str(&format!("<details><summary>{} · {}条记录</summary><div class=scroll><table><thead><tr>",escape(&table.title),table.rows.len()));
                    for c in &table.columns {
                        html.push_str(&format!("<th>{}</th>", escape(c)));
                    }
                    html.push_str("</tr></thead><tbody>");
                    for row in &table.rows {
                        html.push_str("<tr class=data-row>");
                        for c in &table.columns {
                            let v = &row[c];
                            html.push_str(&format!("<td>{}</td>",if v.is_null(){"<span title=\"未取得或不适用，参见state/reason\">未知/不适用</span>".into()}else{escape(&if table.name=="costs.csv"&&c=="amount"{v.as_f64().map_or_else(||cell(v),money)}else{cell(v)})}));
                        }
                        html.push_str("</tr>");
                    }
                    html.push_str("</tbody></table></div></details>");
                }
                let json = serde_json::to_string(report)
                    .unwrap()
                    .replace('<', "\\u003c")
                    .replace('>', "\\u003e")
                    .replace('&', "\\u0026");
                html.push_str(&format!("<details><summary>同源完整数据与接手</summary><p>独立报告JSON不是可恢复工程快照；工程包需单独明确随附。</p><pre id=raw></pre></details><script id=report-data type=application/json>{json}</script><script>const report=JSON.parse(document.getElementById('report-data').textContent);document.getElementById('raw').textContent=JSON.stringify(report,null,2);document.getElementById('filter').addEventListener('input',e=>{{const q=e.target.value.toLocaleLowerCase();for(const row of document.querySelectorAll('.data-row'))row.hidden=!row.textContent.toLocaleLowerCase().includes(q);}});</script></main></html>"));
                outputs.insert(
                    "index.html".into(),
                    file("index.html", "text/html;charset=utf-8", html.into_bytes()),
                );
            }
            EvaluationOutputFormat::Pdf => {
                let font = resources.cjk_font.as_ref().ok_or_else(|| {
                    vec![problem(
                        "missing_font",
                        "缺少合法中文字体Source Han Sans CN2.005R",
                    )]
                })?;
                let license = resources
                    .font_license
                    .as_ref()
                    .filter(|b| !b.is_empty())
                    .ok_or_else(|| vec![problem("missing_font", "缺少字体原始OFL许可文本")])?;
                let bytes =
                    crate::evaluation_pdf::pdf(report, &tables, resources, font, formats, flag)?;
                outputs.insert(
                    "report.pdf".into(),
                    file("report.pdf", "application/pdf", bytes),
                );
                outputs.insert(
                    "licenses/SourceHanSans-LICENSE.txt".into(),
                    file(
                        "licenses/SourceHanSans-LICENSE.txt",
                        "text/plain;charset=utf-8",
                        license.clone(),
                    ),
                );
            }
        }
    }
    cancelled(flag)?;
    Ok(outputs.into_values().collect())
}
