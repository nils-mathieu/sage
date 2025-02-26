use crate::{
    app::App,
    system::{RawSystem, System},
};

/// A directed acyclic graph of systems to run (eventually in parallel).
pub struct Schedule<I = ()> {
    /// The systems that make up the schedule.
    systems: Vec<RawSystem<I>>,
}

impl<I> Schedule<I> {
    /// Adds a system to the schedule.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all systems inserted in the schedule are associated with the
    /// same [`App`].
    #[inline]
    pub unsafe fn add_system_raw(&mut self, system: RawSystem<I>) {
        self.systems.push(system);
    }

    /// Adds a system to the schedule.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all systems inserted in the schedule are associated with the
    /// same [`App`].
    #[inline]
    pub unsafe fn add_system(&mut self, system: impl for<'a> System<In<'a> = I, Out = ()>) {
        unsafe { self.add_system_raw(RawSystem::new(system)) };
    }

    /// Runs the schedule on the given state.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the systems in the schedule are expected to run on the given
    /// state.
    pub unsafe fn run(&mut self, input: &I, app: &mut App)
    where
        I: Clone,
    {
        for system in &mut self.systems {
            unsafe { system.run(input.clone(), app) };
        }
        for system in &mut self.systems {
            unsafe { system.apply_deferred(app) };
        }
    }
}

impl<I> Default for Schedule<I> {
    fn default() -> Self {
        Self {
            systems: Vec::new(),
        }
    }
}
