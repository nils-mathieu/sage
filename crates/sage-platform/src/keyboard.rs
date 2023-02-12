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
// TODO: fill this link
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {}

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
