// The only place the UI talks to the Rust backend. Types come from `src/bindings/`, generated
// from the Rust types with ts-rs (never edit them by hand). The UI holds no domain state: it
// asks through commands and applies the backend's events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import type { AppError } from "@/bindings/AppError";
import type { AppInfo } from "@/bindings/AppInfo";
import type { BoardSnapshot } from "@/bindings/BoardSnapshot";
import type { CommandName } from "@/bindings/CommandName";
import type { EventName } from "@/bindings/EventName";
import type { HistorySnapshot } from "@/bindings/HistorySnapshot";
import type { ConversationSnapshot } from "@/bindings/ConversationSnapshot";
import type { EntriesEvent } from "@/bindings/EntriesEvent";
import type { GeneralEntries } from "@/bindings/GeneralEntries";
import type { GeneralSnapshot } from "@/bindings/GeneralSnapshot";
import type { GeneralState } from "@/bindings/GeneralState";
import type { JobsSnapshot } from "@/bindings/JobsSnapshot";
import type { Notice } from "@/bindings/Notice";
import type { OmpStatus } from "@/bindings/OmpStatus";
import type { InitialiseResult } from "@/bindings/InitialiseResult";
import type { ProjectChanged } from "@/bindings/ProjectChanged";
import type { ProjectStatus } from "@/bindings/ProjectStatus";
import type { SkillsReport } from "@/bindings/SkillsReport";
import type { TreeEntry } from "@/bindings/TreeEntry";
import type { ShowRequest } from "@/bindings/ShowRequest";
import type { SettingChange } from "@/bindings/SettingChange";
import type { SettingsSnapshot } from "@/bindings/SettingsSnapshot";
import type { ViewSnapshot } from "@/bindings/ViewSnapshot";
import type { StartupFailure } from "@/bindings/StartupFailure";
import type { UpdateStatus } from "@/bindings/UpdateStatus";
import type { RequestView } from "@/bindings/RequestView";
import type { RequestsEvent } from "@/bindings/RequestsEvent";
import type { ActionInput } from "@/bindings/ActionInput";
import type { QuestionRoundChanged } from "@/bindings/QuestionRoundChanged";
import type { QuestionRoundView } from "@/bindings/QuestionRoundView";
import type { RequestsSnapshot } from "@/bindings/RequestsSnapshot";
import type { WelcomeState } from "@/bindings/WelcomeState";

/** Payload of each event, by name. */
export interface EventPayloads {
  "project-changed": ProjectChanged;
  "startup-failed": StartupFailure;
  "update-changed": UpdateStatus;
  /** A setting was written: the whole snapshot, so a window never merges. */
  "settings-changed": SettingsSnapshot;
  /** omp's readiness changed: the whole status of the Machine `omp-setup`, with its revision. */
  "omp-changed": OmpStatus;
  /** This window came forward because of a notification or a second launch: what to show. */
  "show-requested": ShowRequest;
  /** Something to read at the top of the Board: a refused decision, a Proposal sent back to Draft. */
  notice: Notice;
  /** The Jobs list of a project changed: the whole list. */
  "jobs-changed": JobsSnapshot;
  /** The conversation of a job changed: the entries that changed, in batches counted by `seq`. */
  "job-entries": EntriesEvent;
  /** The state of the General of a project changed: its whole state. */
  "general-changed": GeneralState;
  /** The conversation of a General changed: the entries that changed, in batches counted by `seq`. */
  "general-entries": GeneralEntries;
  /** The open design requests of a project changed (a decision, or a file changed on disk). */
  "requests-changed": RequestsEvent;
  /** The window is closing: the request editor saves what is pending and answers with `requests_flushed`. */
  "requests-flush": RequestsEvent;
  /** The round on screen in a project changed (one opened, ended, or queued behind it). */
  "question-round-changed": QuestionRoundChanged;
  /** The state of a project, the files it installs and the opening check changed: the whole status. */
  "project-state": ProjectStatus;
}

/** Arguments and result of each command, by name. */
export interface Commands {
  app_info: { args: Record<string, never>; result: AppInfo };
  startup_failures: { args: Record<string, never>; result: StartupFailure[] };
  /** Reveals the log file in the system's file manager. */
  open_log: { args: Record<string, never>; result: null };
  /** Opens an `http`/`https` link in the system browser (the backend refuses any other). */
  open_url: { args: { url: string }; result: null };
  /** The update on offer (none when there is none or **Later** hid it). */
  update_status: { args: Record<string, never>; result: UpdateStatus };
  /** **Later**: hides the update until the next check. */
  update_later: { args: Record<string, never>; result: null };
  /** **Install and restart**: downloads the update, then quits Weaver through its quit path. */
  update_install: { args: Record<string, never>; result: null };
  /** The settings in force, what the file ignored and the description of every setting. */
  settings_get: { args: Record<string, never>; result: SettingsSnapshot };
  /** Validates and stores one change; `invalid-input` carries the message for the field. */
  settings_set: { args: { change: SettingChange }; result: SettingsSnapshot };
  /** The whole Board of an open project, worked out by the backend at one revision. */
  board_snapshot: { args: { projectId: string }; result: BoardSnapshot };
  /** **Delete** on a Task in To do; `refused` carries the notice when it no longer holds. */
  delete_task: { args: { projectId: string; taskId: string }; result: null };
  /** **Approve** on a Proposal: approved, or Refining for a stale change; `refused` carries the notice. */
  approve_proposal: { args: { projectId: string; proposalId: string }; result: null };
  /** **Reject** (`feedback` is the optional reason) and, with `rework`, **Rework with feedback…**. */
  reject_proposal: { args: { projectId: string; proposalId: string; feedback: string | null; rework: boolean }; result: null };
  /** **Discard** on a Draft a refinement left half-way. */
  discard_proposal: { args: { projectId: string; proposalId: string }; result: null };
  /** The model control of a Proposal: a model of omp, or `null` for the Settings models. */
  choose_proposal_model: { args: { projectId: string; proposalId: string; model: string | null }; result: null };
  /** **Resolve** on a Drift: a round of the prepared question, or a resolve job; `refused` carries the notice. */
  resolve_drift: { args: { projectId: string; driftId: string }; result: null };
  /** **Fix the code** on a Drift: the fix option (required when it offers some) and the note. */
  fix_drift_code: { args: { projectId: string; driftId: string; option: number | null; note: string | null }; result: null };
  /** **Accept Proposal** on a Drift with an attached Proposal. */
  accept_drift_proposal: { args: { projectId: string; driftId: string }; result: null };
  /** **Dismiss Drift**, once confirmed. */
  dismiss_drift: { args: { projectId: string; driftId: string }; result: null };
  /** The whole History of an open project (entries newest first), worked out at one revision. */
  history_snapshot: { args: { projectId: string }; result: HistorySnapshot };
  /**
   * The Requirements and Specs of an open project at one revision, with the Proposal `overlay`
   * drawn over the Model when it is still open (`null`: the current Model). The answer names the
   * Proposal it really drew.
   */
  view_snapshot: { args: { projectId: string; overlay: string | null }; result: ViewSnapshot };
  /** The state of the project (Not initialised, Empty Model, Has a Model), what Get Started lists and the opening check. */
  project_status: { args: { projectId: string }; result: ProjectStatus };
  /** **Initialise**: creates `.weaver` and the agent files; `force` confirms overwriting agent files that differ (else they come back as `conflicts`). */
  initialise_project: { args: { projectId: string; force: boolean }; result: InitialiseResult };
  /** **Update skills**: the structured report the command line prints too. */
  update_skills: { args: { projectId: string }; result: SkillsReport };
  /** The entries of one folder of the project (`path` relative, empty for the project folder) for the import tree. */
  project_folder: { args: { projectId: string; path: string }; result: TreeEntry[] };
  /** **Start import**: starts the import job, leaving out the `excluded` paths (relative, `/`). */
  import_code: { args: { projectId: string; excluded: string[] }; result: null };
  /** The status of omp now (the Machine `omp-setup`). */
  omp_status: { args: Record<string, never>; result: OmpStatus };
  /** **Install omp** / **Open installer again**: opens the system terminal with the official installer. */
  omp_install: { args: Record<string, never>; result: null };
  /** **Log in**: opens the system terminal with `omp login <provider>`; `provider` is an id of the fixed list. */
  omp_log_in: { args: { provider: string }; result: null };
  /** **Log out**: `omp auth-broker logout <provider>`; the outcome arrives with `omp-changed`. */
  omp_log_out: { args: { provider: string }; result: null };
  /** The name, version and recent projects of the welcome window. */
  welcome_state: { args: Record<string, never>; result: WelcomeState };
  /** `developer.OPEN_PROJECT` for a folder: opens its window or brings it forward. */
  open_project: { args: { path: string }; result: null };
  /** The system folder picker, then the same as `open_project`; `false` when none was chosen. */
  open_folder: { args: Record<string, never>; result: boolean };
  /** Removes a recent project; answers the welcome state without it. */
  remove_recent: { args: { path: string }; result: WelcomeState };
  /** `Open in editor`: `file` (absolute, or relative to the project) at `line`; a failure carries the reason. */
  open_in_editor: { args: { file: string; line?: number | null; projectId?: string | null }; result: null };
  /** The Jobs list of an open project. */
  jobs_snapshot: { args: { projectId: string }; result: JobsSnapshot };
  /** The conversation of a job, with the count of change batches it holds. */
  job_conversation: { args: { projectId: string; jobId: string }; result: ConversationSnapshot };
  /** A message to the agent of a job; it reaches the agent without stopping it. */
  job_send_message: { args: { projectId: string; jobId: string; text: string }; result: null };
  /** `Stop the agent`. */
  job_stop: { args: { projectId: string; jobId: string }; result: null };
  /** The developer's choice on a permission card: one of the options omp offered. */
  job_answer_permission: { args: { projectId: string; jobId: string; request: number; option: string }; result: null };
  /** The `X` of a finished job's row. */
  job_remove: { args: { projectId: string; jobId: string }; result: null };
  /** `Run` on a Task; a refusal (`refused`) carries the notice. */
  run_task: { args: { projectId: string; taskId: string }; result: null };
  /** `Run all`; a refusal carries the notice. */
  run_all: { args: { projectId: string }; result: null };
  /** `Scan System`, once confirmed. */
  scan_system: { args: { projectId: string }; result: null };
  /** The conversation and the state of the General of an open project. */
  general_snapshot: { args: { projectId: string }; result: GeneralSnapshot };
  /** A message to the General; omp starts when it is not running. */
  general_send_message: { args: { projectId: string; text: string }; result: null };
  /** `Stop` during a turn of the General. */
  general_stop: { args: { projectId: string }; result: null };
  /** The developer's choice on a permission card of the General. */
  general_answer_permission: { args: { projectId: string; request: number; option: string }; result: null };
  /** `Clear General?`, once confirmed: the conversation and its record are deleted. */
  general_clear: { args: { projectId: string }; result: null };
  /** The open design requests of a project (writing or sent), newest first. */
  requests_list: { args: { projectId: string }; result: RequestsSnapshot };
  /** The text and standing of one request. */
  request_read: { args: { projectId: string; requestId: string }; result: RequestView };
  /** `developer.NEW_REQUEST`: an empty request named after the local time `YYYY-MM-DDTHH:MM:SS`; its id. */
  request_new: { args: { projectId: string; localTime: string }; result: string };
  /** Saves the text of a request being written. */
  request_save: { args: { projectId: string; requestId: string; text: string }; result: null };
  /** `developer.SEND_REQUEST`; an empty request does nothing. */
  request_send: { args: { projectId: string; requestId: string }; result: null };
  /** `developer.DISCARD_REQUEST`: the file and the images are deleted. */
  request_discard: { args: { projectId: string; requestId: string }; result: null };
  /** Saves a pasted image (base64 `data`); the path to link, relative to the request file. */
  request_image_add: { args: { projectId: string; requestId: string; data: string }; result: string };
  /** Links a dropped image file; the path to link, relative to the request file. */
  request_image_import: { args: { projectId: string; requestId: string; path: string }; result: string };
  /** An image of the request's own images folder as a data URL, for the preview. */
  request_image: { args: { projectId: string; requestId: string; link: string }; result: string };
  /** The editor saved what was pending (the answer to `requests-flush`). */
  requests_flushed: { args: { projectId: string }; result: null };
  /** The round presented in the window of a project, when there is one. */
  question_round_get: { args: { projectId: string }; result: QuestionRoundView | null };
  /** What the developer does in the presented round; the round on screen afterwards. `refused` carries why not. */
  question_round_act: { args: { projectId: string; roundId: number; action: ActionInput }; result: QuestionRoundView | null };
}

// Compile-time proof that the maps above cover exactly the generated names.
type AssertSame<A, B> = [A] extends [B] ? ([B] extends [A] ? true : never) : never;
const _commands: AssertSame<keyof Commands, CommandName> = true;
const _events: AssertSame<keyof EventPayloads, EventName> = true;
void _commands;
void _events;

/** A command failed with the typed error of the backend. */
export function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

export function call<N extends CommandName>(
  name: N,
  args: Commands[N]["args"] = {},
): Promise<Commands[N]["result"]> {
  return invoke<Commands[N]["result"]>(name, args);
}

export function on<N extends EventName>(
  name: N,
  handler: (payload: EventPayloads[N]) => void,
): Promise<UnlistenFn> {
  return listen<EventPayloads[N]>(name, (event) => handler(event.payload));
}

/**
 * The label of this webview's window: `welcome`, or `p-<project hash>` for a project window. Outside
 * Tauri (the dev server in a browser) there is no window and the label is `welcome`.
 */
export function currentWindowLabel(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    return "welcome";
  }
}

/**
 * Calls `handler` with the paths of the files dropped on this window (the system drag and drop of
 * the webview; outside Tauri nothing is ever dropped this way). Returns what stops the listening.
 */
export async function onFilesDropped(handler: (paths: string[]) => void): Promise<UnlistenFn> {
  try {
    return await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") handler(event.payload.paths);
    });
  } catch {
    return () => {};
  }
}
