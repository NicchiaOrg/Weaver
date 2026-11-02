//! How omp is started for a job (`requirements/tech#job`): the command line, the environment
//! and the overlay. Pure functions: the app spawns the process.
//!
//! ```text
//! <omp> [--model <m>] [--slow <scan>] [--advisor] --config <overlay> --approval-mode <yolo|always-ask> acp
//! ```

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::Path;

use super::kinds::{JobKind, Role};
use crate::settings::{ModelSettings, Permissions};

/// What the command line of a job depends on, copied from the settings when the job starts:
/// running jobs never read the settings again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan<'a> {
    pub kind: JobKind,
    pub models: &'a ModelSettings,
    /// The model the developer chose for the Proposal the job works for: it replaces the analysis
    /// and the execution model.
    pub proposal_model: Option<&'a str>,
    pub permissions: Permissions,
    pub overlay: &'a Path,
}

/// The model of the job's role, trimmed; empty when omp's own default applies. A thinking suffix
/// `:<level>` stays whole.
pub fn role_model<'a>(
    kind: JobKind,
    models: &'a ModelSettings,
    proposal_model: Option<&'a str>,
) -> &'a str {
    let role = kind.role();
    let chosen = proposal_model.map(str::trim).filter(|m| !m.is_empty());
    match (role, chosen) {
        (Role::Scan, _) => models.scan.trim(),
        (_, Some(chosen)) => chosen,
        (Role::Analysis, None) => models.analysis.trim(),
        (Role::Execution, None) => models.execution.trim(),
    }
}

/// The arguments after the program.
pub fn args(plan: &LaunchPlan) -> Vec<String> {
    let mut args = Vec::new();
    let model = role_model(plan.kind, plan.models, plan.proposal_model);
    if !model.is_empty() {
        args.push("--model".to_owned());
        args.push(model.to_owned());
    }
    let scan = plan.models.scan.trim();
    if plan.kind.uses_analysis_agents() && !scan.is_empty() {
        args.push("--slow".to_owned());
        args.push(scan.to_owned());
    }
    if plan.kind.is_run() {
        args.push("--advisor".to_owned());
    }
    args.push("--config".to_owned());
    args.push(plan.overlay.to_string_lossy().into_owned());
    args.push("--approval-mode".to_owned());
    args.push(approval_mode(plan.permissions).to_owned());
    args.push("acp".to_owned());
    args
}

/// The value of `--approval-mode`.
pub fn approval_mode(permissions: Permissions) -> &'static str {
    match permissions {
        Permissions::Yolo => "yolo",
        Permissions::Ask => "always-ask",
    }
}

fn yaml_string(text: &str) -> String {
    serde_json::to_string(text).expect("a string serialises")
}

/// The overlay file (`<app local data>/overlays/<job id>.yml`): omp's own auto-backgrounding and
/// tool timeout are off, so that `trama question new` can wait as long as the developer needs. A
/// Run also gets the advisor model (the scan model, when set) and the advisor of the
/// `trama-implementer` sub-agents.
pub fn overlay_text(kind: JobKind, scan_model: &str) -> String {
    let mut text =
        String::from("bash:\n  autoBackground:\n    enabled: false\ntools:\n  maxTimeout: 0\n");
    if kind.is_run() {
        let scan = scan_model.trim();
        if !scan.is_empty() {
            text.push_str(&format!("modelRoles:\n  advisor: {}\n", yaml_string(scan)));
        }
        text.push_str("task:\n  agentAdvisor:\n    trama-implementer: \"on\"\n");
    }
    text
}

/// The variables Weaver adds to the environment of omp.
pub fn weaver_env(
    job: &str,
    endpoint: &str,
    kind: JobKind,
    task: Option<&str>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        ("TRAMA_JOB".to_owned(), job.to_owned()),
        ("TRAMA_ENDPOINT".to_owned(), endpoint.to_owned()),
    ]);
    if kind.is_run() {
        env.insert("TRAMA_RUN".to_owned(), "1".to_owned());
        if let Some(task) = task {
            env.insert("TRAMA_TASK".to_owned(), task.to_owned());
        }
    }
    env
}

/// The whole environment of the process: the inherited variables but every `TRAMA_*`, then the
/// ones Weaver sets, then `PATH`.
pub fn child_env(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    set: &BTreeMap<String, String>,
    path: &OsStr,
) -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> = inherited
        .into_iter()
        .filter(|(name, _)| !is_trama(name) && !name.eq_ignore_ascii_case("PATH"))
        .collect();
    env.extend(
        set.iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value))),
    );
    env.push((OsString::from("PATH"), path.to_owned()));
    env
}

fn is_trama(name: &OsStr) -> bool {
    name.to_string_lossy()
        .to_ascii_uppercase()
        .starts_with("TRAMA_")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn models(analysis: &str, scan: &str, execution: &str) -> ModelSettings {
        ModelSettings {
            analysis: analysis.into(),
            scan: scan.into(),
            execution: execution.into(),
        }
    }

    fn plan<'a>(
        kind: JobKind,
        models: &'a ModelSettings,
        proposal: Option<&'a str>,
        overlay: &'a Path,
        permissions: Permissions,
    ) -> LaunchPlan<'a> {
        LaunchPlan {
            kind,
            models,
            proposal_model: proposal,
            permissions,
            overlay,
        }
    }

    #[test]
    fn each_kind_runs_on_the_model_of_its_role_and_the_proposals_model_replaces_analysis_and_execution(
    ) {
        let m = models("a/analysis:high", "s/scan", "e/exec");
        assert_eq!(role_model(JobKind::Design, &m, None), "a/analysis:high");
        assert_eq!(role_model(JobKind::Resolve, &m, None), "a/analysis:high");
        assert_eq!(role_model(JobKind::Scan, &m, None), "s/scan");
        assert_eq!(role_model(JobKind::Run, &m, None), "e/exec");
        assert_eq!(
            role_model(JobKind::Run, &m, Some(" p/chosen:low ")),
            "p/chosen:low"
        );
        assert_eq!(
            role_model(JobKind::Rework, &m, Some("p/chosen")),
            "p/chosen"
        );
        assert_eq!(
            role_model(JobKind::Scan, &m, Some("p/chosen")),
            "s/scan",
            "the scan model is never replaced"
        );
        assert_eq!(role_model(JobKind::Run, &m, Some("  ")), "e/exec");
    }

    #[test]
    fn the_command_line_has_the_flags_the_job_needs_in_order() {
        let overlay = PathBuf::from("/data/overlays/job.yml");
        let all = models("a/m", "s/m:high", "e/m");
        let line = |kind, proposal| args(&plan(kind, &all, proposal, &overlay, Permissions::Yolo));
        assert_eq!(
            line(JobKind::Run, None),
            [
                "--model",
                "e/m",
                "--advisor",
                "--config",
                "/data/overlays/job.yml",
                "--approval-mode",
                "yolo",
                "acp"
            ]
        );
        assert_eq!(
            line(JobKind::Scan, None),
            [
                "--model",
                "s/m:high",
                "--slow",
                "s/m:high",
                "--config",
                "/data/overlays/job.yml",
                "--approval-mode",
                "yolo",
                "acp"
            ]
        );
        assert_eq!(
            line(JobKind::Design, None),
            [
                "--model",
                "a/m",
                "--config",
                "/data/overlays/job.yml",
                "--approval-mode",
                "yolo",
                "acp"
            ]
        );
        let ask = args(&plan(
            JobKind::Design,
            &all,
            None,
            &overlay,
            Permissions::Ask,
        ));
        assert_eq!(
            &ask[ask.len() - 3..],
            ["--approval-mode", "always-ask", "acp"]
        );
    }

    #[test]
    fn an_empty_role_passes_no_model_and_slow_needs_a_scan_model() {
        let overlay = Path::new("o.yml");
        let empty = models("", " ", "");
        assert_eq!(
            args(&plan(
                JobKind::Run,
                &empty,
                None,
                overlay,
                Permissions::Yolo
            )),
            [
                "--advisor",
                "--config",
                "o.yml",
                "--approval-mode",
                "yolo",
                "acp"
            ]
        );
        assert_eq!(
            args(&plan(
                JobKind::Scan,
                &empty,
                None,
                overlay,
                Permissions::Yolo
            )),
            ["--config", "o.yml", "--approval-mode", "yolo", "acp"]
        );
        assert_eq!(
            args(&plan(
                JobKind::Import,
                &empty,
                None,
                overlay,
                Permissions::Yolo
            )),
            ["--config", "o.yml", "--approval-mode", "yolo", "acp"]
        );
    }

    #[test]
    fn the_overlay_turns_off_omps_timeouts_and_a_run_adds_its_advisor() {
        let base = "bash:\n  autoBackground:\n    enabled: false\ntools:\n  maxTimeout: 0\n";
        assert_eq!(overlay_text(JobKind::Scan, "s/m"), base);
        assert_eq!(overlay_text(JobKind::Design, ""), base);
        assert_eq!(
            overlay_text(JobKind::Run, "s/m:high"),
            format!("{base}modelRoles:\n  advisor: \"s/m:high\"\ntask:\n  agentAdvisor:\n    trama-implementer: \"on\"\n")
        );
        assert_eq!(
            overlay_text(JobKind::Run, " "),
            format!("{base}task:\n  agentAdvisor:\n    trama-implementer: \"on\"\n")
        );
        // It is YAML.
        let value: serde_yaml::Value =
            serde_yaml::from_str(&overlay_text(JobKind::Run, "a\"b")).unwrap();
        assert_eq!(
            value["modelRoles"]["advisor"],
            serde_yaml::Value::from("a\"b")
        );
        assert_eq!(value["tools"]["maxTimeout"], serde_yaml::Value::from(0));
    }

    #[test]
    fn weaver_adds_the_job_the_endpoint_and_the_run_marker() {
        assert_eq!(
            weaver_env("jjjjjjjj", "/run/w.sock", JobKind::Run, Some("aaaaaaaa")),
            BTreeMap::from([
                ("TRAMA_ENDPOINT".to_owned(), "/run/w.sock".to_owned()),
                ("TRAMA_JOB".to_owned(), "jjjjjjjj".to_owned()),
                ("TRAMA_RUN".to_owned(), "1".to_owned()),
                ("TRAMA_TASK".to_owned(), "aaaaaaaa".to_owned()),
            ])
        );
        let all = weaver_env("jjjjjjjj", "e", JobKind::Run, None);
        assert!(all.contains_key("TRAMA_RUN") && !all.contains_key("TRAMA_TASK"));
        let design = weaver_env("jjjjjjjj", "e", JobKind::Design, Some("aaaaaaaa"));
        assert_eq!(
            design.keys().collect::<Vec<_>>(),
            ["TRAMA_ENDPOINT", "TRAMA_JOB"]
        );
    }

    #[test]
    fn inherited_trama_variables_are_removed_and_path_is_the_childs() {
        let inherited = [
            ("HOME", "/h"),
            ("TRAMA_JOB", "old"),
            ("trama_general", "g"),
            ("PATH", "/old"),
            ("TRAMA_RUN", "1"),
        ]
        .map(|(n, v)| (OsString::from(n), OsString::from(v)));
        let set = BTreeMap::from([("TRAMA_JOB".to_owned(), "new".to_owned())]);
        let env = child_env(inherited, &set, OsStr::new("/w/bin:/x"));
        let names: Vec<String> = env
            .iter()
            .map(|(n, _)| n.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["HOME", "TRAMA_JOB", "PATH"]);
        assert_eq!(env[1].1, "new");
        assert_eq!(env[2].1, "/w/bin:/x");
    }
}
