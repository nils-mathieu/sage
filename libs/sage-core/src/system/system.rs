use {
    crate::{OpaquePtr, app::App},
    std::{marker::PhantomData, mem::MaybeUninit},
};

/// The VTable associated with a [`RawSystem`].
#[repr(C)]
pub struct RawSystemVTable {
    debug_name: &'static str,
    drop_fn: unsafe extern "C" fn(data: OpaquePtr),
    run_fn: unsafe extern "C" fn(data: OpaquePtr, i: OpaquePtr, o: OpaquePtr, app: &App),
}

/// An FFI-safe type that contains the state of a system.
#[repr(C)]
pub struct RawSystem<I = (), O = ()> {
    data: OpaquePtr,
    vtable: &'static RawSystemVTable,
    _marker: PhantomData<fn(I) -> O>,
}

impl<I, O> RawSystem<I, O> {
    /// Creates a new [`RawSystem`] from a [`System`] implementation.
    pub fn new<S>(system: S) -> Self
    where
        S: for<'a> System<In<'a> = I, Out = O>,
    {
        unsafe extern "C" fn drop_fn<S>(data: OpaquePtr)
        where
            S: System,
        {
            _ = unsafe { Box::from_raw(data.as_ptr::<S>()) };
        }

        unsafe extern "C" fn run_fn<S>(
            data: OpaquePtr,
            input: OpaquePtr,
            output: OpaquePtr,
            app: &App,
        ) where
            S: System,
        {
            unsafe {
                output.as_ptr::<S::Out>().write(
                    data.as_mut::<S>()
                        .run(input.as_ptr::<S::In<'_>>().read(), app),
                )
            }
        }

        trait ProvideVTable {
            const VTABLE: RawSystemVTable;
        }

        impl<S: System> ProvideVTable for S {
            const VTABLE: RawSystemVTable = RawSystemVTable {
                debug_name: S::DEBUG_NAME,
                drop_fn: drop_fn::<S>,
                run_fn: run_fn::<S>,
            };
        }

        Self {
            data: unsafe { OpaquePtr::from_raw(Box::into_raw(Box::new(system))) },
            vtable: &S::VTABLE,
            _marker: PhantomData,
        }
    }

    /// Runs the system with the given input and output arguments.
    ///
    /// # Safety
    ///
    /// See [`System::run`] for safety requirements.
    pub unsafe fn run(&mut self, input: I, app: &App) -> O {
        unsafe {
            let mut input = MaybeUninit::new(input);
            let mut output = MaybeUninit::uninit();
            (self.vtable.run_fn)(
                self.data,
                OpaquePtr::from_raw(&mut input),
                OpaquePtr::from_raw(&mut output),
                app,
            );
            output.assume_init()
        }
    }
}

/// Contains the resources that a [`System`] may access during its execution.
#[derive(Default)]
pub struct SystemAccess {}

/// A system that runs and affects the application state.
///
/// # Safety
///
/// Implementators must ensure that:
///
/// 1. The [`register_access`] method must reflect correctly the resources that will be accessed in
///    the [`run`] method.
///
/// [`register_access`]: System::register_access
/// [`run`]: System::run
pub unsafe trait System: 'static + Send + Sync {
    /// A debug name for the system.
    const DEBUG_NAME: &'static str = std::any::type_name::<Self>();

    /// The input of the system.
    ///
    /// This may be arbitrary data which will be passed to the system when it is executed.
    type In<'a>;

    /// The output of the system.
    ///
    /// This may be arbitrary data which will be returned when the system is executed.
    type Out;

    /// Registers the resources that the system accesses.
    ///
    /// # Panics
    ///
    /// This function panics if any of the required accesses are marked as being already
    /// accessed by the provided [`SystemAccess`].
    fn register_access(&mut self, access: &mut SystemAccess);

    /// Runs the system to completion.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the resources accessed by the system may be accessed
    /// during the system's execution.
    ///
    /// The system must be associated with the given [`App`] instance.
    unsafe fn run(&mut self, input: Self::In<'_>, app: &App) -> Self::Out;
}

/// A trait for Rust types that can be turned into a [`System`] implementation.
pub trait IntoSystem<M> {
    /// The output system of this conversion.
    type System: System;

    /// Converts this type into its associated [`System`] implementation.
    fn into_system(self, app: &mut App) -> Self::System;
}
