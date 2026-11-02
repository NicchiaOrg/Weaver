//! The log file of the app (`requirements/quality#Logs`).
//!
//! * Folder: the OS log folder Tauri resolves for the app (`app_log_dir`), given to [`init`].
//! * One file per day of use, `weaver-YYYY-MM-DD.log`; a file that reaches 10 MB is renamed to
//!   `weaver-YYYY-MM-DD.<n>.log` and a new one starts. The 7 most recent files are kept; older
//!   ones are deleted at start (and after each rotation).
//! * Level `info`; `debug` only when `WEAVER_LOG=debug`.
//! * One line: `<ISO 8601 UTC instant> <LEVEL> <module> [project="<folder name>"] [job="<id>"]: <text>`.
//!
//! What never gets in: prompts, agent messages, answers, request text, file contents,
//! environment values, secrets. The writer cannot know a message's meaning, so the rule is kept
//! by the callers (they pass names, ids, paths, codes and durations) and backed by two guards
//! here: the values of sensitive environment variables and every secret given to
//! [`Logger::redact`] are replaced by `[redacted]`, and a line is one line of bounded length.
//!
//! The `trama` command line never writes a log file; it does not use this module.

use std::backtrace::Backtrace;
use std::fmt::{self, Write as _};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A log file is closed and renamed once it holds this many bytes.
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
/// Files kept in the folder.
pub const KEPT_FILES: usize = 7;
/// An operation at least this long is logged at `info`.
pub const SLOW_OPERATION: Duration = Duration::from_millis(200);
/// A line is cut after this many characters (a panic's backtrace is the longest line).
const MAX_LINE_CHARS: usize = 8000;
const REDACTED: &str = "[redacted]";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// `debug` only for the value `debug` of `WEAVER_LOG` (any case); everything else is `info`.
    pub fn from_env_value(value: Option<&str>) -> Level {
        match value {
            Some(v) if v.trim().eq_ignore_ascii_case("debug") => Level::Debug,
            _ => Level::Info,
        }
    }
}

/// The project (folder name) and job (id) a line belongs to, when there is one.
#[derive(Debug, Clone, Copy, Default)]
pub struct Context<'a> {
    pub project: Option<&'a str>,
    pub job: Option<&'a str>,
}

impl<'a> Context<'a> {
    pub const NONE: Context<'static> = Context {
        project: None,
        job: None,
    };

    pub fn project(project: &'a str) -> Self {
        Self {
            project: Some(project),
            job: None,
        }
    }
}

/// The clock of a [`Logger`]; tests inject theirs.
pub type Clock = Box<dyn Fn() -> SystemTime + Send + Sync>;

pub struct Logger {
    level: Level,
    max_bytes: u64,
    kept: usize,
    clock: Clock,
    secrets: Mutex<Vec<String>>,
    state: Mutex<State>,
}

struct State {
    dir: PathBuf,
    open: Option<OpenFile>,
}

struct OpenFile {
    date: String,
    file: File,
    size: u64,
}

impl Logger {
    /// Creates the folder, deletes the files beyond the 7 most recent and returns the logger.
    pub fn new(dir: &Path, level: Level, clock: Clock) -> io::Result<Self> {
        Self::with_limits(dir, level, clock, MAX_FILE_BYTES, KEPT_FILES)
    }

    pub fn with_limits(
        dir: &Path,
        level: Level,
        clock: Clock,
        max_bytes: u64,
        kept: usize,
    ) -> io::Result<Self> {
        fs::create_dir_all(dir)?;
        prune(dir, kept)?;
        Ok(Self {
            level,
            max_bytes,
            kept,
            clock,
            secrets: Mutex::new(Vec::new()),
            state: Mutex::new(State {
                dir: dir.to_path_buf(),
                open: None,
            }),
        })
    }

    pub fn level(&self) -> Level {
        self.level
    }

    /// From now on `secret` is replaced by `[redacted]` in every line. Values shorter than 4
    /// characters are ignored: they would mangle ordinary text.
    pub fn redact(&self, secret: &str) {
        if secret.chars().count() < 4 {
            return;
        }
        let mut secrets = self.secrets.lock().expect("log secrets poisoned");
        if !secrets.iter().any(|s| s == secret) {
            secrets.push(secret.to_owned());
            // Longest first, so a secret that contains another is replaced whole.
            secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
        }
    }

    /// Redacts the values of the sensitive variables (see [`sensitive_values`]) of `vars`.
    pub fn redact_environment(&self, vars: impl IntoIterator<Item = (String, String)>) {
        for value in sensitive_values(vars) {
            self.redact(&value);
        }
    }

    /// Writes one line. Failures to write are returned, never panics.
    pub fn write(
        &self,
        level: Level,
        module: &str,
        context: Context<'_>,
        message: fmt::Arguments<'_>,
    ) -> io::Result<()> {
        if level < self.level {
            return Ok(());
        }
        let now = (self.clock)();
        let mut text = String::new();
        let _ = write!(text, "{message}");
        let text = self.sanitise(&text);
        let mut line = format!("{} {} {}", iso_utc(now), level.label(), module);
        if let Some(project) = context.project {
            let _ = write!(line, " project={:?}", self.sanitise(project));
        }
        if let Some(job) = context.job {
            let _ = write!(line, " job={:?}", self.sanitise(job));
        }
        let _ = writeln!(line, ": {text}");
        self.append(&date_utc(now), line.as_bytes())
    }

    /// Logs an operation at `info` when it took [`SLOW_OPERATION`] or more: its name, project and
    /// duration in milliseconds, nothing of its content.
    pub fn slow_operation(
        &self,
        name: &str,
        project: Option<&str>,
        elapsed: Duration,
    ) -> io::Result<()> {
        if elapsed < SLOW_OPERATION {
            return Ok(());
        }
        self.write(
            Level::Info,
            module_path!(),
            Context { project, job: None },
            format_args!(
                "slow operation name={name:?} duration_ms={}",
                elapsed.as_millis()
            ),
        )
    }

    /// The file lines go to now (it may not exist before the first line of the day).
    pub fn current_file(&self) -> PathBuf {
        let state = self.state.lock().expect("log state poisoned");
        state.dir.join(file_name(&date_utc((self.clock)()), None))
    }

    pub fn dir(&self) -> PathBuf {
        self.state.lock().expect("log state poisoned").dir.clone()
    }

    fn sanitise(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len().min(MAX_LINE_CHARS));
        for c in text.chars() {
            match c {
                '\n' => out.push_str("\\n"),
                '\r' => {}
                '\t' => out.push(' '),
                c if c.is_control() => out.push('?'),
                c => out.push(c),
            }
        }
        for secret in self.secrets.lock().expect("log secrets poisoned").iter() {
            if out.contains(secret.as_str()) {
                out = out.replace(secret.as_str(), REDACTED);
            }
        }
        if out.chars().count() > MAX_LINE_CHARS {
            out = out.chars().take(MAX_LINE_CHARS).collect::<String>() + "…";
        }
        out
    }

    fn append(&self, date: &str, bytes: &[u8]) -> io::Result<()> {
        let mut state = self.state.lock().expect("log state poisoned");
        let dir = state.dir.clone();
        let wrong_day = state.open.as_ref().is_some_and(|f| f.date != date);
        if wrong_day {
            state.open = None;
        }
        let full = state
            .open
            .as_ref()
            .is_some_and(|f| f.size > 0 && f.size + bytes.len() as u64 > self.max_bytes);
        if full {
            let finished = state.open.take().expect("checked above");
            drop(finished.file);
            let next = next_sequence(&dir, date)?;
            fs::rename(
                dir.join(file_name(date, None)),
                dir.join(file_name(date, Some(next))),
            )?;
            prune(&dir, self.kept)?;
        }
        if state.open.is_none() {
            let path = dir.join(file_name(date, None));
            let file = OpenOptions::new().create(true).append(true).open(&path)?;
            let size = file.metadata()?.len();
            state.open = Some(OpenFile {
                date: date.to_owned(),
                file,
                size,
            });
            // A new day's file may push the oldest one out.
            prune(&dir, self.kept)?;
        }
        let open = state.open.as_mut().expect("opened above");
        open.file.write_all(bytes)?;
        open.size += bytes.len() as u64;
        Ok(())
    }
}

/// The values of the environment variables whose name says they hold a secret.
pub fn sensitive_values(vars: impl IntoIterator<Item = (String, String)>) -> Vec<String> {
    const MARKERS: [&str; 8] = [
        "KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "CREDENTIAL",
        "AUTH",
        "COOKIE",
    ];
    vars.into_iter()
        .filter(|(name, value)| {
            let name = name.to_ascii_uppercase();
            !value.is_empty() && MARKERS.iter().any(|marker| name.contains(marker))
        })
        .map(|(_, value)| value)
        .collect()
}

// --- file names --------------------------------------------------------------------------------

fn file_name(date: &str, sequence: Option<u32>) -> String {
    match sequence {
        None => format!("weaver-{date}.log"),
        Some(n) => format!("weaver-{date}.{n}.log"),
    }
}

/// `(date, sequence)` of a file of ours; the active file of a day has no sequence and is the
/// newest of its day.
fn parse_name(name: &str) -> Option<(String, Option<u32>)> {
    let rest = name.strip_prefix("weaver-")?.strip_suffix(".log")?;
    let (date, sequence) = match rest.split_once('.') {
        Some((date, n)) => (date, Some(n.parse::<u32>().ok()?)),
        None => (rest, None),
    };
    let well_formed = date.len() == 10
        && date.char_indices().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    well_formed.then(|| (date.to_owned(), sequence))
}

fn our_files(dir: &Path) -> io::Result<Vec<(String, Option<u32>, PathBuf)>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if let Some((date, sequence)) = name.to_str().and_then(parse_name) {
            files.push((date, sequence, entry.path()));
        }
    }
    Ok(files)
}

fn next_sequence(dir: &Path, date: &str) -> io::Result<u32> {
    let highest = our_files(dir)?
        .into_iter()
        .filter(|(d, _, _)| d == date)
        .filter_map(|(_, n, _)| n)
        .max();
    Ok(highest.unwrap_or(0) + 1)
}

/// Deletes our files beyond the `kept` most recent ones.
fn prune(dir: &Path, kept: usize) -> io::Result<()> {
    let mut files = our_files(dir)?;
    // Newest first: later date first; within a day the active file, then the higher sequence.
    files.sort_by(|a, b| {
        (&b.0, b.1.map_or(u64::MAX, u64::from)).cmp(&(&a.0, a.1.map_or(u64::MAX, u64::from)))
    });
    for (_, _, path) in files.into_iter().skip(kept) {
        fs::remove_file(path)?;
    }
    Ok(())
}

// --- time --------------------------------------------------------------------------------------

/// Days since 1970-01-01 → (year, month, day), proleptic Gregorian (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn unix_parts(time: SystemTime) -> (i64, u32) {
    match time.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_millis()),
        Err(e) => {
            let d = e.duration();
            let (secs, millis) = (d.as_secs() as i64, d.subsec_millis());
            if millis == 0 {
                (-secs, 0)
            } else {
                (-secs - 1, 1000 - millis)
            }
        }
    }
}

/// `YYYY-MM-DD` in UTC.
pub fn date_utc(time: SystemTime) -> String {
    let (secs, _) = unix_parts(time);
    let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

/// ISO 8601 in UTC with milliseconds: `2026-11-02T07:29:32.875Z`.
pub fn iso_utc(time: SystemTime) -> String {
    let (secs, millis) = unix_parts(time);
    let in_day = secs.rem_euclid(86_400);
    format!(
        "{}T{:02}:{:02}:{:02}.{millis:03}Z",
        date_utc(time),
        in_day / 3600,
        in_day % 3600 / 60,
        in_day % 60
    )
}

// --- the app's logger --------------------------------------------------------------------------

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Opens the app's log in `dir` (the `app_log_dir`), reading the level from `WEAVER_LOG`, and
/// installs the panic hook. Returns the logger; a second call returns the first one.
pub fn init(dir: &Path) -> io::Result<&'static Logger> {
    if let Some(logger) = LOGGER.get() {
        return Ok(logger);
    }
    let level = Level::from_env_value(std::env::var("WEAVER_LOG").ok().as_deref());
    let logger = Logger::new(dir, level, Box::new(SystemTime::now))?;
    logger.redact_environment(std::env::vars());
    install_panic_hook();
    Ok(LOGGER.get_or_init(|| logger))
}

pub fn logger() -> Option<&'static Logger> {
    LOGGER.get()
}

/// Writes a line to the app's log; before [`init`] (or when the file cannot be written) warnings
/// and errors go to stderr so they are not lost.
pub fn log(level: Level, module: &str, context: Context<'_>, message: fmt::Arguments<'_>) {
    let written = match LOGGER.get() {
        Some(logger) => logger.write(level, module, context, message).is_ok(),
        None => level < Level::Info,
    };
    if !written && level >= Level::Warn {
        eprintln!("weaver: {} {module}: {message}", level.label());
    }
}

/// Logs `name` at `info` when `elapsed` is at least [`SLOW_OPERATION`].
pub fn report_if_slow(name: &str, project: Option<&str>, elapsed: Duration) {
    if let Some(logger) = LOGGER.get() {
        let _ = logger.slow_operation(name, project, elapsed);
    }
}

/// Runs `f` and reports it when it is slow.
pub fn timed<T>(name: &str, project: Option<&str>, f: impl FnOnce() -> T) -> T {
    let started = std::time::Instant::now();
    let result = f();
    report_if_slow(name, project, started.elapsed());
    result
}

/// A panic is logged with its place and backtrace, never its message (it may quote content).
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let place = info.location().map_or_else(
            || "unknown place".to_owned(),
            |l| format!("{}:{}", l.file(), l.line()),
        );
        let thread = std::thread::current();
        log(
            Level::Error,
            "weaver::panic",
            Context::NONE,
            format_args!(
                "panic in thread {:?} at {place}\n{}",
                thread.name().unwrap_or("unnamed"),
                Backtrace::force_capture()
            ),
        );
    }));
}

/// `log_info!("text {}", x)` or `log_info!(ctx: context, "text")`, with `log_debug!`,
/// `log_warn!` and `log_error!` alike. The module is the caller's.
#[macro_export]
macro_rules! log_at {
    ($level:expr, ctx: $ctx:expr, $($arg:tt)+) => {
        $crate::logging::log($level, module_path!(), $ctx, format_args!($($arg)+))
    };
    ($level:expr, $($arg:tt)+) => {
        $crate::logging::log($level, module_path!(), $crate::logging::Context::NONE, format_args!($($arg)+))
    };
}

#[macro_export]
macro_rules! log_debug { ($($arg:tt)+) => { $crate::log_at!($crate::logging::Level::Debug, $($arg)+) }; }
#[macro_export]
macro_rules! log_info { ($($arg:tt)+) => { $crate::log_at!($crate::logging::Level::Info, $($arg)+) }; }
#[macro_export]
macro_rules! log_warn { ($($arg:tt)+) => { $crate::log_at!($crate::logging::Level::Warn, $($arg)+) }; }
#[macro_export]
macro_rules! log_error { ($($arg:tt)+) => { $crate::log_at!($crate::logging::Level::Error, $($arg)+) }; }

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    use super::*;

    /// 2026-11-02T07:29:32.875Z
    const NOON: u64 = 1_793_604_572;

    fn at(secs: u64, millis: u32) -> SystemTime {
        UNIX_EPOCH + Duration::new(secs, millis * 1_000_000)
    }

    fn clock_at(secs: Arc<AtomicU64>) -> Clock {
        Box::new(move || at(secs.load(Ordering::SeqCst), 875))
    }

    fn read_all(dir: &Path) -> String {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
            .iter()
            .map(|n| fs::read_to_string(dir.join(n)).unwrap())
            .collect()
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn instants_are_iso_8601_utc_and_dates_follow_the_utc_day() {
        assert_eq!(iso_utc(at(0, 0)), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_utc(at(NOON, 875)), "2026-11-02T07:29:32.875Z");
        assert_eq!(date_utc(at(951_782_400, 0)), "2000-02-29");
        assert_eq!(date_utc(at(951_782_399, 999)), "2000-02-28");
    }

    #[test]
    fn a_line_has_instant_level_module_project_and_job() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::new(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
        )
        .unwrap();
        logger
            .write(
                Level::Info,
                "weaver_lib::jobs",
                Context {
                    project: Some("Acme app"),
                    job: Some("j7"),
                },
                format_args!("started exit_code={}", 0),
            )
            .unwrap();
        logger
            .write(
                Level::Warn,
                "weaver_lib::x",
                Context::NONE,
                format_args!("no context"),
            )
            .unwrap();
        let text = fs::read_to_string(dir.path().join("weaver-2026-11-02.log")).unwrap();
        assert_eq!(
            text,
            "2026-11-02T07:29:32.875Z INFO weaver_lib::jobs project=\"Acme app\" job=\"j7\": started exit_code=0\n\
             2026-11-02T07:29:32.875Z WARN weaver_lib::x: no context\n"
        );
    }

    #[test]
    fn debug_lines_are_written_only_when_the_level_is_debug() {
        assert_eq!(Level::from_env_value(None), Level::Info);
        assert_eq!(Level::from_env_value(Some("trace")), Level::Info);
        assert_eq!(Level::from_env_value(Some("DEBUG")), Level::Debug);

        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(AtomicU64::new(NOON));
        let info = Logger::new(dir.path(), Level::Info, clock_at(clock.clone())).unwrap();
        info.write(Level::Debug, "m", Context::NONE, format_args!("hidden"))
            .unwrap();
        assert!(
            names(dir.path()).is_empty(),
            "a debug line must not even create the file"
        );
        let debug = Logger::new(dir.path(), Level::Debug, clock_at(clock)).unwrap();
        debug
            .write(Level::Debug, "m", Context::NONE, format_args!("shown"))
            .unwrap();
        assert!(read_all(dir.path()).contains("DEBUG m: shown"));
    }

    #[test]
    fn secrets_and_sensitive_environment_values_never_reach_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::new(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
        )
        .unwrap();
        let vars = [
            ("HOME".to_owned(), "/Users/dev".to_owned()),
            ("OPENAI_API_KEY".to_owned(), "sk-live-abc123".to_owned()),
            ("GITHUB_TOKEN".to_owned(), "ghp_zzz999".to_owned()),
            ("EMPTY_SECRET".to_owned(), String::new()),
        ];
        assert_eq!(
            sensitive_values(vars.clone()),
            vec!["sk-live-abc123", "ghp_zzz999"]
        );
        logger.redact_environment(vars);
        logger.redact("runtime-secret-0f0f");
        logger
            .write(
                Level::Error,
                "m",
                Context::project("sk-live-abc123"),
                format_args!(
                    "failed with sk-live-abc123 and ghp_zzz999 and runtime-secret-0f0f in {}",
                    "/Users/dev/p"
                ),
            )
            .unwrap();
        let text = read_all(dir.path());
        for forbidden in ["sk-live-abc123", "ghp_zzz999", "runtime-secret-0f0f"] {
            assert!(!text.contains(forbidden), "{forbidden} leaked into {text}");
        }
        assert!(
            text.contains("failed with [redacted] and [redacted] and [redacted] in /Users/dev/p"),
            "{text}"
        );
    }

    #[test]
    fn a_message_is_one_bounded_line_without_control_characters() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::new(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
        )
        .unwrap();
        logger
            .write(
                Level::Info,
                "m",
                Context::NONE,
                format_args!("a\r\nb\u{7}c"),
            )
            .unwrap();
        logger
            .write(
                Level::Info,
                "m",
                Context::NONE,
                format_args!("{}", "x".repeat(20_000)),
            )
            .unwrap();
        let text = read_all(dir.path());
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with(": a\\nb?c"), "{}", lines[0]);
        assert!(lines[1].chars().count() < 8100);
        assert!(lines[1].ends_with('…'));
    }

    #[test]
    fn only_operations_of_200_ms_or_more_are_logged_with_name_project_and_duration() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::new(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
        )
        .unwrap();
        logger
            .slow_operation("scan", Some("Acme"), Duration::from_millis(199))
            .unwrap();
        assert!(names(dir.path()).is_empty());
        logger
            .slow_operation("scan", Some("Acme"), Duration::from_millis(250))
            .unwrap();
        assert!(read_all(dir.path()).contains("INFO weaver_lib::logging project=\"Acme\": slow operation name=\"scan\" duration_ms=250"));
    }

    #[test]
    fn a_new_day_starts_a_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(AtomicU64::new(NOON));
        let logger = Logger::new(dir.path(), Level::Info, clock_at(clock.clone())).unwrap();
        logger
            .write(Level::Info, "m", Context::NONE, format_args!("day one"))
            .unwrap();
        clock.store(NOON + 86_400, Ordering::SeqCst);
        logger
            .write(Level::Info, "m", Context::NONE, format_args!("day two"))
            .unwrap();
        assert_eq!(
            names(dir.path()),
            ["weaver-2026-11-02.log", "weaver-2026-11-03.log"]
        );
        assert_eq!(
            logger.current_file(),
            dir.path().join("weaver-2026-11-03.log")
        );
    }

    #[test]
    fn a_file_reaching_the_limit_rotates_and_nothing_is_lost() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::with_limits(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
            200,
            100,
        )
        .unwrap();
        for i in 0..12 {
            logger
                .write(
                    Level::Info,
                    "m",
                    Context::NONE,
                    format_args!("line number {i:02} with some padding text"),
                )
                .unwrap();
        }
        let files = names(dir.path());
        assert!(files.len() >= 3, "{files:?}");
        assert!(files.contains(&"weaver-2026-11-02.log".to_owned()));
        assert!(files.contains(&"weaver-2026-11-02.1.log".to_owned()));
        for name in &files {
            let size = fs::metadata(dir.path().join(name)).unwrap().len();
            assert!(size <= 200, "{name} holds {size} bytes");
        }
        let text = read_all(dir.path());
        for i in 0..12 {
            assert_eq!(text.matches(&format!("line number {i:02} ")).count(), 1);
        }
    }

    #[test]
    fn only_the_seven_most_recent_files_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        for day in 1..=9 {
            fs::write(
                dir.path().join(format!("weaver-2026-10-{day:02}.log")),
                "old",
            )
            .unwrap();
        }
        fs::write(dir.path().join("weaver-2026-10-09.1.log"), "old").unwrap();
        fs::write(dir.path().join("notes.txt"), "not ours").unwrap();
        Logger::new(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
        )
        .unwrap();
        assert_eq!(
            names(dir.path()),
            [
                "notes.txt",
                "weaver-2026-10-04.log",
                "weaver-2026-10-05.log",
                "weaver-2026-10-06.log",
                "weaver-2026-10-07.log",
                "weaver-2026-10-08.log",
                "weaver-2026-10-09.1.log",
                "weaver-2026-10-09.log",
            ]
        );
    }

    #[test]
    fn rotation_keeps_the_count_of_files_at_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let logger = Logger::with_limits(
            dir.path(),
            Level::Info,
            clock_at(Arc::new(AtomicU64::new(NOON))),
            100,
            3,
        )
        .unwrap();
        for i in 0..30 {
            logger
                .write(
                    Level::Info,
                    "m",
                    Context::NONE,
                    format_args!("padding padding padding {i}"),
                )
                .unwrap();
        }
        let files = names(dir.path());
        assert_eq!(files.len(), 3, "{files:?}");
        assert!(
            files.contains(&"weaver-2026-11-02.log".to_owned()),
            "the active file survives: {files:?}"
        );
    }

    #[test]
    fn file_names_are_parsed_strictly() {
        assert_eq!(
            parse_name("weaver-2026-11-02.log"),
            Some(("2026-11-02".to_owned(), None))
        );
        assert_eq!(
            parse_name("weaver-2026-11-02.3.log"),
            Some(("2026-11-02".to_owned(), Some(3)))
        );
        for bad in [
            "weaver-2026-11-2.log",
            "weaver-x.log",
            "weaver-2026-11-02.x.log",
            "other-2026-11-02.log",
            "weaver-2026-11-02.txt",
        ] {
            assert_eq!(parse_name(bad), None, "{bad}");
        }
    }
}
