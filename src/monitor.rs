//! Pure decision logic for the clipboard poller.
//!
//! macOS exposes only an ever-increasing `changeCount` for the pasteboard, so
//! the poller compares it with what it saw last time and must also recognise
//! changes it caused itself (when pasting a history entry).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    /// Nothing changed since the last poll.
    Unchanged,
    /// Changed, but the change must not be recorded (own write or paused).
    Ignored,
    /// A new external copy that should be captured.
    Capture,
}

#[derive(Debug)]
pub struct ChangeTracker {
    last_seen: isize,
    own_change: Option<isize>,
    paused: bool,
}

impl ChangeTracker {
    /// `last_seen` starts at -1 so content already on the clipboard at launch
    /// is captured once.
    pub fn new() -> Self {
        Self {
            last_seen: -1,
            own_change: None,
            paused: false,
        }
    }

    pub fn observe(&mut self, change_count: isize) -> Observation {
        if change_count == self.last_seen {
            return Observation::Unchanged;
        }
        self.last_seen = change_count;
        if self.own_change == Some(change_count) || self.paused {
            return Observation::Ignored;
        }
        Observation::Capture
    }

    /// Record the change count produced by MacClip's own pasteboard write.
    pub fn mark_own_write(&mut self, change_count: isize) {
        self.own_change = Some(change_count);
        self.last_seen = change_count;
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_observation_captures_existing_content() {
        let mut t = ChangeTracker::new();
        assert_eq!(t.observe(7), Observation::Capture);
        assert_eq!(t.observe(7), Observation::Unchanged);
    }

    #[test]
    fn external_change_is_captured() {
        let mut t = ChangeTracker::new();
        t.observe(1);
        assert_eq!(t.observe(2), Observation::Capture);
    }

    #[test]
    fn own_write_is_never_captured() {
        let mut t = ChangeTracker::new();
        t.observe(1);
        t.mark_own_write(5);
        assert_eq!(t.observe(5), Observation::Unchanged);
        assert_eq!(
            t.observe(6),
            Observation::Capture,
            "later external copies still work"
        );
    }

    #[test]
    fn paused_changes_are_ignored_and_not_replayed_on_resume() {
        let mut t = ChangeTracker::new();
        t.observe(1);
        t.set_paused(true);
        assert_eq!(t.observe(2), Observation::Ignored);
        t.set_paused(false);
        assert_eq!(
            t.observe(2),
            Observation::Unchanged,
            "copy made while paused is not captured later"
        );
        assert_eq!(t.observe(3), Observation::Capture);
    }
}
