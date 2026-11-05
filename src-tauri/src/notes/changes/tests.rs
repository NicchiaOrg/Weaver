use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use weaver_core::file_tab::read::sha256_bytes;
use weaver_core::paths::canonical_root;
use weaver_core::project::image::ProjectImage;
use weaver_core::project::layout::{self, NOTES_DIR};

use super::*;

const PROJECT: &str = "0123456789abcdef";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    changes: NotesChanges,
    events: Arc<Mutex<Vec<NotesChanged>>>,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_root(dir.path()).unwrap();
    fs::create_dir(root.join(".weaver")).unwrap();
    let changes = NotesChanges::new();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink_events = events.clone();
    changes.attach(Arc::new(move |event| {
        sink_events.lock().unwrap().push(event)
    }));
    Fixture {
        _dir: dir,
        root,
        changes,
        events,
    }
}

impl Fixture {
    fn notes(&self) -> PathBuf {
        notes::notes_dir(&self.root)
    }

    fn put(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.notes().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    fn batch(&self, paths: &[&Path]) -> NotesBatch {
        split_batch(
            &self.root,
            Rebuild::Paths(paths.iter().map(|p| p.to_path_buf()).collect()),
        )
        .notes
        .expect("the batch concerns the notes")
    }

    fn announce(&self, batch: &NotesBatch, known: Option<&String>) -> Option<NotesChanged> {
        let known = known.cloned();
        self.changes
            .announce(PROJECT, &self.root, batch, &move || known.clone())
    }

    fn events(&self) -> Vec<NotesChanged> {
        self.events.lock().unwrap().clone()
    }
}

fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

// --- a batch of watcher events ---------------------------------------------------------------

#[test]
fn the_events_under_the_notes_folder_never_reach_the_rest_of_a_batch() {
    let root = PathBuf::from("/project");
    let weaver = root.join(".weaver");
    let split = split_batch(
        &root,
        Rebuild::Paths(vec![
            weaver.join("notes"),
            weaver.join("notes/notes.md"),
            weaver.join("notes/shot.png"),
            weaver.join("notes/tasks/vxdhoylk-a.md"),
            weaver.join("tasks/a.md"),
            weaver.join("notes-old/b.md"),
        ]),
    );
    assert_eq!(
        split.rest,
        Some(Rebuild::Paths(vec![
            weaver.join("tasks/a.md"),
            weaver.join("notes-old/b.md")
        ]))
    );
    let notes = split.notes.unwrap();
    assert!(notes.sheet);
    assert!(notes.all_images, "the folder itself changed");
    assert_eq!(
        notes.candidates.into_iter().collect::<Vec<_>>(),
        ["shot.png", "tasks/vxdhoylk-a.md"]
    );

    let only = split_batch(&root, Rebuild::Paths(vec![weaver.join("notes/notes.md")]));
    assert_eq!(only.rest, None);
    assert_eq!(
        only.notes,
        Some(NotesBatch {
            sheet: true,
            ..NotesBatch::default()
        })
    );

    let none = split_batch(&root, Rebuild::Paths(vec![weaver.join("model/system.md")]));
    assert_eq!(none.notes, None);
    assert_eq!(
        none.rest,
        Some(Rebuild::Paths(vec![weaver.join("model/system.md")]))
    );
}

#[test]
fn the_hidden_files_of_the_folder_are_taken_out_and_tell_the_view_nothing() {
    let root = PathBuf::from("/project");
    let weaver = root.join(".weaver");
    let split = split_batch(
        &root,
        Rebuild::Paths(vec![
            weaver.join("notes/.gitignore"),
            weaver.join("notes/.notes.md.0123.tmp"),
        ]),
    );
    assert_eq!(split.notes, None);
    assert_eq!(split.rest, None);
}

#[test]
fn events_that_name_no_path_concern_the_notes_and_the_rest() {
    let split = split_batch(Path::new("/project"), Rebuild::All);
    assert_eq!(split.rest, Some(Rebuild::All));
    let notes = split.notes.unwrap();
    assert!(notes.sheet && notes.all_images);
}

#[test]
fn the_events_under_the_notes_folder_never_touch_the_picture() {
    let f = fixture();
    // Files that would be cards, Model files or Specs anywhere else.
    f.put("notes.md", b"garbage");
    f.put("tasks/vxdhoylk-a-task.md", b"garbage");
    f.put("model/system.md", b"garbage");
    f.put("specs.yaml", b"garbage");
    let mut image = ProjectImage::load(&f.root);
    let diagnostics = image.diagnostics().len();

    let dir = f.notes();
    let split = split_batch(
        &f.root,
        Rebuild::Paths(vec![
            dir.clone(),
            dir.join("notes.md"),
            dir.join("tasks/vxdhoylk-a-task.md"),
            dir.join("model/system.md"),
            dir.join("specs.yaml"),
        ]),
    );
    // Nothing is left for the rebuild, the image and `workspace.FILES_CHANGED`.
    assert_eq!(split.rest, None);
    // Even a path that slipped through would not make an entry.
    image.refresh(&[dir.join("notes.md"), dir.join("tasks/vxdhoylk-a-task.md")]);
    assert!(image.paths().all(|path| !path.starts_with(NOTES_DIR)));
    assert_eq!(image.diagnostics().len(), diagnostics);
    assert!(layout::is_notes_rel("notes/tasks/vxdhoylk-a-task.md"));
}

// --- the event --------------------------------------------------------------------------------

#[test]
fn a_write_of_weaver_itself_is_not_sent() {
    let f = fixture();
    let path = f.put("notes.md", b"my notes\n");
    let own = sha256_bytes(b"my notes\n");
    let batch = f.batch(&[&path]);

    assert_eq!(f.announce(&batch, Some(&own)), None);
    assert!(f.events().is_empty(), "nothing reached the window");

    // Written again by another program: the hash differs and the window hears of it.
    fs::write(&path, "their notes\n").unwrap();
    let sent = f.announce(&batch, Some(&own)).unwrap();
    assert_eq!(
        sent,
        NotesChanged {
            project_id: PROJECT.to_owned(),
            generation: 1,
            sheet_changed: true,
            hash: Some(sha256_bytes(b"their notes\n")),
            images: vec![],
        }
    );
    assert_eq!(f.events(), [sent]);
}

#[test]
fn a_sheet_that_weaver_never_wrote_or_read_is_sent() {
    let f = fixture();
    let path = f.put("notes.md", b"x\n");
    let sent = f.announce(&f.batch(&[&path]), None).unwrap();
    assert!(sent.sheet_changed);
    assert_eq!(sent.hash, Some(sha256_bytes(b"x\n")));
}

#[test]
fn a_deleted_sheet_is_sent_without_a_hash_and_a_missing_one_weaver_knew_is_not() {
    let f = fixture();
    let path = f.put("notes.md", b"x\n");
    let batch = f.batch(&[&path]);
    fs::remove_file(&path).unwrap();

    let sent = f
        .announce(&batch, Some(&sha256_bytes(b"x\n")))
        .expect("the sheet was deleted");
    assert!(sent.sheet_changed);
    assert_eq!(sent.hash, None);

    // Weaver read it as missing: the event of another deletion is nothing new.
    assert_eq!(f.announce(&batch, None), None);
}

#[test]
fn the_names_of_the_images_that_changed_go_with_the_event_and_the_generation_counts_up() {
    let f = fixture();
    let sheet = f.put("notes.md", b"x\n");
    let known = sha256_bytes(b"x\n");
    let a = f.put("a.png", b"png");
    let gone = f.notes().join("gone.png");
    f.put(".gitignore", b"*\n");

    let batch = f.batch(&[&sheet, &a, &gone, &f.notes().join(".gitignore")]);
    let first = f.announce(&batch, Some(&known)).unwrap();
    assert!(!first.sheet_changed);
    assert_eq!(first.hash, None);
    assert_eq!(first.images, strings(&["a.png", "gone.png"]));
    assert_eq!(first.generation, 1);

    let second = f.announce(&batch, Some(&known)).unwrap();
    assert_eq!(second.generation, 2);
    assert_eq!(f.events().len(), 2);

    // A batch of the sheet alone, unchanged, sends nothing and does not count.
    assert_eq!(f.announce(&f.batch(&[&sheet]), Some(&known)), None);
    let third = f.announce(&batch, Some(&known)).unwrap();
    assert_eq!(third.generation, 3);
}

#[test]
fn a_folder_of_images_is_named_by_its_files_and_lost_events_name_them_all() {
    let f = fixture();
    f.put("notes.md", b"x\n");
    f.put("pasted/one.png", b"1");
    f.put("pasted/two.png", b"2");
    f.put("top.png", b"3");
    let known = sha256_bytes(b"x\n");

    let folder = f.batch(&[&f.notes().join("pasted")]);
    let sent = f.announce(&folder, Some(&known)).unwrap();
    assert_eq!(sent.images, strings(&["pasted/one.png", "pasted/two.png"]));

    let lost = split_batch(&f.root, Rebuild::All).notes.unwrap();
    let sent = f.announce(&lost, Some(&known)).unwrap();
    assert!(!sent.sheet_changed);
    assert_eq!(
        sent.images,
        strings(&["pasted/one.png", "pasted/two.png", "top.png"])
    );
}

#[test]
fn nothing_is_made_by_reading_and_each_project_counts_its_own_generation() {
    let f = fixture();
    let path = f.notes().join("notes.md");
    let batch = NotesBatch {
        sheet: true,
        ..NotesBatch::default()
    };
    assert_eq!(f.announce(&batch, None), None);
    assert!(
        !f.notes().exists(),
        "the folder is made by a write, never by a read"
    );

    f.put("notes.md", b"x\n");
    assert_eq!(f.announce(&batch, None).unwrap().generation, 1);
    let other = f
        .changes
        .announce("fedcba9876543210", &f.root, &batch, &|| None)
        .unwrap();
    assert_eq!(other.generation, 1);
    f.changes.close_project(PROJECT);
    assert_eq!(f.announce(&batch, None).unwrap().generation, 1);
    assert!(path.is_file());
}

#[test]
fn the_payload_serialises_in_camel_case_under_its_event_name() {
    let changed = NotesChanged {
        project_id: PROJECT.to_owned(),
        generation: 3,
        sheet_changed: true,
        hash: None,
        images: strings(&["a.png"]),
    };
    assert_eq!(
        serde_json::to_value(&changed).unwrap(),
        serde_json::json!({
            "projectId": PROJECT,
            "generation": 3,
            "sheetChanged": true,
            "hash": null,
            "images": ["a.png"],
        })
    );
    assert_eq!(
        crate::events::EventName::NotesChanged.as_str(),
        "notes-changed"
    );
}
