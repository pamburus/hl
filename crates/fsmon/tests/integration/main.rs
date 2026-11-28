//! Integration tests that follow files with the crate through its public API, against the file system, devices and
//! signals of the OS, as specified in `specs/011-file-following/spec.md`.
//!
//! All of them are in one test binary, so the crate is linked once.
//! Each area of behaviour has a module of its own, and helpers shared by the modules are in `support`.

mod support;

mod files;
mod pipes;
mod special;
mod stop;
mod watches;
