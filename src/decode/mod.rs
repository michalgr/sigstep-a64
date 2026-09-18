//! AArch64 instruction representation and decoding engine.
//!
//! Provides the `Instruction` enum representing decoded AArch64 instructions
//! and the `Decoder` engine for converting raw 32-bit opcodes into `Instruction`.

pub mod branch;

use crate::core::error::DecodeError;
use crate::core::reg::Gpr;
use crate::core::traits::AsyncSignalSafe;
use crate::decode::branch::decode_branch;

/// Decoded AArch64 instruction representation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// `B` — Branch (Immediate)
    ///
    /// Reference: Arm ARM DDI 0487K.a, Section C6.2.25
    B {
        /// Signed 64-bit byte offset (`imm26 << 2`).
        offset: i64,
    },

    /// `BL` — Branch with Link (Immediate)
    ///
    /// Reference: Arm ARM DDI 0487K.a, Section C6.2.33
    Bl {
        /// Signed 64-bit byte offset (`imm26 << 2`).
        offset: i64,
    },

    /// `BR` — Branch to Register
    ///
    /// Reference: Arm ARM DDI 0487K.a, Section C6.2.34
    Br {
        /// General-purpose register holding target address.
        rn: Gpr,
    },

    /// `BLR` — Branch with Link to Register
    ///
    /// Reference: Arm ARM DDI 0487K.a, Section C6.2.35
    Blr {
        /// General-purpose register holding target address.
        rn: Gpr,
    },

    /// `RET` — Return from Subroutine
    ///
    /// Reference: Arm ARM DDI 0487K.a, Section C6.2.235
    Ret {
        /// General-purpose register holding return address (defaults to X30).
        rn: Gpr,
    },
}

unsafe impl AsyncSignalSafe for Instruction {}

/// Main instruction decoder for AArch64 opcodes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Decoder;

unsafe impl AsyncSignalSafe for Decoder {}

impl Decoder {
    /// Decodes a raw 32-bit instruction word into a strongly-typed `Instruction`.
    ///
    /// Returns `Err(DecodeError::Undefined)` if opcode does not match any known instruction.
    /// Returns `Err(DecodeError::Reserved)` if register fields or encodings are reserved by ISA.
    pub fn decode(raw: u32) -> Result<Instruction, DecodeError> {
        if let Some(inst) = decode_branch(raw)? {
            return Ok(inst);
        }
        Err(DecodeError::Undefined(raw))
    }
}
