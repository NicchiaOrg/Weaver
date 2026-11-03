//! The Machine `file-view` of one project window: Closed, Open and Releasing, the listings the tree
//! shows and the watches kept for them. The tabs are not here; the window's tab service closes
//! them before [`FileView::leave`].
//!
//! | State | What is held |
//! |-------|--------------|
//! | Closed | nothing: no folder of the code read, nothing watched |
//! | Open | the root's entries, those of each expanded folder, one watch per listed folder |
//! | Releasing | the same, kept for [`RELEASE_DELAY`] after the view stopped being shown |
//!
//! | Event | Method |
//! |-------|--------|
//! | `developer.OPEN_FILE_VIEW` | [`FileView::open`] |
//! | `developer.EXPAND_FOLDER` / `COLLAPSE_FOLDER` | [`FileView::expand`] / [`FileView::collapse`] |
//! | `workspace.CODE_FOLDER_CHANGED` | [`FileView::folder_changed`] |
//! | `developer.CREATE_ENTRY`, `RENAME_ENTRY`, `MOVE_ENTRY`, `DELETE_ENTRY` | [`FileView::after_change`], once the action ran |
//! | `developer.LEAVE_FILE_VIEW` | [`FileView::leave`] |
//! | `time.FILE_VIEW_RELEASE_DUE` | [`FileView::release_due`], from an injected [`Clock`] |
//! | `window.CLOSED` | [`FileView::close`] |
//!
//! The watches themselves are the app's: this module asks a [`Watches`] to watch and unwatch the
//! folders it lists, so the tests can say which folders are watched at each step.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

use super::{is_within, listing, FileEntry, FolderListing, Refusal};
use crate::file_tab::path;
use crate::project::clock::Clock;

/// How long the tree and the watches are kept after the view stops being shown.
pub const RELEASE_DELAY: Duration = Duration::from_secs(10);

/// Watches the folders the view lists, one non-recursive watch each.
pub trait Watches: Send {
    /// Starts watching the folder `rel` (relative to the root, empty for the root), which is
    /// `folder` on disk (canonical).
    fn watch(&mut self, rel: &str, folder: &Path) -> Result<(), String>;
    /// Stops watching the folder `rel`.
    fn unwatch(&mut self, rel: &str);
    /// Stops every watch.
    fn unwatch_all(&mut self);
}

/// The state of a window's File view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Closed,
    Open,
    Releasing,
}

/// The folders to show, with the generation they were read at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TreeSnapshot {
    #[ts(type = "number")]
    pub generation: u64,
    /// The root (empty path) and every expanded folder, parents before their children.
    pub folders: Vec<FolderListing>,
}

/// What changed in the listings: folders read again, and folders (with those inside them) that are
/// no longer listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TreeUpdate {
    #[ts(type = "number")]
    pub generation: u64,
    pub folders: Vec<FolderListing>,
    pub removed: Vec<String>,
}

impl TreeUpdate {
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty() && self.removed.is_empty()
    }
}

/// The File view of one project window.
pub struct FileView<W: Watches> {
    root: PathBuf,
    clock: Arc<dyn Clock>,
    watches: W,
    state: State,
    /// The listed folders by relative path; the root is `""`.
    folders: BTreeMap<String, Vec<FileEntry>>,
    /// When the view stopped being shown, while Releasing (milliseconds of the clock).
    released_at: Option<i64>,
    generation: u64,
}

fn not_open() -> Refusal {
    Refusal::new("The File view is not open.")
}

/// A relative folder path as its checked parts joined, `""` for the root.
fn normal(rel: &str) -> Result<String, Refusal> {
    Ok(path::parts(rel)?.join("/"))
}

impl<W: Watches> FileView<W> {
    /// A Closed view of the project at `root` (canonical): nothing is read or watched.
    pub fn new(root: PathBuf, clock: Arc<dyn Clock>, watches: W) -> Self {
        Self {
            root,
            clock,
            watches,
            state: State::Closed,
            folders: BTreeMap::new(),
            released_at: None,
            generation: 0,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The entries held for the folder `rel`, when it is listed.
    pub fn entries(&self, rel: &str) -> Option<&[FileEntry]> {
        self.folders.get(rel).map(Vec::as_slice)
    }

    /// The paths of the folders held, the root included.
    pub fn listed(&self) -> Vec<&str> {
        self.folders.keys().map(String::as_str).collect()
    }

    fn bump(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }

    fn snapshot(&self) -> TreeSnapshot {
        TreeSnapshot {
            generation: self.generation,
            folders: self
                .folders
                .iter()
                .map(|(path, entries)| FolderListing {
                    path: path.clone(),
                    entries: entries.clone(),
                })
                .collect(),
        }
    }

    /// Lists the folder `rel` and watches it; nothing is kept when either fails.
    fn read_and_watch(&mut self, rel: &str) -> Result<Vec<FileEntry>, Refusal> {
        let entries = listing::list(&self.root, rel)?;
        let folder = path::resolve(&self.root, rel)?;
        self.watches.watch(rel, &folder).map_err(|reason| {
            Refusal::new(format!(
                "The folder \"{rel}\" could not be watched: {reason}"
            ))
        })?;
        Ok(entries)
    }

    /// Drops the listing and the watch of every folder held that is `rel` or inside it; the paths
    /// dropped.
    fn forget(&mut self, rel: &str) -> Vec<String> {
        let gone: Vec<String> = self
            .folders
            .keys()
            .filter(|key| is_within(rel, key))
            .cloned()
            .collect();
        for key in &gone {
            self.folders.remove(key);
            self.watches.unwatch(key);
        }
        gone
    }

    // --- developer.OPEN_FILE_VIEW -----------------------------------------------------------

    /// `developer.OPEN_FILE_VIEW`. From Closed it reads only the names and kinds of the root's
    /// entries and watches the root alone; from Releasing the tree comes back as it was, with
    /// nothing read again. Open already: what is held.
    pub fn open(&mut self) -> Result<TreeSnapshot, Refusal> {
        match self.state {
            State::Closed => {
                let entries = self.read_and_watch("")?;
                self.folders.insert(String::new(), entries);
                self.state = State::Open;
                self.bump();
            }
            State::Releasing => {
                self.released_at = None;
                self.state = State::Open;
                self.bump();
            }
            State::Open => {}
        }
        Ok(self.snapshot())
    }

    // --- developer.EXPAND_FOLDER / COLLAPSE_FOLDER ------------------------------------------

    /// `developer.EXPAND_FOLDER`: reads only that folder's entries and watches it alone, without
    /// the folders inside it. A folder already expanded answers what is held.
    pub fn expand(&mut self, rel: &str) -> Result<FolderListing, Refusal> {
        if self.state != State::Open {
            return Err(not_open());
        }
        let rel = normal(rel)?;
        if let Some(entries) = self.folders.get(&rel) {
            return Ok(FolderListing {
                path: rel,
                entries: entries.clone(),
            });
        }
        let entries = self.read_and_watch(&rel)?;
        self.folders.insert(rel.clone(), entries.clone());
        self.bump();
        Ok(FolderListing { path: rel, entries })
    }

    /// `developer.COLLAPSE_FOLDER`: stops watching that folder and every folder inside it, and
    /// forgets their entries. The root never collapses.
    pub fn collapse(&mut self, rel: &str) -> Result<(), Refusal> {
        if self.state != State::Open {
            return Err(not_open());
        }
        let rel = normal(rel)?;
        if rel.is_empty() {
            return Err(Refusal::new("The project folder cannot be collapsed."));
        }
        self.forget(&rel);
        self.bump();
        Ok(())
    }

    // --- workspace.CODE_FOLDER_CHANGED ------------------------------------------------------

    /// `workspace.CODE_FOLDER_CHANGED`, in Open and in Releasing: reads the folder `rel` again.
    /// Empty when it is not listed or its entries are the same; a folder that cannot be read any
    /// more (gone, or no longer a folder) is dropped with the folders inside it.
    pub fn folder_changed(&mut self, rel: &str) -> TreeUpdate {
        let mut update = TreeUpdate {
            generation: self.generation,
            folders: Vec::new(),
            removed: Vec::new(),
        };
        if self.state == State::Closed || !self.folders.contains_key(rel) {
            return update;
        }
        match listing::list(&self.root, rel) {
            Ok(entries) => {
                if self.folders.get(rel) != Some(&entries) {
                    self.folders.insert(rel.to_owned(), entries.clone());
                    update.folders.push(FolderListing {
                        path: rel.to_owned(),
                        entries,
                    });
                }
            }
            Err(_) if !rel.is_empty() => update.removed = self.forget(rel),
            // The root cannot be read now: what is held stays until the next change.
            Err(_) => {}
        }
        if !update.is_empty() {
            update.generation = self.bump();
        }
        update
    }

    // --- create, rename, move, delete -------------------------------------------------------

    /// Brings the listings in line after an action of the tree ran. The expanded folders that
    /// were `old` or inside it are dropped and, when the entry moved to `new`, listed again under
    /// the new path (and watched); each folder of `touched` that is listed is read again.
    pub fn after_change(
        &mut self,
        old: Option<&str>,
        new: Option<&str>,
        touched: &[&str],
    ) -> TreeUpdate {
        let mut update = TreeUpdate {
            generation: self.generation,
            folders: Vec::new(),
            removed: Vec::new(),
        };
        if self.state == State::Closed {
            return update;
        }
        if let Some(old) = old {
            let dropped = self.forget(old);
            if let Some(new) = new {
                for key in &dropped {
                    let moved = format!("{new}{}", &key[old.len()..]);
                    if let Ok(entries) = self.read_and_watch(&moved) {
                        self.folders.insert(moved.clone(), entries.clone());
                        update.folders.push(FolderListing {
                            path: moved,
                            entries,
                        });
                    }
                }
            }
            update.removed = dropped;
        }
        for rel in touched {
            let changed = self.folder_changed(rel);
            for folder in changed.folders {
                // Listed already by the remap above: the newer reading wins.
                update.folders.retain(|f| f.path != folder.path);
                update.folders.push(folder);
            }
            update.removed.extend(changed.removed);
        }
        if !update.is_empty() {
            update.generation = self.bump();
        }
        update
    }

    // --- developer.LEAVE_FILE_VIEW / time.FILE_VIEW_RELEASE_DUE -----------------------------

    /// `developer.LEAVE_FILE_VIEW` (its tabs closed): Open → Releasing; the tree and the watches
    /// are kept for [`RELEASE_DELAY`].
    pub fn leave(&mut self) {
        if self.state == State::Open {
            self.state = State::Releasing;
            self.released_at = Some(self.clock.now_ms());
        }
    }

    /// How long until the release is due, while Releasing.
    pub fn release_in(&self) -> Option<Duration> {
        let since = self.released_at?;
        let elapsed = u64::try_from(self.clock.now_ms().saturating_sub(since)).unwrap_or(0);
        Some(RELEASE_DELAY.saturating_sub(Duration::from_millis(elapsed)))
    }

    /// `time.FILE_VIEW_RELEASE_DUE`: when the delay has passed, Releasing → Closed and every
    /// watch and listing is dropped. `true` when it released.
    pub fn release_due(&mut self) -> bool {
        if self.state == State::Releasing && self.release_in() == Some(Duration::ZERO) {
            self.close();
            return true;
        }
        false
    }

    /// `window.CLOSED`: every watch and every listing is dropped, at once, from any state.
    pub fn close(&mut self) {
        self.watches.unwatch_all();
        self.folders.clear();
        self.released_at = None;
        self.state = State::Closed;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::sync::Mutex;

    use super::*;
    use crate::paths::canonical_root;
    use crate::project::clock::ManualClock;

    /// Records which folders are watched.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<BTreeSet<String>>>);

    impl Recorder {
        fn watched(&self) -> Vec<String> {
            self.0.lock().unwrap().iter().cloned().collect()
        }
    }

    impl Watches for Recorder {
        fn watch(&mut self, rel: &str, folder: &Path) -> Result<(), String> {
            assert!(folder.is_dir(), "{folder:?} is watched");
            self.0.lock().unwrap().insert(rel.to_owned());
            Ok(())
        }

        fn unwatch(&mut self, rel: &str) {
            self.0.lock().unwrap().remove(rel);
        }

        fn unwatch_all(&mut self) {
            self.0.lock().unwrap().clear();
        }
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        clock: Arc<ManualClock>,
        recorder: Recorder,
        view: FileView<Recorder>,
    }

    fn put(root: &Path, rel: &str) {
        let target = root.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, "x").unwrap();
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        put(&root, "a.txt");
        put(&root, "src/main.rs");
        put(&root, "src/deep/inner.rs");
        put(&root, "src/deep/more/leaf.rs");
        let clock = Arc::new(ManualClock::new(1_000));
        let recorder = Recorder::default();
        let view = FileView::new(root.clone(), clock.clone(), recorder.clone());
        Fixture {
            _dir: dir,
            root,
            clock,
            recorder,
            view,
        }
    }

    fn names(entries: &[FileEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    #[test]
    fn a_closed_view_reads_nothing_and_watches_nothing() {
        let f = fixture();
        assert_eq!(f.view.state(), State::Closed);
        assert!(f.view.listed().is_empty());
        assert!(f.recorder.watched().is_empty());
    }

    #[test]
    fn opening_reads_only_the_root_and_watches_only_the_root() {
        let mut f = fixture();
        let snapshot = f.view.open().unwrap();
        assert_eq!(f.view.state(), State::Open);
        assert_eq!(snapshot.folders.len(), 1);
        assert_eq!(snapshot.folders[0].path, "");
        assert_eq!(names(&snapshot.folders[0].entries), ["src", "a.txt"]);
        assert_eq!(f.view.listed(), [""]);
        assert_eq!(f.recorder.watched(), [""]);
    }

    #[test]
    fn expanding_reads_and_watches_that_folder_alone() {
        let mut f = fixture();
        f.view.open().unwrap();
        let src = f.view.expand("src").unwrap();
        assert_eq!(src.path, "src");
        assert_eq!(names(&src.entries), ["deep", "main.rs"]);
        assert_eq!(f.view.listed(), ["", "src"]);
        assert_eq!(
            f.recorder.watched(),
            ["", "src"],
            "not the folders inside src"
        );
        // Expanded again: nothing read, same answer.
        fs::write(f.root.join("src/late.rs"), "x").unwrap();
        assert_eq!(f.view.expand("src").unwrap().entries, src.entries);
    }

    #[test]
    fn collapsing_drops_the_folder_and_every_expanded_folder_inside_it() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.expand("src/deep").unwrap();
        f.view.expand("src/deep/more").unwrap();
        assert_eq!(
            f.recorder.watched(),
            ["", "src", "src/deep", "src/deep/more"]
        );
        f.view.collapse("src/deep").unwrap();
        assert_eq!(f.recorder.watched(), ["", "src"]);
        assert_eq!(f.view.listed(), ["", "src"]);
        assert!(f.view.entries("src/deep/more").is_none());
        f.view.collapse("src").unwrap();
        assert_eq!(f.recorder.watched(), [""]);
        assert!(f.view.collapse("").is_err(), "the root never collapses");
        assert_eq!(f.recorder.watched(), [""]);
    }

    #[test]
    fn expanding_and_collapsing_need_the_view_open_and_a_folder_of_the_project() {
        let mut f = fixture();
        assert!(f.view.expand("src").is_err());
        assert!(f.view.collapse("src").is_err());
        f.view.open().unwrap();
        assert!(f.view.expand("..").is_err());
        assert!(f.view.expand("a.txt").is_err());
        assert!(f.view.expand("missing").is_err());
        assert!(f.view.expand(".git").is_err());
        assert_eq!(f.recorder.watched(), [""]);
    }

    #[test]
    fn a_folder_that_changes_is_read_again_in_open_and_in_releasing() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        assert!(f.view.folder_changed("src").is_empty(), "nothing differs");
        put(&f.root, "src/new.rs");
        let update = f.view.folder_changed("src");
        assert_eq!(update.folders.len(), 1);
        assert_eq!(
            names(&update.folders[0].entries),
            ["deep", "main.rs", "new.rs"]
        );
        assert!(update.generation > 0);
        // Not listed: nothing to read.
        assert!(f.view.folder_changed("src/deep").is_empty());

        f.view.leave();
        put(&f.root, "src/another.rs");
        let update = f.view.folder_changed("src");
        assert_eq!(
            names(&update.folders[0].entries),
            ["deep", "another.rs", "main.rs", "new.rs"]
        );
    }

    #[test]
    fn a_folder_that_vanished_is_dropped_with_the_folders_inside_it() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.expand("src/deep").unwrap();
        fs::remove_dir_all(f.root.join("src")).unwrap();
        let update = f.view.folder_changed("src");
        assert_eq!(update.removed, ["src", "src/deep"]);
        assert_eq!(f.recorder.watched(), [""]);
        assert_eq!(f.view.listed(), [""]);
    }

    #[test]
    fn leaving_keeps_the_tree_and_the_watches_and_opening_again_reads_nothing() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.leave();
        assert_eq!(f.view.state(), State::Releasing);
        assert_eq!(f.recorder.watched(), ["", "src"]);
        // Something appears while the view is away and nobody told the view: not read again.
        fs::write(f.root.join("src/unseen.rs"), "x").unwrap();
        f.clock.advance(9_999);
        assert!(!f.view.release_due());
        let snapshot = f.view.open().unwrap();
        assert_eq!(f.view.state(), State::Open);
        let paths: Vec<&str> = snapshot.folders.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["", "src"]);
        assert!(!names(&snapshot.folders[1].entries).contains(&"unseen.rs"));
        // The release that was due is not: the view is shown again.
        f.clock.advance(5_000);
        assert!(!f.view.release_due());
        assert_eq!(f.view.state(), State::Open);
    }

    #[test]
    fn ten_seconds_after_leaving_no_watch_and_no_listing_is_left() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.leave();
        assert_eq!(f.view.release_in(), Some(Duration::from_secs(10)));
        f.clock.advance(9_999);
        assert_eq!(f.view.release_in(), Some(Duration::from_millis(1)));
        assert!(!f.view.release_due());
        assert_eq!(f.recorder.watched(), ["", "src"]);
        f.clock.advance(1);
        assert!(f.view.release_due());
        assert_eq!(f.view.state(), State::Closed);
        assert!(f.view.listed().is_empty());
        assert!(f.recorder.watched().is_empty());
        assert_eq!(f.view.release_in(), None);
        // Shown again later: the root is read again.
        fs::write(f.root.join("fresh.txt"), "x").unwrap();
        let snapshot = f.view.open().unwrap();
        assert!(names(&snapshot.folders[0].entries).contains(&"fresh.txt"));
        assert_eq!(snapshot.folders.len(), 1);
    }

    #[test]
    fn closing_the_window_drops_everything_at_once_from_open_or_releasing() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.close();
        assert_eq!(f.view.state(), State::Closed);
        assert!(f.view.listed().is_empty() && f.recorder.watched().is_empty());

        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.leave();
        f.view.close();
        assert_eq!(f.view.state(), State::Closed);
        assert!(f.view.listed().is_empty() && f.recorder.watched().is_empty());
        assert!(!f.view.release_due());
    }

    #[test]
    fn a_closed_view_ignores_what_changes() {
        let mut f = fixture();
        assert!(f.view.folder_changed("").is_empty());
        assert!(f.view.after_change(Some("a.txt"), None, &[""]).is_empty());
        f.view.leave();
        assert_eq!(f.view.state(), State::Closed);
    }

    #[test]
    fn a_renamed_folder_is_listed_and_watched_again_under_its_new_path() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        f.view.expand("src/deep").unwrap();
        fs::rename(f.root.join("src"), f.root.join("lib")).unwrap();
        let update = f.view.after_change(Some("src"), Some("lib"), &[""]);
        assert_eq!(update.removed, ["src", "src/deep"]);
        let paths: Vec<&str> = update.folders.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["lib", "lib/deep", ""]);
        assert_eq!(f.recorder.watched(), ["", "lib", "lib/deep"]);
        assert_eq!(f.view.entries("lib/deep").unwrap()[0].path, "lib/deep/more");
        assert_eq!(names(&update.folders[2].entries), ["lib", "a.txt"]);
    }

    #[test]
    fn a_deleted_folder_and_a_created_file_update_what_the_tree_shows() {
        let mut f = fixture();
        f.view.open().unwrap();
        f.view.expand("src").unwrap();
        fs::remove_dir_all(f.root.join("src")).unwrap();
        let update = f.view.after_change(Some("src"), None, &[""]);
        assert_eq!(update.removed, ["src"]);
        assert_eq!(names(&update.folders[0].entries), ["a.txt"]);
        assert_eq!(f.recorder.watched(), [""]);

        put(&f.root, "b.txt");
        let update = f.view.after_change(None, None, &[""]);
        assert_eq!(names(&update.folders[0].entries), ["a.txt", "b.txt"]);
        assert!(update.removed.is_empty());
    }

    #[test]
    fn a_watch_that_fails_leaves_nothing_behind() {
        struct Failing;
        impl Watches for Failing {
            fn watch(&mut self, _: &str, _: &Path) -> Result<(), String> {
                Err("no watches left".to_owned())
            }
            fn unwatch(&mut self, _: &str) {}
            fn unwatch_all(&mut self) {}
        }
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        let mut view = FileView::new(root, Arc::new(ManualClock::new(1)), Failing);
        assert!(view
            .open()
            .unwrap_err()
            .to_string()
            .contains("no watches left"));
        assert_eq!(view.state(), State::Closed);
        assert!(view.listed().is_empty());
    }
}
