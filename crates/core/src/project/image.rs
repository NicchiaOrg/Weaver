//! The project image: Model, Requirements, Specs, cards and diagnostics of one project,
//! held in memory.
//!
//! Analysis is incremental. Every file is kept with the SHA-256 of its normalized text, and
//! [`ProjectImage::refresh`] re-reads only the paths it is given (changed or removed); a file
//! whose hash did not change is not parsed again. The data derived from all the files (the
//! assembled Model, the Areas, the cross-file checks, the order of the cards) is recomputed in
//! full after every refresh, at a cost linear in the number of files.
//!
//! A file that cannot be read or interpreted becomes a [`Diagnostic`] with its path and the
//! reason; it is left as it is on disk, and every other file still loads.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::cards::{parse_card, Card, CardKind, CardName};
use super::changes::{Change, ChangeSet};
use super::check::{check_duplicate_identities, check_sections};
use super::diagnostic::{has_errors, Diagnostic};
use super::ids::draw_identity;
use super::layout::{
    self, classify, FileClass, DRIFTS_DIR, MODEL_DIR, PROPOSALS_DIR, TASKS_DIR, VERSIONED_ENTRIES,
};
use super::model::{
    parse_actor, parse_machine, parse_titled, Actor, Machine, Model, ModelFiles, Titled,
};
use super::order::compare;
use super::random::RandomSource;
use super::refs::Resolver;
use super::requirements::{
    assemble_areas, parse_area_doc, parse_fragment, Area, AreaDoc, Fragment,
};
use super::shortnum::{self, ShortNumbers};
use super::specs::{parse_specs, Specs};
use super::text;

/// Folders nested deeper than this are not followed (protects against symlink loops).
const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone)]
enum Parsed {
    System(Titled),
    Actor(Actor),
    Feature {
        id: String,
        titled: Titled,
    },
    Machine(Machine),
    AreaDoc {
        area: String,
        doc: AreaDoc,
    },
    Fragment {
        area: String,
        fragment: Fragment,
    },
    Specs(Specs),
    Card(Card),
    /// Unreadable, not interpretable, or in the wrong place: only diagnostics.
    Failed,
}

#[derive(Debug, Clone)]
struct Entry {
    class: FileClass,
    /// SHA-256 of the normalized text; `None` when the file could not be read.
    hash: Option<String>,
    parsed: Parsed,
    diagnostics: Vec<Diagnostic>,
}

/// Where the text of a path comes from.
enum Source {
    Disk(PathBuf),
    Text(String),
}

#[derive(Debug, Clone, Default)]
struct Derived {
    model: Model,
    areas: Vec<Area>,
    specs: Option<Specs>,
    cards: Vec<Card>,
    diagnostics: Vec<Diagnostic>,
}

/// The in-memory image of a project.
#[derive(Debug, Clone)]
pub struct ProjectImage {
    root: PathBuf,
    entries: BTreeMap<String, Entry>,
    /// Folders that could not be listed: path → reason.
    folder_issues: BTreeMap<String, String>,
    derived: Derived,
    parses: usize,
}

impl ProjectImage {
    /// An image with no file.
    pub fn empty(project_root: &Path) -> Self {
        Self {
            root: project_root.to_path_buf(),
            entries: BTreeMap::new(),
            folder_issues: BTreeMap::new(),
            derived: Derived::default(),
            parses: 0,
        }
    }

    /// Reads the whole project.
    pub fn load(project_root: &Path) -> Self {
        let mut image = Self::empty(project_root);
        image.refresh_all();
        image
    }

    /// Reads only the cards (Proposals, Tasks and Drifts), never the Model, the Requirements,
    /// the Specs or the History.
    pub fn load_cards(project_root: &Path) -> Self {
        let mut image = Self::empty(project_root);
        let weaver = layout::weaver_dir(project_root);
        let mut present = BTreeSet::new();
        for name in [PROPOSALS_DIR, TASKS_DIR, DRIFTS_DIR] {
            image.scan(&weaver.join(name), name, 0, &mut present);
        }
        image.rebuild();
        image
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Reads every file again, parsing only those whose content changed, and forgets the files
    /// that are gone.
    pub fn refresh_all(&mut self) {
        let weaver = layout::weaver_dir(&self.root);
        let mut present = BTreeSet::new();
        self.folder_issues.clear();
        for name in VERSIONED_ENTRIES {
            self.scan(&weaver.join(name), name, 0, &mut present);
        }
        self.entries.retain(|rel, _| present.contains(rel));
        self.rebuild();
    }

    /// Re-reads only the given paths (absolute, inside `.weaver/`): a file is read again, a
    /// removed file or folder is forgotten, a folder is scanned again. Paths outside `.weaver/`
    /// and hidden ones are ignored.
    pub fn refresh(&mut self, paths: &[PathBuf]) {
        let mut touched = false;
        for path in paths {
            let Some(rel) = layout::rel_key(&self.root, path) else {
                continue;
            };
            if layout::is_hidden(&rel) {
                continue;
            }
            touched = true;
            let prefix = format!("{rel}/");
            match fs::metadata(path) {
                Ok(meta) if meta.is_dir() => {
                    let mut present = BTreeSet::new();
                    self.folder_issues
                        .retain(|p, _| p != &rel && !p.starts_with(&prefix));
                    self.scan(path, &rel, 0, &mut present);
                    self.entries
                        .retain(|p, _| !p.starts_with(&prefix) || present.contains(p));
                }
                Ok(_) => {
                    self.entries.retain(|p, _| !p.starts_with(&prefix));
                    self.install(&rel, Source::Disk(path.clone()));
                }
                Err(_) => {
                    self.entries.remove(&rel);
                    self.entries.retain(|p, _| !p.starts_with(&prefix));
                    self.folder_issues
                        .retain(|p, _| p != &rel && !p.starts_with(&prefix));
                }
            }
        }
        if touched {
            self.rebuild();
        }
    }

    /// The image the project would have after `changes`, built without touching the disk.
    pub fn with_changes(&self, changes: &ChangeSet) -> ProjectImage {
        let mut after = self.clone();
        for (rel, change) in changes.iter() {
            if layout::is_hidden(rel) {
                continue;
            }
            match change {
                Change::Write(content) => {
                    after.install(rel, Source::Text(text::file_content(content)))
                }
                Change::Delete => {
                    after.entries.remove(rel);
                }
            }
        }
        after.rebuild();
        after
    }

    /// Lists `dir` (the folder `rel`), installing every file found and noting the unreadable
    /// folders.
    fn scan(&mut self, dir: &Path, rel: &str, depth: usize, present: &mut BTreeSet<String>) {
        let meta = match fs::metadata(dir) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                self.folder_issues
                    .insert(rel.to_owned(), format!("cannot read the folder: {e}"));
                return;
            }
        };
        if !meta.is_dir() {
            if matches!(classify(rel), FileClass::Ignored) {
                return;
            }
            present.insert(rel.to_owned());
            self.install(rel, Source::Disk(dir.to_path_buf()));
            return;
        }
        if depth > MAX_DEPTH {
            return;
        }
        let listing = match fs::read_dir(dir) {
            Ok(listing) => listing,
            Err(e) => {
                self.folder_issues
                    .insert(rel.to_owned(), format!("cannot read the folder: {e}"));
                return;
            }
        };
        let mut names: Vec<String> = Vec::new();
        for item in listing {
            match item {
                Ok(item) => {
                    let name = item.file_name().to_string_lossy().into_owned();
                    if !name.starts_with('.') {
                        names.push(name);
                    }
                }
                Err(e) => {
                    self.folder_issues
                        .insert(rel.to_owned(), format!("cannot read the folder: {e}"));
                }
            }
        }
        names.sort();
        for name in names {
            let child_rel = format!("{rel}/{name}");
            self.scan(&dir.join(&name), &child_rel, depth + 1, present);
        }
    }

    /// Reads (or takes) the text of `rel` and analyses it, unless its hash did not change.
    fn install(&mut self, rel: &str, source: Source) {
        let class = classify(rel);
        match &class {
            FileClass::Ignored => {
                self.entries.remove(rel);
                return;
            }
            FileClass::Misplaced(reason) => {
                self.entries.insert(
                    rel.to_owned(),
                    Entry {
                        class: class.clone(),
                        hash: None,
                        parsed: Parsed::Failed,
                        diagnostics: vec![Diagnostic::error(rel, reason.clone())],
                    },
                );
                return;
            }
            _ => {}
        }
        let text = match source {
            Source::Text(text) => text,
            Source::Disk(path) => match text::read_text(&path) {
                Ok(text) => text,
                Err(e) => {
                    self.entries.insert(
                        rel.to_owned(),
                        Entry {
                            class,
                            hash: None,
                            parsed: Parsed::Failed,
                            diagnostics: vec![Diagnostic::error(
                                rel,
                                format!("cannot read the file: {e}"),
                            )],
                        },
                    );
                    return;
                }
            },
        };
        let hash = text::sha256_hex(&text);
        if let Some(existing) = self.entries.get(rel) {
            if existing.hash.as_deref() == Some(hash.as_str()) && existing.class == class {
                return;
            }
        }
        self.parses += 1;
        let (parsed, errors) = analyse(rel, &class, &text);
        let diagnostics = errors
            .into_iter()
            .map(|m| Diagnostic::error(rel, m))
            .collect();
        self.entries.insert(
            rel.to_owned(),
            Entry {
                class,
                hash: Some(hash),
                parsed,
                diagnostics,
            },
        );
    }

    /// Recomputes everything derived from the files.
    fn rebuild(&mut self) {
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        let mut system = None;
        let mut actors = Vec::new();
        let mut features = Vec::new();
        let mut machines = Vec::new();
        let mut area_docs = Vec::new();
        let mut fragments = Vec::new();
        let mut specs = None;
        let mut cards: Vec<Card> = Vec::new();
        for (path, entry) in &self.entries {
            diagnostics.extend(entry.diagnostics.iter().cloned());
            match &entry.parsed {
                Parsed::System(t) => system = Some((path.as_str(), t)),
                Parsed::Actor(a) => actors.push((path.as_str(), a)),
                Parsed::Feature { id, titled } => {
                    features.push((path.as_str(), id.as_str(), titled))
                }
                Parsed::Machine(m) => machines.push((path.as_str(), m)),
                Parsed::AreaDoc { area, doc } => {
                    area_docs.push((path.as_str(), area.as_str(), doc))
                }
                Parsed::Fragment { area, fragment } => {
                    fragments.push((path.as_str(), area.as_str(), fragment))
                }
                Parsed::Specs(s) => specs = Some(s.clone()),
                Parsed::Card(c) => cards.push(c.clone()),
                Parsed::Failed => {}
            }
        }
        for (path, issue) in &self.folder_issues {
            diagnostics.push(Diagnostic::error(path, issue.clone()));
        }

        // A Machine needs the `feature.md` of its folder.
        let feature_files: BTreeSet<&str> = self
            .entries
            .values()
            .filter_map(|e| match &e.class {
                FileClass::Feature { id } => Some(id.as_str()),
                _ => None,
            })
            .collect();
        for (path, machine) in &machines {
            if !feature_files.contains(machine.feature.as_str()) {
                diagnostics.push(Diagnostic::error(
                    *path,
                    format!("the folder `{MODEL_DIR}/{}` has no `feature.md`: every Machine belongs to a Feature", machine.feature),
                ));
            }
        }

        let (model, model_diagnostics) = Model::assemble(&ModelFiles {
            system,
            actors,
            features,
            machines,
        });
        diagnostics.extend(model_diagnostics);
        let (areas, area_diagnostics) = assemble_areas(&area_docs, &fragments);
        diagnostics.extend(area_diagnostics);
        diagnostics.extend(check_sections(&model, &areas, specs.as_ref()));
        diagnostics.extend(check_duplicate_identities(
            &cards.iter().collect::<Vec<_>>(),
        ));

        cards.sort_by(|a, b| {
            a.kind.cmp(&b.kind).then_with(|| {
                compare(
                    a.created.as_deref().unwrap_or(""),
                    &a.id,
                    b.created.as_deref().unwrap_or(""),
                    &b.id,
                )
            })
        });
        diagnostics.sort();
        diagnostics.dedup();
        self.derived = Derived {
            model,
            areas,
            specs,
            cards,
            diagnostics,
        };
    }

    // --- what the image holds ---

    pub fn model(&self) -> &Model {
        &self.derived.model
    }

    /// The Requirement Areas, by id.
    pub fn areas(&self) -> &[Area] {
        &self.derived.areas
    }

    pub fn specs(&self) -> Option<&Specs> {
        self.derived.specs.as_ref()
    }

    /// Every card: by kind, each kind in creation order (oldest first, ties by identity).
    pub fn cards(&self) -> &[Card] {
        &self.derived.cards
    }

    pub fn cards_of(&self, kind: CardKind) -> impl Iterator<Item = &Card> {
        self.derived.cards.iter().filter(move |c| c.kind == kind)
    }

    /// The card of `kind` with identity `id` (for a Task of a Proposal, any Proposal's).
    pub fn card(&self, kind: CardKind, id: &str) -> Option<&Card> {
        self.cards_of(kind).find(|c| c.id == id)
    }

    /// Every diagnostic: unreadable and misinterpreted files, and the rules the files break
    /// together; sorted by path.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.derived.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        has_errors(&self.derived.diagnostics)
    }

    /// Looks references up in this image.
    pub fn resolver(&self) -> Resolver<'_> {
        Resolver {
            model: &self.derived.model,
            areas: &self.derived.areas,
            specs: self.derived.specs.as_ref(),
        }
    }

    /// The SHA-256 (lowercase hex) of the normalized content of a read file.
    pub fn file_hash(&self, rel: &str) -> Option<&str> {
        self.entries.get(rel).and_then(|e| e.hash.as_deref())
    }

    /// Paths of the files the image holds (including the ones with diagnostics).
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// How many files were parsed since this image was created; a refresh of a file whose
    /// content did not change does not add to it.
    pub fn parse_count(&self) -> usize {
        self.parses
    }

    /// The identities in use for cards of `kind`, from file and folder names, so that a card
    /// that failed to load still keeps its identity. For Tasks (and Proposal Tasks) this is
    /// `.weaver/tasks/`, the `cleared` copies in `history/` and the `tasks/` folders of every
    /// Proposal.
    pub fn used_ids(&self, kind: CardKind) -> BTreeSet<String> {
        let tasks = matches!(kind, CardKind::Task | CardKind::ProposalTask);
        let mut ids = BTreeSet::new();
        for entry in self.entries.values() {
            if let FileClass::Card { kind: k, id, .. } = &entry.class {
                let same = if tasks {
                    matches!(k, CardKind::Task | CardKind::ProposalTask)
                } else {
                    *k == kind
                };
                if same {
                    ids.insert(id.clone());
                }
            }
            if tasks {
                if let Parsed::Card(card) = &entry.parsed {
                    if card.kind == CardKind::History {
                        ids.extend(card.cleared_task_ids());
                    }
                }
            }
        }
        ids
    }

    /// Draws an identity for a new card of `kind`, not used by any card of its type.
    pub fn new_identity(&self, kind: CardKind, rng: &mut dyn RandomSource) -> String {
        let used = self.used_ids(kind);
        draw_identity(rng, &|id| used.contains(id))
    }

    /// The short numbers of the live cards, rebuilding the local cache when it is wrong.
    pub fn short_numbers(&self) -> ShortNumbers {
        shortnum::load(&self.root, &self.derived.cards)
    }
}

/// Parses the text of one file according to what it is; the errors are the reasons it failed.
fn analyse(rel: &str, class: &FileClass, text: &str) -> (Parsed, Vec<String>) {
    fn done<T>(
        result: Result<T, Vec<String>>,
        wrap: impl FnOnce(T) -> Parsed,
    ) -> (Parsed, Vec<String>) {
        match result {
            Ok(value) => (wrap(value), Vec::new()),
            Err(errors) => (Parsed::Failed, errors),
        }
    }
    match class {
        FileClass::System => done(parse_titled(text), Parsed::System),
        FileClass::Actor { id } => done(parse_actor(id, text), Parsed::Actor),
        FileClass::Feature { id } => done(parse_titled(text), |titled| Parsed::Feature {
            id: id.clone(),
            titled,
        }),
        FileClass::Machine { feature, id } => {
            done(parse_machine(feature, id, text), Parsed::Machine)
        }
        FileClass::AreaDoc { area } => done(parse_area_doc(text), |doc| Parsed::AreaDoc {
            area: area.clone(),
            doc,
        }),
        FileClass::Fragment { area, name } => {
            done(parse_fragment(name, text), |fragment| Parsed::Fragment {
                area: area.clone(),
                fragment,
            })
        }
        FileClass::Specs => done(parse_specs(text), Parsed::Specs),
        FileClass::Card {
            kind,
            id,
            slug,
            parent,
        } => {
            let name = CardName {
                kind: *kind,
                id,
                slug,
                parent: parent.as_deref(),
                path: rel,
            };
            done(parse_card(name, text), Parsed::Card)
        }
        FileClass::Ignored | FileClass::Misplaced(_) => (Parsed::Failed, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{ORDER_MACHINE, USER_ACTOR};
    use crate::project::random::ScriptedRandom;
    use crate::project::testing::{put, TASK};

    fn sample(root: &Path) {
        put(root, "model/system.md", "# Shop\n\nA shop.\n");
        put(root, "model/shop/feature.md", "# Shopping\n");
        put(root, "model/shop/order.md", ORDER_MACHINE);
        put(root, "model/actors/user.md", USER_ACTOR);
        put(
            root,
            "requirements/ui/area.md",
            "---\ndescription: d\n---\n# UI\n",
        );
        put(
            root,
            "requirements/ui/main.md",
            "---\ndescription: d\n---\n# Main\n\n## General\n- x\n\n## order/Paid\n- y\n",
        );
        put(root, "specs.yaml", "languages:\n  - name: Rust\n");
        put(root, "tasks/vxdhoylk-a-task.md", TASK);
    }

    #[test]
    fn loads_every_part_of_a_project() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let image = ProjectImage::load(dir.path());
        assert_eq!(image.model().system.as_ref().unwrap().title, "Shop");
        assert!(image.model().machines.contains_key("order"));
        assert!(image.model().actors.contains_key("user"));
        assert!(image.model().features.contains_key("shop"));
        assert_eq!(image.areas().len(), 1);
        assert_eq!(image.areas()[0].fragments[0].sections.len(), 2);
        assert!(image.specs().unwrap().has_field("languages"));
        let task = image.card(CardKind::Task, "vxdhoylk").unwrap();
        assert_eq!(task.title, "A task");
        assert_eq!(task.list_key("refs"), Some(vec!["order".to_owned()]));
        assert!(image
            .resolver()
            .resolve_text("order/Paid/user.CANCEL")
            .is_ok());
        // The Machine's other owners (cart, payment, ...) are not declared: that is reported.
        assert!(
            image
                .diagnostics()
                .iter()
                .all(|d| d.path == "model/shop/order.md"),
            "{:?}",
            image.diagnostics()
        );
        assert!(image.has_errors());
    }

    #[test]
    fn hidden_and_local_files_are_never_read() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        put(dir.path(), "tasks/.gitkeep", "");
        put(dir.path(), "model/.draft.md", "garbage");
        put(dir.path(), "model/shop/.hidden.md", "garbage");
        put(dir.path(), "requirements/ui/.old.md", "garbage");
        put(dir.path(), "requests/2026.md", "garbage");
        put(dir.path(), "short-numbers/assigned", "garbage");
        let image = ProjectImage::load(dir.path());
        assert!(image.paths().all(|p| !p.contains("/.")
            && !p.starts_with("requests")
            && !p.starts_with("short-numbers")));
        assert!(image
            .diagnostics()
            .iter()
            .all(|d| d.path == "model/shop/order.md"));
    }

    #[test]
    fn an_unreadable_file_is_reported_and_the_others_load() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let bad = put(dir.path(), "tasks/zzzzzzzz-bad.md", "");
        fs::write(&bad, [0x23, 0x20, 0xFF, 0xFE]).unwrap();
        put(
            dir.path(),
            "tasks/yyyyyyyy-broken.md",
            "---\ncolumn: [\n---\n# T\n",
        );
        put(dir.path(), "tasks/notacard.md", "# x\n");
        put(
            dir.path(),
            "model/shop/broken.md",
            "# Broken\n\n## States\n\n### A\n",
        );
        let before = fs::read(&bad).unwrap();
        let image = ProjectImage::load(dir.path());

        let by_path = |p: &str| {
            image
                .diagnostics()
                .iter()
                .filter(|d| d.path == p)
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        };
        assert!(by_path("tasks/zzzzzzzz-bad.md")[0].contains("not valid UTF-8"));
        assert!(by_path("tasks/yyyyyyyy-broken.md")[0].contains("invalid front matter"));
        assert!(by_path("tasks/notacard.md")[0].contains("<id>-<slug>.md"));
        assert!(by_path("model/shop/broken.md")[0].contains("exactly one"));
        // The good ones are loaded...
        assert!(image.card(CardKind::Task, "vxdhoylk").is_some());
        assert!(image.model().machines.contains_key("order"));
        assert!(!image.model().machines.contains_key("broken"));
        // ...the broken ones are neither dropped from the disk nor rewritten.
        assert_eq!(fs::read(&bad).unwrap(), before);
        // A broken card still keeps its identity.
        assert!(image.used_ids(CardKind::Task).contains("zzzzzzzz"));
        assert!(image.used_ids(CardKind::Task).contains("yyyyyyyy"));
    }

    #[test]
    fn bom_and_crlf_files_read_like_clean_ones() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let crlf = TASK.replace('\n', "\r\n");
        let path = put(
            dir.path(),
            "tasks/crlfcrlf-x.md",
            &format!("\u{feff}{crlf}"),
        );
        let image = ProjectImage::load(dir.path());
        let card = image.card(CardKind::Task, "crlfcrlf").unwrap();
        assert_eq!(card.title, "A task");
        assert!(!card.body.contains('\r'));
        // Same hash as the clean text: hashing happens after normalizing.
        let clean = put(dir.path(), "tasks/lfonlyxx-x.md", TASK);
        let image = ProjectImage::load(dir.path());
        assert_eq!(
            image.file_hash("tasks/crlfcrlf-x.md"),
            image.file_hash("tasks/lfonlyxx-x.md")
        );
        assert!(path.exists() && clean.exists());
    }

    #[test]
    fn a_project_in_a_folder_with_spaces_and_non_ascii_characters() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("il mio progetto — così");
        sample(&root);
        put(
            &root,
            "model/shop/ordine così.md",
            "# Ordine\n\n## States\n\n### Bozza (initial)\n",
        );
        let image = ProjectImage::load(&root);
        // The id `ordine così` is not kebab-case: reported with its path, not dropped silently.
        assert!(
            image
                .diagnostics()
                .iter()
                .any(|d| d.path == "model/shop/ordine così.md"
                    && d.message.contains("not a valid id"))
        );
        assert!(image.card(CardKind::Task, "vxdhoylk").is_some());
    }

    #[test]
    fn refresh_parses_only_what_changed() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let mut image = ProjectImage::load(dir.path());
        let loaded = image.parse_count();
        assert_eq!(loaded, 8);

        // Nothing changed: no parse.
        let task_path = layout::absolute(dir.path(), "tasks/vxdhoylk-a-task.md");
        image.refresh(std::slice::from_ref(&task_path));
        image.refresh_all();
        assert_eq!(image.parse_count(), loaded);

        // One file changed: one parse, and the derived data follow.
        fs::write(&task_path, TASK.replace("column: todo", "column: doing")).unwrap();
        image.refresh(std::slice::from_ref(&task_path));
        assert_eq!(image.parse_count(), loaded + 1);
        assert_eq!(
            image
                .card(CardKind::Task, "vxdhoylk")
                .unwrap()
                .str_key("column"),
            Some("doing")
        );

        // A file appears: picked up by refreshing its folder.
        put(dir.path(), "tasks/aaaaaaaa-new.md", TASK);
        image.refresh(&[layout::weaver_dir(dir.path()).join("tasks")]);
        assert_eq!(image.parse_count(), loaded + 2);
        assert_eq!(image.cards_of(CardKind::Task).count(), 2);

        // A file is removed, then a whole folder.
        fs::remove_file(layout::absolute(dir.path(), "tasks/aaaaaaaa-new.md")).unwrap();
        image.refresh(&[layout::absolute(dir.path(), "tasks/aaaaaaaa-new.md")]);
        assert_eq!(image.cards_of(CardKind::Task).count(), 1);
        fs::remove_dir_all(layout::absolute(dir.path(), "requirements")).unwrap();
        image.refresh(&[layout::absolute(dir.path(), "requirements")]);
        assert!(image.areas().is_empty());
        assert_eq!(image.parse_count(), loaded + 2);

        // Paths outside `.weaver/` and hidden ones are ignored.
        image.refresh(&[
            dir.path().join("elsewhere.md"),
            layout::absolute(dir.path(), "tasks/.gitkeep"),
        ]);
        assert_eq!(image.parse_count(), loaded + 2);
    }

    #[test]
    fn a_file_that_becomes_unreadable_is_reported_and_back_when_fixed() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let mut image = ProjectImage::load(dir.path());
        let path = layout::absolute(dir.path(), "tasks/vxdhoylk-a-task.md");
        fs::write(&path, "---\nbroken: [\n---\n# x\n").unwrap();
        image.refresh(std::slice::from_ref(&path));
        assert!(image.card(CardKind::Task, "vxdhoylk").is_none());
        assert!(image
            .diagnostics()
            .iter()
            .any(|d| d.path == "tasks/vxdhoylk-a-task.md"));
        fs::write(&path, TASK).unwrap();
        image.refresh(&[path]);
        assert!(image.card(CardKind::Task, "vxdhoylk").is_some());
        assert!(!image
            .diagnostics()
            .iter()
            .any(|d| d.path == "tasks/vxdhoylk-a-task.md"));
    }

    #[test]
    fn cross_file_rules_are_recomputed_after_a_refresh() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let mut image = ProjectImage::load(dir.path());
        assert!(image
            .diagnostics()
            .iter()
            .all(|d| d.path != "requirements/ui/main.md"));
        // Removing the Machine makes the section `order/Paid` point at nothing.
        let machine = layout::absolute(dir.path(), "model/shop/order.md");
        fs::remove_file(&machine).unwrap();
        image.refresh(&[machine]);
        assert!(image
            .diagnostics()
            .iter()
            .any(|d| d.path == "requirements/ui/main.md" && d.message.contains("order/Paid")));
    }

    #[test]
    fn a_machine_needs_its_features_file() {
        let dir = tempfile::tempdir().unwrap();
        put(
            dir.path(),
            "model/lonely/m.md",
            "# M\n\n## States\n\n### A (initial)\n",
        );
        let image = ProjectImage::load(dir.path());
        assert!(image
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("has no `feature.md`")));
    }

    #[test]
    fn with_changes_shows_the_state_after_without_touching_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        sample(dir.path());
        let image = ProjectImage::load(dir.path());
        let mut changes = ChangeSet::new();
        changes.write("tasks/aaaaaaaa-new.md", TASK.replace('\n', "\r\n"));
        changes.delete("tasks/vxdhoylk-a-task.md");
        let after = image.with_changes(&changes);
        assert!(after.card(CardKind::Task, "aaaaaaaa").is_some());
        assert!(after.card(CardKind::Task, "vxdhoylk").is_none());
        assert!(image.card(CardKind::Task, "vxdhoylk").is_some());
        assert!(!layout::absolute(dir.path(), "tasks/aaaaaaaa-new.md").exists());
        // What is hashed is what will be on disk: LF with the final newline.
        assert_eq!(
            after.file_hash("tasks/aaaaaaaa-new.md").unwrap(),
            text::sha256_hex(&text::file_content(TASK))
        );
    }

    #[test]
    fn duplicate_identities_of_a_type_are_errors() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "tasks/vxdhoylk-one.md", TASK);
        put(dir.path(), "tasks/vxdhoylk-two.md", TASK);
        let image = ProjectImage::load(dir.path());
        assert_eq!(
            image
                .diagnostics()
                .iter()
                .filter(|d| d.message.contains("identity `vxdhoylk`"))
                .count(),
            2
        );
    }

    #[test]
    fn task_identities_are_unique_across_tasks_history_copies_and_proposal_folders() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "tasks/aaaaaaaa-live.md", TASK);
        put(
            dir.path(),
            "history/hhhhhhhh-entry.md",
            "---\ncreated: 2026-11-02T07:00:01.000Z\nupdated: 2026-11-02T07:00:01.000Z\nsummary: s\ntasks:\n  - bbbbbbbb\ncleared:\n  bbbbbbbb:\n    title: T\n---\n# H\n",
        );
        put(
            dir.path(),
            "proposals/pppppppp-x/proposal.md",
            "---\nstatus: open\ncreated: 2026-11-02T07:00:01.000Z\n---\n# P\n",
        );
        put(
            dir.path(),
            "proposals/pppppppp-x/tasks/cccccccc-t.md",
            "---\ntechnical: true\n---\n# T\n",
        );
        put(
            dir.path(),
            "drifts/dddddddd-d.md",
            "---\nstatus: open\ncreated: 2026-11-02T07:00:01.000Z\n---\n# D\n",
        );
        let image = ProjectImage::load(dir.path());
        let tasks = image.used_ids(CardKind::Task);
        assert_eq!(
            tasks.iter().map(String::as_str).collect::<Vec<_>>(),
            ["aaaaaaaa", "bbbbbbbb", "cccccccc"]
        );
        assert_eq!(image.used_ids(CardKind::ProposalTask), tasks);
        assert_eq!(image.used_ids(CardKind::Drift).len(), 1);
        assert_eq!(image.used_ids(CardKind::Proposal).len(), 1);
        assert_eq!(image.used_ids(CardKind::History).len(), 1);

        // Drawing again on collision with any of those.
        let mut rng = ScriptedRandom::new([
            0, 0, 0, 0, 0, 0, 0, 0, // aaaaaaaa: a live Task
            1, 1, 1, 1, 1, 1, 1, 1, // bbbbbbbb: a cleared copy
            2, 2, 2, 2, 2, 2, 2, 2, // cccccccc: a Task of a Proposal
            3, 3, 3, 3, 3, 3, 3, 3, // dddddddd: free for a Task
        ]);
        assert_eq!(image.new_identity(CardKind::Task, &mut rng), "dddddddd");
        // The same identity is free for a Drift only when no Drift uses it.
        let mut rng = ScriptedRandom::new([3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4]);
        assert_eq!(image.new_identity(CardKind::Drift, &mut rng), "eeeeeeee");
    }

    #[test]
    fn cards_come_in_creation_order() {
        let dir = tempfile::tempdir().unwrap();
        for (id, created) in [("bbbbbbbb", "02"), ("aaaaaaaa", "02"), ("cccccccc", "01")] {
            put(
                dir.path(),
                &format!("tasks/{id}-x.md"),
                &TASK.replace("07:00:01", &format!("07:00:{created}")),
            );
        }
        let image = ProjectImage::load(dir.path());
        let ids: Vec<_> = image
            .cards_of(CardKind::Task)
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(ids, ["cccccccc", "aaaaaaaa", "bbbbbbbb"]);
    }

    #[test]
    fn short_numbers_come_from_the_image() {
        let dir = tempfile::tempdir().unwrap();
        for (id, created) in [("bbbbbbbb", "02"), ("aaaaaaaa", "03")] {
            put(
                dir.path(),
                &format!("tasks/{id}-x.md"),
                &TASK.replace("07:00:01", &format!("07:00:{created}")),
            );
        }
        let image = ProjectImage::load(dir.path());
        let numbers = image.short_numbers();
        assert_eq!(
            numbers.label_of(CardKind::Task, "bbbbbbbb").as_deref(),
            Some("T1")
        );
        assert_eq!(
            numbers.label_of(CardKind::Task, "aaaaaaaa").as_deref(),
            Some("T2")
        );
        // The cache is local-only and ignored by git; the image never reads it as a card.
        let ignore = layout::weaver_dir(dir.path()).join("short-numbers/.gitignore");
        assert_eq!(fs::read(ignore).unwrap(), b"*\n");
    }
}
