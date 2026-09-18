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
pub mod decode;
pub mod exec;
pub mod test_utils;

pub use crate::core::{
    extract_bits, sign_extend, AccessKind, AsyncSignalSafe, CoreReg, DecodeError, ExecError, Gpr,
    MemoryInterface, MemoryOrdering, Nzcv, RegisterBank, VReg,
};
pub use crate::decode::{Decoder, Instruction};
pub use crate::exec::Interpreter;
pub use crate::test_utils::{NoopMem, VirtualRegs};
