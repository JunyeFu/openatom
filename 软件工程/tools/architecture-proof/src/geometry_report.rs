use parry3d_f64::{
    math::{Pose, Vector},
    query::{ShapeCastOptions, ShapeCastStatus, cast_shapes, distance, intersection_test},
    shape::Cuboid,
};
use printpdf::{
    FontId, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt,
    RawImage, TextItem, XObjectTransform,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, io, path::Path};

type TrialResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Row {
    id: String,
    description: String,
    count: u32,
    duration_ms: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Report {
    title: String,
    source_kind: String,
    rows: Vec<Row>,
}

fn text(ops: &mut Vec<Op>, font: &FontId, x: f32, y: f32, size: f32, content: String) {
    ops.extend([
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(x), Mm(y)),
        },
        Op::SetFont {
            font: PdfFontHandle::External(font.clone()),
            size: Pt(size),
        },
        Op::ShowText {
            items: vec![TextItem::Text(content)],
        },
        Op::EndTextSection,
    ]);
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn run() -> TrialResult<()> {
    let output = Path::new("output");
    fs::create_dir_all(output)?;
    let cube = Cuboid::new(Vector::splat(1.0));
    let origin = Pose::IDENTITY;
    let obstacle = Pose::from_translation(Vector::new(3.0, 0.0, 0.0));
    let end = Pose::from_translation(Vector::new(6.0, 0.0, 0.0));
    let gap = distance(&origin, &cube, &obstacle, &cube)?.distance;
    assert!((gap - 1.0).abs() < 1e-10);
    let start_intersects = intersection_test(&origin, &cube, &obstacle, &cube)?.intersecting;
    let end_intersects = intersection_test(&end, &cube, &obstacle, &cube)?.intersecting;
    assert!(!start_intersects && !end_intersects);
    let overlap = intersection_test(
        &Pose::from_translation(Vector::new(1.5, 0.0, 0.0)),
        &cube,
        &origin,
        &cube,
    )?
    .intersecting;
    assert!(overlap);
    let hit = cast_shapes(
        &origin,
        Vector::new(1.0, 0.0, 0.0),
        &cube,
        &obstacle,
        Vector::ZERO,
        &cube,
        ShapeCastOptions {
            max_time_of_impact: 6.0,
            ..Default::default()
        },
    )?
    .ok_or_else(|| io::Error::other("Expected analytic cube path collision"))?;
    assert_eq!(hit.status, ShapeCastStatus::Converged);
    assert!((hit.time_of_impact - 1.0).abs() < 1e-10);
    let geometry = json!({"source":"self-built analytic two cubes, not packaging job", "library":"parry3d-f64 0.31.1",
        "box_half_extents":[1,1,1],"initial_centers":[[0,0,0],[3,0,0]],"initial_gap":gap,
        "initial_and_end_disjoint":[!start_intersects,!end_intersects],"overlap_case":overlap,
        "actual_continuous_first_hit":hit.time_of_impact,"status":format!("{:?}",hit.status),
        "endpoint_only_would_miss_collision":true,"complete_packaging_algorithm_proved":false});
    fs::write(
        output.join("geometry-trial-result.json"),
        serde_json::to_vec_pretty(&geometry)?,
    )?;

    let mut report = Report {
        title: "架构试做报告 - 自建输入".into(),
        source_kind: "fixture_not_factory_result".into(),
        rows: (1..=48)
            .map(|n| Row {
                id: format!("LAB-{n:03}"),
                description: format!("第{n}条自建记录，非生产结果"),
                count: n,
                duration_ms: n * 1000,
            })
            .collect(),
    };
    report.rows[0].description = "逗号, 引号\"与换行\n以及 <tag> &".into();
    let json_bytes = serde_json::to_vec_pretty(&report)?;
    fs::write(output.join("report-fixture.json"), &json_bytes)?;
    assert_eq!(serde_json::from_slice::<Report>(&json_bytes)?, report);
    let mut writer = csv::Writer::from_path(output.join("report-fixture.csv"))?;
    for row in &report.rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    let csv_rows: Vec<Row> = csv::Reader::from_path(output.join("report-fixture.csv"))?
        .deserialize()
        .collect::<Result<_, _>>()?;
    assert_eq!(csv_rows, report.rows);
    let mut html = format!(
        "<!doctype html><meta charset='utf-8'><title>{}</title><h1>{}</h1><p>技术试做，不是工厂运行结果。</p><table><thead><tr><th>ID</th><th>说明</th><th>件数</th><th>ms</th></tr></thead><tbody>",
        escape_html(&report.title),
        escape_html(&report.title)
    );
    for row in &report.rows {
        html += &format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&row.id),
            escape_html(&row.description),
            row.count,
            row.duration_ms
        );
    }
    html += "</tbody></table>";
    assert!(html.contains("&lt;tag&gt; &amp;") && !html.contains("<tag>"));
    fs::write(output.join("report-fixture.html"), html)?;
    let mut font_warnings = Vec::new();
    let font = ParsedFont::from_bytes(
        &fs::read("assets/SourceHanSansCN-Regular.otf")?,
        0,
        &mut font_warnings,
    )
    .ok_or_else(|| io::Error::other("Chinese font parse failed"))?;
    let mut warnings = Vec::new();
    let mut pdf = PdfDocument::new(&report.title);
    let font_id = pdf.add_font(&font);
    let image = RawImage::decode_from_bytes(
        &fs::read("assets/self-built-image-fixture.png")?,
        &mut warnings,
    )
    .map_err(io::Error::other)?;
    let image_id = pdf.add_image(&image);
    let mut pages = Vec::new();
    for (page_number, chunk) in report.rows.chunks(24).enumerate() {
        let mut ops = Vec::new();
        text(&mut ops, &font_id, 20.0, 278.0, 18.0, report.title.clone());
        text(
            &mut ops,
            &font_id,
            20.0,
            265.0,
            10.0,
            "仅测试中文、分页、图像及四格式同源导出。".into(),
        );
        text(
            &mut ops,
            &font_id,
            20.0,
            253.0,
            10.0,
            "全部记录是自建测试数据，不是工厂结果。".into(),
        );
        ops.push(Op::UseXobject {
            id: image_id.clone(),
            transform: XObjectTransform {
                translate_x: Some(Mm(149.0).into()),
                translate_y: Some(Mm(237.0).into()),
                scale_x: Some(14.0),
                scale_y: Some(14.0),
                dpi: Some(300.0),
                ..Default::default()
            },
        });
        text(
            &mut ops,
            &font_id,
            149.0,
            231.0,
            8.0,
            "自建图像嵌入测试".into(),
        );
        for (x, label) in [
            (20.0, "ID"),
            (48.0, "说明"),
            (155.0, "件数"),
            (174.0, "时间ms"),
        ] {
            text(&mut ops, &font_id, x, 219.0, 10.0, label.into());
        }
        for (i, row) in chunk.iter().enumerate() {
            let y = 209.0 - i as f32 * 7.6;
            for (x, value) in [
                (20.0, row.id.clone()),
                (48.0, row.description.replace('\n', " / ")),
                (155.0, row.count.to_string()),
                (174.0, row.duration_ms.to_string()),
            ] {
                text(&mut ops, &font_id, x, y, 9.0, value);
            }
        }
        text(
            &mut ops,
            &font_id,
            20.0,
            15.0,
            9.0,
            format!("S05-05 技术试做 | 第{}页 / 2", page_number + 1),
        );
        pages.push(PdfPage::new(Mm(210.0), Mm(297.0), ops));
    }
    let pdf_bytes = pdf
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    fs::write(output.join("report-fixture.pdf"), &pdf_bytes)?;
    let result = json!({"kind":"S05-05-isolated-geometry-report-actual-trial-not-product",
        "status":"actual_files_created_visual_and_text_checks_pending","geometry":geometry,
        "source_kind":report.source_kind,"rows":report.rows.len(),"pdf_pages_authored":2,
        "actual_JSON_and_CSV_reopened_equal":true,"HTML_escape_and_offline_file_written":true,
        "PDF_bytes":pdf_bytes.len(),"library_warnings":warnings,
        "font_warning_count":font_warnings.len(),
        "actual_product_runtime":null,"actual_Figma_started":false});
    fs::write(
        output.join("geometry-report-trial-result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
