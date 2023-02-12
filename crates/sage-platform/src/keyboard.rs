//! Defines the types required to represent the state of a keyboard.
//!
//! # Physical And Symbolic Keys
//!
//! You will notice that this module defines two separate types, both representing keyboard keys.
//!
//! First, there is the [`Key`] enumeration. It is meant to represent the symblic meaning of a key
//! rather than its physical location on the keyboard. In other words, [`Key`] depends on the
//! user's *keyboard layout*. It was once a physical key-code, but has been translated into a
//! symbolic key by the operating system or window manager.
//!
//! On the other hand, we have the [`ScanCode`] struct. Rather than a symbol, a scan-code
//! represents a concrete keyboard key. It is a (rough) representation of the data sent by the
//! keyboard down the wire.
//!
//! What should you choose then? If the physical location of a key matters more than its logical
//! meaning, you should use the [`ScanCode`] type. Conversly, if you need the know what key the
//! user *meant* to press, then the [`Key`] enumeration is a better choice.

/// Keyboard keys.
///
/// # Symbolic Keys
///
/// Note that this enumeration is meant to represent the *symbolic* meaning of a given keyboard
/// key. In other words, a key with a specific *symblic meaning* may exist only as a combinaison
/// of multiple *physical keys*. In other words, one should not rely on the physical location of
/// such key.
///
/// When the physical location of the key matters more than its logical meaning, the [`ScanCode`]
/// type should be used instead.
///
/// More on that in the [top-level documentation](index.html#physical-and-symbolic-keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// The **Escape** key.
    Escape,

    /// The **F1** key.
    F1,
    /// The **F2** key.
    F2,
    /// The **F3** key.
    F3,
    /// The **F4** key.
    F4,
    /// The **F5** key.
    F5,
    /// The **F6** key.
    F6,
    /// The **F7** key.
    F7,
    /// The **F8** key.
    F8,
    /// The **F9** key.
    F9,
    /// The **F10** key.
    F10,
    /// The **F11** key.
    F11,
    /// The **F12** key.
    F12,
    /// The **F13** key.
    F13,
    /// The **F14** key.
    F14,
    /// The **F15** key.
    F15,
    /// The **F16** key.
    F16,
    /// The **F17** key.
    F17,
    /// The **F18** key.
    F18,
    /// The **F19** key.
    F19,
    /// The **F20** key.
    F20,
    /// The **F21** key.
    F21,
    /// The **F22** key.
    F22,
    /// The **F23** key.
    F23,
    /// The **F24** key.
    F24,

    /// The **Print Screen** key.
    ///
    /// This key is sometimes named **Snapshot**.
    #[doc(alias = "Snapshot")]
    PrintScreen,
    /// The **Scroll Lock** key.
    ScrollLock,
    /// The **Pause** key.
    Pause,

    /// The **0** key, over the letters.
    Zero,
    /// The **1** key, over the letters.
    One,
    /// The **2** key, over the letters.
    Two,
    /// The **3** key, over the letters.
    Three,
    /// The **4** key, over the letters.
    Four,
    /// The **5** key, over the letters.
    Five,
    /// The **6** key, over the letters.
    Six,
    /// The **7** key, over the letters.
    Seven,
    /// The **8** key, over the letters.
    Eight,
    /// The **9** key, over the letters.
    Nine,

    /// The **Tab** key.
    Tab,
    /// The **Caps Lock** key.
    ///
    /// This key is sometimes named **Capital**.
    #[doc(alias = "Capital")]
    CapsLock,
    /// The left **Shift** key.
    LeftShift,
    /// The left **Control** key.
    #[doc(alias = "Ctrl")]
    LeftControl,
    /// The left **Meta** key.
    ///
    /// This key is sometimes named **Super**, **Command**, or **Win**.
    #[doc(alias("Super", "Command", "Win"))]
    LeftMeta,
    /// The left **Alt** key.
    ///
    /// This key is sometimes named **Menu**.
    #[doc(alias = "Menu")]
    LeftAlt,
    /// The **Space** bar key.
    Space,
    /// The right **Alt** key.
    ///
    /// This key is sometimes named **Menu**.
    #[doc(alias = "Menu")]
    RightAlt,
    /// The right **Meta** key.
    ///
    /// This key is sometimes named **Super**, **Command**, or **Win**.
    #[doc(alias("Super", "Command", "Win"))]
    RightMeta,
    /// The right **Shift** key.
    RightShift,
    /// The right **Control** key
    #[doc(alias = "Ctrl")]
    RightControl,
    /// The **Enter** key.
    ///
    /// This key is sometimes named **Return**.
    #[doc(alias = "Return")]
    Enter,
    /// The **Backspace** key.
    ///
    /// This key is sometimes named **Delete**.
    #[doc(alias = "Delete")]
    Backspace,

    /// The **A** key.
    A,
    /// The **B** key.
    B,
    /// The **C** key.
    C,
    /// The **D** key.
    D,
    /// The **E** key.
    E,
    /// The **F** key.
    F,
    /// The **G** key.
    G,
    /// The **H** key.
    H,
    /// The **I** key.
    I,
    /// The **J** key.
    J,
    /// The **K** key.
    K,
    /// The **L** key.
    L,
    /// The **M** key.
    M,
    /// The **N** key.
    N,
    /// The **O** key.
    O,
    /// The **P** key.
    P,
    /// The **Q** key.
    Q,
    /// The **R** key.
    R,
    /// The **S** key.
    S,
    /// The **T** key.
    T,
    /// The **U** key.
    U,
    /// The **V** key.
    V,
    /// The **W** key.
    W,
    /// The **X** key.
    X,
    /// The **Y** key.
    Y,
    /// The **Z** key.
    Z,

    /// The **Insert** key.
    Insert,
    /// The **Delete** key.
    Delete,
    /// The **Home** key.
    Home,
    /// The **End** key.
    End,
    /// The **Page Up** key.
    ///
    /// This key is sometimes named **Prior**.
    #[doc(alias = "Prior")]
    PageUp,
    /// The **Page Down** key.
    ///
    /// This key is sometimes named **Next**.
    #[doc(alias = "Next")]
    PageDown,

    /// The **Left** arrow key.
    #[doc(alias = "Arrow")]
    Left,
    /// The **Up** arrow key.
    #[doc(alias = "Arrow")]
    Up,
    /// The **Right** arrow key.
    #[doc(alias = "Arrow")]
    Right,
    /// The **Down** arrow key.
    #[doc(alias = "Arrow")]
    Down,

    /// The **Num Lock** key.
    ///
    /// This key is sometimes named **Clear**.
    #[doc(alias = "Clear")]
    NumLock,
    /// The **/** key, on the numeric keypad.
    Divide,
    /// The **\*** key, on the numeric keypad.
    Multiply,
    /// The **-** key, on the numeric keypad.
    Subtract,
    /// The **+** key, on the numeric keypad.
    Add,
    /// The **.** key, on the numeric keypad.
    Decimal,
    /// The **Enter** key, on the numeric keypad.
    ///
    /// This key is sometimes named **Return**.
    #[doc(alias = "Return")]
    NumpadEnter,
    /// The **0** key, on the numeric keypad.
    Numpad0,
    /// The **1** key, on the numeric keypad.
    Numpad1,
    /// The **2** key, on the numeric keypad.
    Numpad2,
    /// The **3** key, on the numeric keypad.
    Numpad3,
    /// The **4** key, on the numeric keypad.
    Numpad4,
    /// The **5** key, on the numeric keypad.
    Numpad5,
    /// The **6** key, on the numeric keypad.
    Numpad6,
    /// The **7** key, on the numeric keypad.
    Numpad7,
    /// The **8** key, on the numeric keypad.
    Numpad8,
    /// The **9** key, on the numeric keypad.
    Numpad9,
}

/// Physical keyboard keys.
///
/// # Physical Keys
///
/// This type is meant to represent the physical location of a key. It contains a
/// platform-dependent identifier (often a make-code), representing a layout-independent physical
/// key. This means that the `A` key on a QWERTY layout will have the same [`ScanCode`] as the Q
/// key on an AZERTY layout.
///
/// When the symbolic meaning of keys is more important than its physical location, the [`Key`]
/// type should be used instead.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScanCode(u32);

// TODO: provide associated constants for `ScanCode` named after the symbolic meaning of each key
// on a 101-key standard US keyboard.
