use core::mem::ManuallyDrop;

/// A guard that runs a closure when it is dropped.
pub struct Guard<F: FnOnce()>(ManuallyDrop<F>);

impl<F: FnOnce()> Guard<F> {
    /// Creates a new guard that runs the provided closure when it is dropped.
    #[cfg_attr(feature = "inline-more", inline)]
    pub const fn new(f: F) -> Self {
        Self(ManuallyDrop::new(f))
    }

    /// Defuses the guard and prevents the closure from running when the guard is dropped.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn defuse(self) {
        let mut this = ManuallyDrop::new(self);
        unsafe { ManuallyDrop::drop(&mut this.0) };
    }
}

impl<F: FnOnce()> Drop for Guard<F> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn drop(&mut self) {
        unsafe { ManuallyDrop::take(&mut self.0)() };
    }
}

/// Defers the execution of a closure until the returned guard is dropped.
#[cfg_attr(feature = "inline-more", inline)]
pub fn defer(f: impl FnOnce()) -> Guard<impl FnOnce()> {
    Guard::new(f)
}

/// Returns a guard that aborts the program if the closure is dropped without being defused.
#[cfg_attr(feature = "inline-more", inline)]
pub fn abort_guard() -> Guard<impl FnOnce()> {
    // FIXME: how to abort in no_std?
    defer(|| loop {
        core::hint::spin_loop();
    })
}
