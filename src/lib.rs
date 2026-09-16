//! # sigstep-a64
//!
//! A `no_std` ARM64 (AArch64) instruction decoder and execution engine
//! parameterized over register and memory semantics, with compile-time
//! async-signal-safety guarantees.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod core;

pub use crate::core::{
    AccessKind, AsyncSignalSafe, CoreReg, DecodeError, ExecError, Gpr, MemoryInterface,
    MemoryOrdering, Nzcv, RegisterBank, VReg,
};
