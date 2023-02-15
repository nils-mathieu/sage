# Sage: A Game Engine

Sage is a game engine/framework for the Rust programming language. It tries to remain as modular
as possible, ensuring that you don't have to pay for what you don't need.

The project is still in its *very* early stages. You can try to toy around with the project, but you
expect bugs and regular breaking changes. And while I'm on the subject of breaking changes, the
project won't follow [semantic versioning](https://semver.org/) until its `1.0` release. Breaking
change may (and will) occur *every* release. I will try to keep the different parts of the ecosystem
as backward-compatible as possible, but you shouldn't rely on that just now.

## Ecosystem

Because it tries to be as modular as possible, there is no single "sage" crate. Instead, you can
import and choose from the following packages.

 * `sage-platform` is the main platform abstraction. It provides common event types, ways to
   represent the state of input devices, as well as ways to create a window for Sage applications.
 * `sage-render` is a type-safe and opinionated wrapper around the Vulkan API. It tries to cut off
   most of the boilerplate of Vulkan while remaining versatile enough for most real-time
   applications.

## Contributing

Contribution is greatly appreciated!
