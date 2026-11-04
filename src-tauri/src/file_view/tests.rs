//! The File view service with temporary folders, a clock moved by hand, real watches with a short
//! quiet time and fake tab hooks.

use std::collections::BTreeSet;
use std::fs;
use std::sync::mpsc;
use std::sync::Mutex;

use weaver_core::file_view::machine::State;
use weaver_core::project::clock::ManualClock;

use super::*;

const PROJECT: &str = "0123456789abcdef";

/// The tabs and the lock of the window, as the tree sees them.
#[derive(Default)]
struct FakeTabs {
    locked: Mutex<BTreeSet<String>>,
    calls: Mutex<Vec<String>>,
}

impl FakeTabs {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl TabHooks for FakeTabs {
    fn is_locked_under(&self, _: &str, rel: &str) -> bool {
        self.locked
            .lock()
            .unwrap()
            .iter()
            .any(|path| weaver_core::file_view::is_within(rel, path))
    }

    fn rename_under(&self, _: &str, old: &str, new: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("rename {old} -> {new}"));
    }

    fn close_under(&self, _: &str, rel: &str) {
        self.calls.lock().unwrap().push(format!("close {rel}"));
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    clock: Arc<ManualClock>,
    tabs: Arc<FakeTabs>,
    service: FileViewService,
    heard: mpsc::Receiver<TreeChanged>,
}

fn put(root: &Path, rel: &str) {
    let target = root.join(rel);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, "x").unwrap();
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = weaver_core::paths::canonical_root(dir.path()).unwrap();
    put(&root, "a.txt");
    put(&root, "src/main.rs");
    put(&root, "src/deep/inner.rs");
    put(&root, ".weaver/tasks/t.md");
    let clock = Arc::new(ManualClock::new(1_000));
    let tabs = Arc::new(FakeTabs::default());
    let service =
        FileViewService::with_timing(clock.clone(), Duration::from_millis(40), tabs.clone());
    let (sender, heard) = mpsc::channel();
    let sender = Mutex::new(sender);
    service.attach(Arc::new(move |changed| {
        let _ = sender.lock().unwrap().send(changed);
    }));
    Fixture {
        _dir: dir,
        root,
        clock,
        tabs,
        service,
        heard,
    }
}

fn names(folder: &FolderListing) -> Vec<&str> {
    folder.entries.iter().map(|e| e.name.as_str()).collect()
}

fn settle(f: &Fixture) {
    std::thread::sleep(Duration::from_millis(400));
    while f.heard.try_recv().is_ok() {}
}

fn reason<T: std::fmt::Debug>(result: Result<T, Refusal>) -> String {
    result.unwrap_err().to_string()
}

#[test]
fn a_window_that_never_showed_the_view_holds_nothing_and_refuses_its_actions() {
    let f = fixture();
    assert_eq!(f.service.state(PROJECT), State::Closed);
    assert_eq!(
        reason(f.service.expand(PROJECT, "src")),
        "The File view is not open."
    );
    assert_eq!(
        reason(f.service.create(PROJECT, "", "n", false)),
        "The File view is not open."
    );
    assert_eq!(
        reason(f.service.delete(PROJECT, "a.txt")),
        "The File view is not open."
    );
    assert!(f.root.join("a.txt").exists());
    assert_eq!(f.service.leave(PROJECT), None);
    assert!(!f.service.release_due(PROJECT));
}

#[test]
fn showing_the_view_gives_the_root_and_expanding_one_folder_gives_that_folder() {
    let f = fixture();
    let snapshot = f.service.open(PROJECT, &f.root).unwrap();
    assert_eq!(snapshot.folders.len(), 1);
    assert_eq!(names(&snapshot.folders[0]), [".weaver", "src", "a.txt"]);
    let update = f.service.expand(PROJECT, "src").unwrap();
    assert_eq!(update.folders.len(), 1);
    assert_eq!(names(&update.folders[0]), ["deep", "main.rs"]);
    assert!(update.generation > snapshot.generation);
    f.service.collapse(PROJECT, "src").unwrap();
    let again = f.service.open(PROJECT, &f.root).unwrap();
    assert_eq!(again.folders.len(), 1, "the collapsed folder is forgotten");
}

#[test]
fn a_new_entry_in_an_expanded_folder_reaches_the_window_with_its_generation() {
    let f = fixture();
    let first = f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    settle(&f);
    put(&f.root, "src/new.rs");
    let changed = f
        .heard
        .recv_timeout(Duration::from_secs(20))
        .expect("an update");
    assert_eq!(changed.project_id, PROJECT);
    assert!(changed.update.generation > first.generation);
    let listing = listing_of(&changed.update, "src").expect("the folder src");
    assert_eq!(names(&listing), ["deep", "main.rs", "new.rs"]);
    // Only the folders that were read again travel.
    assert!(listing_of(&changed.update, "").is_none());
}

#[test]
fn the_event_serialises_flat_with_the_project_id_and_the_generation() {
    let changed = TreeChanged {
        project_id: PROJECT.to_owned(),
        update: TreeUpdate {
            generation: 3,
            folders: Vec::new(),
            removed: vec!["src".to_owned()],
        },
    };
    assert_eq!(
        serde_json::to_value(changed).unwrap(),
        serde_json::json!({ "projectId": PROJECT, "generation": 3, "folders": [], "removed": ["src"] })
    );
}

#[test]
fn leaving_keeps_the_tree_for_ten_seconds_and_then_releases_everything() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    assert_eq!(f.service.leave(PROJECT), Some(Duration::from_secs(10)));
    assert_eq!(f.service.state(PROJECT), State::Releasing);
    f.clock.advance(9_999);
    assert!(!f.service.release_due(PROJECT));
    f.clock.advance(1);
    assert!(f.service.release_due(PROJECT));
    assert_eq!(f.service.state(PROJECT), State::Closed);
    assert_eq!(
        reason(f.service.expand(PROJECT, "src")),
        "The File view is not open."
    );
    // Nothing is watched any more: what happens in the folders is not heard.
    settle(&f);
    put(&f.root, "src/after-release.rs");
    assert!(f.heard.recv_timeout(Duration::from_millis(700)).is_err());
}

#[test]
fn showing_the_view_again_in_time_cancels_the_release_and_reads_nothing_again() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    f.service.leave(PROJECT);
    f.clock.advance(9_000);
    let snapshot = f.service.open(PROJECT, &f.root).unwrap();
    let paths: Vec<&str> = snapshot.folders.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["", "src"], "the tree is as it was");
    f.clock.advance(5_000);
    assert!(!f.service.release_due(PROJECT));
    assert_eq!(f.service.state(PROJECT), State::Open);
}

#[test]
fn a_folder_that_changes_while_released_a_short_while_is_read_again_without_telling_the_window() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    settle(&f);
    f.service.leave(PROJECT);
    put(&f.root, "src/while-away.rs");
    assert!(
        f.heard.recv_timeout(Duration::from_millis(900)).is_err(),
        "the window shows another view"
    );
    let snapshot = f.service.open(PROJECT, &f.root).unwrap();
    let src = snapshot.folders.iter().find(|f| f.path == "src").unwrap();
    assert!(
        names(src).contains(&"while-away.rs"),
        "the tree is right when the view opens again"
    );
}

#[test]
fn closing_the_window_drops_everything_from_open_and_from_releasing() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    f.service.close_project(PROJECT);
    assert_eq!(f.service.state(PROJECT), State::Closed);
    settle(&f);
    put(&f.root, "src/closed.rs");
    assert!(f.heard.recv_timeout(Duration::from_millis(700)).is_err());

    f.service.open(PROJECT, &f.root).unwrap();
    f.service.leave(PROJECT);
    f.service.close_project(PROJECT);
    assert_eq!(f.service.state(PROJECT), State::Closed);
    assert!(!f.service.release_due(PROJECT));
}

#[test]
fn creating_puts_the_entry_in_the_tree_at_once_and_inside_weaver_it_is_refused() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    let made = f.service.create(PROJECT, "src", "lib.rs", false).unwrap();
    assert_eq!(made.path, "src/lib.rs");
    assert_eq!(
        names(&listing_of(&made.update, "src").unwrap()),
        ["deep", "lib.rs", "main.rs"]
    );
    assert_eq!(fs::read_to_string(f.root.join("src/lib.rs")).unwrap(), "");
    let folder = f.service.create(PROJECT, "", "docs", true).unwrap();
    assert!(names(&listing_of(&folder.update, "").unwrap()).contains(&"docs"));
    assert_eq!(
        reason(f.service.create(PROJECT, "src", "main.rs", false)),
        "A file or folder named \"main.rs\" already exists here."
    );
    assert_eq!(
        reason(f.service.create(PROJECT, ".weaver", "x", false)),
        ".weaver is written by Weaver: read-only here."
    );
    assert!(!f.root.join(".weaver/x").exists());
}

#[test]
fn renaming_moves_the_tabs_the_tree_and_the_watches_of_a_folder() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    f.service.expand(PROJECT, "src/deep").unwrap();
    let renamed = f.service.rename(PROJECT, "src", "lib").unwrap();
    assert_eq!(renamed.path, "lib");
    assert_eq!(f.tabs.calls(), ["rename src -> lib"]);
    assert_eq!(renamed.update.removed, ["src", "src/deep"]);
    let paths: Vec<&str> = renamed
        .update
        .folders
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    assert_eq!(paths, ["lib", "lib/deep", ""]);
    // The moved folders are watched under their new names.
    settle(&f);
    put(&f.root, "lib/deep/fresh.rs");
    let changed = f
        .heard
        .recv_timeout(Duration::from_secs(20))
        .expect("an update");
    assert!(listing_of(&changed.update, "lib/deep").is_some());
}

#[test]
fn a_refused_rename_changes_nothing_and_tells_the_tabs_nothing() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    assert_eq!(
        reason(f.service.rename(PROJECT, "a.txt", "src")),
        "A file or folder named \"src\" already exists here."
    );
    assert!(reason(f.service.rename(PROJECT, "a.txt", "x/y")).contains("cannot contain"));
    assert_eq!(
        reason(f.service.rename(PROJECT, ".weaver", "w")),
        ".weaver is written by Weaver: read-only here."
    );
    f.tabs
        .locked
        .lock()
        .unwrap()
        .insert("src/main.rs".to_owned());
    assert_eq!(
        reason(f.service.rename(PROJECT, "src", "lib")),
        "The agent is editing this file."
    );
    assert_eq!(
        reason(f.service.rename(PROJECT, "src/main.rs", "m.rs")),
        "The agent is editing this file."
    );
    assert!(f.root.join("src/main.rs").exists() && f.root.join("a.txt").exists());
    assert!(f.tabs.calls().is_empty());
}

#[test]
fn a_refused_rename_or_a_refusing_trash_leaves_the_folders_watched_under_their_old_names() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    f.service.expand(PROJECT, "src/deep").unwrap();
    assert!(f.service.rename(PROJECT, "src", "a.txt").is_err());
    settle(&f);
    put(&f.root, "src/deep/fresh.rs");
    let changed = f
        .heard
        .recv_timeout(Duration::from_secs(20))
        .expect("an update");
    assert!(listing_of(&changed.update, "src/deep").is_some());

    let refusing = |_: &Path| -> Result<(), String> { Err("no trash here".to_owned()) };
    assert!(f.service.delete_with(PROJECT, "src", &refusing).is_err());
    settle(&f);
    put(&f.root, "src/deep/fresher.rs");
    let changed = f
        .heard
        .recv_timeout(Duration::from_secs(20))
        .expect("an update");
    assert!(listing_of(&changed.update, "src/deep").is_some());
    assert!(f.tabs.calls().is_empty());
}

#[test]
fn moving_takes_the_tabs_along_and_refuses_what_the_rules_refuse() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    let moved = f.service.move_into(PROJECT, "a.txt", "src").unwrap();
    assert_eq!(moved.path, "src/a.txt");
    assert_eq!(f.tabs.calls(), ["rename a.txt -> src/a.txt"]);
    assert!(names(&listing_of(&moved.update, "src").unwrap()).contains(&"a.txt"));
    assert!(reason(f.service.move_into(PROJECT, "src", "src/deep")).contains("into itself"));
    assert_eq!(
        reason(f.service.move_into(PROJECT, "src/a.txt", ".weaver")),
        ".weaver is written by Weaver: read-only here."
    );
    assert_eq!(f.tabs.calls().len(), 1);
}

#[test]
fn moving_an_expanded_folder_keeps_it_and_the_folders_inside_it_watched_under_their_new_names() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    f.service.expand(PROJECT, "src/deep").unwrap();
    let moved = f.service.move_into(PROJECT, "src/deep", "").unwrap();
    assert_eq!(moved.path, "deep");
    assert_eq!(moved.update.removed, ["src/deep"]);
    settle(&f);
    put(&f.root, "deep/fresh.rs");
    let changed = f
        .heard
        .recv_timeout(Duration::from_secs(20))
        .expect("an update");
    assert!(listing_of(&changed.update, "deep").is_some());
}

#[test]
fn deleting_goes_through_the_trash_closes_the_tabs_and_a_refusing_trash_deletes_nothing() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    let refusing = |_: &Path| -> Result<(), String> { Err("no trash here".to_owned()) };
    assert_eq!(
        reason(f.service.delete_with(PROJECT, "src", &refusing)),
        "Could not move \"src\" to the trash: no trash here"
    );
    assert!(f.root.join("src/main.rs").exists());
    assert!(f.tabs.calls().is_empty());

    let bin = tempfile::tempdir().unwrap();
    let bin_path = bin.path().to_path_buf();
    let fake = move |entry: &Path| -> Result<(), String> {
        fs::rename(entry, bin_path.join(entry.file_name().unwrap())).map_err(|e| e.to_string())
    };
    let update = f.service.delete_with(PROJECT, "src", &fake).unwrap();
    assert!(!f.root.join("src").exists());
    assert_eq!(f.tabs.calls(), ["close src"]);
    assert_eq!(update.removed, ["src"]);
    assert_eq!(
        names(&listing_of(&update, "").unwrap()),
        [".weaver", "a.txt"]
    );
    assert_eq!(
        reason(f.service.delete_with(PROJECT, ".weaver", &fake)),
        ".weaver is written by Weaver: read-only here."
    );
}

/// A folder next to the project with something to drop.
fn dropped(names: &[&str]) -> (tempfile::TempDir, Vec<PathBuf>) {
    let dir = tempfile::tempdir().unwrap();
    let base = weaver_core::paths::canonical_root(dir.path()).unwrap();
    let paths = names
        .iter()
        .map(|name| {
            put(&base, name);
            base.join(name.split('/').next().unwrap())
        })
        .collect();
    (dir, paths)
}

#[test]
fn copying_in_puts_the_entries_in_the_target_folder_and_in_the_tree_and_keeps_the_originals() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    f.service.expand(PROJECT, "src").unwrap();
    let (_outside, paths) = dropped(&["notes.txt", "assets/logo.png"]);

    let result = f.service.copy_in(PROJECT, "src", &paths).unwrap();

    assert_eq!(result.paths, ["src/assets", "src/notes.txt"]);
    assert!(f.root.join("src/assets/logo.png").is_file());
    assert!(f.root.join("src/notes.txt").is_file());
    assert!(paths.iter().all(|path| path.exists()));
    assert_eq!(
        names(&listing_of(&result.update, "src").unwrap()),
        ["assets", "deep", "main.rs", "notes.txt"]
    );
    // The guard went with the copy: another one can start.
    let (_more, more) = dropped(&["more.txt"]);
    assert!(f.service.copy_in(PROJECT, "", &more).is_ok());
}

#[test]
fn a_refused_drop_copies_nothing_and_the_view_must_be_open() {
    let f = fixture();
    let (_outside, paths) = dropped(&["x.txt"]);
    assert_eq!(
        reason(f.service.copy_in(PROJECT, "", &paths)),
        "The File view is not open."
    );
    f.service.open(PROJECT, &f.root).unwrap();
    assert_eq!(
        reason(f.service.copy_in(PROJECT, ".weaver", &paths)),
        ".weaver is written by Weaver: read-only here."
    );
    assert_eq!(
        reason(f.service.copy_in(PROJECT, "", &[f.root.join("a.txt")])),
        "\"a.txt\" is already in the project: drag it inside the tree to move it."
    );
    assert!(!f.root.join("x.txt").exists());
    assert!(!f.root.join(".weaver/x.txt").exists());
}

#[test]
fn a_failure_part_way_is_told_as_the_error_and_leaves_the_project_as_it_was() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    let (_outside, paths) = dropped(&["one.txt", "two.txt"]);
    let calls = std::cell::Cell::new(0);
    let failing = |_: &Path| {
        calls.set(calls.get() + 1);
        if calls.get() == 2 {
            Err(io::Error::other("disk on fire"))
        } else {
            Ok(())
        }
    };

    assert_eq!(
        reason(f.service.copy_in_with(PROJECT, "", &paths, &failing)),
        "Could not copy \"two.txt\": disk on fire. Nothing was copied."
    );
    assert!(!f.root.join("one.txt").exists());
    assert!(!f.root.join("two.txt").exists());
    // The failed copy released the guard.
    assert!(f.service.copy_in(PROJECT, "", &paths).is_ok());
}

/// Runs a drop that waits in its first step until told, so a test can act while it runs.
fn with_running_copy<R>(
    f: &Fixture,
    paths: &[PathBuf],
    while_running: impl FnOnce() -> R,
) -> (R, Result<CopyInResult, Refusal>) {
    let (started, wait_started) = mpsc::channel::<()>();
    let (release, wait_release) = mpsc::channel::<()>();
    let wait_release = Mutex::new(wait_release);
    let started = Mutex::new(started);
    let service = &f.service;
    std::thread::scope(|scope| {
        let copy = scope.spawn(|| {
            let first = std::cell::Cell::new(true);
            service.copy_in_with(PROJECT, "", paths, &|_| {
                if first.replace(false) {
                    started.lock().unwrap().send(()).unwrap();
                    wait_release.lock().unwrap().recv().unwrap();
                }
                Ok(())
            })
        });
        wait_started.recv().unwrap();
        let during = while_running();
        release.send(()).unwrap();
        (during, copy.join().unwrap())
    })
}

#[test]
fn a_second_drop_is_refused_while_one_copies_and_copies_nothing() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    let (_first_dir, first) = dropped(&["first.txt"]);
    let (_second_dir, second) = dropped(&["second.txt"]);

    let (second_result, first_result) =
        with_running_copy(&f, &first, || f.service.copy_in(PROJECT, "", &second));

    assert_eq!(
        reason(second_result),
        "Another copy is still running in this window."
    );
    assert!(!f.root.join("second.txt").exists());
    assert_eq!(first_result.unwrap().paths, ["first.txt"]);
    assert!(f.root.join("first.txt").is_file());
    assert!(f.service.copy_in(PROJECT, "", &second).is_ok());
}

#[test]
fn the_copy_goes_to_its_end_when_the_view_is_left_or_the_window_closes() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    let (_dir, paths) = dropped(&["assets/a.png", "assets/b.png", "z.txt"]);
    let paths = vec![paths[0].clone(), paths[2].clone()];

    let (_, result) = with_running_copy(&f, &paths, || {
        f.service.leave(PROJECT);
        f.service.close_project(PROJECT);
    });

    assert_eq!(result.unwrap().paths, ["assets", "z.txt"]);
    assert!(f.root.join("assets/a.png").is_file());
    assert!(f.root.join("assets/b.png").is_file());
    assert!(f.root.join("z.txt").is_file());
    // The guard went with the window: a window of the same project starts clean.
    assert_eq!(f.service.state(PROJECT), State::Closed);
    f.service.open(PROJECT, &f.root).unwrap();
    let (_more_dir, more) = dropped(&["late.txt"]);
    assert!(f.service.copy_in(PROJECT, "", &more).is_ok());
}

#[test]
fn the_folders_for_the_move_dialog_are_listed_one_folder_at_a_time_and_not_held() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    let folders = f.service.folders(PROJECT, "").unwrap();
    let names: Vec<&str> = folders.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [".weaver", "src"]);
    let inner = f.service.folders(PROJECT, "src").unwrap();
    assert_eq!(inner[0].path, "src/deep");
    // Nothing was expanded by it.
    let snapshot = f.service.open(PROJECT, &f.root).unwrap();
    assert_eq!(snapshot.folders.len(), 1);
}

#[test]
fn the_release_timer_waits_what_is_left_and_ends_when_the_view_was_shown_again() {
    let f = fixture();
    f.service.open(PROJECT, &f.root).unwrap();
    assert_eq!(
        f.service.release_step(PROJECT),
        None,
        "an open view waits for no release"
    );
    f.service.leave(PROJECT);
    f.clock.advance(9_000);
    assert_eq!(
        f.service.release_step(PROJECT),
        Some(Duration::from_secs(1))
    );
    f.service.open(PROJECT, &f.root).unwrap();
    f.clock.advance(5_000);
    assert_eq!(f.service.release_step(PROJECT), None);
    assert_eq!(f.service.state(PROJECT), State::Open);
    f.service.leave(PROJECT);
    f.clock.advance(10_000);
    assert_eq!(f.service.release_step(PROJECT), None);
    assert_eq!(f.service.state(PROJECT), State::Closed);
}
