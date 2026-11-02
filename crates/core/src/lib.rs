//! `trama-core`: the library shared by the Weaver app and the `trama` command line.
//!
//! It has no Tauri dependency, no async runtime and does synchronous `std` I/O only.
//! Modules extended by later work (project files, rules, mutations, skills) are added
//! next to the ones below; each module keeps one responsibility.
//!
//! | Module      | Responsibility |
//! |-------------|----------------|
//! | [`paths`]   | App identifier, config / local data / runtime folders, project hash |
//! | [`atomic`]  | Atomic file writes (temp file + rename), optional Unix mode |
//! | [`endpoint`]| CSPRNG helpers, socket / named-pipe endpoint naming |
//! | [`runtime`] | `runtime.json`: `{endpoint, secret, pid, version}` |
//! | [`protocol`]| CLI↔app messages: framing, size limit, protocol version, secret check |
//! | [`ipc`]     | Local-socket transport (`interprocess`) and one-request-per-connection serving |
//! | [`lock`]    | OS advisory project lock shared by the app and the CLI |
//! | [`process`] | The one function every child process is started with |
//! | [`project`] | Project files under `.trama/`: parsers, rules, changes, in-memory image |
//! | [`settings`]| Weaver's settings: typed schema, defaults, validation, `settings.json` store |
//! | [`agent_files`] | The texts installed for the agent (skills, sub-agent definitions, advisor instructions), embedded |
//! | [`job_instruction`] | The one-line instruction of each job |
//! | [`command`] | The request kind `command`: a `trama` command run by the app, and its reply |
//! | [`jobs`]    | Jobs of the agent: kinds, the Machine `job`, admission, launch, registry record |
//! | [`process_group`] | Children that never outlive Weaver, ended as a group |
//! | [`omp`]     | omp: lookup, version, models, the Machine `omp-setup`, the system terminal |
//! | [`notification`] | System notifications (Machine `notification`): foreground test, texts, grouping, click routing |
//! | [`project_files`] | Files Weaver installs for the agent: generated Requirement skills, initialisation, sync |
//! | [`question`] | Question rounds (Machine `question-round`): request validation, the round, the queue of a project |

pub mod agent_files;
pub mod atomic;
pub mod command;
pub mod endpoint;
pub mod ipc;
pub mod job_instruction;
pub mod jobs;
pub mod lock;
pub mod notification;
pub mod omp;
pub mod paths;
pub mod process;
pub mod process_group;
pub mod project;
pub mod project_files;
pub mod protocol;
pub mod question;
pub mod runtime;
pub mod settings;
