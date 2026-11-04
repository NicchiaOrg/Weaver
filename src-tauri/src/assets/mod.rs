//! The Assets view in the app: the list of each open project's assets, in memory only, and the
//! actions of the view (the Machine `asset-view`, `assets` Feature).
//!
//! The rules and the disk work are in [`weaver_core::assets`]; this module keeps one [`Project`]
//! per open project with the name and size of every asset, reads it when the project opens and
//! again after each batch of watcher events under `.weaver/assets/`, and tells the window of the
//! project when it changed (`assets-changed`, [`AssetsChanged`]). It is `workspace.ASSETS_CHANGED`.
//! The commands the UI calls are in [`commands`]; a drag out to other apps is in [`drag`].
//!
//! The list is kept for every open project, whether or not the Assets view is shown, so that the
//! `@` tags can be recognised against it: [`AssetsService::assets`] answers it without touching
//! the disk. No event under `.weaver/assets/` reaches the project's picture, its rebuild,
//! `project-changed` or `workspace.FILES_CHANGED` ([`split_batch`] takes them out of a batch).
//!
//! Only names and sizes are ever read of an asset: one `read_dir` and the metadata it returns.
//! The content of an asset is read only to copy it in.

pub mod commands;
pub mod drag;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use ts_rs::TS;
use weaver_core::assets::copy_in::{self, CopyInError, Outcome};
use weaver_core::assets::{ops, Asset};
use weaver_core::file_view::Refusal;

use crate::log_warn;
use crate::pictures::Rebuild;

/// The assets of a project at a generation: the whole list, in the order of the grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetsSnapshot {
    /// Counts the changes of the list since the project opened; a list with a lower number than
    /// the one a window holds is older and is ignored.
    #[ts(type = "number")]
    pub generation: u64,
    pub assets: Vec<Asset>,
}

/// `assets-changed`: the assets of a project changed (`workspace.ASSETS_CHANGED`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetsChanged {
    /// The project hash of the window.
    pub project_id: String,
    #[ts(type = "number")]
    pub generation: u64,
    pub assets: Vec<Asset>,
}

/// What a drop did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetsCopied {
    /// The assets the drop made, in the order of the grid; empty when the drop was ignored.
    pub copied: Vec<String>,
    /// The ones of them larger than the size limit.
    pub large: Vec<String>,
    pub snapshot: AssetsSnapshot,
}

/// What a rename did: the name the asset has now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetRenamed {
    pub name: String,
    pub snapshot: AssetsSnapshot,
}

/// An asset the trash refused, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetRefusal {
    pub name: String,
    pub reason: String,
}

/// What a deletion did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssetsDeleted {
    pub trashed: Vec<String>,
    pub refused: Vec<AssetRefusal>,
    pub snapshot: AssetsSnapshot,
}

/// Receives each change of a project's list; the app sends it to the project's window.
pub type AssetsSink = Arc<dyn Fn(AssetsChanged) + Send + Sync>;

/// Sends `assets-changed` to the window of the project it is about.
pub fn emitter(app: tauri::AppHandle) -> AssetsSink {
    use tauri::Emitter;
    Arc::new(move |changed: AssetsChanged| {
        let label = crate::windows::registry::label_of(&changed.project_id);
        if let Err(error) = app.emit_to(
            &label,
            crate::events::EventName::AssetsChanged.as_str(),
            &changed,
        ) {
            crate::log_warn!("the window {label} was not told of a change of its assets: {error}");
        }
    })
}

/// The one copy a window may run: held from the start of a drop's copy to its end, and gone with
/// the window.
struct CopyGuard<'a>(&'a AtomicBool);

impl<'a> CopyGuard<'a> {
    fn start(copying: &'a AtomicBool) -> Result<Self, Refusal> {
        copying
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self(copying))
            .map_err(|_| Refusal::new("Another copy is still running in this window."))
    }
}

impl Drop for CopyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// The list a project holds.
struct State {
    /// `false` until the list was read once.
    loaded: bool,
    generation: u64,
    assets: Arc<Vec<Asset>>,
}

/// One open project's assets.
struct Project {
    state: Mutex<State>,
    /// Held while the folder is read and the list replaced, so that two reads never finish out of
    /// order and leave the older list in place.
    reading: Mutex<()>,
    /// A drop from outside is being copied.
    copying: AtomicBool,
}

/// The assets of every open project.
pub struct AssetsService {
    sink: Arc<OnceLock<AssetsSink>>,
    projects: Mutex<HashMap<String, Arc<Project>>>,
}

impl Default for AssetsService {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetsService {
    pub fn new() -> Self {
        Self {
            sink: Arc::new(OnceLock::new()),
            projects: Mutex::new(HashMap::new()),
        }
    }

    /// Where `assets-changed` goes; only the first sink counts. The service exists before the app
    /// handle does.
    pub fn attach(&self, sink: AssetsSink) {
        let _ = self.sink.set(sink);
    }

    fn project(&self, project_id: &str) -> Arc<Project> {
        self.projects
            .lock()
            .expect("assets projects poisoned")
            .entry(project_id.to_owned())
            .or_insert_with(|| {
                Arc::new(Project {
                    state: Mutex::new(State {
                        loaded: false,
                        generation: 0,
                        assets: Arc::new(Vec::new()),
                    }),
                    reading: Mutex::new(()),
                    copying: AtomicBool::new(false),
                })
            })
            .clone()
    }

    fn existing(&self, project_id: &str) -> Option<Arc<Project>> {
        self.projects
            .lock()
            .expect("assets projects poisoned")
            .get(project_id)
            .cloned()
    }

    fn snapshot_of(project: &Project) -> AssetsSnapshot {
        let state = project.state.lock().expect("assets state poisoned");
        AssetsSnapshot {
            generation: state.generation,
            assets: state.assets.as_ref().clone(),
        }
    }

    /// The assets of the open project `project_id` as the app holds them now, in the order of the
    /// grid: names and sizes, no disk work. Empty for a project whose list was not read yet. This
    /// is what the `@` tags are recognised against.
    pub fn assets(&self, project_id: &str) -> Arc<Vec<Asset>> {
        self.existing(project_id).map_or_else(
            || Arc::new(Vec::new()),
            |project| {
                project
                    .state
                    .lock()
                    .expect("assets state poisoned")
                    .assets
                    .clone()
            },
        )
    }

    /// Reads the assets folder of the project at `root` (canonical) again. When the list is not
    /// the one held, it replaces it at the next generation and the window of the project is told
    /// (`assets-changed`). A folder that cannot be read leaves the list as it was. Returns the
    /// list as it is after the read.
    pub fn refresh(&self, project_id: &str, root: &Path) -> AssetsSnapshot {
        let project = self.project(project_id);
        let _reading = project.reading.lock().expect("assets reading poisoned");
        let read = weaver_core::assets::list::list(root);
        let changed = {
            let mut state = project.state.lock().expect("assets state poisoned");
            match read {
                Ok(list) if !state.loaded || *state.assets != list => {
                    state.loaded = true;
                    state.generation += 1;
                    state.assets = Arc::new(list);
                    true
                }
                Ok(_) => false,
                Err(error) => {
                    log_warn!(
                        "the assets of {} could not be read: {error}",
                        root.display()
                    );
                    false
                }
            }
        };
        let snapshot = Self::snapshot_of(&project);
        if changed {
            if let Some(sink) = self.sink.get() {
                sink(AssetsChanged {
                    project_id: project_id.to_owned(),
                    generation: snapshot.generation,
                    assets: snapshot.assets.clone(),
                });
            }
        }
        snapshot
    }

    /// The list of the project for a window that starts: the one held, read first when the
    /// project's list was never read.
    pub fn snapshot(&self, project_id: &str, root: &Path) -> AssetsSnapshot {
        let project = self.project(project_id);
        if project.state.lock().expect("assets state poisoned").loaded {
            return Self::snapshot_of(&project);
        }
        self.refresh(project_id, root)
    }

    /// `window.CLOSED`: nothing is kept for the project. A copy still running goes on to its end.
    pub fn close_project(&self, project_id: &str) {
        self.projects
            .lock()
            .expect("assets projects poisoned")
            .remove(project_id);
    }

    // --- the actions of the view ------------------------------------------------------------

    /// `developer.DROP_ASSETS`: copies the dropped `sources` (absolute paths, as Tauri reports
    /// them) into `.weaver/assets/` of the project at `root`. One copy at a time per window; the
    /// copy runs to its end even when the view is left or the window closes.
    pub fn copy_in(
        &self,
        project_id: &str,
        root: &Path,
        sources: &[PathBuf],
    ) -> Result<AssetsCopied, Refusal> {
        self.copy_in_with(project_id, root, sources, &|_| Ok(()))
    }

    /// [`Self::copy_in`] with `step` told each destination before it is made (see
    /// [`copy_in::copy_in_with`]), for the tests.
    pub fn copy_in_with(
        &self,
        project_id: &str,
        root: &Path,
        sources: &[PathBuf],
        step: &dyn Fn(&Path) -> io::Result<()>,
    ) -> Result<AssetsCopied, Refusal> {
        let project = self.project(project_id);
        let _running = CopyGuard::start(&project.copying)?;
        let outcome = copy_in::copy_in_with(root, sources, step)
            .map_err(|error: CopyInError| Refusal::new(error.to_string()))?;
        let (copied, large) = match outcome {
            Outcome::Copied(done) => (done.names, done.large),
            Outcome::Ignored => (Vec::new(), Vec::new()),
        };
        let snapshot = self.refresh(project_id, root);
        Ok(AssetsCopied {
            copied,
            large,
            snapshot,
        })
    }

    /// `developer.RENAME_ASSET`: the asset `name` gets the name `new_name`.
    pub fn rename(
        &self,
        project_id: &str,
        root: &Path,
        name: &str,
        new_name: &str,
    ) -> Result<AssetRenamed, Refusal> {
        let name = ops::rename(root, name, new_name)?;
        let snapshot = self.refresh(project_id, root);
        Ok(AssetRenamed { name, snapshot })
    }

    /// `developer.DELETE_ASSETS`: each asset goes to the trash on its own, through `trash`.
    pub fn delete_with(
        &self,
        project_id: &str,
        root: &Path,
        names: &[String],
        trash: &dyn Fn(&Path) -> Result<(), String>,
    ) -> AssetsDeleted {
        let done = ops::delete(root, names, trash);
        let snapshot = self.refresh(project_id, root);
        AssetsDeleted {
            trashed: done.trashed,
            refused: done
                .refused
                .into_iter()
                .map(|refused| AssetRefusal {
                    name: refused.name,
                    reason: refused.reason,
                })
                .collect(),
            snapshot,
        }
    }

    /// [`Self::delete_with`] the system's trash.
    pub fn delete(&self, project_id: &str, root: &Path, names: &[String]) -> AssetsDeleted {
        self.delete_with(project_id, root, names, &ops::system_trash)
    }
}

/// A batch of watcher events of a project, split in what concerns the assets and what concerns
/// the rest of `.weaver/`.
#[derive(Debug, PartialEq, Eq)]
pub struct Split {
    /// The batch touched `.weaver/assets/` (or may have: events were lost): the list is read again.
    pub assets: bool,
    /// What is left for the project's picture; `None` when nothing is.
    pub rest: Option<Rebuild>,
}

/// Takes the paths under `.weaver/assets/` of the project at `root` out of `batch`, which is
/// what keeps them away from the picture, its rebuild, `project-changed` and
/// `workspace.FILES_CHANGED`. A batch that names no path (events were lost, `.weaver/` appeared
/// or went away) concerns both.
pub fn split_batch(root: &Path, batch: Rebuild) -> Split {
    match batch {
        Rebuild::All => Split {
            assets: true,
            rest: Some(Rebuild::All),
        },
        Rebuild::Paths(paths) => {
            let dir = weaver_core::assets::assets_dir(root);
            let (assets, rest): (Vec<PathBuf>, Vec<PathBuf>) =
                paths.into_iter().partition(|path| path.starts_with(&dir));
            Split {
                assets: !assets.is_empty(),
                rest: (!rest.is_empty()).then_some(Rebuild::Paths(rest)),
            }
        }
    }
}

#[cfg(test)]
mod tests;
