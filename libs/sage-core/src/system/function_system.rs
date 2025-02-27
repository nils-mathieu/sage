use {
    super::SystemInput,
    crate::{
        app::{App, AppCell},
        system::{IntoSystem, System, SystemAccess, SystemParam},
    },
};

/// A trait for Rust closure suitable for use as a system.
pub trait SystemFunction<Marker>: Send + Sync + 'static {
    /// The input of the system.
    type In: SystemInput;

    /// The output of the system.
    type Out;

    /// The parameters of the function system.
    type Param: SystemParam;

    /// Runs the system with the given input and parameters.
    fn run(
        &mut self,
        input: <Self::In as SystemInput>::Item<'_>,
        param: <Self::Param as SystemParam>::Item<'_>,
    ) -> Self::Out;
}

/// A [`System`] that uses a Rust closure to run.
pub struct FunctionSystem<Marker, F: SystemFunction<Marker> + ?Sized> {
    param_state: <F::Param as SystemParam>::State,
    access: SystemAccess,
    closure: F,
}

impl<M, F: SystemFunction<M>> FunctionSystem<M, F> {
    /// Creates a new [`FunctionSystem`] from a closure.
    pub fn new(app: &mut App, closure: F) -> Self {
        let mut access = SystemAccess::default();
        let param_state = <F::Param as SystemParam>::initialize(app, &mut access);

        Self {
            closure,
            param_state,
            access,
        }
    }
}

unsafe impl<Marker, F> System for FunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemFunction<Marker>,
{
    type In = F::In;
    type Out = F::Out;

    #[inline(always)]
    fn access(&self) -> &SystemAccess {
        &self.access
    }

    #[inline]
    unsafe fn run(&mut self, input: <F::In as SystemInput>::Item<'_>, app: AppCell) -> F::Out {
        let param = unsafe { <F::Param as SystemParam>::fetch(&mut self.param_state, app) };
        self.closure.run(input, param)
    }

    #[inline]
    unsafe fn apply_deferred(&mut self, app: &mut App) {
        unsafe { <F::Param as SystemParam>::apply_deferred(&mut self.param_state, app) };
    }
}

impl<F, M> IntoSystem<M, F::In, F::Out> for F
where
    M: 'static,
    F: SystemFunction<M>,
{
    type System = FunctionSystem<M, Self>;

    #[inline]
    fn into_system(this: Self, app: &mut App) -> Self::System {
        FunctionSystem::new(app, this)
    }
}

#[doc(hidden)]
pub struct SystemFunctionHasSystemInput;

macro_rules! tuple_impl {
    ($($name:ident)*) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        impl<Func, Ret, $($name,)*> SystemFunction<fn($($name,)*) -> Ret> for Func
        where
            Func: 'static + Send + Sync +
                FnMut($(<$name as SystemParam>::Item<'_>,)*) -> Ret +
                FnMut($($name,)*) -> Ret,
            $($name: SystemParam,)*
        {
            type In = ();
            type Out = Ret;
            type Param = ($($name,)*);

            fn run(
                &mut self,
                _input: <Self::In as SystemInput>::Item<'_>,
                param: <Self::Param as SystemParam>::Item<'_>,
            ) -> Self::Out {
                let ($($name,)*) = param;
                self($($name,)*)
            }
        }

        #[allow(non_snake_case)]
        impl<Func, Ret, In, $($name,)*> SystemFunction<(SystemFunctionHasSystemInput, fn(In, $($name,)*) -> Ret)> for Func
        where
            In: SystemInput,
            Func: 'static + Send + Sync +
                FnMut(<In as SystemInput>::Item<'_>, $(<$name as SystemParam>::Item<'_>,)*) -> Ret +
                FnMut(In, $($name,)*) -> Ret,
            $($name: SystemParam,)*
        {
            type In = In;
            type Out = Ret;
            type Param = ($($name,)*);

            fn run(
                &mut self,
                input: <Self::In as SystemInput>::Item<'_>,
                param: <Self::Param as SystemParam>::Item<'_>,
            ) -> Self::Out {
                let ($($name,)*) = param;
                self(input, $($name,)*)
            }
        }
    };
}

tuple_impl!();
tuple_impl!(A);
tuple_impl!(A B);
tuple_impl!(A B C);
tuple_impl!(A B C D);
tuple_impl!(A B C D E);
tuple_impl!(A B C D E F);
tuple_impl!(A B C D E F G);
tuple_impl!(A B C D E F G H);
