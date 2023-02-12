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

 * `sage-platform-core` is the main platform abstraction. It defines common types and traits used by
   other sage windowing abstractions.
 * `sage-platform-windows` abstracts the windowing system of the **Windows** operating system.

## Contributing

Contribution is greatly appreciated!
