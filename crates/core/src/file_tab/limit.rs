//! The limit of tabs and the rule of the watch of a tab's file.

/// At most this many files are open in tabs together.
pub const MAX_TABS: usize = 10;

/// A tab, as the UI describes it when asking to open another file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub path: String,
    /// Whether the tab has unsaved changes.
    pub unsaved: bool,
}

/// What opening one more file does to the tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Room {
    /// There is room.
    Fits,
    /// The tab without unsaved changes that was shown least recently closes.
    Close(String),
    /// Every tab has unsaved changes: nothing opens.
    Refused,
}

/// Makes room for one more tab. `tabs` are the open ones, the one shown least recently first.
pub fn make_room(tabs: &[TabInfo]) -> Room {
    if tabs.len() < MAX_TABS {
        return Room::Fits;
    }
    match tabs.iter().find(|tab| !tab.unsaved) {
        Some(tab) => Room::Close(tab.path.clone()),
        None => Room::Refused,
    }
}

/// What a watch event on a tab's file means for the tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskEvent {
    /// `workspace.OPEN_FILE_CHANGED`: changed on disk, or appeared again.
    Changed,
    /// `workspace.OPEN_FILE_DELETED`.
    Deleted,
}

/// The meaning of a watch event, given the SHA-256 of the file on disk now (`None` when it does
/// not exist), the one the tab holds (after a save of its own, the one it wrote) and whether the
/// tab already knows that the file is gone.
///
/// An event whose file has the hash the tab holds is not a change, which is how a tab's own
/// writes are told from the others; the only exception is a file that appears again with the very
/// content the tab holds, which still ends the `deleted` mark. `None` when nothing is to be told.
pub fn disk_event(on_disk: Option<&str>, held: &str, known_deleted: bool) -> Option<DiskEvent> {
    match (on_disk, known_deleted) {
        (None, false) => Some(DiskEvent::Deleted),
        (None, true) => None,
        (Some(_), true) => Some(DiskEvent::Changed),
        (Some(hash), false) if hash == held => None,
        (Some(_), false) => Some(DiskEvent::Changed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(path: &str, unsaved: bool) -> TabInfo {
        TabInfo {
            path: path.to_owned(),
            unsaved,
        }
    }

    fn many(count: usize, unsaved: impl Fn(usize) -> bool) -> Vec<TabInfo> {
        (0..count)
            .map(|n| tab(&format!("f{n}"), unsaved(n)))
            .collect()
    }

    #[test]
    fn there_is_room_below_ten_tabs() {
        assert_eq!(make_room(&[]), Room::Fits);
        assert_eq!(make_room(&many(9, |_| true)), Room::Fits);
    }

    #[test]
    fn the_eleventh_file_closes_the_least_recently_shown_tab_without_changes() {
        let tabs = many(10, |n| n < 3);
        assert_eq!(make_room(&tabs), Room::Close("f3".to_owned()));
        assert_eq!(
            make_room(&many(10, |_| false)),
            Room::Close("f0".to_owned())
        );
    }

    #[test]
    fn with_ten_tabs_that_all_have_changes_nothing_opens() {
        assert_eq!(make_room(&many(10, |_| true)), Room::Refused);
    }

    #[test]
    fn an_event_with_the_hash_the_tab_holds_is_not_a_change() {
        assert_eq!(disk_event(Some("h"), "h", false), None);
        assert_eq!(
            disk_event(Some("other"), "h", false),
            Some(DiskEvent::Changed)
        );
    }

    #[test]
    fn after_a_save_the_hash_written_is_the_one_held_so_the_self_write_is_ignored() {
        // The tab saved and holds the hash it wrote: the watch event of that write names it.
        let written = "w";
        assert_eq!(disk_event(Some(written), written, false), None);
    }

    #[test]
    fn a_missing_file_is_told_once() {
        assert_eq!(disk_event(None, "h", false), Some(DiskEvent::Deleted));
        assert_eq!(disk_event(None, "h", true), None);
    }

    #[test]
    fn a_file_that_appears_again_is_a_change_even_with_the_same_content() {
        assert_eq!(disk_event(Some("h"), "h", true), Some(DiskEvent::Changed));
        assert_eq!(disk_event(Some("x"), "h", true), Some(DiskEvent::Changed));
    }
}
