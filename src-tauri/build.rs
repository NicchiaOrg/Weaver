use std::path::{Path, PathBuf};
use std::process::Command;

/// Names of the Tauri commands the webview may call. One permission `allow-<name>` is generated
/// for each and granted in `capabilities/default.json`; keep this list equal to
/// `weaver_lib::commands::CommandName` (a test checks it).
const COMMANDS: &[&str] = &[
    "app_info",
    "startup_failures",
    "open_log",
    "open_url",
    "update_status",
    "update_later",
    "update_install",
    "settings_get",
    "settings_set",
    "board_snapshot",
    "delete_task",
    "approve_proposal",
    "reject_proposal",
    "discard_proposal",
    "choose_proposal_model",
    "resolve_drift",
    "fix_drift_code",
    "accept_drift_proposal",
    "dismiss_drift",
    "history_snapshot",
    "view_snapshot",
    "project_status",
    "initialise_project",
    "update_skills",
    "project_folder",
    "import_code",
    "omp_status",
    "omp_install",
    "omp_log_in",
    "omp_log_out",
    "welcome_state",
    "open_project",
    "open_folder",
    "remove_recent",
    "open_in_editor",
    "jobs_snapshot",
    "job_conversation",
    "job_send_message",
    "job_stop",
    "job_answer_permission",
    "job_remove",
    "run_task",
    "run_all",
    "scan_system",
    "general_snapshot",
    "general_send_message",
    "general_stop",
    "general_answer_permission",
    "general_clear",
    "requests_list",
    "request_read",
    "request_new",
    "request_save",
    "request_send",
    "request_discard",
    "request_image_add",
    "request_image_import",
    "request_image",
    "requests_flushed",
    "question_round_get",
    "question_round_act",
];

fn main() {
    build_sidecar();
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    tauri_build::try_build(attributes).expect("failed to run the Tauri build script");
}

/// Builds the `trama` CLI and stores it as `binaries/trama-<target triple>[.exe]`, the file the
/// `externalBin` entry of `tauri.conf.json` requires.
///
/// It is built here, with the same profile and target as the app, into a separate target folder
/// (`<target>/sidecar`): the app crate cannot depend on a binary of another package, and
/// `tauri-build` copies the sidecar to `<target>/<profile>/trama`, the path of the workspace's own
/// `trama` binary, so the two builds must not share a build folder.
fn build_sidecar() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let workspace = manifest_dir
        .parent()
        .expect("src-tauri has a parent folder");
    let target = std::env::var("TARGET").expect("TARGET is set by cargo");
    let release = std::env::var("PROFILE").is_ok_and(|p| p == "release");
    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };

    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("crates").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.toml").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );

    let sidecar_target = out_target_dir(&manifest_dir).join("sidecar");
    let mut cargo = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cargo
        .current_dir(workspace)
        .args([
            "build",
            "--package",
            "trama-cli",
            "--bin",
            "trama",
            "--locked",
            "--target",
            &target,
        ])
        .arg("--target-dir")
        .arg(&sidecar_target);
    if release {
        cargo.arg("--release");
    }
    // The outer build's flags and jobserver must not leak into the nested one.
    for var in [
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTFLAGS",
        "CARGO_BUILD_TARGET",
        "RUSTC_WORKSPACE_WRAPPER",
    ] {
        cargo.env_remove(var);
    }
    let status = cargo
        .status()
        .expect("failed to run cargo to build the trama sidecar");
    assert!(status.success(), "building the trama sidecar failed");

    let built = sidecar_target
        .join(&target)
        .join(if release { "release" } else { "debug" })
        .join(format!("trama{ext}"));
    let destination = manifest_dir
        .join("binaries")
        .join(format!("trama-{target}{ext}"));
    copy_if_different(&built, &destination);
}

/// `<workspace target>` as seen from `OUT_DIR`: `<target>/<profile>/build/<pkg>-<hash>/out`.
fn out_target_dir(manifest_dir: &Path) -> PathBuf {
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("set by cargo"));
    // Up from `out`: <pkg>-<hash>, build, <profile> (maybe <triple>/<profile>), <target>.
    let target = out
        .ancestors()
        .find(|dir| {
            dir.file_name().is_some_and(|name| name == "target")
                || dir.join("CACHEDIR.TAG").exists()
        })
        .map(Path::to_path_buf);
    target.unwrap_or_else(|| manifest_dir.join("..").join("target"))
}

fn copy_if_different(from: &Path, to: &Path) {
    let bytes =
        std::fs::read(from).unwrap_or_else(|e| panic!("cannot read {}: {e}", from.display()));
    if std::fs::read(to).is_ok_and(|existing| existing == bytes) {
        return;
    }
    std::fs::create_dir_all(to.parent().expect("has a parent")).expect("create binaries folder");
    // `fs::copy` keeps the executable bit of the built binary.
    std::fs::copy(from, to).unwrap_or_else(|e| panic!("cannot write {}: {e}", to.display()));
}
