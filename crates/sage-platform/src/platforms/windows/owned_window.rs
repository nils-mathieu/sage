use std::mem::MaybeUninit;

use scopeguard::ScopeGuard;

use windows_sys::Win32::Foundation::{ERROR_CLASS_ALREADY_EXISTS, ERROR_INVALID_PARAMETER};
use windows_sys::Win32::Foundation::{HINSTANCE, HWND};

use crate::app::Config;

use super::wndproc::WndprocFn;
use super::{wndproc, Ctx, Error};

/// Owns a window and its resources.
///
/// This type assumes that its **GWLP_USERDATA** field is set to a valid [`T`] instance. Or a null
/// pointer.
pub struct Window {
    /// The module instance handle.
    hinstance: HINSTANCE,
    /// The window handle.
    hwnd: HWND,
    /// The window class name.
    class_atom: u16,
}

impl Window {
    /// Creates a new [`Window`].
    pub fn new(config: &Config, cback: WndprocFn) -> Result<Self, Error> {
        use windows_sys::Win32::Devices::HumanInterfaceDevice::HID_USAGE_GENERIC_KEYBOARD;
        use windows_sys::Win32::Devices::HumanInterfaceDevice::HID_USAGE_GENERIC_MOUSE;
        use windows_sys::Win32::Devices::HumanInterfaceDevice::HID_USAGE_PAGE_GENERIC;
        use windows_sys::Win32::UI::Input::RegisterRawInputDevices;
        use windows_sys::Win32::UI::Input::RAWINPUTDEVICE;
        use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyWindow, UnregisterClassW};

        let hinstance = get_module_handle()?;

        let class_atom = register_class(hinstance, config, cback)?;
        let class_atom_guard = scopeguard::guard((), move |()| unsafe {
            UnregisterClassW(class_atom as _, hinstance);
        });

        let hwnd = create_window(hinstance, class_atom, config)?;
        let hwnd_guard = scopeguard::guard((), move |()| unsafe {
            DestroyWindow(hwnd);
        });

        // Ensure that the window receives raw WM_INPUT messages.
        let raw_input_devices = [
            RAWINPUTDEVICE {
                usUsagePage: HID_USAGE_PAGE_GENERIC,
                usUsage: HID_USAGE_GENERIC_KEYBOARD,
                dwFlags: 0,
                hwndTarget: hwnd,
            },
            RAWINPUTDEVICE {
                usUsagePage: HID_USAGE_PAGE_GENERIC,
                usUsage: HID_USAGE_GENERIC_MOUSE,
                dwFlags: 0,
                hwndTarget: hwnd,
            },
        ];
        let ret = unsafe {
            RegisterRawInputDevices(
                raw_input_devices.as_ptr(),
                raw_input_devices.len() as _,
                std::mem::size_of::<RAWINPUTDEVICE>() as _,
            )
        };

        if ret == windows_sys::Win32::Foundation::FALSE {
            return Err(Error::UnexpectedBehavior);
        }

        ScopeGuard::into_inner(class_atom_guard);
        ScopeGuard::into_inner(hwnd_guard);

        Ok(Self {
            hinstance,
            hwnd,
            class_atom,
        })
    }

    /// Returns an exclusive [`Ctx`] reference to this window.
    #[inline(always)]
    pub fn as_ctx(&mut self) -> Ctx {
        // SAFETY:
        //  We are borrowing the `Window` mutably, so we know that the created `Ctx`
        //  will remain valid for the lifetime of `self`.
        unsafe { Ctx::new(self.hwnd) }
    }

    /// Sets the **GWLP_USERDATA** field of this window to `userdata`.
    ///
    /// Note that this function thread-safe and takes a regular shared reference to `self`.
    #[inline(always)]
    pub fn set_userdata(&self, userdata: usize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;
        use windows_sys::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;

        unsafe { SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, userdata as isize) };
    }

    /// Calls the callback function defined for this window.
    ///
    /// If an event was waiting in the queue and that the callback function was succesfully
    /// executed, the function returns `true`. Otherwise, it returns `false`.
    ///
    /// Note that a singel call to this function may cause multiple events to be processed by the
    /// callback function.
    ///
    /// # Panics
    ///
    /// This function panics if the callback function panicked.
    pub fn peek_message(&mut self) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::MSG;
        use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, TranslateMessage};
        use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};

        let mut msg: MaybeUninit<MSG> = MaybeUninit::uninit();

        // SAFETY:
        //  This is always safe, and `msg` is a valid pointer.
        let b = unsafe { PeekMessageW(msg.as_mut_ptr(), self.hwnd, 0, 0, PM_REMOVE) };
        let b = b == windows_sys::Win32::Foundation::TRUE;

        if b {
            // SAFETY:
            //  The succesful call to `PeekMessageW` above ensures that `msg` is initialized.
            let msg = unsafe { msg.assume_init_ref() };

            // SAFETY:
            //  `msg` is a valid pointer.
            unsafe {
                TranslateMessage(msg);
                DispatchMessageW(msg);
            }
        }

        b
    }

    /// Calls the callback function defined for this window.
    ///
    /// If an event was waiting in the queue and that the callback function was succesfully
    /// executed, the function returns. Otherwise, it blocks until an event is received.
    ///
    /// Note that a single call to this function may cause multiple events to be processed by the
    /// callback function.
    pub fn get_message(&mut self) -> Result<(), Error> {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetMessageW;
        use windows_sys::Win32::UI::WindowsAndMessaging::MSG;
        use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, TranslateMessage};

        let mut msg: MaybeUninit<MSG> = MaybeUninit::uninit();

        // SAFETY:
        //  This is always safe, and `msg` is a valid pointer.
        let b = unsafe { GetMessageW(msg.as_mut_ptr(), self.hwnd, 0, 0) };

        match b {
            -1 => return Err(Error::UnexpectedBehavior),
            0 => return Ok(()),
            _ => (),
        }

        // SAFETY:
        //  The succesful call to `PeekMessageW` above ensures that `msg` is initialized.
        let msg = unsafe { msg.assume_init_ref() };

        // SAFETY:
        //  `msg` is a valid pointer.
        unsafe {
            TranslateMessage(msg);
            DispatchMessageW(msg);
        }

        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyWindow, UnregisterClassW};

            DestroyWindow(self.hwnd);
            UnregisterClassW(self.class_atom as _, self.hinstance);
        }
    }
}

/// Returns the module handle of the current process.
fn get_module_handle() -> Result<HINSTANCE, Error> {
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    // SAFETY:
    //  This function is always safe to call with a null pointer.
    let hinstance = unsafe { GetModuleHandleW(core::ptr::null_mut()) };

    if hinstance == 0 {
        Err(Error::UnexpectedBehavior)
    } else {
        Ok(hinstance)
    }
}

/// Creates a new window class.
fn register_class(
    hinstance: HINSTANCE,
    _config: &Config,
    wndproc: wndproc::WndprocFn,
) -> Result<u16, Error> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{RegisterClassExW, WNDCLASSEXW};

    let class_info = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinstance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: core::ptr::null(),
        lpszClassName: windows_sys::w!("Sage Window Class"),
        hIconSm: 0,
    };

    let class_atom = unsafe { RegisterClassExW(&class_info) };

    if class_atom == 0 {
        if super::last_error_code() == ERROR_CLASS_ALREADY_EXISTS {
            Err(Error::ClassAlreadyRegistered)
        } else {
            Err(Error::UnexpectedBehavior)
        }
    } else {
        Ok(class_atom)
    }
}

/// Computes the window styles for a given configuration.
///
/// The first element if the extended window style, and the second element is the regular window
/// style.
fn compute_window_styles(config: &Config) -> (u32, u32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        WS_EX_ACCEPTFILES, WS_EX_OVERLAPPEDWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW,
        WS_VISIBLE,
    };

    let mut ex_style = 0;
    let mut style = 0;

    style |= WS_OVERLAPPEDWINDOW;
    ex_style |= WS_EX_ACCEPTFILES;
    ex_style |= WS_EX_OVERLAPPEDWINDOW;

    if config.transparent {
        ex_style |= WS_EX_TRANSPARENT;
    }

    if config.visible {
        style |= WS_VISIBLE;
    }

    (ex_style, style)
}

/// Creates a new window.
fn create_window(hinstance: HINSTANCE, class_atom: u16, config: &Config) -> Result<HWND, Error> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, CW_USEDEFAULT};

    let (w, h) = match config.size {
        Some((w, h)) => (w as i32, h as i32),
        None => (CW_USEDEFAULT, CW_USEDEFAULT),
    };
    let (x, y) = match config.position {
        Some((x, y)) => (x, y),
        None => (CW_USEDEFAULT, CW_USEDEFAULT),
    };

    if config.title.contains('\0') {
        return Err(Error::UnsupportedConfig);
    }

    let title = config
        .title
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<u16>>();

    let (ex_style, style) = compute_window_styles(config);

    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class_atom as _,
            title.as_ptr(),
            style,
            x,
            y,
            w,
            h,
            0,
            0,
            hinstance,
            core::ptr::null_mut(),
        )
    };

    if hwnd == 0 {
        if super::last_error_code() == ERROR_INVALID_PARAMETER {
            Err(Error::UnsupportedConfig)
        } else {
            Err(Error::UnexpectedBehavior)
        }
    } else {
        Ok(hwnd)
    }
}
