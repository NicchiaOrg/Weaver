//! The tabs of the File view against temporary folders, with a fake watcher that tells nothing
//! until a test says a file changed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::json;
use weaver_core::file_tab::limit::TabInfo;
use weaver_core::file_tab::read::sha256_bytes;

use super::events::testing::{recording, Recorder};
use super::watch::{ChangeSink, TabWatch, TabWatcher};
use super::wire::{
    FileContent, FileKind, FileOpenResult, FileReloadResult, ReadOnlyReason, TabChange,
};
use super::{FileTabs, SaveMode, SaveOutcome};
use crate::file_lock::FileLockService;
use crate::pictures::Rebuild;

/// The watch of tests: remembers the folders watched and the sink, and drops with the tabs.
#[derive(Default)]
struct FakeWatch {
    sink: Mutex<Option<ChangeSink>>,
    folders: Arc<Mutex<Vec<PathBuf>>>,
    started: Mutex<usize>,
    alive: Arc<Mutex<usize>>,
}

struct FakeWatcher {
    folders: Arc<Mutex<Vec<PathBuf>>>,
    alive: Arc<Mutex<usize>>,
}

impl TabWatcher for FakeWatcher {
    fn watch(&mut self, folder: &Path) {
        self.folders.lock().unwrap().push(folder.to_path_buf());
    }

    fn unwatch(&mut self, folder: &Path) {
        let mut folders = self.folders.lock().unwrap();
        if let Some(at) = folders.iter().position(|f| f == folder) {
            folders.remove(at);
        }
    }
}

impl Drop for FakeWatcher {
    fn drop(&mut self) {
        *self.alive.lock().unwrap() -= 1;
    }
}

impl TabWatch for FakeWatch {
    fn start(&self, sink: ChangeSink) -> Result<Box<dyn TabWatcher>, String> {
        *self.sink.lock().unwrap() = Some(sink);
        *self.started.lock().unwrap() += 1;
        *self.alive.lock().unwrap() += 1;
        Ok(Box::new(FakeWatcher {
            folders: self.folders.clone(),
            alive: self.alive.clone(),
        }))
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    id: String,
    tabs: Arc<FileTabs>,
    locks: Arc<FileLockService>,
    recorder: Arc<Recorder>,
    watch: Arc<FakeWatch>,
}

fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = weaver_core::paths::canonical_root(dir.path()).unwrap();
    let (events, recorder) = recording();
    let locks = FileLockService::new(events.clone());
    let watch = Arc::new(FakeWatch::default());
    let tabs = FileTabs::new(locks.clone(), events, watch.clone());
    Harness {
        _dir: dir,
        root,
        id: "p1".to_owned(),
        tabs,
        locks,
        recorder,
        watch,
    }
}

impl Harness {
    fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    fn open(&self, rel: &str) -> FileOpenResult {
        self.open_among(rel, &[])
    }

    fn open_among(&self, rel: &str, shown: &[TabInfo]) -> FileOpenResult {
        self.tabs.open(&self.id, &self.root, rel, shown).unwrap()
    }

    /// The watch says that `rel` changed.
    fn changed(&self, rel: &str) {
        let sink = self
            .watch
            .sink
            .lock()
            .unwrap()
            .clone()
            .expect("a watcher started");
        sink(Rebuild::Paths(vec![self.root.join(rel)]));
    }

    fn tab_events(&self) -> Vec<(String, TabChange)> {
        self.recorder
            .tabs
            .lock()
            .unwrap()
            .iter()
            .map(|e| (e.path.clone(), e.change))
            .collect()
    }

    fn tool(&self, conversation: &str, update: serde_json::Value) {
        self.locks
            .tool_update(&self.id, &self.root, conversation, &update);
    }
}

fn opened(result: FileOpenResult) -> (String, FileContent, bool, Option<String>) {
    match result {
        FileOpenResult::Opened {
            path,
            content,
            locked,
            closed,
        } => (path, content, locked, closed),
        other => panic!("not opened: {other:?}"),
    }
}

fn tab(path: &str, unsaved: bool) -> TabInfo {
    TabInfo {
        path: path.to_owned(),
        unsaved,
    }
}

#[test]
fn a_text_file_opens_with_lf_no_bom_and_the_hash_of_its_raw_bytes() {
    let h = harness();
    let bytes = b"\xEF\xBB\xBFone\r\ntwo\r\n";
    h.write("src/a.txt", bytes);
    let (path, content, locked, closed) = opened(h.open("src/a.txt"));
    assert_eq!(path, "src/a.txt");
    assert_eq!(content.kind, FileKind::Text);
    assert_eq!(content.text.as_deref(), Some("one\ntwo\n"));
    assert_eq!(content.hash, sha256_bytes(bytes));
    assert_eq!(content.read_only, None);
    assert!(!locked && closed.is_none());
    assert_eq!(*h.watch.started.lock().unwrap(), 1);
    assert_eq!(
        h.watch.folders.lock().unwrap().clone(),
        vec![h.root.join("src")]
    );
}

#[test]
fn a_binary_file_opens_without_text() {
    let h = harness();
    h.write("image.bin", &[0, 1, 2, 3]);
    let (_, content, _, _) = opened(h.open("image.bin"));
    assert_eq!(content.kind, FileKind::Binary);
    assert_eq!(content.text, None);
    assert_eq!(content.hash, sha256_bytes(&[0, 1, 2, 3]));
}

#[test]
fn a_large_file_and_a_file_in_weaver_are_read_only_and_never_written() {
    let h = harness();
    h.write("big.txt", &vec![b'a'; 2_097_153]);
    h.write(".weaver/model/system.md", b"# System\n");
    let (_, big, _, _) = opened(h.open("big.txt"));
    assert_eq!(big.read_only, Some(ReadOnlyReason::TooLarge));
    let (_, weaver, _, _) = opened(h.open(".weaver/model/system.md"));
    assert_eq!(weaver.read_only, Some(ReadOnlyReason::Weaver));
    for rel in ["big.txt", ".weaver/model/system.md"] {
        let outcome = h.tabs.save(&h.id, rel, "changed\n", SaveMode::Overwrite);
        assert!(matches!(outcome, SaveOutcome::Failed(_)), "{outcome:?}");
    }
    assert_eq!(
        fs::read(h.root.join(".weaver/model/system.md")).unwrap(),
        b"# System\n"
    );
}

#[test]
fn paths_that_leave_the_project_or_name_missing_files_and_folders_are_refused() {
    let h = harness();
    h.write("a.txt", b"x");
    fs::create_dir(h.root.join("dir")).unwrap();
    for rel in ["../x", "/etc/hosts", ".git/config", "missing.txt", "dir"] {
        assert!(
            h.tabs.open(&h.id, &h.root, rel, &[]).is_err(),
            "{rel} must be refused"
        );
    }
}

#[test]
fn opening_a_file_that_is_open_shows_its_tab() {
    let h = harness();
    h.write("a.txt", b"x");
    opened(h.open("a.txt"));
    assert_eq!(
        h.open("./a.txt"),
        FileOpenResult::AlreadyOpen {
            path: "a.txt".to_owned()
        }
    );
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_and_its_target_are_the_same_file() {
    let h = harness();
    h.write("real.txt", b"x");
    std::os::unix::fs::symlink(h.root.join("real.txt"), h.root.join("link.txt")).unwrap();
    let (path, _, _, _) = opened(h.open("link.txt"));
    assert_eq!(
        path, "real.txt",
        "the tab is on the file the link points to"
    );
    assert_eq!(
        h.open("real.txt"),
        FileOpenResult::AlreadyOpen {
            path: "real.txt".to_owned()
        }
    );
}

#[test]
fn the_eleventh_file_closes_the_least_recently_shown_tab_without_changes() {
    let h = harness();
    let mut shown = Vec::new();
    for n in 0..10 {
        let rel = format!("f{n}.txt");
        h.write(&rel, b"x");
        opened(h.open_among(&rel, &shown));
        shown.push(tab(&rel, n < 2));
    }
    h.write("eleventh.txt", b"x");
    let (_, _, _, closed) = opened(h.open_among("eleventh.txt", &shown));
    assert_eq!(closed.as_deref(), Some("f2.txt"));
    // The closed tab is gone: its file can be opened again and counts as a new tab.
    assert!(matches!(
        h.open_among("f2.txt", &[tab("f3.txt", false)]),
        FileOpenResult::Opened { .. }
    ));
}

#[test]
fn with_ten_tabs_that_all_have_changes_nothing_opens() {
    let h = harness();
    let mut shown = Vec::new();
    for n in 0..10 {
        let rel = format!("f{n}.txt");
        h.write(&rel, b"x");
        opened(h.open_among(&rel, &shown));
        shown.push(tab(&rel, true));
    }
    h.write("eleventh.txt", b"x");
    assert_eq!(h.open_among("eleventh.txt", &shown), FileOpenResult::Full);
    assert_eq!(
        h.tabs.save(&h.id, "eleventh.txt", "x", SaveMode::Check),
        SaveOutcome::Failed("This file is not open in a tab.".to_owned())
    );
}

#[test]
fn a_save_writes_the_text_with_the_line_ending_and_bom_of_the_tab() {
    let h = harness();
    let path = h.write("a.txt", b"\xEF\xBB\xBFone\r\ntwo\r\n");
    opened(h.open("a.txt"));
    let SaveOutcome::Saved { hash } =
        h.tabs
            .save(&h.id, "a.txt", "one\nTWO\nthree", SaveMode::Check)
    else {
        panic!("not saved")
    };
    assert_eq!(fs::read(&path).unwrap(), b"\xEF\xBB\xBFone\r\nTWO\r\nthree");
    assert_eq!(hash, sha256_bytes(b"\xEF\xBB\xBFone\r\nTWO\r\nthree"));
    // The tab holds the hash it wrote: a second save passes the check.
    assert!(matches!(
        h.tabs.save(&h.id, "a.txt", "again\n", SaveMode::Check),
        SaveOutcome::Saved { .. }
    ));
}

#[test]
fn a_save_after_another_program_changed_the_file_asks_instead_of_writing() {
    let h = harness();
    let path = h.write("a.txt", b"mine\n");
    opened(h.open("a.txt"));
    fs::write(&path, b"theirs\n").unwrap();
    assert_eq!(
        h.tabs.save(&h.id, "a.txt", "mine!\n", SaveMode::Check),
        SaveOutcome::Changed
    );
    assert_eq!(fs::read(&path).unwrap(), b"theirs\n");
    // Overwrite writes anyway, and then the check passes again.
    assert!(matches!(
        h.tabs.save(&h.id, "a.txt", "mine!\n", SaveMode::Overwrite),
        SaveOutcome::Saved { .. }
    ));
    assert!(matches!(
        h.tabs.save(&h.id, "a.txt", "mine!!\n", SaveMode::Check),
        SaveOutcome::Saved { .. }
    ));
}

#[test]
fn the_check_compares_the_file_with_the_hash_the_command_carries() {
    let h = harness();
    let path = h.write("a.txt", b"mine\n");
    opened(h.open("a.txt"));
    let stale = sha256_bytes(b"older\n");
    assert_eq!(
        h.tabs
            .save_read(&h.id, "a.txt", "x\n", SaveMode::Check, Some(&stale)),
        SaveOutcome::Changed
    );
    assert_eq!(fs::read(&path).unwrap(), b"mine\n");
    let read = sha256_bytes(b"mine\n");
    assert!(matches!(
        h.tabs
            .save_read(&h.id, "a.txt", "x\n", SaveMode::Check, Some(&read)),
        SaveOutcome::Saved { .. }
    ));
}

#[test]
fn a_save_of_a_file_that_is_gone_says_so_and_save_again_writes_it_back() {
    let h = harness();
    let path = h.write("a.txt", b"x\n");
    opened(h.open("a.txt"));
    fs::remove_file(&path).unwrap();
    assert_eq!(
        h.tabs.save(&h.id, "a.txt", "y\n", SaveMode::Check),
        SaveOutcome::Gone
    );
    assert!(!path.exists());
    assert!(matches!(
        h.tabs.save(&h.id, "a.txt", "y\n", SaveMode::Again),
        SaveOutcome::Saved { .. }
    ));
    assert_eq!(fs::read(&path).unwrap(), b"y\n");
    // Save again refuses when something exists at the path now.
    fs::write(&path, b"other\n").unwrap();
    assert_eq!(
        h.tabs.save(&h.id, "a.txt", "z\n", SaveMode::Again),
        SaveOutcome::Changed
    );
}

#[test]
fn a_failed_write_gives_the_reason_and_changes_nothing() {
    let h = harness();
    h.write("a.txt", b"x\n");
    opened(h.open("a.txt"));
    // The file is replaced by a folder with something inside: the write cannot replace it.
    fs::remove_file(h.root.join("a.txt")).unwrap();
    fs::create_dir(h.root.join("a.txt")).unwrap();
    fs::write(h.root.join("a.txt").join("inner"), b"keep").unwrap();
    let SaveOutcome::Failed(reason) = h.tabs.save(&h.id, "a.txt", "y\n", SaveMode::Overwrite)
    else {
        panic!("the write should fail")
    };
    assert!(!reason.is_empty());
    assert_eq!(
        fs::read(h.root.join("a.txt").join("inner")).unwrap(),
        b"keep"
    );
}

#[test]
fn the_watch_event_of_the_tabs_own_save_is_not_a_change() {
    let h = harness();
    h.write("a.txt", b"x\n");
    opened(h.open("a.txt"));
    h.tabs.save(&h.id, "a.txt", "y\n", SaveMode::Check);
    h.changed("a.txt");
    assert_eq!(h.tab_events(), vec![]);
}

#[test]
fn another_programs_change_deletion_and_return_are_told_once_each() {
    let h = harness();
    let path = h.write("a.txt", b"x\n");
    opened(h.open("a.txt"));

    fs::write(&path, b"changed\n").unwrap();
    h.changed("a.txt");
    h.changed("a.txt");
    assert_eq!(
        h.tab_events(),
        vec![("a.txt".to_owned(), TabChange::Changed)]
    );

    fs::remove_file(&path).unwrap();
    h.changed("a.txt");
    h.changed("a.txt");
    assert_eq!(h.tab_events().len(), 2);
    assert_eq!(h.tab_events()[1].1, TabChange::Deleted);

    // It comes back, even with the content the tab holds.
    fs::write(&path, b"x\n").unwrap();
    h.changed("a.txt");
    assert_eq!(h.tab_events().len(), 3);
    assert_eq!(h.tab_events()[2].1, TabChange::Changed);
}

#[test]
fn an_event_on_a_file_with_the_hash_the_tab_holds_is_not_a_change() {
    let h = harness();
    let path = h.write("a.txt", b"x\n");
    opened(h.open("a.txt"));
    fs::write(&path, b"x\n").unwrap();
    h.changed("a.txt");
    assert_eq!(h.tab_events(), vec![]);
}

#[test]
fn an_event_that_names_other_files_touches_no_tab() {
    let h = harness();
    h.write("a.txt", b"x\n");
    let other = h.write("b.txt", b"y\n");
    opened(h.open("a.txt"));
    fs::write(&other, b"changed\n").unwrap();
    h.changed("b.txt");
    assert_eq!(h.tab_events(), vec![]);
}

#[test]
fn a_reload_reads_the_file_again_and_the_check_then_passes() {
    let h = harness();
    let path = h.write("a.txt", b"one\n");
    opened(h.open("a.txt"));
    fs::write(&path, b"two\r\nthree\r\n").unwrap();
    let FileReloadResult::Read { content } = h.tabs.reload(&h.id, "a.txt").unwrap() else {
        panic!("gone")
    };
    assert_eq!(content.text.as_deref(), Some("two\nthree\n"));
    assert_eq!(content.hash, sha256_bytes(b"two\r\nthree\r\n"));
    assert!(matches!(
        h.tabs
            .save(&h.id, "a.txt", "two\nthree\nfour\n", SaveMode::Check),
        SaveOutcome::Saved { .. }
    ));
    assert_eq!(fs::read(&path).unwrap(), b"two\r\nthree\r\nfour\r\n");
    fs::remove_file(&path).unwrap();
    assert_eq!(
        h.tabs.reload(&h.id, "a.txt").unwrap(),
        FileReloadResult::Gone
    );
}

#[test]
fn closing_a_tab_frees_its_watch_and_the_last_one_frees_the_watcher() {
    let h = harness();
    h.write("src/a.txt", b"x");
    h.write("src/b.txt", b"y");
    opened(h.open("src/a.txt"));
    opened(h.open("src/b.txt"));
    assert_eq!(h.watch.folders.lock().unwrap().len(), 2);
    h.tabs.close(&h.id, "src/a.txt");
    assert_eq!(h.watch.folders.lock().unwrap().len(), 1);
    assert_eq!(*h.watch.alive.lock().unwrap(), 1);
    h.tabs.close(&h.id, "src/b.txt");
    assert!(h.watch.folders.lock().unwrap().is_empty());
    assert_eq!(
        *h.watch.alive.lock().unwrap(),
        0,
        "no watch is left with no tab"
    );
    assert!(h.tabs.reload(&h.id, "src/a.txt").is_err());
}

#[test]
fn closing_the_window_drops_every_tab_and_watch() {
    let h = harness();
    h.write("a.txt", b"x");
    h.write("b.txt", b"y");
    opened(h.open("a.txt"));
    opened(h.open("b.txt"));
    h.tabs.close_project(&h.id);
    assert_eq!(*h.watch.alive.lock().unwrap(), 0);
    assert!(h.tabs.reload(&h.id, "a.txt").is_err());
}

#[test]
fn a_tab_on_a_renamed_entry_or_inside_it_follows_and_one_on_a_deleted_entry_closes() {
    let h = harness();
    h.write("old/a.txt", b"x");
    h.write("old/deep/b.txt", b"y");
    h.write("other.txt", b"z");
    opened(h.open("old/a.txt"));
    opened(h.open("old/deep/b.txt"));
    opened(h.open("other.txt"));
    fs::rename(h.root.join("old"), h.root.join("new")).unwrap();
    h.tabs.rename_under(&h.id, "old", "new");
    assert!(h.tabs.reload(&h.id, "new/a.txt").is_ok());
    assert!(h.tabs.reload(&h.id, "new/deep/b.txt").is_ok());
    assert!(h.tabs.reload(&h.id, "old/a.txt").is_err());
    // Its watch followed too.
    let folders = h.watch.folders.lock().unwrap().clone();
    assert!(folders.contains(&h.root.join("new")) && !folders.contains(&h.root.join("old")));

    h.tabs.close_under(&h.id, "new");
    assert!(h.tabs.reload(&h.id, "new/a.txt").is_err());
    assert!(h.tabs.reload(&h.id, "new/deep/b.txt").is_err());
    assert!(
        h.tabs.reload(&h.id, "other.txt").is_ok(),
        "only that folder's tabs close"
    );
}

#[test]
fn a_prefix_that_is_not_a_folder_boundary_does_not_match() {
    let h = harness();
    h.write("src/a.txt", b"x");
    h.write("src2/b.txt", b"y");
    opened(h.open("src/a.txt"));
    opened(h.open("src2/b.txt"));
    h.tabs.close_under(&h.id, "src");
    assert!(h.tabs.reload(&h.id, "src2/b.txt").is_ok());
}

fn edit(call: &str, status: &str, locations: &[&Path]) -> serde_json::Value {
    let locations: Vec<_> = locations.iter().map(|p| json!({ "path": p })).collect();
    json!({ "sessionUpdate": "tool_call", "toolCallId": call, "kind": "edit", "status": status, "locations": locations })
}

#[test]
fn a_file_that_opens_while_the_agent_changes_it_opens_locked() {
    let h = harness();
    let path = h.write("a.txt", b"x");
    h.write("b.txt", b"y");
    h.tool("job:1", edit("t1", "in_progress", &[&path]));
    assert!(opened(h.open("a.txt")).2);
    assert!(!opened(h.open("b.txt")).2);
    assert!(h.locks.is_locked_under(&h.id, "a.txt"));
    assert!(!h.locks.is_locked_under(&h.id, "b.txt"));
}

#[test]
fn the_lock_follows_a_file_the_agent_moved_and_the_tab_with_it() {
    let h = harness();
    let path = h.write("a.txt", b"x\n");
    let (_, content, _, _) = opened(h.open("a.txt"));
    h.tool("job:1", edit("t1", "in_progress", &[&path]));
    assert_eq!(h.recorder.locks.lock().unwrap().len(), 1);
    assert!(h.recorder.locks.lock().unwrap()[0].locked);

    // The agent moves the file; omp says where in the final update.
    let destination = h.root.join("moved").join("a.txt");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::rename(&path, &destination).unwrap();
    h.tool(
        "job:1",
        json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed",
            "locations": [{ "path": path }, { "path": destination }],
            "rawOutput": { "details": { "sourcePath": path } }
        }),
    );
    let locks = h.recorder.locks.lock().unwrap().clone();
    assert_eq!(locks.len(), 2);
    assert!(!locks[1].locked);
    assert_eq!(locks[1].path, "a.txt");
    assert_eq!(locks[1].moved_to.as_deref(), Some("moved/a.txt"));
    // The tab is at the new path, with the hash it read: the file moved intact, so the check passes.
    assert!(h.tabs.reload(&h.id, "a.txt").is_err());
    assert!(matches!(
        h.tabs.save(&h.id, "moved/a.txt", "y\n", SaveMode::Check),
        SaveOutcome::Saved { .. }
    ));
    assert_eq!(content.hash, sha256_bytes(b"x\n"));
    assert!(h
        .watch
        .folders
        .lock()
        .unwrap()
        .contains(&h.root.join("moved")));
}

#[test]
fn closing_a_tab_ends_its_lock_tracking() {
    let h = harness();
    let path = h.write("a.txt", b"x");
    opened(h.open("a.txt"));
    h.tool("job:1", edit("t1", "in_progress", &[&path]));
    h.tabs.close(&h.id, "a.txt");
    h.tool("job:1", edit("t1", "completed", &[&path]));
    assert_eq!(
        h.recorder.locks.lock().unwrap().len(),
        1,
        "no release for a closed tab"
    );
    assert!(!h.locks.is_locked_under(&h.id, "a.txt"));
}
