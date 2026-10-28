use ts_rs::TS;

fn main() {
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/generated");
    std::fs::create_dir_all(&target).expect("create engineering type directory");
    std::fs::write(
        target.join("EngineeringStatus.ts"),
        format!(
            "// Generated from Rust EngineeringStatus; not a business DTO.\nexport {}\n",
            sf_desktop_host::EngineeringStatus::decl(&ts_rs::Config::new())
        ),
    )
    .expect("write engineering types");
    std::fs::write(
        target.join("DesktopCloseState.ts"),
        format!(
            "// Generated from Rust DesktopCloseState; private main-window lifecycle DTO.\nexport {}\n",
            sf_commands::DesktopCloseState::decl(&ts_rs::Config::new())
        ),
    )
    .expect("write desktop close types");
}
