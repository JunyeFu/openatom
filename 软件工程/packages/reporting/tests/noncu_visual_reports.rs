//! Re-render the five frozen real report contexts without recalculating any facts.
use serde_json::Value;
use sf_domain::{EvaluationReport, Versioned};
use sf_reporting::{EvaluationOutputFormat, EvaluationResources, render_evaluation};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::atomic::AtomicBool};

#[test]
#[ignore = "explicit saved-report consumer; requires NONCU_VISUAL_REPORTS manifest"]
fn current_pdf_layout_preserves_every_other_format_and_frozen_report_context() {
    let manifest = PathBuf::from(
        std::env::var_os("NONCU_VISUAL_REPORTS").expect("visual report input manifest"),
    );
    let cases: Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = root.join(
        "tools/architecture-proof/assets",
    );
    let mut resources = EvaluationResources {
        cjk_font: Some(fs::read(assets.join("SourceHanSansCN-Regular.otf")).unwrap()),
        font_license: Some(fs::read(assets.join("LICENSE.SourceHanSans.txt")).unwrap()),
        files: BTreeMap::new(),
    };
    for case in cases.as_array().unwrap() {
        let source = PathBuf::from(case["source"].as_str().unwrap());
        let output = PathBuf::from(case["output"].as_str().unwrap());
        let report: Versioned<EvaluationReport> =
            serde_json::from_slice(&fs::read(source.join("report.json")).unwrap()).unwrap();
        resources.files = report
            .data
            .layout_and_schedule
            .iter()
            .map(|image| {
                let path = image.scene_ref.file.path.as_str();
                (path.to_owned(), fs::read(source.join(path)).unwrap())
            })
            .collect();
        let frozen = serde_json::to_value(&report).unwrap();
        let files = render_evaluation(
            &report,
            &[
                EvaluationOutputFormat::Pdf,
                EvaluationOutputFormat::Html,
                EvaluationOutputFormat::Csv,
                EvaluationOutputFormat::Json,
            ],
            &resources,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&report).unwrap(), frozen);
        assert_eq!(
            files
                .iter()
                .filter(|file| file.relative_path.starts_with("csv/"))
                .count(),
            15
        );
        for file in files {
            if file.relative_path == "index.html" {
                // The human reading layer may change; embedded evidence must not.
                let html = std::str::from_utf8(&file.bytes).unwrap();
                let embedded = html
                    .split("<script id=report-data type=application/json>")
                    .nth(1)
                    .unwrap()
                    .split("</script>")
                    .next()
                    .unwrap();
                assert_eq!(serde_json::from_str::<Value>(embedded).unwrap(), frozen);
            } else if file.relative_path != "report.pdf" {
                assert_eq!(
                    fs::read(source.join(&file.relative_path)).unwrap(),
                    file.bytes,
                    "{}",
                    file.relative_path
                );
            }
            let at = output.join(&file.relative_path);
            fs::create_dir_all(at.parent().unwrap()).unwrap();
            fs::write(at, &file.bytes).unwrap();
        }
    }
}
