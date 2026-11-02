//! The Diagram, Requirements and Specs views in the app: the snapshot the UI draws.
//!
//! Everything the views show is worked out in `trama_core::project::view` (graph levels, details,
//! overlay labels, where each reference is drawn); this module only reads what that needs from
//! disk, applies the chosen Proposal with `trama_core::project::proposal::analysis` and wraps the
//! result with the revision it was computed from. The UI asks again whenever a `project-changed`
//! event names the `model`, `requirements`, `specs` or `board` slice (Proposals and cards are in
//! `board`), and drops an answer older than one it already shows.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use tauri::State;
use trama_core::project::cards::CardKind;
use trama_core::project::changes::Change;
use trama_core::project::image::ProjectImage;
use trama_core::project::layout;
use trama_core::project::proposal::analysis;
use trama_core::project::proposal::data::{self, Status};
use trama_core::project::proposal::files::{self, Folders};
use trama_core::project::shortnum;
use trama_core::project::text;
use trama_core::project::view::{self, Files, OverlayOption, ViewData, ViewInput};
use ts_rs::TS;

use crate::blocking;
use crate::error::AppError;
use crate::state::AppState;

/// The three views of a project at one revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ViewSnapshot {
    pub project_id: String,
    /// The revision of the picture this was worked out from.
    #[ts(type = "number")]
    pub revision: u64,
    /// The Proposals that can be drawn over the Model, by creation order.
    pub overlays: Vec<OverlayOption>,
    /// The Proposal drawn: the one asked for when it can still be drawn, else `None` (the
    /// current Model).
    pub overlay: Option<String>,
    /// The scope of the Proposal drawn (references), empty without one.
    pub scope: Vec<String>,
    pub data: ViewData,
}

fn read_rel(root: &Path, rel: &str) -> Option<String> {
    text::read_text(&layout::absolute(root, rel)).ok()
}

/// What each path a Proposal chain writes is: `Some((held path, text))`, or `None` when the
/// chain deletes it.
fn held_files(
    image: &ProjectImage,
    folders: &Folders,
    id: &str,
) -> BTreeMap<String, Option<(String, String)>> {
    let mut chain = analysis::dependency_order(image, id);
    chain.push(id.to_owned());
    let empty = files::Folder::default();
    let mut held = BTreeMap::new();
    for proposal in chain {
        let Some(card) = image.card(CardKind::Proposal, &proposal) else {
            continue;
        };
        if !data::status_of(card)
            .is_some_and(|s| matches!(s, Status::Draft | Status::Open | Status::Refining))
        {
            continue;
        }
        for (rel, change) in
            files::changes_of(card, folders.get(&proposal).unwrap_or(&empty)).iter()
        {
            match change {
                Change::Write(content) => {
                    held.insert(
                        rel.clone(),
                        Some((format!("{}/{rel}", data::folder_of(card)), content.clone())),
                    );
                }
                Change::Delete => {
                    held.insert(rel.clone(), None);
                }
            }
        }
    }
    held
}

/// The three views of `image`, with `overlay` (a Proposal identity) drawn over the Model when it
/// can be.
pub fn snapshot(
    project_id: &str,
    revision: u64,
    root: &Path,
    image: &ProjectImage,
    overlay: Option<&str>,
) -> ViewSnapshot {
    let numbers = shortnum::peek(root, image.cards());
    let overlays = view::eligible_overlays(image, &numbers);
    let chosen = overlay
        .filter(|id| overlays.iter().any(|o| o.id == *id))
        .map(str::to_owned);

    let on_disk = |rel: &str| read_rel(root, rel);
    let (data, scope) = match &chosen {
        None => {
            let shown = |rel: &str| read_rel(root, rel).map(|t| (rel.to_owned(), t));
            let input = ViewInput {
                current: image,
                shown: image,
                overlaid: false,
                numbers: &numbers,
                files: Files {
                    current: &on_disk,
                    shown: &shown,
                },
                extra_refs: &[],
            };
            (view::build(&input), Vec::new())
        }
        Some(id) => {
            let folders = files::read_folders(root, image);
            let shown_image = analysis::overlay(image, &folders, id);
            let scope = analysis::scope_refs(image, &folders, id);
            let held = held_files(image, &folders, id);
            let shown = |rel: &str| match held.get(rel) {
                Some(found) => found.clone(),
                None => read_rel(root, rel).map(|t| (rel.to_owned(), t)),
            };
            let input = ViewInput {
                current: image,
                shown: &shown_image,
                overlaid: true,
                numbers: &numbers,
                files: Files {
                    current: &on_disk,
                    shown: &shown,
                },
                extra_refs: &scope,
            };
            (view::build(&input), scope.clone())
        }
    };
    ViewSnapshot {
        project_id: project_id.to_owned(),
        revision,
        overlays,
        overlay: chosen,
        scope,
        data,
    }
}

/// The views of an open project. The layout of the Diagram is not worked out here: the UI does it
/// off its main thread from the levels this returns.
#[tauri::command]
pub async fn view_snapshot(
    state: State<'_, AppState>,
    project_id: String,
    overlay: Option<String>,
) -> Result<ViewSnapshot, AppError> {
    let picture = state.pictures.get(&project_id)?;
    let name = picture
        .root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    blocking::disk_named("view snapshot", name, move || {
        snapshot(
            &project_id,
            picture.revision,
            &picture.root,
            &picture.image,
            overlay.as_deref(),
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    use trama_core::project::view::Change as ViewChange;

    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(".trama").join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    const MACHINE: &str =
        "# Order\n\nAn order.\n\n## States\n\n### Draft (initial)\nChoosing.\n\n### Done (final)\n";
    const PROPOSAL: &str = "---\nstatus: open\nkind: change\nbase:\n  model/shop/order.md: abc\ncreated: 2026-11-02T07:00:01.000Z\n---\n# Reshape the order\n";

    fn project(root: &Path) {
        write(root, "model/system.md", "# Shop\n");
        write(root, "model/shop/feature.md", "# Shopping\n");
        write(root, "model/shop/order.md", MACHINE);
        write(root, "proposals/aaaaaaaa-reshape/proposal.md", PROPOSAL);
        write(
            root,
            "proposals/aaaaaaaa-reshape/model/shop/order.md",
            &MACHINE
                .replace("Choosing.", "Picking.")
                .replace("### Done (final)", "### Done (final)\n\n### Lost\n"),
        );
    }

    #[test]
    fn an_open_proposal_is_offered_and_drawn_over_the_model_with_the_file_of_its_copy() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let image = ProjectImage::load(dir.path());

        let plain = snapshot("0123456789abcdef", 3, dir.path(), &image, None);
        assert_eq!((plain.revision, plain.overlay.as_deref()), (3, None));
        assert_eq!(
            plain
                .overlays
                .iter()
                .map(|o| (o.id.as_str(), o.title.as_str()))
                .collect::<Vec<_>>(),
            [("aaaaaaaa", "Reshape the order")]
        );
        assert!(plain.data.model.details["order/Draft"].change.is_none());

        let drawn = snapshot("0123456789abcdef", 3, dir.path(), &image, Some("aaaaaaaa"));
        assert_eq!(drawn.overlay.as_deref(), Some("aaaaaaaa"));
        let draft = &drawn.data.model.details["order/Draft"];
        assert_eq!(draft.change, Some(ViewChange::Changed));
        assert_eq!(
            draft.file.as_deref(),
            Some(".trama/proposals/aaaaaaaa-reshape/model/shop/order.md")
        );
        assert_eq!(
            drawn.data.model.details["order/Lost"].change,
            Some(ViewChange::Added)
        );
        // The scope of the Proposal is what the focus opens on.
        assert!(
            drawn.scope.contains(&"order/Draft".to_owned()),
            "{:?}",
            drawn.scope
        );
        assert!(drawn.data.targets.contains_key("order/Draft"));
    }

    #[test]
    fn a_proposal_that_can_no_longer_be_drawn_gives_back_the_current_model() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        write(
            dir.path(),
            "proposals/aaaaaaaa-reshape/proposal.md",
            &PROPOSAL.replace("status: open", "status: refining"),
        );
        let image = ProjectImage::load(dir.path());
        let shown = snapshot("0123456789abcdef", 1, dir.path(), &image, Some("aaaaaaaa"));
        assert!(shown.overlays.is_empty() && shown.overlay.is_none() && shown.scope.is_empty());
        assert!(shown.data.model.details["order/Draft"].change.is_none());
    }

    #[test]
    fn the_wire_form_is_camel_case() {
        let dir = tempfile::tempdir().unwrap();
        project(dir.path());
        let image = ProjectImage::load(dir.path());
        let json = serde_json::to_value(snapshot("0123456789abcdef", 2, dir.path(), &image, None))
            .unwrap();
        assert_eq!(json["projectId"], "0123456789abcdef");
        assert_eq!(json["data"]["model"]["levels"][0]["key"], "");
        assert_eq!(json["data"]["model"]["levels"][0]["kind"], "root");
    }
}
