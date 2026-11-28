//! Integration tests that run the `hl` binary, or its library, against the file system of the OS.
//!
//! All of them are in one test binary, so the library is linked once.
//! Each area of behaviour has a module of its own, and helpers shared by the modules are in `support`.

mod support;

mod input;

mod config;
mod query;
