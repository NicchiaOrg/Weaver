//! When the installed files are checked: at most one check per project at a time, and requests
//! made meanwhile coalesce into one more run (`requirements/tech#project`).
//!
//! [`Runs`] is the pure rule; the app owns one per open project and starts the work it names.

/// What a run looks at, from the least to the most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Want {
    /// The Requirement skills only (an approval, a change of the Requirements).
    RequirementSkills,
    /// Every installed file and the Requirement skills (the opening check).
    OpeningCheck,
    /// The opening check and the starting Areas when the Requirements are missing (`Update skills`).
    UpdateSkills,
}

impl Want {
    /// The one run that covers both.
    fn merged(self, other: Want) -> Want {
        self.max(other)
    }
}

/// The runs of one project.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Runs {
    running: bool,
    queued: Option<Want>,
}

impl Runs {
    /// A run is asked for. `Some(want)` says it starts now; `None` says one is running and this
    /// request will be served by one more run when it ends.
    pub fn request(&mut self, want: Want) -> Option<Want> {
        if self.running {
            self.queued = Some(self.queued.map_or(want, |queued| queued.merged(want)));
            None
        } else {
            self.running = true;
            Some(want)
        }
    }

    /// The running one ended. `Some(want)` is the one run that serves every request made meanwhile
    /// and starts now.
    pub fn finished(&mut self) -> Option<Want> {
        match self.queued.take() {
            Some(next) => Some(next),
            None => {
                self.running = false;
                None
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_with_nothing_running_starts_at_once() {
        let mut runs = Runs::default();
        assert_eq!(runs.request(Want::OpeningCheck), Some(Want::OpeningCheck));
        assert!(runs.is_running());
        assert_eq!(runs.finished(), None);
        assert!(!runs.is_running());
    }

    #[test]
    fn requests_made_while_one_runs_coalesce_into_one_more_run_of_the_widest_kind() {
        let mut runs = Runs::default();
        assert_eq!(
            runs.request(Want::RequirementSkills),
            Some(Want::RequirementSkills)
        );
        assert_eq!(runs.request(Want::RequirementSkills), None);
        assert_eq!(runs.request(Want::UpdateSkills), None);
        assert_eq!(runs.request(Want::OpeningCheck), None);
        assert_eq!(runs.request(Want::RequirementSkills), None);
        assert_eq!(runs.finished(), Some(Want::UpdateSkills));
        // The extra run is the one running now; nothing else is waiting.
        assert!(runs.is_running());
        assert_eq!(runs.finished(), None);
        assert_eq!(
            runs.request(Want::RequirementSkills),
            Some(Want::RequirementSkills)
        );
    }

    #[test]
    fn the_runs_of_two_projects_never_meet() {
        let (mut one, mut two) = (Runs::default(), Runs::default());
        assert!(one.request(Want::OpeningCheck).is_some());
        assert!(two.request(Want::OpeningCheck).is_some());
    }
}
