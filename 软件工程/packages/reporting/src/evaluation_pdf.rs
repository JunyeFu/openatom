use crate::evaluation::problem;
use crate::evaluation_output::{EvaluationOutputFormat, EvaluationResources, cancelled};
use crate::evaluation_rows::Table;
use printpdf::*;
use sf_domain::{EvaluationReport, Issue, Versioned};
use std::sync::atomic::AtomicBool;

struct Pages {
    doc: PdfDocument,
    font: FontId,
    font_metrics: ParsedFont,
    pages: Vec<PdfPage>,
    ops: Vec<Op>,
    y: f32,
    heading: String,
}
impl Pages {
    fn new(font: &[u8]) -> Result<Self, Vec<Issue>> {
        let parsed = ParsedFont::from_bytes(font, 0, &mut vec![])
            .ok_or_else(|| vec![problem("missing_font", "合法中文字体无法解析")])?;
        let mut doc = PdfDocument::new("超级工厂工程评估报告");
        let id = doc.add_font(&parsed);
        let mut p = Self {
            doc,
            font: id,
            font_metrics: parsed,
            pages: vec![],
            ops: vec![],
            y: 0.,
            heading: String::new(),
        };
        p.begin();
        Ok(p)
    }
    fn text(&mut self, text: &str, x: f32, y: f32, size: f32) {
        self.ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point { x: Pt(x), y: Pt(y) },
            },
            Op::SetFont {
                font: PdfFontHandle::External(self.font.clone()),
                size: Pt(size),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_owned())],
            },
            Op::EndTextSection,
        ]);
    }
    fn begin(&mut self) {
        self.y = 774.;
        self.text("超级工厂  |  工程评估与可编辑方案交付", 56.7, 805., 10.);
    }
    fn page(&mut self) {
        let n = self.pages.len() + 1;
        self.text(
            &format!("第 {n} 页  ·  基础数据按原单位保留；未知不补零"),
            56.7,
            32.,
            9.,
        );
        self.pages.push(PdfPage::new(
            Mm(210.),
            Mm(297.),
            std::mem::take(&mut self.ops),
        ));
        self.begin();
        if !self.heading.is_empty() {
            let title = format!("{}（续）", self.heading);
            self.line(&title, 12.);
            self.y -= 4.;
        }
    }
    fn line(&mut self, s: &str, size: f32) {
        // Use the same font advances as the PDF writer. Wide Latin identifiers
        // cannot be measured as half-width characters without clipping.
        let mut line = String::new();
        let mut width = 0.;
        for c in s.chars() {
            let w = self
                .font_metrics
                .lookup_glyph_index(c as u32)
                .and_then(|gid| self.font_metrics.get_or_decode_glyph(gid))
                .map_or(0., |g| g.horz_advance as f32)
                * size
                / self.font_metrics.pdf_font_metrics.units_per_em as f32;
            if c == '\n' || width + w > 480. {
                if self.y < 57. {
                    self.page();
                }
                self.text(&line, 56.7, self.y, size);
                self.y -= size * 1.45;
                line.clear();
                width = 0.;
                if c == '\n' {
                    continue;
                }
            }
            line.push(c);
            width += w;
        }
        if !line.is_empty() {
            if self.y < 57. {
                self.page();
            }
            self.text(&line, 56.7, self.y, size);
            self.y -= size * 1.45;
        }
    }
    fn section(&mut self, title: &str) {
        if self.y < 110. {
            self.heading.clear();
            self.page();
        }
        self.heading = title.to_owned();
        self.y -= 10.;
        self.line(title, 14.);
        self.y -= 5.;
    }
    fn wrap(&mut self, text: &str, width: f32, size: f32) -> Vec<String> {
        let mut lines = vec![];
        let mut line = String::new();
        let mut used = 0.;
        for c in text.chars() {
            let advance = self
                .font_metrics
                .lookup_glyph_index(c as u32)
                .and_then(|gid| self.font_metrics.get_or_decode_glyph(gid))
                .map_or(0., |g| g.horz_advance as f32)
                * size
                / self.font_metrics.pdf_font_metrics.units_per_em as f32;
            if c == '\n' || used + advance > width {
                lines.push(std::mem::take(&mut line));
                used = 0.;
                if c == '\n' {
                    continue;
                }
            }
            line.push(c);
            used += advance;
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }
    fn row(&mut self, label: &str, value: &str) {
        let labels = self.wrap(label, 117., 10.);
        let values = self.wrap(value, 346., 10.5);
        let total = labels.len().max(values.len());
        // Keep ordinary rows together; long rows continue with the section heading.
        if total < 36 && self.y - total as f32 * 15.5 < 62. {
            self.page();
        }
        for index in 0..total {
            if self.y < 62. {
                self.page();
            }
            if let Some(line) = labels.get(index) {
                self.text(line, 56.7, self.y, 10.);
            }
            if let Some(line) = values.get(index) {
                self.text(line, 186.7, self.y, 10.5);
            }
            self.y -= 15.5;
        }
        self.y -= 7.;
    }
    fn finish(mut self) -> Vec<u8> {
        self.heading.clear();
        self.page();
        self.doc
            .with_pages(self.pages)
            .save(&PdfSaveOptions::default(), &mut vec![])
    }
}
pub(crate) fn pdf(
    report: &Versioned<EvaluationReport>,
    tables: &[Table],
    resources: &EvaluationResources,
    font: &[u8],
    formats: &[EvaluationOutputFormat],
    flag: &AtomicBool,
) -> Result<Vec<u8>, Vec<Issue>> {
    let has_json = formats
        .iter()
        .any(|f| matches!(f, EvaluationOutputFormat::Json));
    let has_csv = formats
        .iter()
        .any(|f| matches!(f, EvaluationOutputFormat::Csv));
    let mut p = Pages::new(font)?;
    p.line("超级工厂 · 中文工程评估", 22.);
    p.y -= 8.;
    p.line(
        &format!(
            "报告 {} · 工程 {} · 导出修订 {}",
            report.data.id.as_str(),
            report.data.project_id.as_str(),
            report.data.project_revision.0.value()
        ),
        11.,
    );
    for section in crate::evaluation_reading::sections(report) {
        cancelled(flag)?;
        p.section(&section.title);
        let history = section.title == "运行历史阅读示例与等待摘要";
        for (label, value) in section.rows {
            p.row(&label, &value);
        }
        if history {
            if has_csv {
                p.line(
                    "完整事件分表：csv/events.csv；等待分表：csv/waits.csv、csv/wait_reasons.csv。",
                    10.5,
                );
            }
            if has_json {
                for (index, run) in report.data.run_evaluations.iter().enumerate() {
                    p.line(
                        &format!(
                            "Run {} 完整事件来源：report.json#/data/run_evaluations/{index}/events",
                            run.run_id.as_str()
                        ),
                        10.5,
                    );
                }
            }
        }
    }
    if report.data.layout_and_schedule.is_empty() {
        p.section("布局与场景图证据");
        p.line(
            "未随附带来源、许可与场景修订的视口图；本报告不使用AI图片伪造执行证据。",
            11.,
        );
    }
    for (index, image) in report.data.layout_and_schedule.iter().enumerate() {
        cancelled(flag)?;
        let bytes = &resources.files[image.scene_ref.file.path.as_str()];
        let raw = RawImage::decode_from_bytes(bytes, &mut vec![]).map_err(|e| {
            vec![problem(
                "missing_asset",
                format!("{}图像无法解析：{e}", image.title),
            )]
        })?;
        let scale = (480. / (raw.width as f32 * 0.75)).min(420. / (raw.height as f32 * 0.75));
        let h = raw.height as f32 * 0.75 * scale;
        if index == 0 {
            if p.y < h + 170. {
                p.heading.clear();
                p.page();
            }
            p.section("布局与场景图证据");
        }
        if p.y < h + 130. {
            p.page();
        }
        let id = p.doc.add_image(&raw);
        p.ops.push(Op::UseXobject {
            id,
            transform: XObjectTransform {
                translate_x: Some(Pt(56.7)),
                translate_y: Some(Pt(p.y - h)),
                scale_x: Some(scale),
                scale_y: Some(scale),
                dpi: Some(96.),
                ..Default::default()
            },
        });
        p.y -= h + 16.;
        let view_ref = if has_json {
            format!("见 report.json#/data/layout_and_schedule/{index}/scene_ref/view")
        } else {
            format!(
                "字段为 /data/layout_and_schedule/{index}/scene_ref/view；本次未选JSON，如需完整声明可另选JSON导出"
            )
        };
        p.line(
            &format!(
                "{}；场景修订{}；源工程修订{}；视图{}；Run {}；时点{}；证据类别{}。完整相机、LOD和坐标声明{view_ref}。",
                image.title,
                image.scene_ref.scene_revision.0.value(),
                image.scene_ref.view.project_revision.0.value(),
                serde_json::to_string(&image.scene_ref.view.kind).unwrap(),
                image.scene_ref.view.run_id.as_ref().map_or("未运行", |id| id.as_str()),
                image.scene_ref.view.view_time_ms.as_ref().map_or("不适用".into(), |at| format!("{} ms",at.0.value())),
                serde_json::to_string(&image.evidence_kind).unwrap()
            ),
            11.,
        );
    }
    p.section("完整数据分表索引");
    p.line("正文已列关键结果、单位、冻结输入、实际变化和未知原因。以下为同源完整数据的分表目录；PDF单独导出也包含上述阅读摘要。", 11.);
    for table in tables {
        cancelled(flag)?;
        p.row(
            &table.title,
            &format!("{}；{}条记录", table.name, table.rows.len()),
        );
    }
    p.line(
        if has_json {
            "本次随附完整 report.json；若选择HTML，其文件亦内嵌完整数据。CSV是否随附以实际选择格式为准。"
        } else if has_csv {
            "本次未选择JSON；随附CSV保留完整分表记录。"
        } else {
            "本次未选择JSON或CSV；逐条完整明细未随附，本PDF保留关键阅读摘要；需要全部记录时可另选JSON/CSV/HTML。"
        },
        10.5,
    );
    Ok(p.finish())
}
