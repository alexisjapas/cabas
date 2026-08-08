//! The event log — what the data itself cannot remember.
//!
//! Creations and additions are attributed on the object: a list entry carries
//! `added_by`, a checked line carries `checked_by`. Deletions and edits leave
//! no such trace — once a recipe is gone there is no field left to hold "and
//! Alice deleted it" — so they are recorded here instead (DECISIONS 0024).
//!
//! The log is **capped on purpose**. It is a courtesy feature, not an audit
//! trail, and every device holds a full copy of the document: an unbounded
//! append-only list is a slow leak paid for by the smallest phone in the
//! family.

use crate::{IngredientId, ListEntryId, RecipeId, Timestamp, UserId};

/// What an event happened to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    Recipe(RecipeId),
    Ingredient(IngredientId),
    ListEntry(ListEntryId),
}

/// What happened to it.
///
/// Creation is deliberately absent: it is already attributed on the object,
/// and logging it as well would fill the cap with entries that carry nothing
/// the data does not already say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Edited,
    Deleted,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub at: Timestamp,
    pub by: UserId,
    pub action: Action,
    pub subject: Subject,
    /// The subject's name **as it was when the event happened**.
    ///
    /// Copied, not looked up. The whole point of logging a deletion is that
    /// the object is gone, so a log that resolved names by id would render
    /// precisely the entries that matter as "deleted (unknown)".
    pub label: String,
}

impl Event {
    pub fn new(
        at: Timestamp,
        by: UserId,
        action: Action,
        subject: Subject,
        label: impl Into<String>,
    ) -> Self {
        Self {
            at,
            by,
            action,
            subject,
            label: label.into(),
        }
    }
}

/// The log, oldest first.
///
/// Order is the order entries were merged in, not a global timeline: the
/// timestamps come from different devices with different clocks, so sorting
/// by `at` across replicas would imply a precision nobody has. The UI reads
/// it through [`EventLog::recent`] and shows relative times.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventLog {
    pub events: Vec<Event>,
}

impl EventLog {
    /// How many events survive. Months of family use at a handful of edits a
    /// week — far past the point where anybody scrolls.
    pub const CAP: usize = 200;

    /// Appends an event and re-applies the cap.
    pub fn record(&mut self, event: Event) {
        self.events.push(event);
        self.trim(Self::CAP);
    }

    /// Drops the oldest entries beyond `max`.
    ///
    /// A **soft bound, not an invariant**. Two devices that trim while
    /// offline both delete from the front and then merge, so the result is a
    /// coherent log of *about* `max` entries, not exactly `max`. Anything
    /// stricter would need coordination, which is the one thing a CRDT is
    /// chosen to avoid — and the exact length of a courtesy log is worth
    /// nothing. `store` re-applies this after a merge for the same reason.
    pub fn trim(&mut self, max: usize) {
        if self.events.len() > max {
            self.events.drain(..self.events.len() - max);
        }
    }

    /// Most recent first — the order the log is read in.
    pub fn recent(&self) -> impl Iterator<Item = &Event> {
        self.events.iter().rev()
    }

    /// Every event about one subject, oldest first.
    pub fn about<'a>(&'a self, subject: &'a Subject) -> impl Iterator<Item = &'a Event> {
        self.events.iter().filter(move |e| &e.subject == subject)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deletion(at: i64, label: &str) -> Event {
        Event::new(
            Timestamp(at),
            UserId::from_raw("alice"),
            Action::Deleted,
            Subject::Recipe(RecipeId::from_raw("tart")),
            label,
        )
    }

    #[test]
    fn a_deleted_subject_is_still_nameable() {
        // The property the whole `label` field exists for.
        let mut log = EventLog::default();
        log.record(deletion(1, "Apple tart"));
        let event = log.recent().next().expect("one event");
        assert_eq!(event.label, "Apple tart");
        assert_eq!(event.action, Action::Deleted);
    }

    #[test]
    fn the_cap_drops_the_oldest_and_keeps_the_newest() {
        let mut log = EventLog::default();
        for i in 0..(EventLog::CAP as i64 + 50) {
            log.record(deletion(i, &format!("recipe {i}")));
        }
        assert_eq!(log.events.len(), EventLog::CAP);
        // The 50 oldest are the ones that went.
        assert_eq!(log.events[0].at, Timestamp(50));
        assert_eq!(
            log.recent().next().expect("newest").at,
            Timestamp(EventLog::CAP as i64 + 49)
        );
    }

    #[test]
    fn trimming_an_already_short_log_changes_nothing() {
        let mut log = EventLog::default();
        log.record(deletion(1, "Apple tart"));
        log.trim(10);
        assert_eq!(log.events.len(), 1);
    }

    #[test]
    fn trimming_after_a_merge_is_idempotent() {
        // What `store` does once two replicas' logs have been merged: the
        // concatenation may exceed the cap, and re-trimming must be safe to
        // run on every merge.
        let mut log = EventLog {
            events: (0..300).map(|i| deletion(i, "x")).collect(),
        };
        log.trim(EventLog::CAP);
        assert_eq!(log.events.len(), EventLog::CAP);
        log.trim(EventLog::CAP);
        assert_eq!(log.events.len(), EventLog::CAP);
    }

    #[test]
    fn events_can_be_filtered_by_subject() {
        let mut log = EventLog::default();
        log.record(deletion(1, "Apple tart"));
        log.record(Event::new(
            Timestamp(2),
            UserId::from_raw("bob"),
            Action::Edited,
            Subject::Ingredient(IngredientId::from_raw("flour")),
            "Flour",
        ));

        let tart = Subject::Recipe(RecipeId::from_raw("tart"));
        assert_eq!(log.about(&tart).count(), 1);
        let entry = Subject::ListEntry(ListEntryId::from_raw("e1"));
        assert_eq!(log.about(&entry).count(), 0);
    }
}
