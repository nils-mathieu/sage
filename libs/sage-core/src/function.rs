use crate::opaque_ptr::OpaquePtr;

/// The VTable for [`Function`].
#[repr(C)]
struct FunctionVTable<T, R> {
    /// The pointer to be invoked when the function is called.
    call: unsafe extern "C" fn(data: OpaquePtr, args: T) -> R,
    /// The pointer to be invoked when the function object is dropped.
    drop: unsafe extern "C" fn(data: OpaquePtr),
}

/// An FFI-safe function pointer.
///
/// This is basically like [`Box<dyn FnMut(T) -> R>`], but it's FFI-safe.
#[repr(C)]
pub struct Function<T: 'static = (), R: 'static = ()> {
    data: OpaquePtr,
    vtable: &'static FunctionVTable<T, R>,
}

impl<T: 'static, R: 'static> Function<T, R> {
    /// Creates a new [`Function<T, R>`] instance from a boxed closure.
    pub fn new<F>(f: Box<F>) -> Self
    where
        F: Send + Sync + FnMut(T) -> R + 'static,
    {
        unsafe extern "C" fn call<T, R, F>(data: OpaquePtr, args: T) -> R
        where
            F: Send + Sync + FnMut(T) -> R + 'static,
        {
            unsafe { data.as_mut::<F>()(args) }
        }

        unsafe extern "C" fn drop<T, R, F>(data: OpaquePtr)
        where
            F: Send + Sync + FnMut(T) -> R + 'static,
        {
            _ = unsafe { Box::from_raw(data.as_ptr::<F>()) };
        }

        trait VTableProvider<T, R> {
            const VTABLE: FunctionVTable<T, R>;
        }

        impl<T, R, F> VTableProvider<T, R> for F
        where
            F: Send + Sync + FnMut(T) -> R + 'static,
        {
            const VTABLE: FunctionVTable<T, R> = FunctionVTable {
                call: call::<T, R, F>,
                drop: drop::<T, R, F>,
            };
        }

        Self {
            data: unsafe { OpaquePtr::from_raw(Box::into_raw(f)) },
            vtable: &F::VTABLE,
        }
    }

    /// Calls the function with the provided arguments.
    #[inline]
    pub fn call(&mut self, args: T) -> R {
        unsafe { (self.vtable.call)(self.data, args) }
    }
}

impl<T: 'static, R: 'static> Drop for Function<T, R> {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.vtable.drop)(self.data) };
    }
}
