//! The events of the File view's tabs and of the agent lock: `file-tab-changed` and
//! `file-lock-changed`. Each carries the project id and a generation number that only grows per
//! project, shared by both events, so that the UI sees what came after what.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{AppHandle, Emitter};

use super::wire::{FileLockChanged, FileTabChanged, TabChange};
use crate::events::EventName;

/// Where the events go.
pub trait EventSink: Send + Sync {
    fn tab_changed(&self, event: &FileTabChanged);
    fn lock_changed(&self, event: &FileLockChanged);
}

/// The events to the windows of the app.
pub struct AppSink(pub AppHandle);

impl EventSink for AppSink {
    fn tab_changed(&self, event: &FileTabChanged) {
        let _ = self.0.emit(EventName::FileTabChanged.as_str(), event);
    }

    fn lock_changed(&self, event: &FileLockChanged) {
        let _ = self.0.emit(EventName::FileLockChanged.as_str(), event);
    }
}

/// Sends the events of the tabs and of the lock, once a sink is attached. The services exist
/// before the app handle does, so events sent earlier are dropped: nothing is open then.
#[derive(Default)]
pub struct FileEvents {
    sink: OnceLock<Arc<dyn EventSink>>,
    generations: Mutex<HashMap<String, u64>>,
}

impl FileEvents {
    /// Attaches the sink; only the first one counts.
    pub fn attach(&self, sink: Arc<dyn EventSink>) {
        let _ = self.sink.set(sink);
    }

    fn next(&self, project_id: &str) -> u64 {
        let mut generations = self.generations.lock().expect("generations poisoned");
        let generation = generations.entry(project_id.to_owned()).or_insert(0);
        *generation += 1;
        *generation
    }

    /// Forgets the counter of a project whose window closed.
    pub fn forget(&self, project_id: &str) {
        self.generations
            .lock()
            .expect("generations poisoned")
            .remove(project_id);
    }

    pub fn tab_changed(&self, project_id: &str, path: &str, change: TabChange) {
        let Some(sink) = self.sink.get() else { return };
        sink.tab_changed(&FileTabChanged {
            project_id: project_id.to_owned(),
            generation: self.next(project_id),
            path: path.to_owned(),
            change,
        });
    }

    pub fn lock_changed(&self, project_id: &str, path: &str, locked: bool, moved_to: Option<&str>) {
        let Some(sink) = self.sink.get() else { return };
        sink.lock_changed(&FileLockChanged {
            project_id: project_id.to_owned(),
            generation: self.next(project_id),
            path: path.to_owned(),
            locked,
            moved_to: moved_to.map(str::to_owned),
        });
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;

    /// A sink that keeps what it was sent.
    #[derive(Default)]
    pub struct Recorder {
        pub tabs: Mutex<Vec<FileTabChanged>>,
        pub locks: Mutex<Vec<FileLockChanged>>,
    }

    impl EventSink for Recorder {
        fn tab_changed(&self, event: &FileTabChanged) {
            self.tabs.lock().unwrap().push(event.clone());
        }

        fn lock_changed(&self, event: &FileLockChanged) {
            self.locks.lock().unwrap().push(event.clone());
        }
    }

    /// Events that go to a [`Recorder`].
    pub fn recording() -> (Arc<FileEvents>, Arc<Recorder>) {
        let events = Arc::new(FileEvents::default());
        let recorder = Arc::new(Recorder::default());
        events.attach(recorder.clone());
        (events, recorder)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::recording;
    use super::*;

    #[test]
    fn the_generation_grows_per_project_across_both_events() {
        let (events, recorder) = recording();
        events.tab_changed("p1", "a.rs", TabChange::Changed);
        events.lock_changed("p1", "a.rs", true, None);
        events.tab_changed("p2", "b.rs", TabChange::Deleted);
        events.lock_changed("p1", "a.rs", false, Some("c.rs"));
        let tabs = recorder.tabs.lock().unwrap();
        let locks = recorder.locks.lock().unwrap();
        assert_eq!(tabs[0].generation, 1);
        assert_eq!(locks[0].generation, 2);
        assert_eq!(tabs[1].generation, 1, "another project has its own counter");
        assert_eq!(locks[1].generation, 3);
        assert_eq!(locks[1].moved_to.as_deref(), Some("c.rs"));
        assert_eq!(locks[1].project_id, "p1");
    }

    #[test]
    fn the_events_are_named_for_the_ui() {
        assert_eq!(EventName::FileTabChanged.as_str(), "file-tab-changed");
        assert_eq!(EventName::FileLockChanged.as_str(), "file-lock-changed");
    }
}
