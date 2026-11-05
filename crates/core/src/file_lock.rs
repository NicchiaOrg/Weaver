//! The Machine `file-lock`: whether an agent of the project is changing a file open in a tab of the
//! File view (`requirements/rules#file-lock`, `requirements/tech#file-lock`).
//!
//! One [`FileLocks`] per project. It knows the tools that change files and are running, per
//! conversation (a job, General, an Ask more conversation), and the files open in tabs; a file open
//! in a tab is **held** while at least one running tool names it or names no file at all. It is an
//! indication only: the check at saving stays the protection.
//!
//! This module is pure but for the canonical form of the paths it is given, which it asks the file
//! system for (a path that no longer exists is its parent's canonical form plus its name). It starts
//! no process and writes nothing. The app feeds it from the live `session/update` notifications of
//! each conversation, never from replayed ones, and ends the tools of a conversation when its turn
//! ends or fails, its omp exits, its job ends, General is cleared or its Ask more conversation
//! closes.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::file_tab::path::{canonical_or_parent, is_inside, same_file};

/// The conversation of a job.
pub fn job_conversation(job: &str) -> String {
    format!("job:{job}")
}

/// The General of a project.
pub const GENERAL_CONVERSATION: &str = "general";

/// An Ask more conversation.
pub fn ask_conversation(id: &str) -> String {
    format!("ask:{id}")
}

/// The tools of the omp that serves General or an Ask more conversation (`family`) for the
/// `generation`th time: a new omp must not end what an older one left to end.
pub fn generation_conversation(family: &str, generation: u64) -> String {
    format!("{family}#{generation}")
}

/// The kinds of tool call that change files: `omp` reports its edits, writes and the deletions
/// and moves inside an edit as `edit`.
pub fn is_file_kind(kind: &str) -> bool {
    matches!(kind, "edit" | "delete" | "move")
}

/// What changed for a file open in a tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockChange {
    /// `file-lock.LOCKED`: Free to Held.
    Locked(PathBuf),
    /// `file-lock.RELEASED`: Held to Free, with the place the agent moved the file to when omp
    /// said so.
    Released {
        path: PathBuf,
        moved_to: Option<PathBuf>,
    },
}

/// Where omp says a tool moved a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    /// The source, as omp reports it (absolute).
    pub source: String,
    /// The destination, as omp reports it (absolute).
    pub destination: String,
}

#[derive(Debug, Default)]
struct Tool {
    /// Names no file: counts for every file open in a tab, until it ends.
    everything: bool,
    /// The files it names, inside the project, in canonical form.
    paths: BTreeSet<PathBuf>,
}

/// The lock state of one project.
#[derive(Debug)]
pub struct FileLocks {
    /// The canonical project root.
    root: PathBuf,
    /// The running tools: conversation, then tool call.
    tools: HashMap<String, BTreeMap<String, Tool>>,
    /// The files open in tabs (canonical).
    open: BTreeSet<PathBuf>,
    /// The open files whose lock is held.
    held: BTreeSet<PathBuf>,
    /// Where omp moved a held file, delivered with its release.
    moved: BTreeMap<PathBuf, PathBuf>,
}

impl FileLocks {
    /// The state of the project at `root`, which is canonical.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            tools: HashMap::new(),
            open: BTreeSet::new(),
            held: BTreeSet::new(),
            moved: BTreeMap::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether nothing is open and no tool runs: the state can be dropped.
    pub fn is_idle(&self) -> bool {
        self.open.is_empty() && self.tools.is_empty()
    }

    /// Whether the tool `call` of `conversation` is counted now.
    pub fn is_running(&self, conversation: &str, call: &str) -> bool {
        self.tools
            .get(conversation)
            .is_some_and(|tools| tools.contains_key(call))
    }

    /// `omp.FILE_TOOL_STARTED`: the tool `call` of `conversation` changes files. `locations` are
    /// the absolute paths it names; none means that it counts for every file open in a tab.
    pub fn tool_started(
        &mut self,
        conversation: &str,
        call: &str,
        locations: &[String],
    ) -> Vec<LockChange> {
        let mut tool = Tool::default();
        if locations.is_empty() {
            tool.everything = true;
        } else {
            tool.paths = self.inside(locations);
        }
        self.tools
            .entry(conversation.to_owned())
            .or_default()
            .insert(call.to_owned(), tool);
        self.refresh()
    }

    /// `locations` arrived in a later update of a running tool: they add their paths. A tool that
    /// named no file keeps counting for every file until it ends.
    pub fn tool_located(
        &mut self,
        conversation: &str,
        call: &str,
        locations: &[String],
    ) -> Vec<LockChange> {
        let added = self.inside(locations);
        if let Some(tool) = self
            .tools
            .get_mut(conversation)
            .and_then(|tools| tools.get_mut(call))
        {
            tool.paths.extend(added);
        }
        self.refresh()
    }

    /// `omp.FILE_TOOL_ENDED`: the tool ended, `completed` or `failed`. `moved` is where omp says it
    /// moved a file.
    pub fn tool_ended(
        &mut self,
        conversation: &str,
        call: &str,
        moved: Option<&Moved>,
    ) -> Vec<LockChange> {
        if let Some(moved) = moved {
            self.note_move(moved);
        }
        if let Some(tools) = self.tools.get_mut(conversation) {
            tools.remove(call);
            if tools.is_empty() {
                self.tools.remove(conversation);
            }
        }
        self.refresh()
    }

    /// The window closed: no file is open in a tab any more. The running tools stay.
    pub fn forget_open_files(&mut self) {
        self.open.clear();
        self.held.clear();
        self.moved.clear();
    }

    /// `omp.TURN_ENDED`, `omp.TURN_FAILED`, `omp.EXITED`, `job.ENDED`, `developer.CLEAR_GENERAL`
    /// and `developer.CLOSE_ASK`: every tool of `conversation` stops counting. Ending twice is
    /// harmless.
    pub fn conversation_ended(&mut self, conversation: &str) -> Vec<LockChange> {
        self.tools.remove(conversation);
        self.refresh()
    }

    /// Like [`conversation_ended`](Self::conversation_ended) for a conversation that omp serves
    /// again after each end (General, an Ask more conversation): the tools of every omp it had,
    /// `family` and `family#<generation>`.
    pub fn conversation_family_ended(&mut self, family: &str) -> Vec<LockChange> {
        let prefix = format!("{family}#");
        self.tools
            .retain(|key, _| key != family && !key.starts_with(&prefix));
        self.refresh()
    }

    /// `developer.OPEN_FILE`: the canonical `path` opens in a tab. Returns whether its lock is
    /// held already, so that the tab opens locked.
    pub fn file_opened(&mut self, path: PathBuf) -> bool {
        self.open.insert(path.clone());
        if self.counts_for(&path) {
            self.held.insert(path);
            true
        } else {
            false
        }
    }

    /// `file-tab.CLOSED`: the file is not tracked any more.
    pub fn file_closed(&mut self, path: &Path) {
        self.open.remove(path);
        self.held.remove(path);
        self.moved.remove(path);
    }

    /// A tab follows its file to another path. Its lock state goes with it.
    pub fn file_renamed(&mut self, from: &Path, to: PathBuf) {
        if self.open.remove(from) {
            self.open.insert(to.clone());
        }
        if self.held.remove(from) {
            self.held.insert(to);
        }
        self.moved.remove(from);
    }

    /// Whether the lock of the open file `path` is held.
    pub fn is_held(&self, path: &Path) -> bool {
        self.held.contains(path)
    }

    /// Whether a file open in a tab and held lies at `dir` or inside it (`dir` canonical).
    pub fn is_held_under(&self, dir: &Path) -> bool {
        self.held.iter().any(|path| path.starts_with(dir))
    }

    /// A path inside the project, in canonical form; `None` outside it or when it cannot be
    /// resolved.
    pub fn canonical_inside(&self, raw: &str) -> Option<PathBuf> {
        let absolute = Path::new(raw);
        if !absolute.is_absolute() {
            return None;
        }
        let canonical = canonical_or_parent(absolute).ok()?;
        is_inside(&self.root, &canonical).then_some(canonical)
    }

    fn inside(&self, raw: &[String]) -> BTreeSet<PathBuf> {
        raw.iter()
            .filter_map(|path| self.canonical_inside(path))
            .collect()
    }

    fn note_move(&mut self, moved: &Moved) {
        // omp's `sourcePath` may be relative to the project it runs in.
        let absolute = |raw: &str| {
            let path = Path::new(raw);
            if path.is_absolute() {
                raw.to_owned()
            } else {
                self.root.join(path).to_string_lossy().into_owned()
            }
        };
        let (Some(source), Some(destination)) = (
            self.canonical_inside(&absolute(&moved.source)),
            self.canonical_inside(&absolute(&moved.destination)),
        ) else {
            return;
        };
        if self.open.iter().any(|open| same_file(open, &source)) {
            self.moved.insert(source, destination);
        }
    }

    fn counts_for(&self, path: &Path) -> bool {
        self.tools
            .values()
            .flat_map(BTreeMap::values)
            .any(|tool| tool.everything || tool.paths.contains(path))
    }

    /// Works out which open files are held now and reports the differences.
    fn refresh(&mut self) -> Vec<LockChange> {
        let now: BTreeSet<PathBuf> = self
            .open
            .iter()
            .filter(|path| self.counts_for(path))
            .cloned()
            .collect();
        let mut changes = Vec::new();
        for path in now.difference(&self.held) {
            changes.push(LockChange::Locked(path.clone()));
        }
        let released: Vec<PathBuf> = self.held.difference(&now).cloned().collect();
        for path in released {
            let moved_to = self.moved.remove(&path);
            changes.push(LockChange::Released { path, moved_to });
        }
        self.held = now;
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Project {
        _dir: tempfile::TempDir,
        root: PathBuf,
    }

    fn project() -> Project {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        Project { _dir: dir, root }
    }

    impl Project {
        fn file(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, b"x").unwrap();
            path
        }

        fn raw(&self, name: &str) -> String {
            self.root.join(name).to_string_lossy().into_owned()
        }

        fn locks(&self) -> FileLocks {
            FileLocks::new(self.root.clone())
        }
    }

    fn locked(path: &Path) -> LockChange {
        LockChange::Locked(path.to_path_buf())
    }

    fn released(path: &Path) -> LockChange {
        LockChange::Released {
            path: path.to_path_buf(),
            moved_to: None,
        }
    }

    #[test]
    fn a_tool_that_names_the_file_locks_it_until_it_ends() {
        let p = project();
        let (a, b) = (p.file("a.rs"), p.file("b.rs"));
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.file_opened(b.clone());
        assert_eq!(
            locks.tool_started("job:1", "t1", &[p.raw("a.rs")]),
            vec![locked(&a)]
        );
        assert!(locks.is_held(&a) && !locks.is_held(&b));
        assert_eq!(locks.tool_ended("job:1", "t1", None), vec![released(&a)]);
        assert!(!locks.is_held(&a));
    }

    #[test]
    fn a_tool_that_names_no_file_locks_every_open_file_until_it_ends() {
        let p = project();
        let (a, b) = (p.file("a.rs"), p.file("b.rs"));
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.file_opened(b.clone());
        let changes = locks.tool_started("job:1", "t1", &[]);
        assert_eq!(changes, vec![locked(&a), locked(&b)]);
        // Later locations add paths; the tool still counts for every file.
        assert!(locks
            .tool_located("job:1", "t1", &[p.raw("a.rs")])
            .is_empty());
        assert!(locks.is_held(&b));
        assert_eq!(
            locks.tool_ended("job:1", "t1", None),
            vec![released(&a), released(&b)]
        );
    }

    #[test]
    fn locations_of_a_later_update_add_their_paths() {
        let p = project();
        let (a, b) = (p.file("a.rs"), p.file("b.rs"));
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.file_opened(b.clone());
        locks.tool_started("job:1", "t1", &[p.raw("a.rs")]);
        assert_eq!(
            locks.tool_located("job:1", "t1", &[p.raw("b.rs")]),
            vec![locked(&b)]
        );
    }

    #[test]
    fn several_tools_on_one_file_hold_it_until_the_last_ends() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        assert_eq!(
            locks.tool_started("job:1", "t1", &[p.raw("a.rs")]),
            vec![locked(&a)]
        );
        assert!(locks
            .tool_started("job:1", "t2", &[p.raw("a.rs")])
            .is_empty());
        assert!(locks.tool_ended("job:1", "t1", None).is_empty());
        assert!(locks.is_held(&a));
        assert_eq!(locks.tool_ended("job:1", "t2", None), vec![released(&a)]);
    }

    #[test]
    fn tools_of_two_conversations_keep_the_lock_until_both_end() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[p.raw("a.rs")]);
        locks.tool_started("general", "t1", &[p.raw("a.rs")]);
        assert!(locks.conversation_ended("job:1").is_empty());
        assert!(locks.is_held(&a));
        assert_eq!(locks.conversation_ended("general"), vec![released(&a)]);
    }

    #[test]
    fn the_end_of_a_conversation_ends_each_of_its_tools_once() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("ask:9", "t1", &[p.raw("a.rs")]);
        locks.tool_started("ask:9", "t2", &[]);
        assert_eq!(locks.conversation_ended("ask:9"), vec![released(&a)]);
        // Turn end, then exit, then the end of the job: nothing more to end.
        assert!(locks.conversation_ended("ask:9").is_empty());
        assert!(locks.tool_ended("ask:9", "t1", None).is_empty());
        assert!(!locks.is_held(&a));
    }

    #[test]
    fn a_file_that_opens_during_a_tool_opens_held() {
        let p = project();
        let (a, b) = (p.file("a.rs"), p.file("b.rs"));
        let mut locks = p.locks();
        locks.tool_started("job:1", "t1", &[p.raw("a.rs")]);
        assert!(locks.file_opened(a.clone()));
        assert!(!locks.file_opened(b.clone()));
        assert_eq!(locks.tool_started("job:1", "t2", &[]), vec![locked(&b)]);
        assert!(locks.is_held(&a));
        assert_eq!(locks.tool_ended("job:1", "t2", None), vec![released(&b)]);
        assert!(locks.is_held(&a), "t1 still counts");
    }

    #[test]
    fn a_file_opened_while_a_tool_without_locations_runs_opens_held() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.tool_started("general", "t", &[]);
        assert!(locks.file_opened(a));
    }

    #[test]
    fn closing_the_tab_forgets_the_file() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[]);
        locks.file_closed(&a);
        assert!(!locks.is_held(&a));
        assert!(locks.tool_ended("job:1", "t1", None).is_empty());
        assert!(!locks.file_opened(a), "the tool is over");
    }

    #[test]
    fn paths_outside_the_project_are_ignored() {
        let p = project();
        let outside = project();
        let a = p.file("a.rs");
        let elsewhere = outside.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a);
        // A tool that names only a file outside the project counts for nothing.
        let raw = elsewhere.to_string_lossy().into_owned();
        assert!(locks.tool_started("job:1", "t1", &[raw]).is_empty());
        assert!(locks.is_running("job:1", "t1"));
    }

    #[test]
    fn a_path_that_no_longer_exists_is_its_parents_canonical_form_plus_its_name() {
        let p = project();
        let a = p.file("src/a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        fs::remove_file(&a).unwrap();
        assert_eq!(
            locks.tool_started("job:1", "t1", &[p.raw("src/a.rs")]),
            vec![locked(&a)],
            "a deletion in progress names a path that is already gone"
        );
        assert_eq!(locks.tool_ended("job:1", "t1", None), vec![released(&a)]);
    }

    #[test]
    fn relative_and_unresolvable_paths_are_ignored() {
        let p = project();
        let locks = p.locks();
        assert_eq!(locks.canonical_inside("src/a.rs"), None);
        assert_eq!(locks.canonical_inside(""), None);
    }

    #[test]
    fn a_move_carries_its_destination_with_the_release() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[p.raw("a.rs")]);
        let moved = Moved {
            source: p.raw("a.rs"),
            destination: p.raw("moved/b.rs"),
        };
        let changes = locks.tool_ended("job:1", "t1", Some(&moved));
        assert_eq!(
            changes,
            vec![LockChange::Released {
                path: a,
                moved_to: Some(p.root.join("moved").join("b.rs")),
            }]
        );
    }

    #[test]
    fn a_move_that_names_a_file_not_open_in_a_tab_changes_nothing() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[]);
        let moved = Moved {
            source: p.raw("other.rs"),
            destination: p.raw("x.rs"),
        };
        assert_eq!(
            locks.tool_ended("job:1", "t1", Some(&moved)),
            vec![released(&a)]
        );
    }

    #[test]
    fn a_move_noted_while_another_tool_still_holds_the_file_comes_with_the_last_release() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[p.raw("a.rs")]);
        locks.tool_started("general", "t2", &[p.raw("a.rs")]);
        let moved = Moved {
            source: p.raw("a.rs"),
            destination: p.raw("b.rs"),
        };
        assert!(locks.tool_ended("job:1", "t1", Some(&moved)).is_empty());
        assert_eq!(
            locks.conversation_ended("general"),
            vec![LockChange::Released {
                path: a,
                moved_to: Some(p.root.join("b.rs")),
            }]
        );
    }

    #[test]
    fn a_deletion_releases_without_a_destination() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[p.raw("a.rs")]);
        fs::remove_file(&a).unwrap();
        assert_eq!(locks.tool_ended("job:1", "t1", None), vec![released(&a)]);
    }

    #[test]
    fn a_family_ends_with_every_omp_it_had_and_no_other_conversation() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started(&generation_conversation(GENERAL_CONVERSATION, 1), "t", &[]);
        locks.tool_started(&generation_conversation(GENERAL_CONVERSATION, 2), "t", &[]);
        locks.tool_started(&ask_conversation("x"), "t", &[]);
        assert!(locks
            .conversation_family_ended(GENERAL_CONVERSATION)
            .is_empty());
        assert!(locks.is_held(&a), "the Ask more conversation still counts");
        assert_eq!(
            locks.conversation_family_ended(&ask_conversation("x")),
            vec![released(&a)]
        );
    }

    #[test]
    fn a_renamed_tab_keeps_its_lock() {
        let p = project();
        let a = p.file("a.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        locks.tool_started("job:1", "t1", &[]);
        let b = p.root.join("b.rs");
        locks.file_renamed(&a, b.clone());
        assert!(locks.is_held(&b) && !locks.is_held(&a));
        assert!(locks.is_held_under(&p.root));
        assert!(!locks.is_held_under(&p.root.join("elsewhere")));
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_paths_compare_without_regard_to_case() {
        let p = project();
        let a = p.file("A.rs");
        let mut locks = p.locks();
        locks.file_opened(a.clone());
        let upper = p.raw("A.rs").to_uppercase();
        assert_eq!(
            locks.tool_started("job:1", "t1", &[upper]),
            vec![locked(&a)]
        );
    }

    #[test]
    fn only_edits_deletions_and_moves_are_file_kinds() {
        for kind in ["edit", "delete", "move"] {
            assert!(is_file_kind(kind));
        }
        for kind in ["execute", "other", "read", "search", "think", "fetch"] {
            assert!(!is_file_kind(kind));
        }
    }
}
