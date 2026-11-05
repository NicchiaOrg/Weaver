//! The notes service: remembered hash, writes that never overlap and coalesce, the trash, and the
//! shapes the UI reads.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use tokio::sync::oneshot::error::TryRecvError;
use weaver_core::notes::images::{Outcome, Why};
use weaver_core::notes::{notes_dir, sheet};
use weaver_core::paths::canonical_root;

use super::*;

const PROJECT: &str = "0123456789abcdef";

fn project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_root(dir.path()).unwrap();
    (dir, root)
}

fn wait(receiver: oneshot::Receiver<WriteResult>) -> WriteResult {
    receiver.blocking_recv().expect("the write was answered")
}

/// Waits until the write thread of the project has left its loop.
fn settled(service: &NotesService) {
    for _ in 0..400 {
        if !service.project(PROJECT).writer.lock().unwrap().running {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the write thread never ended");
}

// --- the remembered hash ------------------------------------------------------------------------

#[test]
fn nothing_is_remembered_for_a_project_that_was_never_read_or_written() {
    let service = NotesService::new();
    assert_eq!(service.last_hash(PROJECT), None);
}

#[test]
fn a_hash_is_remembered_until_the_window_closes() {
    let service = NotesService::new();
    service.remember_hash(PROJECT, Some("abc".into()));
    assert_eq!(service.last_hash(PROJECT).as_deref(), Some("abc"));
    service.remember_hash(PROJECT, None);
    assert_eq!(service.last_hash(PROJECT), None);
    service.remember_hash(PROJECT, Some("def".into()));
    service.close_project(PROJECT);
    assert_eq!(service.last_hash(PROJECT), None);
}

#[test]
fn a_write_creates_the_folder_and_remembers_the_hash_of_what_it_wrote() {
    let (_dir, root) = project();
    let service = NotesService::new();
    assert!(!notes_dir(&root).exists());
    let hash = wait(service.write(PROJECT, &root, "# Notes\n".into())).unwrap();
    assert_eq!(hash, sheet::hash_of("# Notes\n"));
    assert_eq!(service.last_hash(PROJECT), Some(hash));
    assert_eq!(
        fs::read_to_string(notes_dir(&root).join("notes.md")).unwrap(),
        "# Notes\n"
    );
    assert_eq!(
        fs::read(notes_dir(&root).join(".gitignore")).unwrap(),
        b"*\n"
    );
}

#[test]
fn the_hash_is_remembered_before_the_file_is_written() {
    let service: Arc<OnceLock<Arc<NotesService>>> = Arc::new(OnceLock::new());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let store = {
        let service = service.clone();
        let seen = seen.clone();
        Arc::new(move |_root: &Path, text: &str| {
            let remembered = service.get().unwrap().last_hash(PROJECT);
            seen.lock().unwrap().push((remembered, text.to_owned()));
            Ok(sheet::hash_of(text))
        })
    };
    let notes = Arc::new(NotesService::with_store(store));
    assert!(service.set(notes.clone()).is_ok());
    let (_dir, root) = project();
    wait(notes.write(PROJECT, &root, "text".into())).unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        [(Some(sheet::hash_of("text")), "text".to_owned())]
    );
}

#[test]
fn a_failed_write_gives_the_reason_and_puts_the_previous_hash_back() {
    let store = Arc::new(|_root: &Path, text: &str| {
        if text == "bad" {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "read only"))
        } else {
            Ok(sheet::hash_of(text))
        }
    });
    let service = NotesService::with_store(store);
    let (_dir, root) = project();
    let first = wait(service.write(PROJECT, &root, "good".into())).unwrap();
    let error = wait(service.write(PROJECT, &root, "bad".into())).unwrap_err();
    assert!(error.contains("read only"), "{error}");
    assert!(error.starts_with("The notes could not be saved"));
    assert!(
        !error.contains("bad"),
        "the text never travels in a message"
    );
    assert_eq!(service.last_hash(PROJECT), Some(first));
    // The window can write again afterwards.
    settled(&service);
    wait(service.write(PROJECT, &root, "again".into())).unwrap();
}

// --- one write at a time, coalesced -------------------------------------------------------------

/// A service whose store records each text, tells when a write starts, and holds it until the
/// test sends a token.
struct Gated {
    service: NotesService,
    calls: Arc<Mutex<Vec<String>>>,
    started: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    /// How many writes run now, and the most that ran at once.
    overlap: Arc<Mutex<(u32, u32)>>,
}

fn gated() -> Gated {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let overlap = Arc::new(Mutex::new((0u32, 0u32)));
    let (started_tx, started) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);
    let started_tx = Mutex::new(started_tx);
    let store = {
        let calls = calls.clone();
        let overlap = overlap.clone();
        Arc::new(move |_root: &Path, text: &str| {
            {
                let mut running = overlap.lock().unwrap();
                running.0 += 1;
                running.1 = running.1.max(running.0);
            }
            calls.lock().unwrap().push(text.to_owned());
            started_tx.lock().unwrap().send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
            overlap.lock().unwrap().0 -= 1;
            Ok(format!("hash-of-{text}"))
        })
    };
    Gated {
        service: NotesService::with_store(store),
        calls,
        started,
        release,
        overlap,
    }
}

#[test]
fn texts_that_arrive_during_a_write_coalesce_into_one_more_write_of_the_latest() {
    let g = gated();
    let root = PathBuf::from("/unused");
    let first = g.service.write(PROJECT, &root, "one".into());
    g.started.recv().unwrap();
    let second = g.service.write(PROJECT, &root, "two".into());
    let third = g.service.write(PROJECT, &root, "three".into());
    let fourth = g.service.write(PROJECT, &root, "four".into());
    // Nothing else started while the first is held.
    assert!(g.started.try_recv().is_err());
    assert_eq!(*g.calls.lock().unwrap(), ["one"]);

    g.release.send(()).unwrap();
    assert_eq!(wait(first).unwrap(), "hash-of-one");
    g.started.recv().unwrap();
    g.release.send(()).unwrap();
    // Every text that waited gets the result of the one write that carried the latest.
    for receiver in [second, third, fourth] {
        assert_eq!(wait(receiver).unwrap(), "hash-of-four");
    }
    assert_eq!(*g.calls.lock().unwrap(), ["one", "four"]);
    assert_eq!(g.overlap.lock().unwrap().1, 1, "never two writes at once");
}

#[test]
fn the_first_write_is_answered_before_the_coalesced_one_ends() {
    let g = gated();
    let root = PathBuf::from("/unused");
    let first = g.service.write(PROJECT, &root, "one".into());
    g.started.recv().unwrap();
    let mut second = g.service.write(PROJECT, &root, "two".into());
    g.release.send(()).unwrap();
    assert_eq!(wait(first).unwrap(), "hash-of-one");
    g.started.recv().unwrap();
    // The second write is still held: its text is not answered yet.
    assert!(matches!(second.try_recv(), Err(TryRecvError::Empty)));
    g.release.send(()).unwrap();
    assert_eq!(wait(second).unwrap(), "hash-of-two");
}

#[test]
fn a_write_after_the_queue_drained_starts_at_once() {
    let g = gated();
    let root = PathBuf::from("/unused");
    let first = g.service.write(PROJECT, &root, "one".into());
    g.started.recv().unwrap();
    g.release.send(()).unwrap();
    wait(first).unwrap();
    settled(&g.service);
    let next = g.service.write(PROJECT, &root, "two".into());
    g.started.recv().unwrap();
    g.release.send(()).unwrap();
    assert_eq!(wait(next).unwrap(), "hash-of-two");
}

#[test]
fn a_write_of_another_project_is_not_held_by_a_running_one() {
    let g = gated();
    let root = PathBuf::from("/unused");
    let held = g.service.write("aaaaaaaaaaaaaaaa", &root, "one".into());
    g.started.recv().unwrap();
    let other = g.service.write("bbbbbbbbbbbbbbbb", &root, "two".into());
    g.started.recv().unwrap();
    g.release.send(()).unwrap();
    g.release.send(()).unwrap();
    wait(held).unwrap();
    wait(other).unwrap();
    assert_eq!(g.overlap.lock().unwrap().1, 2);
}

#[test]
fn a_panicking_store_fails_its_waiters_and_frees_the_window_for_the_next_write() {
    let store = Arc::new(|_root: &Path, text: &str| -> io::Result<String> {
        if text == "boom" {
            panic!("the store broke");
        }
        Ok(sheet::hash_of(text))
    });
    let service = NotesService::with_store(store);
    let root = PathBuf::from("/unused");
    let broken = service.write(PROJECT, &root, "boom".into());
    assert!(broken.blocking_recv().is_err());
    settled(&service);
    assert_eq!(
        wait(service.write(PROJECT, &root, "fine".into())).unwrap(),
        sheet::hash_of("fine")
    );
}

// --- the flush ----------------------------------------------------------------------------------

#[test]
fn the_flush_answer_wakes_the_window_that_waits() {
    let service = NotesService::new();
    let (answer, mut receiver) = oneshot::channel();
    service
        .flushing
        .lock()
        .unwrap()
        .insert(PROJECT.to_owned(), answer);
    service.finish_flush("another-project");
    assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
    service.finish_flush(PROJECT);
    assert!(receiver.blocking_recv().is_ok());
    // An answer nobody waits for is harmless.
    service.finish_flush(PROJECT);
}

// --- the trash ----------------------------------------------------------------------------------

#[test]
fn the_service_trashes_each_image_on_its_own_and_lists_the_refusals() {
    let (_dir, root) = project();
    let notes = notes_dir(&root);
    fs::create_dir_all(&notes).unwrap();
    for name in ["a.png", "b.png", "c.png"] {
        fs::write(notes.join(name), b"x").unwrap();
    }
    let service = NotesService::new();
    let trash = |path: &Path| {
        if path.file_name().unwrap() == "b.png" {
            Err("no trash here".to_owned())
        } else {
            fs::remove_file(path).map_err(|error| error.to_string())
        }
    };
    let done = service.trash(
        &root,
        &["a.png".to_owned(), "b.png".to_owned(), "c.png".to_owned()],
        &trash,
    );
    assert_eq!(
        done,
        NotesTrashed {
            refused: vec![NotesTrashRefusal {
                name: "b.png".into(),
                reason: "no trash here".into()
            }]
        }
    );
    assert!(!notes.join("a.png").exists() && notes.join("b.png").exists());
    assert!(!notes.join("c.png").exists());
}

// --- the shapes the UI reads --------------------------------------------------------------------

#[test]
fn an_outcome_is_a_tagged_object_the_view_can_switch_on() {
    let json = |outcome: Outcome| serde_json::to_value(NotesImageOutcome::from(outcome)).unwrap();
    assert_eq!(
        json(Outcome::Added {
            name: "a.png".into()
        }),
        serde_json::json!({ "status": "added", "name": "a.png" })
    );
    assert_eq!(
        json(Outcome::Refused {
            source: "big.png".into(),
            why: Why::TooLarge
        }),
        serde_json::json!({ "status": "refused", "source": "big.png", "reason": "tooLarge" })
    );
    assert_eq!(
        json(Outcome::Refused {
            source: "a.svg".into(),
            why: Why::NotAccepted
        }),
        serde_json::json!({ "status": "refused", "source": "a.svg", "reason": "notAccepted" })
    );
    assert_eq!(
        json(Outcome::Failed {
            source: "a.png".into(),
            message: "disk full".into()
        }),
        serde_json::json!({ "status": "failed", "source": "a.png", "message": "disk full" })
    );
    assert_eq!(
        json(Outcome::LeftOut),
        serde_json::json!({ "status": "leftOut" })
    );
}

#[test]
fn a_pasted_image_is_read_from_name_and_data() {
    let image: PastedImage =
        serde_json::from_value(serde_json::json!({ "name": null, "data": "AAAA" })).unwrap();
    assert_eq!(image.name, None);
    assert_eq!(image.data, "AAAA");
}

#[test]
fn the_sheet_answer_has_camel_case_keys_and_nulls_for_a_missing_file() {
    assert_eq!(
        serde_json::to_value(NotesSheet {
            text: None,
            hash: None
        })
        .unwrap(),
        serde_json::json!({ "text": null, "hash": null })
    );
    assert_eq!(
        serde_json::to_value(NotesFlush {
            project_id: PROJECT.into()
        })
        .unwrap(),
        serde_json::json!({ "projectId": PROJECT })
    );
}
