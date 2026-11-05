//! The Tauri commands of the Notes view. They take image names, never paths: the backend builds
//! `.weaver/notes/<name>` after checking the name. The only absolute paths are the sources of a
//! drop from outside, as Tauri reports them. The UI never touches a file.
//!
//! Every command that reads or changes files runs on the blocking pool, so that none of them
//! blocks typing or another command of the window. A failure is an [`AppError`] whose message is
//! the reason the view shows.

use std::path::{Path, PathBuf};

use tauri::State;
use weaver_core::file_view::Refusal;
use weaver_core::notes::images::{self, Source};
use weaver_core::notes::sheet;
use weaver_core::project::requests::id::LocalTime;
use weaver_core::project::requests::image::base64_decode;

use super::{
    NotesImageOutcome, NotesImagesAdded, NotesService, NotesSheet, NotesTrashed, NotesWritten,
    PastedImage,
};
use crate::blocking;
use crate::error::{AppError, ErrorCode};
use crate::log_warn;
use crate::state::AppState;

fn refused(refusal: Refusal) -> AppError {
    AppError::new(ErrorCode::Refused, refusal.to_string())
}

fn root_of(state: &State<'_, AppState>, project_id: &str) -> Result<PathBuf, AppError> {
    Ok(state.pictures.get(project_id)?.root)
}

fn local_time(text: &str) -> Result<LocalTime, AppError> {
    LocalTime::parse(text).ok_or_else(|| {
        AppError::new(
            ErrorCode::InvalidInput,
            "The local time of the images is not a valid time.",
        )
    })
}

async fn write_text(
    service: &NotesService,
    project_id: &str,
    root: &Path,
    text: String,
) -> Result<String, AppError> {
    match service.write(project_id, root, text).await {
        Ok(Ok(hash)) => Ok(hash),
        Ok(Err(message)) => Err(AppError::internal(message)),
        Err(_) => Err(AppError::internal(
            "The notes could not be saved: the write did not complete.",
        )),
    }
}

/// The sheet of the project: its text and the hash of its bytes, or neither when the file is
/// missing. The hash is remembered as the last one read, so that the watcher does not send it.
#[tauri::command]
pub async fn notes_read(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<NotesSheet, AppError> {
    let root = root_of(&state, &project_id)?;
    let service = state.notes.clone();
    blocking::disk_named("notes_read", None, move || match sheet::read(&root) {
        Ok(Some(read)) => {
            service.remember_hash(&project_id, Some(read.hash.clone()));
            Ok(NotesSheet {
                text: Some(read.text),
                hash: Some(read.hash),
            })
        }
        Ok(None) => {
            service.remember_hash(&project_id, None);
            Ok(NotesSheet {
                text: None,
                hash: None,
            })
        }
        Err(error) => Err(AppError::internal(format!(
            "The notes could not be read: {error}"
        ))),
    })
    .await?
}

/// Writes the whole sheet atomically, UTF-8 with LF, and answers the hash of what was written.
/// At most one write per window runs at a time: texts that arrive meanwhile coalesce into one
/// more write of the latest, whose hash all of them answer.
#[tauri::command]
pub async fn notes_write(
    state: State<'_, AppState>,
    project_id: String,
    text: String,
) -> Result<NotesWritten, AppError> {
    let root = root_of(&state, &project_id)?;
    let service = state.notes.clone();
    let hash = write_text(&service, &project_id, &root, text).await?;
    Ok(NotesWritten { hash })
}

/// `data:<mime>;base64,<…>` of the image `name` of the notes folder, from one read of the file.
#[tauri::command]
pub async fn notes_image(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<String, AppError> {
    let root = root_of(&state, &project_id)?;
    blocking::disk_named("notes_image", None, move || {
        images::read_data_url(&root, &name).map_err(refused)
    })
    .await?
}

fn wire(outcomes: Vec<images::Outcome>) -> NotesImagesAdded {
    NotesImagesAdded {
        files: outcomes.into_iter().map(NotesImageOutcome::from).collect(),
    }
}

/// `developer.ADD_NOTES_IMAGES` for pasted images: copies each file, given as bytes in base64
/// with its name when it has one, into the notes folder. `local_time` is the local date and time
/// (`YYYY-MM-DDTHH:MM:SS`) the names without a file name come from.
#[tauri::command]
pub async fn notes_add_pasted(
    state: State<'_, AppState>,
    project_id: String,
    files: Vec<PastedImage>,
    local_time: String,
) -> Result<NotesImagesAdded, AppError> {
    let root = root_of(&state, &project_id)?;
    let time = self::local_time(&local_time)?;
    blocking::disk_named("notes_add_pasted", None, move || {
        let sources: Vec<Source> = files
            .into_iter()
            .map(|file| Source::Pasted {
                bytes: base64_decode(&file.data)
                    .ok_or_else(|| "its content could not be read.".to_owned()),
                name: file.name.filter(|name| !name.is_empty()),
            })
            .collect();
        wire(images::add(&root, &sources, &time))
    })
    .await
}

/// `developer.ADD_NOTES_IMAGES` for files dropped from outside: copies each, given by its
/// absolute path as Tauri reports it, into the notes folder. The originals are never moved or
/// changed and a link is never followed.
#[tauri::command]
pub async fn notes_add_dropped(
    state: State<'_, AppState>,
    project_id: String,
    paths: Vec<String>,
    local_time: String,
) -> Result<NotesImagesAdded, AppError> {
    let root = root_of(&state, &project_id)?;
    let time = self::local_time(&local_time)?;
    blocking::disk_named("notes_add_dropped", None, move || {
        let sources: Vec<Source> = paths
            .into_iter()
            .map(|path| Source::Path(PathBuf::from(path)))
            .collect();
        wire(images::add(&root, &sources, &time))
    })
    .await
}

/// `developer.LEAVE_NOTES_VIEW`: moves each of the images `names` to the system's trash, one at a
/// time; the ones the trash refuses stay and come back with their reasons.
#[tauri::command]
pub async fn notes_trash(
    state: State<'_, AppState>,
    project_id: String,
    names: Vec<String>,
) -> Result<NotesTrashed, AppError> {
    let root = root_of(&state, &project_id)?;
    let service = state.notes.clone();
    blocking::disk_named("notes_trash", None, move || {
        service.trash(&root, &names, &images::system_trash)
    })
    .await
}

/// The answer to `notes-flush`: the window is closing. Writes the text still waiting (none: there
/// is none) and then moves `trash`, the images removed during the visit and no longer named by
/// that text, to the trash. When the text cannot be written nothing is moved.
#[tauri::command]
pub async fn notes_flushed(
    state: State<'_, AppState>,
    project_id: String,
    text: Option<String>,
    trash: Vec<String>,
) -> Result<(), AppError> {
    let service = state.notes.clone();
    let result = flush_work(&state, &service, &project_id, text, trash).await;
    service.finish_flush(&project_id);
    result
}

async fn flush_work(
    state: &State<'_, AppState>,
    service: &std::sync::Arc<NotesService>,
    project_id: &str,
    text: Option<String>,
    trash: Vec<String>,
) -> Result<(), AppError> {
    if text.is_none() && trash.is_empty() {
        return Ok(());
    }
    let root = root_of(state, project_id)?;
    if let Some(text) = text {
        write_text(service, project_id, &root, text).await?;
    }
    if trash.is_empty() {
        return Ok(());
    }
    let service = service.clone();
    let done = blocking::disk_named("notes_flushed", None, move || {
        service.trash(&root, &trash, &images::system_trash)
    })
    .await?;
    for refused in &done.refused {
        log_warn!(
            "the image {} of the notes stayed: {}",
            refused.name,
            refused.reason
        );
    }
    Ok(())
}
