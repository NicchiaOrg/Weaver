use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use weaver_core::assets::{assets_dir, Asset, SIZE_LIMIT};
use weaver_core::paths::canonical_root;

use super::*;

const PROJECT: &str = "0123456789abcdef";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    service: Arc<AssetsService>,
    events: Arc<Mutex<Vec<AssetsChanged>>>,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_root(dir.path()).unwrap();
    fs::create_dir(root.join(".weaver")).unwrap();
    let service = Arc::new(AssetsService::new());
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink_events = events.clone();
    service.attach(Arc::new(move |changed| {
        sink_events.lock().unwrap().push(changed)
    }));
    Fixture {
        _dir: dir,
        root,
        service,
        events,
    }
}

impl Fixture {
    fn put(&self, name: &str, text: &str) {
        let dir = assets_dir(&self.root);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(name), text).unwrap();
    }

    fn names(&self) -> Vec<String> {
        self.service
            .assets(PROJECT)
            .iter()
            .map(|asset| asset.name.clone())
            .collect()
    }

    fn events(&self) -> Vec<AssetsChanged> {
        self.events.lock().unwrap().clone()
    }

    /// Files outside the project to drop.
    fn outside(&self, names: &[&str]) -> (tempfile::TempDir, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        let base = canonical_root(dir.path()).unwrap();
        let paths = names
            .iter()
            .map(|name| {
                let path = base.join(name);
                fs::write(&path, name).unwrap();
                path
            })
            .collect();
        (dir, paths)
    }
}

fn asset(name: &str, size: u64) -> Asset {
    Asset {
        name: name.to_owned(),
        size,
    }
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

// --- the list -----------------------------------------------------------------------------------

#[test]
fn the_list_is_read_when_the_project_opens_and_a_missing_folder_is_an_empty_list() {
    let f = fixture();
    assert!(
        f.service.assets(PROJECT).is_empty(),
        "nothing is known before the first read"
    );
    let snapshot = f.service.refresh(PROJECT, &f.root);
    assert_eq!(
        snapshot,
        AssetsSnapshot {
            generation: 1,
            assets: vec![]
        }
    );
    assert!(
        !assets_dir(&f.root).exists(),
        "reading never makes the folder"
    );
    assert_eq!(f.events().len(), 1, "the window hears of the first list");
}

#[test]
fn a_change_of_the_folder_is_sent_with_the_next_generation_and_the_whole_list() {
    let f = fixture();
    f.service.refresh(PROJECT, &f.root);
    f.put("b.txt", "hello");
    f.put("A.png", "");
    f.put(".hidden", "x");
    let snapshot = f.service.refresh(PROJECT, &f.root);
    assert_eq!(snapshot.generation, 2);
    let events = f.events();
    assert_eq!(
        events.last().unwrap(),
        &AssetsChanged {
            project_id: PROJECT.to_owned(),
            generation: 2,
            assets: vec![asset("A.png", 0), asset("b.txt", 5)],
        }
    );
    // A size change is a change; an unchanged folder is not.
    f.put("b.txt", "hello!");
    assert_eq!(f.service.refresh(PROJECT, &f.root).generation, 3);
    assert_eq!(f.service.refresh(PROJECT, &f.root).generation, 3);
    assert_eq!(f.events().len(), 3);
    assert_eq!(
        f.service.assets(PROJECT).as_ref(),
        &vec![asset("A.png", 0), asset("b.txt", 6)]
    );
}

#[test]
fn the_list_answers_without_the_disk_and_a_window_that_starts_gets_it() {
    let f = fixture();
    f.put("a.txt", "x");
    let first = f.service.snapshot(PROJECT, &f.root);
    assert_eq!(first.assets, [asset("a.txt", 1)]);
    // Taken out of the folder behind the service's back, the held list is what a window gets.
    fs::remove_file(assets_dir(&f.root).join("a.txt")).unwrap();
    assert_eq!(f.service.snapshot(PROJECT, &f.root), first);
    assert_eq!(f.names(), ["a.txt"]);
    assert_eq!(f.service.refresh(PROJECT, &f.root).assets, []);
    assert_eq!(f.names(), Vec::<String>::new());
}

#[test]
fn a_folder_that_cannot_be_read_leaves_the_list_as_it_was() {
    let f = fixture();
    f.put("a.txt", "x");
    f.service.refresh(PROJECT, &f.root);
    fs::remove_dir_all(assets_dir(&f.root)).unwrap();
    fs::write(assets_dir(&f.root), "a file, not a folder").unwrap();
    let snapshot = f.service.refresh(PROJECT, &f.root);
    assert_eq!(snapshot.assets, [asset("a.txt", 1)]);
    assert_eq!(snapshot.generation, 1);
    assert_eq!(f.events().len(), 1);
}

#[test]
fn closing_the_window_forgets_the_list() {
    let f = fixture();
    f.put("a.txt", "x");
    f.service.refresh(PROJECT, &f.root);
    f.service.close_project(PROJECT);
    assert!(f.service.assets(PROJECT).is_empty());
}

// --- a batch of watcher events --------------------------------------------------------------------

#[test]
fn the_events_under_the_assets_folder_never_reach_the_rest_of_a_batch() {
    let root = PathBuf::from("/project");
    let weaver = root.join(".weaver");
    let split = split_batch(
        &root,
        Rebuild::Paths(vec![
            weaver.join("assets"),
            weaver.join("assets/logo.png"),
            weaver.join("tasks/a.md"),
            weaver.join("assets-old/b.md"),
        ]),
    );
    assert!(split.assets);
    assert_eq!(
        split.rest,
        Some(Rebuild::Paths(vec![
            weaver.join("tasks/a.md"),
            weaver.join("assets-old/b.md")
        ]))
    );

    let only = split_batch(&root, Rebuild::Paths(vec![weaver.join("assets/a.png")]));
    assert_eq!(
        only,
        Split {
            assets: true,
            rest: None
        }
    );

    let none = split_batch(&root, Rebuild::Paths(vec![weaver.join("model/system.md")]));
    assert!(!none.assets);
    assert_eq!(
        none.rest,
        Some(Rebuild::Paths(vec![weaver.join("model/system.md")]))
    );

    // Events that name no path concern everything.
    let all = split_batch(&root, Rebuild::All);
    assert!(all.assets);
    assert_eq!(all.rest, Some(Rebuild::All));
}

// --- copy in ------------------------------------------------------------------------------------

#[test]
fn a_drop_is_copied_selected_by_name_and_the_list_follows() {
    let f = fixture();
    let (_out, sources) = f.outside(&["b.txt", "a.png"]);
    let done = f.service.copy_in(PROJECT, &f.root, &sources).unwrap();
    assert_eq!(done.copied, ["a.png", "b.txt"]);
    assert!(done.large.is_empty());
    assert_eq!(done.snapshot.assets, [asset("a.png", 5), asset("b.txt", 5)]);
    assert_eq!(f.events().last().unwrap().assets, done.snapshot.assets);
    assert!(
        sources.iter().all(|path| path.exists()),
        "the originals stay"
    );
}

#[test]
fn a_copied_file_over_the_limit_is_named() {
    let f = fixture();
    let (_out, sources) = f.outside(&["big.bin", "small.txt"]);
    fs::File::options()
        .write(true)
        .open(&sources[0])
        .unwrap()
        .set_len(SIZE_LIMIT + 1)
        .unwrap();
    let done = f.service.copy_in(PROJECT, &f.root, &sources).unwrap();
    assert_eq!(done.large, ["big.bin"]);
    assert!(done.snapshot.assets[0].is_large());
}

#[test]
fn a_refused_drop_copies_nothing_and_says_why() {
    let f = fixture();
    f.put("taken.txt", "mine");
    f.service.refresh(PROJECT, &f.root);
    let (out, mut sources) = f.outside(&["new.txt", "taken.txt"]);
    let reason = f.service.copy_in(PROJECT, &f.root, &sources).unwrap_err();
    assert_eq!(
        reason.to_string(),
        "An asset named \"taken.txt\" already exists."
    );
    sources.truncate(1);
    fs::create_dir(out.path().join("folder")).unwrap();
    sources.push(canonical_root(out.path()).unwrap().join("folder"));
    let reason = f.service.copy_in(PROJECT, &f.root, &sources).unwrap_err();
    assert_eq!(
        reason.to_string(),
        "\"folder\" is a folder: only files can be assets."
    );
    assert_eq!(f.names(), ["taken.txt"]);
    assert_eq!(
        fs::read_to_string(assets_dir(&f.root).join("taken.txt")).unwrap(),
        "mine"
    );
}

#[test]
fn a_second_copy_while_one_runs_is_refused_and_the_first_goes_on() {
    let f = fixture();
    let (_out, first) = f.outside(&["a.txt"]);
    let (_more, second) = f.outside(&["b.txt"]);
    let (started_tx, started_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);

    std::thread::scope(|scope| {
        let running = scope.spawn(|| {
            f.service
                .copy_in_with(PROJECT, &f.root, &first, &|_| {
                    started_tx.send(()).unwrap();
                    release_rx.lock().unwrap().recv().unwrap();
                    Ok(())
                })
                .unwrap()
        });
        started_rx.recv().unwrap();
        let refused = f.service.copy_in(PROJECT, &f.root, &second).unwrap_err();
        assert_eq!(
            refused.to_string(),
            "Another copy is still running in this window."
        );
        release_tx.send(()).unwrap();
        let done = running.join().unwrap();
        assert_eq!(done.copied, ["a.txt"]);
    });
    assert_eq!(
        f.names(),
        ["a.txt"],
        "nothing of the refused drop was copied"
    );
    // The window may copy again once the first ended.
    f.service.copy_in(PROJECT, &f.root, &second).unwrap();
    assert_eq!(f.names(), ["a.txt", "b.txt"]);
}

#[test]
fn a_copy_that_fails_part_way_leaves_nothing_and_a_new_copy_may_start() {
    let f = fixture();
    let (_out, sources) = f.outside(&["a.txt", "b.txt"]);
    let fail = |dest: &Path| {
        if dest.ends_with("b.txt") {
            Err(std::io::Error::other("the disk is full"))
        } else {
            Ok(())
        }
    };
    let error = f
        .service
        .copy_in_with(PROJECT, &f.root, &sources, &fail)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Could not copy \"b.txt\": the disk is full. Nothing was copied."
    );
    assert!(f.names().is_empty());
    assert!(!assets_dir(&f.root).exists());
    f.service.copy_in(PROJECT, &f.root, &sources).unwrap();
}

#[test]
fn dropping_the_assets_back_onto_their_own_view_does_nothing() {
    let f = fixture();
    f.put("a.txt", "x");
    f.service.refresh(PROJECT, &f.root);
    let before = f.events().len();
    let own = vec![assets_dir(&f.root).join("a.txt")];
    let done = f.service.copy_in(PROJECT, &f.root, &own).unwrap();
    assert!(done.copied.is_empty());
    assert!(done.large.is_empty());
    assert_eq!(f.names(), ["a.txt"]);
    assert_eq!(f.events().len(), before, "nothing changed, nothing is sent");
}

// --- rename and delete --------------------------------------------------------------------------

#[test]
fn a_rename_answers_the_new_name_and_the_list_and_never_replaces() {
    let f = fixture();
    f.put("a.txt", "first");
    f.put("b.txt", "second");
    f.service.refresh(PROJECT, &f.root);
    let done = f
        .service
        .rename(PROJECT, &f.root, "a.txt", "c.txt")
        .unwrap();
    assert_eq!(done.name, "c.txt");
    assert_eq!(done.snapshot.assets, [asset("b.txt", 6), asset("c.txt", 5)]);
    let refused = f
        .service
        .rename(PROJECT, &f.root, "b.txt", "c.txt")
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "An asset named \"c.txt\" already exists."
    );
    assert_eq!(
        fs::read_to_string(assets_dir(&f.root).join("c.txt")).unwrap(),
        "first"
    );
}

#[test]
fn a_deletion_reports_the_assets_the_trash_refused_and_the_list_without_the_others() {
    let f = fixture();
    for name in ["a.txt", "b.txt", "c.txt"] {
        f.put(name, name);
    }
    f.service.refresh(PROJECT, &f.root);
    let bin = f.root.join("bin");
    let trash = |path: &Path| {
        if path.ends_with("b.txt") {
            return Err("the trash is full".to_owned());
        }
        fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
        fs::rename(path, bin.join(path.file_name().unwrap())).map_err(|e| e.to_string())
    };
    let done = f.service.delete_with(
        PROJECT,
        &f.root,
        &strings(&["a.txt", "b.txt", "c.txt"]),
        &trash,
    );
    assert_eq!(done.trashed, ["a.txt", "c.txt"]);
    assert_eq!(
        done.refused,
        [AssetRefusal {
            name: "b.txt".into(),
            reason: "the trash is full".into()
        }]
    );
    assert_eq!(done.snapshot.assets, [asset("b.txt", 5)]);
    assert!(bin.join("a.txt").exists() && bin.join("c.txt").exists());
}

#[test]
fn the_payloads_serialise_in_camel_case() {
    let changed = AssetsChanged {
        project_id: PROJECT.to_owned(),
        generation: 4,
        assets: vec![asset("a.txt", 3)],
    };
    assert_eq!(
        serde_json::to_value(&changed).unwrap(),
        serde_json::json!({
            "projectId": PROJECT,
            "generation": 4,
            "assets": [{ "name": "a.txt", "size": 3 }],
        })
    );
    assert_eq!(
        crate::events::EventName::AssetsChanged.as_str(),
        "assets-changed"
    );
}
