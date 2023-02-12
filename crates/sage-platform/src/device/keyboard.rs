use std::fmt;

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
    KeypadEnter,
    /// The **0** key, on the numeric keypad.
    Keypad0,
    /// The **1** key, on the numeric keypad.
    Keypad1,
    /// The **2** key, on the numeric keypad.
    Keypad2,
    /// The **3** key, on the numeric keypad.
    Keypad3,
    /// The **4** key, on the numeric keypad.
    Keypad4,
    /// The **5** key, on the numeric keypad.
    Keypad5,
    /// The **6** key, on the numeric keypad.
    Keypad6,
    /// The **7** key, on the numeric keypad.
    Keypad7,
    /// The **8** key, on the numeric keypad.
    Keypad8,
    /// The **9** key, on the numeric keypad.
    Keypad9,
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
///
/// More on that in the [top-level documentation](index.html#physical-and-symbolic-keys).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScanCode(u32);

impl ScanCode {
    /// Creates a new [`ScanCode`] instance from the provided bytes.
    ///
    /// Note the actual representation of a [`ScanCode`] is platform-dependent, and one should
    /// never actually need to create a [`ScanCode`] instance from raw bytes. This is mostly here
    /// for the sake of completeness.
    #[inline(always)]
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw bytes of this [`ScanCode`].
    ///
    /// Note that the returned value is platform-dependent, and should not be used for
    /// serialization. This will usually represent the make-code of the represented key, but this
    /// is not guaranteed.
    #[inline(always)]
    pub const fn to_raw(self) -> u32 {
        self.0
    }
}

// TODO: provide associated constants for `ScanCode` named after the symbolic meaning of each key
// on a 101-key standard US keyboard.

macro_rules! scan_code_constants {
    ( $(
        $( #[ $args:meta ] )*
        pub const $name:ident = $value:expr;
    )*) => {
        impl ScanCode {
            $(
                $( #[ $args ] )*
                pub const $name: Self = ScanCode::from_raw($value);
            )*
        }

        impl fmt::Debug for ScanCode {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match *self {
                    $(
                        Self::$name => f.write_str(stringify!($name)),
                    )*
                    _ => f.debug_tuple("ScanCode").field(&self.0).finish(),
                }
            }
        }
    };
}

// TODO: figure out whether those codes are actually the same on all platforms. If not, we'll need
// to make this macro platform-dependent.
scan_code_constants! {
    /// The **Escape** key, on 101-key standard US keyboards.
    pub const ESCAPE = 0x01;
    /// The **1!** key, on 101-key standard US keyboards.
    pub const ONE = 0x02;
    /// The **2@** key, on 101-key standard US keyboards.
    pub const TWO = 0x03;
    /// The **3#** key, on 101-key standard US keyboards.
    pub const THREE = 0x04;
    /// The **4$** key, on 101-key standard US keyboards.
    pub const FOUR = 0x05;
    /// The **5%** key, on 101-key standard US keyboards.
    pub const FIVE = 0x06;
    /// The **6^** key, on 101-key standard US keyboards.
    pub const SIX = 0x07;
    /// The **7&** key, on 101-key standard US keyboards.
    pub const SEVEN = 0x08;
    /// The **8&** key, on 101-key standard US keyboards.
    pub const EIGHT = 0x09;
    /// The **9(** key, on 101-key standard US keyboards.
    pub const NINE = 0x0A;
    /// The **0)** key, on 101-key standard US keyboards.
    pub const ZERO = 0x0B;
    /// The **-/** key, on 101-key standard US keyboards.
    pub const MINUS = 0x0C;
    /// The **=+** key, on 101-key standard US keyboards.
    pub const EQUALS = 0x0D;
    /// The **Backspace** key, on 101-key standard US keyboards.
    pub const BACKSPACE = 0x0E;
    /// The **Tab** key, on 101-key standard US keyboards.
    pub const TAB = 0x0F;
    /// The **Q** key, on 101-key standard US keyboards.
    pub const Q = 0x10;
    /// The **W** key, on 101-key standard US keyboards.
    pub const W = 0x11;
    /// The **E** key, on 101-key standard US keyboards.
    pub const E = 0x12;
    /// The **R** key, on 101-key standard US keyboards.
    pub const R = 0x13;
    /// The **T** key, on 101-key standard US keyboards.
    pub const T = 0x14;
    /// The **Y** key, on 101-key standard US keyboards.
    pub const Y = 0x15;
    /// The **U** key, on 101-key standard US keyboards.
    pub const U = 0x16;
    /// The **I** key, on 101-key standard US keyboards.
    pub const I = 0x17;
    /// The **O** key, on 101-key standard US keyboards.
    pub const O = 0x18;
    /// The **P** key, on 101-key standard US keyboards.
    pub const P = 0x19;
    /// The **[{** key, on 101-key standard US keyboards.
    pub const LEFT_BRACKET = 0x1A;
    /// The **]}** key, on 101-key standard US keyboards.
    pub const RIGHT_BRACKET = 0x1B;
    /// The **Enter** key, on 101-key standard US keyboards.
    pub const ENTER = 0x1C;
    /// The **Left Control** key, on 101-key standard US keyboards.
    pub const LEFT_CONTROL = 0x1D;
    /// The **A** key, on 101-key standard US keyboards.
    pub const A = 0x1E;
    /// The **S** key, on 101-key standard US keyboards.
    pub const S = 0x1F;
    /// The **D** key, on 101-key standard US keyboards.
    pub const D = 0x20;
    /// The **F** key, on 101-key standard US keyboards.
    pub const F = 0x21;
    /// The **G** key, on 101-key standard US keyboards.
    pub const G = 0x22;
    /// The **H** key, on 101-key standard US keyboards.
    pub const H = 0x23;
    /// The **J** key, on 101-key standard US keyboards.
    pub const J = 0x24;
    /// The **K** key, on 101-key standard US keyboards.
    pub const K = 0x25;
    /// The **L** key, on 101-key standard US keyboards.
    pub const L = 0x26;
    /// The **;:** key, on 101-key standard US keyboards.
    pub const SEMICOLON = 0x27;
    /// The **'** key, on 101-key standard US keyboards.
    pub const APOSTROPHE = 0x28;
    /// The **`~** key, on 101-key standard US keyboards.
    pub const GRAVE = 0x29;
    /// The **Left Shift** key, on 101-key standard US keyboards.
    pub const LEFT_SHIFT = 0x2A;
    /// The **\|** key, on 101-key standard US keyboards.
    pub const BACKSLASH = 0x2B;
    /// The **Z** key, on 101-key standard US keyboards.
    pub const Z = 0x2C;
    /// The **X** key, on 101-key standard US keyboards.
    pub const X = 0x2D;
    /// The **C** key, on 101-key standard US keyboards.
    pub const C = 0x2E;
    /// The **V** key, on 101-key standard US keyboards.
    pub const V = 0x2F;
    /// The **B** key, on 101-key standard US keyboards.
    pub const B = 0x30;
    /// The **N** key, on 101-key standard US keyboards.
    pub const N = 0x31;
    /// The **M** key, on 101-key standard US keyboards.
    pub const M = 0x32;
    /// The **,** key, on 101-key standard US keyboards.
    pub const COMMA = 0x33;
    /// The **.>** key, on 101-key standard US keyboards.
    pub const PERIOD = 0x34;
    /// The **/** key, on 101-key standard US keyboards.
    pub const SLASH = 0x35;
    /// The **Right Shift** key, on 101-key standard US keyboards.
    pub const RIGHT_SHIFT = 0x36;
    /// The **\*** key, on 101-key standard US keyboards.
    pub const KEYPAD_ASTERISK = 0x37;
    /// The **Left Alt** key, on 101-key standard US keyboards.
    pub const LEFT_ALT = 0x38;
    /// The **Space** key, on 101-key standard US keyboards.
    pub const SPACE = 0x39;
    /// The **Caps Lock** key, on 101-key standard US keyboards.
    pub const CAPS_LOCK = 0x3A;
    /// The **F1** key, on 101-key standard US keyboards.
    pub const F1 = 0x3B;
    /// The **F2** key, on 101-key standard US keyboards.
    pub const F2 = 0x3C;
    /// The **F3** key, on 101-key standard US keyboards.
    pub const F3 = 0x3D;
    /// The **F4** key, on 101-key standard US keyboards.
    pub const F4 = 0x3E;
    /// The **F5** key, on 101-key standard US keyboards.
    pub const F5 = 0x3F;
    /// The **F6** key, on 101-key standard US keyboards.
    pub const F6 = 0x40;
    /// The **F7** key, on 101-key standard US keyboards.
    pub const F7 = 0x41;
    /// The **F8** key, on 101-key standard US keyboards.
    pub const F8 = 0x42;
    /// The **F9** key, on 101-key standard US keyboards.
    pub const F9 = 0x43;
    /// The **F10** key, on 101-key standard US keyboards.
    pub const F10 = 0x44;
    /// The **Num Lock** key, on 101-key standard US keyboards.
    pub const NUM_LOCK = 0x45;
    /// The **Scroll Lock** key, on 101-key standard US keyboards.
    pub const SCROLL_LOCK = 0x46;
    /// The **7** key, on 101-key standard US keyboards.
    pub const KEYPAD_7 = 0x47;
    /// The **8** key, on 101-key standard US keyboards.
    pub const KEYPAD_8 = 0x48;
    /// The **9** key, on 101-key standard US keyboards.
    pub const KEYPAD_9 = 0x49;
    /// The **-** key, on 101-key standard US keyboards.
    pub const KEYPAD_MINUS = 0x4A;
    /// The **4** key, on 101-key standard US keyboards.
    pub const KEYPAD_4 = 0x4B;
    /// The **5** key, on 101-key standard US keyboards.
    pub const KEYPAD_5 = 0x4C;
    /// The **6** key, on 101-key standard US keyboards.
    pub const KEYPAD_6 = 0x4D;
    /// The **+** key, on 101-key standard US keyboards.
    pub const KEYPAD_PLUS = 0x4E;
    /// The **1** key, on 101-key standard US keyboards.
    pub const KEYPAD_1 = 0x4F;
    /// The **2** key, on 101-key standard US keyboards.
    pub const KEYPAD_2 = 0x50;
    /// The **3** key, on 101-key standard US keyboards.
    pub const KEYPAD_3 = 0x51;
    /// The **0** key, on 101-key standard US keyboards.
    pub const KEYPAD_0 = 0x52;
    /// The **.** key, on 101-key standard US keyboards.
    pub const KEYPAD_DECIMAL = 0x53;
    /// The **F11** key, on 101-key standard US keyboards.
    pub const F11 = 0x57;
    /// The **F12** key, on 101-key standard US keyboards.
    pub const F12 = 0x58;
}
