use crate::{
    app::App,
    system::{System, SystemAccess, SystemParam},
};

/// A trait for Rust closure suitable for use as a system.
pub trait SystemFunction<Marker>: Send + Sync + 'static {
    /// The input of the system.
    type In<'a>;

    /// The output of the system.
    type Out;

    /// The parameters of the function system.
    type Param: SystemParam;

    /// Runs the system with the given input and parameters.
    fn run(
        &mut self,
        input: Self::In<'_>,
        param: <Self::Param as SystemParam>::Item<'_>,
    ) -> Self::Out;
}

/// A [`System`] that uses a Rust closure to run.
pub struct FunctionSystem<Marker, F: SystemFunction<Marker>> {
    param_state: <F::Param as SystemParam>::State,
    closure: F,
}

impl<M, F: SystemFunction<M>> FunctionSystem<M, F> {
    /// Creates a new [`FunctionSystem`] from a closure.
    pub fn new(app: &mut App, closure: F) -> Self {
        Self {
            closure,
            param_state: <F::Param as SystemParam>::create_state(app),
        }
    }
}

unsafe impl<Marker, F> System for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemFunction<Marker>,
{
    type In<'a> = F::In<'a>;
    type Out = F::Out;

    #[inline]
    fn register_access(&mut self, access: &mut SystemAccess) {
        <F::Param as SystemParam>::register_access(access);
    }

    unsafe fn run(&mut self, input: F::In<'_>, app: &App) -> F::Out {
        let param = unsafe { <F::Param as SystemParam>::fetch(&self.param_state, app) };
        self.closure.run(input, param)
    }
}
