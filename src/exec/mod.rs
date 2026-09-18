//! AArch64 execution engine and transactional interpreter.
//!
//! Exposes `Interpreter` for executing decoded instructions against register bank
//! and memory interface backends.

pub mod interpreter;

pub use interpreter::Interpreter;
