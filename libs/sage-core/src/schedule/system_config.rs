use crate::Uuid;

type Set<T> = hashbrown::HashSet<T, foldhash::fast::FixedState>;

/// A collection of constraints that a system must satisfy before/after running.
#[derive(Debug, Clone, Default)]
pub struct SystemConfig {
    /// The tags associated with the system.
    pub tags: Set<Uuid>,
    /// The system must run before any system with these tags.
    pub run_before: Set<Uuid>,
    /// The system must run after any system with these tags.
    pub run_after: Set<Uuid>,
}

impl SystemConfig {
    /// Adds a tag to the system.
    pub fn tag(mut self, tag: Uuid) -> Self {
        self.tags.insert(tag);
        self
    }

    /// Indicates that the system must run before the provided tag.
    pub fn run_before(mut self, tag: Uuid) -> Self {
        self.run_before.insert(tag);
        self
    }

    /// Indicates that the system must run after the provided tag.
    pub fn run_after(mut self, tag: Uuid) -> Self {
        self.run_after.insert(tag);
        self
    }
}
