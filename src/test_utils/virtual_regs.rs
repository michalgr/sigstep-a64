//! Virtual register bank implementation for testing and execution harnesses.

use crate::core::reg::{Gpr, Nzcv};
use crate::core::traits::{AsyncSignalSafe, RegisterBank};

/// Virtual register bank storing 31 general-purpose registers, SP, PC, and condition flags.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VirtualRegs {
    /// General-purpose physical registers X0–X30.
    pub gpr: [u64; 31],
    /// Stack Pointer (SP).
    pub sp: u64,
    /// Program Counter (PC).
    pub pc: u64,
    /// PSTATE condition flags (NZCV).
    pub flags: Nzcv,
}

impl VirtualRegs {
    /// Creates a new `VirtualRegs` instance with zeroed registers, SP, PC, and default flags.
    pub fn new() -> Self {
        Self::default()
    }
}

impl RegisterBank for VirtualRegs {
    fn read_x(&self, reg: Gpr) -> u64 {
        self.gpr[reg.index() as usize]
    }

    fn write_x(&mut self, reg: Gpr, val: u64) {
        self.gpr[reg.index() as usize] = val;
    }

    fn read_sp(&self) -> u64 {
        self.sp
    }

    fn write_sp(&mut self, val: u64) {
        self.sp = val;
    }

    fn get_pc(&self) -> u64 {
        self.pc
    }

    fn set_pc(&mut self, val: u64) {
        self.pc = val;
    }

    fn get_flags(&self) -> Nzcv {
        self.flags
    }

    fn set_flags(&mut self, flags: Nzcv) {
        self.flags = flags;
    }
}

unsafe impl AsyncSignalSafe for VirtualRegs {}
