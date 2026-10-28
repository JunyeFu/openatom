//! Offline consumer of the public reporting API for an already saved report.
use sf_domain::{EvaluationReport, Versioned};
use sf_reporting::{EvaluationOutputFormat as Format, EvaluationResources, render_evaluation};
use std::{env, fs, path::PathBuf, sync::atomic::AtomicBool};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if !(args.len() == 4 || args.len() == 5 && args[4] == "--pdf-only") {
        return Err(
            "usage: re_render_saved INPUT_DIR OUTPUT_DIR FONT_FILE LICENSE_FILE [--pdf-only]"
                .into(),
        );
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let report: Versioned<EvaluationReport> =
        serde_json::from_slice(&fs::read(input.join("report.json"))?)?;
    let mut resources = EvaluationResources {
        cjk_font: Some(fs::read(&args[2])?),
        font_license: Some(fs::read(&args[3])?),
        ..Default::default()
    };
    for image in &report.data.layout_and_schedule {
        let path = image.scene_ref.file.path.as_str();
        resources
            .files
            .insert(path.into(), fs::read(input.join(path))?);
    }
    let formats = if args.len() == 5 {
        vec![Format::Pdf]
    } else {
        vec![Format::Pdf, Format::Html, Format::Csv, Format::Json]
    };
    let files = render_evaluation(&report, &formats, &resources, &AtomicBool::new(false))
        .map_err(|issues| format!("reporting errors: {issues:?}"))?;
    for file in &files {
        let path = output.join(&file.relative_path);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, &file.bytes)?;
    }
    println!(
        "{} Run(s), {} generated files -> {}",
        report.data.run_refs.len(),
        files.len(),
        output.display()
    );
    Ok(())
}
