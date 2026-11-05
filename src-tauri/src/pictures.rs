//! The in-memory picture of each open project.
//!
//! One [`ProjectImage`] per open project, keyed by the project hash (the `p-<hash>` label of its
//! window), with a revision that grows at every new picture. Parsing is incremental: a rebuild
//! re-reads only the paths it is told about. The picture is derived data: views (the Board, ...)
//! are worked out from it on demand, never stored.
//!
//! Who feeds it:
//! * a window that opens a project calls [`Pictures::open`] and, when it closes, [`Pictures::close`];
//! * a decision that wrote files publishes the image it got back ([`crate::decisions::decide`],
//!   [`Pictures::publish`]);
//! * the file watcher, once per batch of changes, calls [`Pictures::rebuild`] with the paths that
//!   changed (or [`Rebuild::All`]).
//!
//! Each of them gets back the new [`Revision`] (or nothing when no file differs), to be sent as
//! one `project-changed` event ([`crate::events`]) whose slices are the parts of the picture that
//! changed, found by comparing the content hash of each file.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use weaver_core::paths;
use weaver_core::project::image::ProjectImage;
use weaver_core::project::layout;

use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::events::ProjectChanged;

/// The slice of the picture the Board is worked out from: Tasks, Drifts, Proposals and the Model
/// (it decides which things concern the same thing).
pub const SLICE_BOARD: &str = "board";
pub const SLICE_MODEL: &str = "model";
pub const SLICE_REQUIREMENTS: &str = "requirements";
pub const SLICE_SPECS: &str = "specs";
pub const SLICE_HISTORY: &str = "history";
/// Every slice, in the order an event lists them.
pub const ALL_SLICES: [&str; 5] = [
    SLICE_BOARD,
    SLICE_MODEL,
    SLICE_REQUIREMENTS,
    SLICE_SPECS,
    SLICE_HISTORY,
];

/// What a rebuild re-reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rebuild {
    /// Every file (only the ones whose content changed are parsed again).
    All,
    /// These paths (absolute, inside `.weaver/`).
    Paths(Vec<PathBuf>),
}

/// A new picture was published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub project_id: String,
    pub revision: u64,
    pub slices: Vec<String>,
}

impl Revision {
    /// The payload of the `project-changed` event.
    pub fn event(&self) -> ProjectChanged {
        ProjectChanged {
            project_id: self.project_id.clone(),
            revision: self.revision,
            slices: self.slices.clone(),
        }
    }
}

/// The label of a project window is `p-<project hash>`.
pub const WINDOW_LABEL_PREFIX: &str = "p-";

/// The project hash named by a window label, `None` for any other window (the welcome window).
pub fn project_of_label(label: &str) -> Option<&str> {
    label.strip_prefix(WINDOW_LABEL_PREFIX)
}

/// The slices a change of the file `rel` (relative to `.weaver/`) touches. Local-only folders
/// (requests, short numbers, images) touch none.
fn slices_touched_by(rel: &str) -> &'static [&'static str] {
    match rel.split('/').next().unwrap_or("") {
        layout::MODEL_DIR => &[SLICE_BOARD, SLICE_MODEL],
        layout::REQUIREMENTS_DIR => &[SLICE_REQUIREMENTS],
        layout::SPECS_FILE => &[SLICE_SPECS],
        layout::TASKS_DIR | layout::DRIFTS_DIR | layout::PROPOSALS_DIR => &[SLICE_BOARD],
        layout::HISTORY_DIR => &[SLICE_HISTORY],
        _ => &[],
    }
}

/// The slices in which `new` differs from `old`: a file added, removed or with another content.
pub fn changed_slices(old: &ProjectImage, new: &ProjectImage) -> Vec<String> {
    let rels: BTreeSet<&str> = old.paths().chain(new.paths()).collect();
    let mut touched: BTreeSet<&str> = BTreeSet::new();
    for rel in rels {
        if old.file_hash(rel) != new.file_hash(rel) {
            touched.extend(slices_touched_by(rel).iter().copied());
        }
    }
    ALL_SLICES
        .iter()
        .filter(|slice| touched.contains(*slice))
        .map(|slice| (*slice).to_owned())
        .collect()
}

/// A picture and the revision it has.
#[derive(Clone)]
pub struct Picture {
    pub root: PathBuf,
    pub image: Arc<ProjectImage>,
    pub revision: u64,
}

#[derive(Default)]
pub struct Pictures {
    open: Mutex<HashMap<String, Picture>>,
}

fn not_open(project_id: &str) -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        format!("the project `{project_id}` is not open"),
    )
}

/// The outcome of [`Pictures::swap`].
#[derive(Debug, PartialEq, Eq)]
enum Swapped {
    Published(Option<Revision>),
    Stale,
}

impl Pictures {
    pub fn new() -> Self {
        Self::default()
    }

    fn table(&self) -> std::sync::MutexGuard<'_, HashMap<String, Picture>> {
        self.open.lock().expect("picture table poisoned")
    }

    /// Reads the project at `root` (on the blocking pool) and returns its project hash; a project
    /// that is already open keeps its picture.
    pub async fn open(&self, root: PathBuf) -> Result<String, AppError> {
        let canonical = blocking::disk(move || paths::canonical_root(&root)).await??;
        let id = paths::project_hash_of_canonical(&canonical);
        if self.table().contains_key(&id) {
            return Ok(id);
        }
        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        let loaded = {
            let canonical = canonical.clone();
            blocking::disk_named("open project", name, move || ProjectImage::load(&canonical))
                .await?
        };
        self.table().entry(id.clone()).or_insert(Picture {
            root: canonical,
            image: Arc::new(loaded),
            revision: 1,
        });
        Ok(id)
    }

    /// Forgets the picture of a project whose window closed.
    pub fn close(&self, project_id: &str) {
        self.table().remove(project_id);
    }

    /// The current picture of an open project.
    pub fn get(&self, project_id: &str) -> Result<Picture, AppError> {
        self.table()
            .get(project_id)
            .cloned()
            .ok_or_else(|| not_open(project_id))
    }

    /// Replaces the picture with `image` (the one a decision got back after writing). `None`
    /// when no file differs from the picture held.
    pub fn publish(
        &self,
        project_id: &str,
        image: ProjectImage,
    ) -> Result<Option<Revision>, AppError> {
        match self.swap(project_id, None, image)? {
            Swapped::Published(revision) => Ok(revision),
            Swapped::Stale => unreachable!("a swap with no base revision is never stale"),
        }
    }

    /// A new revision for something the files do not show (which jobs work, a ghost that came or
    /// went): the picture stays, the UI is told to ask again for `slices`.
    pub fn touch(&self, project_id: &str, slices: &[&str]) -> Result<Revision, AppError> {
        let mut table = self.table();
        let picture = table
            .get_mut(project_id)
            .ok_or_else(|| not_open(project_id))?;
        picture.revision += 1;
        Ok(Revision {
            project_id: project_id.to_owned(),
            revision: picture.revision,
            slices: slices.iter().map(|slice| (*slice).to_owned()).collect(),
        })
    }

    /// Replaces the picture with `image`, unless `base` is given and the picture is no longer
    /// at that revision: a newer picture came first and `image` is dropped.
    fn swap(
        &self,
        project_id: &str,
        base: Option<u64>,
        image: ProjectImage,
    ) -> Result<Swapped, AppError> {
        let mut table = self.table();
        let picture = table
            .get_mut(project_id)
            .ok_or_else(|| not_open(project_id))?;
        if base.is_some_and(|base| base != picture.revision) {
            return Ok(Swapped::Stale);
        }
        let slices = changed_slices(&picture.image, &image);
        if slices.is_empty() {
            // Same files: the held picture stays, and nothing is announced.
            return Ok(Swapped::Published(None));
        }
        picture.image = Arc::new(image);
        picture.revision += 1;
        Ok(Swapped::Published(Some(Revision {
            project_id: project_id.to_owned(),
            revision: picture.revision,
            slices,
        })))
    }

    /// Re-reads what changed and publishes the new picture (`None` when no file differs). A
    /// result computed from a picture that a decision replaced meanwhile is dropped and computed
    /// again from the newer one.
    pub async fn rebuild(
        &self,
        project_id: &str,
        change: Rebuild,
    ) -> Result<Option<Revision>, AppError> {
        loop {
            let base = self.get(project_id)?;
            let name = base
                .root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            let work = change.clone();
            let image = base.image.clone();
            let next = blocking::disk_named("rebuild picture", name, move || {
                let mut next = (*image).clone();
                match work {
                    Rebuild::All => next.refresh_all(),
                    Rebuild::Paths(paths) => next.refresh(&paths),
                }
                next
            })
            .await?;
            if let Swapped::Published(revision) =
                self.swap(project_id, Some(base.revision), next)?
            {
                return Ok(revision);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
        let path = root.join(".weaver").join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    const TASK: &str = "---\ncolumn: todo\ntechnical: false\nrefs: []\ncreated: 2026-11-02T07:00:01.000Z\n---\n# A task\n";

    #[tokio::test]
    async fn open_reads_the_project_once_and_a_rebuild_bumps_the_revision_and_names_the_slices() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "tasks/aaaaaaaa-one.md", TASK);
        let pictures = Pictures::new();
        let id = pictures.open(dir.path().to_path_buf()).await.unwrap();
        assert_eq!(id.len(), 16);
        assert_eq!(
            pictures.open(dir.path().to_path_buf()).await.unwrap(),
            id,
            "a project open already is not read again"
        );
        let first = pictures.get(&id).unwrap();
        assert_eq!((first.revision, first.image.cards().len()), (1, 1));

        // The watcher reports paths under the canonical root, the one the picture holds.
        let added = write(
            &pictures.get(&id).unwrap().root,
            "tasks/bbbbbbbb-two.md",
            TASK,
        );
        let revision = pictures
            .rebuild(&id, Rebuild::Paths(vec![added]))
            .await
            .unwrap();
        assert_eq!(
            revision,
            Some(Revision {
                project_id: id.clone(),
                revision: 2,
                slices: vec!["board".into()]
            })
        );
        let second = pictures.get(&id).unwrap();
        assert_eq!(second.image.cards().len(), 2);
        assert_eq!(
            first.image.cards().len(),
            1,
            "who holds the old picture still sees it whole"
        );
    }

    #[tokio::test]
    async fn a_rebuild_that_finds_nothing_new_sends_no_revision() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(dir.path(), "tasks/aaaaaaaa-one.md", TASK)
            .canonicalize()
            .unwrap();
        let pictures = Pictures::new();
        let id = pictures.open(dir.path().to_path_buf()).await.unwrap();
        assert_eq!(
            pictures
                .rebuild(&id, Rebuild::Paths(vec![file]))
                .await
                .unwrap(),
            None
        );
        assert_eq!(pictures.rebuild(&id, Rebuild::All).await.unwrap(), None);
        assert_eq!(pictures.get(&id).unwrap().revision, 1);
    }

    #[tokio::test]
    async fn a_project_that_is_not_open_is_refused() {
        let pictures = Pictures::new();
        assert_eq!(
            pictures.get("0123456789abcdef").err().unwrap().code,
            ErrorCode::InvalidInput
        );
        assert!(pictures
            .rebuild("0123456789abcdef", Rebuild::All)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn slices_follow_the_folder_of_each_file_that_differs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "tasks/aaaaaaaa-one.md", TASK);
        let before = ProjectImage::load(root);
        write(root, "model/shop/feature.md", "# Shop\n");
        write(root, "history/hhhhhhhh-h.md", "---\ncreated: 2026-11-02T07:00:01.000Z\nupdated: 2026-11-02T07:00:01.000Z\nsummary: s\ntasks: []\n---\n# H\n");
        write(root, "requests/x.md", "request");
        let after = ProjectImage::load(root);
        assert_eq!(
            changed_slices(&before, &after),
            ["board", "model", "history"]
        );
        assert!(changed_slices(&after, &after).is_empty());
        std::fs::remove_file(root.join(".weaver/tasks/aaaaaaaa-one.md")).unwrap();
        assert_eq!(changed_slices(&after, &ProjectImage::load(root)), ["board"]);
    }

    #[tokio::test]
    async fn a_result_computed_from_an_older_picture_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "tasks/aaaaaaaa-one.md", TASK);
        let pictures = Pictures::new();
        let id = pictures.open(dir.path().to_path_buf()).await.unwrap();
        let stale = pictures.get(&id).unwrap();
        write(dir.path(), "tasks/bbbbbbbb-two.md", TASK);
        assert!(pictures
            .publish(&id, ProjectImage::load(dir.path()))
            .unwrap()
            .is_some());
        let late = pictures
            .swap(&id, Some(stale.revision), ProjectImage::load(dir.path()))
            .unwrap();
        assert_eq!(late, Swapped::Stale);
        assert_eq!(pictures.get(&id).unwrap().revision, 2);
    }

    #[tokio::test]
    async fn a_change_the_files_do_not_show_still_gets_a_new_revision() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "tasks/aaaaaaaa-one.md", TASK);
        let pictures = Pictures::new();
        let id = pictures.open(dir.path().to_path_buf()).await.unwrap();
        let revision = pictures.touch(&id, &[SLICE_BOARD]).unwrap();
        assert_eq!(
            revision,
            Revision {
                project_id: id.clone(),
                revision: 2,
                slices: vec!["board".into()]
            }
        );
        assert_eq!(pictures.get(&id).unwrap().revision, 2);
        assert!(pictures.touch("0123456789abcdef", &[SLICE_BOARD]).is_err());
    }

    #[tokio::test]
    async fn the_picture_lives_until_its_window_closes() {
        assert_eq!(
            project_of_label("p-0123456789abcdef"),
            Some("0123456789abcdef")
        );
        assert_eq!(project_of_label("welcome"), None);
        let dir = tempfile::tempdir().unwrap();
        let pictures = Pictures::new();
        let id = pictures.open(dir.path().to_path_buf()).await.unwrap();
        assert!(pictures.get(&id).is_ok());
        pictures.close(&id);
        assert!(pictures.get(&id).is_err());
    }
}
