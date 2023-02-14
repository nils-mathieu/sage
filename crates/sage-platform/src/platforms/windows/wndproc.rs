use std::any::Any;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::panic::AssertUnwindSafe;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::{GetRawInputData, RID_INPUT};
use windows_sys::Win32::UI::Input::{RAWINPUT, RAWINPUTHEADER, RAWKEYBOARD, RAWMOUSE};
use windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WM_CHAR, WM_CLOSE, WM_DESTROY, WM_INPUT, WM_MOUSEMOVE, WM_MOVE, WM_SIZE, WM_SYSCHAR,
};

use crate::app::App;
use crate::device::{Key, MouseButton, ScanCode};

use super::Ctx;

/// The signature of a WNDPROC callback.
pub type WndprocFn = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// Calls the default window procedure, but checks that the message does not break any invariants
/// of the [`Ctx`] type. If it does, the function returns 0.
///
/// # Safety
///
/// This function has the same requirements as [`DefWindowProcW`].
unsafe fn checked_default_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if matches!(msg, WM_CLOSE | WM_DESTROY) {
        0
    } else {
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

/// The state stored in the **GWLP_USERDATA** pointer of a window.
pub struct State<A> {
    /// When the application panics, the panic payload is stored in this field to be later
    /// resumed out of the wndproc callback.
    payload: Option<Box<dyn Any + Send + 'static>>,
    /// When a `WM_CHAR` event is received with a high surrogate, it is stored here until the next
    /// `WM_CHAR` event is received with a low surrogate.
    ///
    /// When the value is 0, it means that there is no high surrogate.
    high_surrogate: u16,
    /// An opaque pointer to an [`App`] implementation.
    app: A,
}

impl<A> State<A> {
    /// Creates a new [`State<A>`] instance.
    #[inline(always)]
    pub const fn new(app: A) -> Self {
        Self {
            payload: None,
            app,
            high_surrogate: 0,
        }
    }

    /// Returns an exclusive reference to the [`App`] implementation.
    #[inline(always)]
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    /// If the payload has been populated, this function resumes the panic.
    ///
    /// # Panics
    ///
    /// That seems pretty obvious.
    pub fn resume_unwind(&mut self) {
        if let Some(payload) = self.payload.take() {
            std::panic::resume_unwind(payload);
        }
    }
}

impl<A: App> State<A> {
    /// The raw WNDPROC callback that will be passed to the Windows API.
    ///
    /// Note that this function is *never* allowed to panic, as that would cause the application to
    /// unwind through foreign code, which is undefined behavior.
    ///
    /// # Safety
    ///
    /// This function assumes that many invariants are upheld.
    ///
    /// * The `hwnd` parameter must be a valid window handle.
    pub unsafe extern "system" fn raw_wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let userdata = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

        if userdata == 0 {
            // The window has not been created yet, so we can't really do anthing. Let's just call
            // the default (but checked) default procedure.

            // SAFETY:
            //  We literally are a window procedure.
            return unsafe { checked_default_window_proc(hwnd, msg, wparam, lparam) };
        }

        // SAFETY:
        //  This is part of the requirements of the `raw_wndproc` function. And this function is
        //  called by the Windows API, so it's not like we can do anything about it.
        //
        //  We know that **GWLP_USERDATA** contains a valid pointer to us because, well, that's the
        //  thing we just got back.
        let ctx = crate::app::Ctx::Windows(unsafe { Ctx::new(hwnd) });

        let state = unsafe { &mut *(userdata as *mut State<A>) };

        // This function may panic. If it does, we need to store the panic payload in the state
        // so that it can be resumed later, but outside of the window procedure.
        let panicky = || match msg {
            WM_CLOSE => {
                state.app.close_request(&ctx);
                0
            }
            WM_SIZE => {
                let lparam = lparam as u32;
                let width = lparam & 0xFFFF;
                let height = lparam >> 16;

                state.app.size(&ctx, width, height);
                0
            }
            WM_MOVE => {
                let lparam = lparam as u32;
                let x = (lparam & 0xFFFF) as i16 as i32;
                let y = (lparam >> 16) as i16 as i32;

                state.app.position(&ctx, x, y);
                0
            }
            WM_INPUT => {
                let mut rawinput: MaybeUninit<RAWINPUT> = MaybeUninit::uninit();
                let mut size = std::mem::size_of::<RAWINPUT>() as u32;
                let ret = unsafe {
                    GetRawInputData(
                        lparam,
                        RID_INPUT,
                        rawinput.as_mut_ptr() as *mut c_void,
                        &mut size,
                        std::mem::size_of::<RAWINPUTHEADER>() as u32,
                    )
                };

                if ret == u32::MAX {
                    return 0;
                }

                unsafe { state.displatch_raw_input(&ctx, rawinput.assume_init_ref()) };

                0
            }
            WM_MOUSEMOVE => {
                let lparam = lparam as u32;
                let x = lparam & 0xFFFF;
                let y = lparam >> 16;
                state.app.cursor(&ctx, x, y);
                0
            }
            WM_CHAR | WM_SYSCHAR => {
                let codepoint = wparam as u16;

                if (0xD800..=0xDBFF).contains(&codepoint) {
                    // When the code-point is a high surrogate, store it until a new `WM_CHAR` event
                    // is received with a low surrogate.
                    state.high_surrogate = codepoint;
                    0
                } else if (0xDC00..=0xDFFF).contains(&codepoint) {
                    if state.high_surrogate == 0 {
                        // There is not much we can do about it, so we'll just ignore it.
                        return 0;
                    }

                    // If the code-point is a low surrogate, combine it with the high surrogate
                    // and send it to the application.
                    let h = state.high_surrogate;
                    state.high_surrogate = 0;

                    if let Some(Ok(c)) = std::char::decode_utf16([h, codepoint]).next() {
                        let mut buf = [0; 4];
                        let s = c.encode_utf8(&mut buf);
                        state.app.text(&ctx, s);
                    }

                    0
                } else {
                    state.high_surrogate = 0;

                    if let Some(c) = char::from_u32(codepoint as u32) {
                        let mut buf = [0; 4];
                        let s = c.encode_utf8(&mut buf);
                        state.app.text(&ctx, s);
                    }

                    0
                }
            }
            _ => unsafe { checked_default_window_proc(hwnd, msg, wparam, lparam) },
        };

        match std::panic::catch_unwind(AssertUnwindSafe(panicky)) {
            Ok(code) => code,
            Err(payload) => {
                state.payload = Some(payload);
                0
            }
        }
    }

    /// Dispatches a raw input event to the application.
    ///
    /// # Safety
    ///
    /// This function assumes that the `input` parameter is reference to a valid `RAWINPUT` struct.
    unsafe fn displatch_raw_input(&mut self, ctx: &crate::app::Ctx, input: &RAWINPUT) {
        use windows_sys::Win32::UI::Input::{RIM_TYPEKEYBOARD, RIM_TYPEMOUSE};

        let device_id = crate::device::DeviceId::Windows(input.header.hDevice);

        match input.header.dwType {
            RIM_TYPEKEYBOARD => {
                let input = unsafe { &input.data.keyboard };
                self.dispatch_raw_keyboard_input(ctx, device_id, input);
            }
            RIM_TYPEMOUSE => {
                let input = unsafe { &input.data.mouse };
                self.dispatch_raw_mouse_input(ctx, device_id, input);
            }
            _ => (),
        }
    }

    /// Dispatches a raw keyboard input event to the application.
    fn dispatch_raw_keyboard_input(
        &mut self,
        ctx: &crate::app::Ctx,
        device: crate::device::DeviceId,
        input: &RAWKEYBOARD,
    ) {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::{RI_KEY_BREAK, RI_KEY_MAKE};
        use windows_sys::Win32::UI::WindowsAndMessaging::{RI_KEY_E0, RI_KEY_E1};

        // Turns out that the Windows API is a bit of a mess when it comes to keyboard
        // input. The following code is based on the following blog post:
        //   https://blog.molecular-matters.com/2011/09/05/properly-handling-keyboard-input/

        // Discard "fake-key" events, which are part of an escaped sequence.
        if input.VKey == 0xFF {
            return;
        }

        let e0 = (input.Flags as u32 & RI_KEY_E0) != 0;
        let e1 = (input.Flags as u32 & RI_KEY_E1) != 0;
        let now_pressed = (input.Flags as u32 & RI_KEY_BREAK) == RI_KEY_MAKE;

        let mut scan_code = input.MakeCode as u32;
        if input.VKey == VK_NUMLOCK {
            // Correct the scan code of the Numlock key, which may be confused with the Pause key.
            // The Pause key sends the same scan-code as the Numlock key, but with a prior E1 (which
            // we previously ignored).
            scan_code = unsafe { MapVirtualKeyW(VK_NUMLOCK as u32, MAPVK_VK_TO_VSC) } | 0x100;
        }
        if e1 {
            // For escaped sequences, turn the virtual key into the correct scan code using
            // MapVirtualKey. MapVirtualKey is unable to map VK_PAUSE (this is a known bug).
            if input.VKey == VK_PAUSE {
                scan_code = 0x45 | 0xE1000000;
            } else {
                scan_code = unsafe { MapVirtualKeyW(input.VKey as u32, MAPVK_VK_TO_VSC) };
            }
        }
        if e0 {
            scan_code |= 0x00E00000;
        }
        let key = vkey_to_key(input.VKey, input.MakeCode, e0, e1);

        self.app
            .keyboard_key(ctx, device, key, ScanCode::from_raw(scan_code), now_pressed);
    }

    /// Dispatches a raw mouse input event to the application.
    fn dispatch_raw_mouse_input(
        &mut self,
        ctx: &crate::app::Ctx,
        device: crate::device::DeviceId,
        input: &RAWMOUSE,
    ) {
        use windows_sys::Win32::Devices::HumanInterfaceDevice::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::*;

        if input.usFlags as u32 & MOUSE_MOVE_ABSOLUTE == MOUSE_MOVE_RELATIVE {
            // Absolute motion are not currently supported because Windows sends coordinates
            // relative to the whole screen, and normalized. That's kinda tricky to represent in
            // a meaningful way to the user.
            // Plus this event is meant to represent raw mouse movements, which are rarely
            // absolute.
            let dx = input.lLastX;
            let dy = input.lLastY;
            self.app.mouse_motion(ctx, device, dx, dy);
        }

        let btn_flags = unsafe { input.Anonymous.Anonymous.usButtonFlags as u32 };

        if btn_flags & RI_MOUSE_LEFT_BUTTON_DOWN != 0 {
            self.app.mouse_button(ctx, device, MouseButton::Left, true);
        }
        if btn_flags & RI_MOUSE_LEFT_BUTTON_UP != 0 {
            self.app.mouse_button(ctx, device, MouseButton::Left, false);
        }
        if btn_flags & RI_MOUSE_RIGHT_BUTTON_DOWN != 0 {
            self.app.mouse_button(ctx, device, MouseButton::Right, true);
        }
        if btn_flags & RI_MOUSE_RIGHT_BUTTON_UP != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Right, false);
        }
        if btn_flags & RI_MOUSE_MIDDLE_BUTTON_DOWN != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Middle, true);
        }
        if btn_flags & RI_MOUSE_MIDDLE_BUTTON_UP != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Middle, false);
        }
        if btn_flags & RI_MOUSE_BUTTON_4_DOWN != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Other(0), true);
        }
        if btn_flags & RI_MOUSE_BUTTON_4_UP != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Other(0), false);
        }
        if btn_flags & RI_MOUSE_BUTTON_5_DOWN != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Other(1), true);
        }
        if btn_flags & RI_MOUSE_BUTTON_5_UP != 0 {
            self.app
                .mouse_button(ctx, device, MouseButton::Other(1), false);
        }

        if btn_flags & RI_MOUSE_WHEEL != 0 {
            let wheel_delta = unsafe { input.Anonymous.Anonymous.usButtonData as i16 } as f32
                / WHEEL_DELTA as f32;
            self.app.scroll(ctx, device, 0.0, wheel_delta);
        }
        if btn_flags & RI_MOUSE_HWHEEL != 0 {
            let wheel_delta = unsafe { input.Anonymous.Anonymous.usButtonData as i16 } as f32
                / WHEEL_DELTA as f32;
            self.app.scroll(ctx, device, wheel_delta, 0.0);
        }
    }
}

/// Attemps to convert a virtual key code into a [`Key`].
fn vkey_to_key(vkey: u16, make_code: u16, e0: bool, _e1: bool) -> Option<Key> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    match vkey {
        VK_ESCAPE => Some(Key::Escape),
        VK_F1 => Some(Key::F1),
        VK_F2 => Some(Key::F2),
        VK_F3 => Some(Key::F3),
        VK_F4 => Some(Key::F4),
        VK_F5 => Some(Key::F5),
        VK_F6 => Some(Key::F6),
        VK_F7 => Some(Key::F7),
        VK_F8 => Some(Key::F8),
        VK_F9 => Some(Key::F9),
        VK_F10 => Some(Key::F10),
        VK_F11 => Some(Key::F11),
        VK_F12 => Some(Key::F12),
        VK_F13 => Some(Key::F13),
        VK_F14 => Some(Key::F14),
        VK_F15 => Some(Key::F15),
        VK_F16 => Some(Key::F16),
        VK_F17 => Some(Key::F17),
        VK_F18 => Some(Key::F18),
        VK_F19 => Some(Key::F19),
        VK_F20 => Some(Key::F20),
        VK_F21 => Some(Key::F21),
        VK_F22 => Some(Key::F22),
        VK_F23 => Some(Key::F23),
        VK_F24 => Some(Key::F24),
        VK_SNAPSHOT => Some(Key::PrintScreen),
        VK_SCROLL => Some(Key::ScrollLock),
        VK_PAUSE => Some(Key::Pause),
        VK_0 => Some(Key::Zero),
        VK_1 => Some(Key::One),
        VK_2 => Some(Key::Two),
        VK_3 => Some(Key::Three),
        VK_4 => Some(Key::Four),
        VK_5 => Some(Key::Five),
        VK_6 => Some(Key::Six),
        VK_7 => Some(Key::Seven),
        VK_8 => Some(Key::Eight),
        VK_9 => Some(Key::Nine),
        VK_TAB => Some(Key::Tab),
        VK_CAPITAL => Some(Key::CapsLock),
        VK_SHIFT => {
            // The **Shift** key is a bit tricky; we have to use `MapVirtualKey` to distinguish
            // between the left and right shift keys.
            let vkey = unsafe { MapVirtualKeyW(make_code as u32, MAPVK_VSC_TO_VK_EX) };

            match vkey as u16 {
                VK_LSHIFT => Some(Key::LeftShift),
                VK_RSHIFT => Some(Key::RightShift),
                _ => None,
            }
        }
        VK_LSHIFT => Some(Key::LeftShift),
        VK_CONTROL => {
            if e0 {
                Some(Key::RightControl)
            } else {
                Some(Key::LeftControl)
            }
        }
        VK_LCONTROL => Some(Key::LeftControl),
        VK_LWIN => Some(Key::LeftMeta),
        VK_MENU => {
            if e0 {
                Some(Key::RightAlt)
            } else {
                Some(Key::LeftAlt)
            }
        }
        VK_LMENU => Some(Key::LeftAlt),
        VK_SPACE => Some(Key::Space),
        VK_RMENU => Some(Key::RightAlt),
        VK_RWIN => Some(Key::RightMeta),
        VK_RSHIFT => Some(Key::RightShift),
        VK_RCONTROL => Some(Key::RightControl),
        VK_RETURN => {
            if e0 {
                Some(Key::KeypadEnter)
            } else {
                Some(Key::Enter)
            }
        }
        VK_BACK => Some(Key::Backspace),
        VK_A => Some(Key::A),
        VK_B => Some(Key::B),
        VK_C => Some(Key::C),
        VK_D => Some(Key::D),
        VK_E => Some(Key::E),
        VK_F => Some(Key::F),
        VK_G => Some(Key::G),
        VK_H => Some(Key::H),
        VK_I => Some(Key::I),
        VK_J => Some(Key::J),
        VK_K => Some(Key::K),
        VK_L => Some(Key::L),
        VK_M => Some(Key::M),
        VK_N => Some(Key::N),
        VK_O => Some(Key::O),
        VK_P => Some(Key::P),
        VK_Q => Some(Key::Q),
        VK_R => Some(Key::R),
        VK_S => Some(Key::S),
        VK_T => Some(Key::T),
        VK_U => Some(Key::U),
        VK_V => Some(Key::V),
        VK_W => Some(Key::W),
        VK_X => Some(Key::X),
        VK_Y => Some(Key::Y),
        VK_Z => Some(Key::Z),
        VK_INSERT => Some(Key::Insert),
        VK_DELETE => Some(Key::Delete),
        VK_HOME => Some(Key::Home),
        VK_END => Some(Key::End),
        VK_PRIOR => Some(Key::PageUp),
        VK_NEXT => Some(Key::PageDown),
        VK_LEFT => Some(Key::Left),
        VK_UP => Some(Key::Up),
        VK_RIGHT => Some(Key::Right),
        VK_DOWN => Some(Key::Down),
        VK_NUMLOCK => Some(Key::NumLock),
        VK_DIVIDE => Some(Key::Divide),
        VK_MULTIPLY => Some(Key::Multiply),
        VK_SUBTRACT => Some(Key::Subtract),
        VK_ADD => Some(Key::Add),
        VK_DECIMAL => Some(Key::Decimal),
        VK_NUMPAD0 => Some(Key::Keypad0),
        VK_NUMPAD1 => Some(Key::Keypad1),
        VK_NUMPAD2 => Some(Key::Keypad2),
        VK_NUMPAD3 => Some(Key::Keypad3),
        VK_NUMPAD4 => Some(Key::Keypad4),
        VK_NUMPAD5 => Some(Key::Keypad5),
        VK_NUMPAD6 => Some(Key::Keypad6),
        VK_NUMPAD7 => Some(Key::Keypad7),
        VK_NUMPAD8 => Some(Key::Keypad8),
        VK_NUMPAD9 => Some(Key::Keypad9),
        _ => None,
    }
}
