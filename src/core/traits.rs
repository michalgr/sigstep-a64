//! Interfaces for register access, memory operations, and async-signal safety.
//!
//! Defines `RegisterBank`, `MemoryInterface`, and `AsyncSignalSafe` traits.

use crate::core::error::{ExecError, MemoryOrdering};
use crate::core::reg::{CoreReg, Gpr, Nzcv, VReg};

/// Abstract view of AArch64 architectural registers.
///
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a), Section B1.2
pub trait RegisterBank {
    /// Read a 64-bit general-purpose physical register (X0–X30).
    fn read_x(&self, reg: Gpr) -> u64;

    /// Write a 64-bit general-purpose physical register (X0–X30).
    fn write_x(&mut self, reg: Gpr, val: u64);

    /// Read the current Stack Pointer (SP).
    fn read_sp(&self) -> u64;

    /// Write the current Stack Pointer (SP).
    fn write_sp(&mut self, val: u64);

    /// Read 64-bit value from decoded `CoreReg` operand.
    fn read_core_x(&self, reg: CoreReg) -> u64 {
        match reg {
            CoreReg::Reg(r) => self.read_x(r),
            CoreReg::Zr => 0,
            CoreReg::Sp => self.read_sp(),
        }
    }

    /// Read 32-bit value from decoded `CoreReg` operand.
    fn read_core_w(&self, reg: CoreReg) -> u32 {
        self.read_core_x(reg) as u32
    }

    /// Write 64-bit value to decoded `CoreReg` operand.
    fn write_core_x(&mut self, reg: CoreReg, val: u64) {
        match reg {
            CoreReg::Reg(r) => self.write_x(r, val),
            CoreReg::Zr => {}
            CoreReg::Sp => self.write_sp(val),
        }
    }

    /// Write 32-bit value to decoded `CoreReg` operand (zero-extending per Arm ARM B1.2.1).
    fn write_core_w(&mut self, reg: CoreReg, val: u32) {
        self.write_core_x(reg, val as u64);
    }

    /// Read the current Program Counter (PC).
    fn get_pc(&self) -> u64;

    /// Write the current Program Counter (PC).
    fn set_pc(&mut self, val: u64);

    /// Advance the Program Counter by standard instruction offset (defaults to +offset).
    #[inline]
    fn advance_pc(&mut self, offset: i64) {
        self.set_pc(self.get_pc().wrapping_add(offset as u64));
    }

    /// Read the NZCV condition flags.
    fn get_flags(&self) -> Nzcv;

    /// Update the NZCV condition flags.
    fn set_flags(&mut self, flags: Nzcv);

    /// Read a 128-bit SIMD / Floating-Point register (V0–V31).
    fn read_v(&self, reg: VReg) -> Result<u128, ExecError> {
        let _ = reg;
        Err(ExecError::SimdNotSupported)
    }

    /// Write a 128-bit SIMD / Floating-Point register (V0–V31).
    fn write_v(&mut self, reg: VReg, val: u128) -> Result<(), ExecError> {
        let _ = (reg, val);
        Err(ExecError::SimdNotSupported)
    }
}

/// Abstraction for address space inspection and modification.
pub trait MemoryInterface {
    // --- Primitive Byte / Word Reads (Little-Endian) ---

    /// Read an unsigned 8-bit byte from `addr`.
    fn read_u8(&mut self, addr: u64) -> Result<u8, ExecError>;

    /// Read an unsigned 16-bit word from `addr`.
    fn read_u16(&mut self, addr: u64) -> Result<u16, ExecError>;

    /// Read an unsigned 32-bit word from `addr`.
    fn read_u32(&mut self, addr: u64) -> Result<u32, ExecError>;

    /// Read an unsigned 64-bit doubleword from `addr`.
    fn read_u64(&mut self, addr: u64) -> Result<u64, ExecError>;

    /// Read an unsigned 128-bit quadword from `addr`.
    fn read_u128(&mut self, addr: u64) -> Result<u128, ExecError>;

    // --- Primitive Byte / Word Writes (Little-Endian) ---

    /// Write an unsigned 8-bit byte to `addr`.
    fn write_u8(&mut self, addr: u64, val: u8) -> Result<(), ExecError>;

    /// Write an unsigned 16-bit word to `addr`.
    fn write_u16(&mut self, addr: u64, val: u16) -> Result<(), ExecError>;

    /// Write an unsigned 32-bit word to `addr`.
    fn write_u32(&mut self, addr: u64, val: u32) -> Result<(), ExecError>;

    /// Write an unsigned 64-bit doubleword to `addr`.
    fn write_u64(&mut self, addr: u64, val: u64) -> Result<(), ExecError>;

    /// Write an unsigned 128-bit quadword to `addr`.
    fn write_u128(&mut self, addr: u64, val: u128) -> Result<(), ExecError>;

    // --- Atomic Operations (LSE Atomics - Armv8.1-A) ---

    /// Atomic Compare-and-Swap (32-bit).
    fn atomic_cas_u32(
        &mut self,
        addr: u64,
        expected: u32,
        new_val: u32,
        order: MemoryOrdering,
    ) -> Result<u32, ExecError> {
        let _ = (addr, expected, new_val, order);
        Err(ExecError::AtomicNotSupported)
    }

    /// Atomic Compare-and-Swap (64-bit).
    fn atomic_cas_u64(
        &mut self,
        addr: u64,
        expected: u64,
        new_val: u64,
        order: MemoryOrdering,
    ) -> Result<u64, ExecError> {
        let _ = (addr, expected, new_val, order);
        Err(ExecError::AtomicNotSupported)
    }

    /// Atomic Swap / Exchange (64-bit).
    fn atomic_swap_u64(
        &mut self,
        addr: u64,
        val: u64,
        order: MemoryOrdering,
    ) -> Result<u64, ExecError> {
        let _ = (addr, val, order);
        Err(ExecError::AtomicNotSupported)
    }

    /// Atomic Fetch-and-Add (64-bit).
    fn atomic_fetch_add_u64(
        &mut self,
        addr: u64,
        val: u64,
        order: MemoryOrdering,
    ) -> Result<u64, ExecError> {
        let _ = (addr, val, order);
        Err(ExecError::AtomicNotSupported)
    }

    // --- Exclusive Monitor Interface (LL / SC - LDXR / STXR) ---

    /// Tag an address in the local exclusive monitor (e.g., on LDXR).
    fn mark_exclusive(&mut self, addr: u64, size: usize) -> Result<(), ExecError> {
        let _ = (addr, size);
        Ok(())
    }

    /// Attempt to store conditionally to an address (32-bit, e.g., on STXR).
    fn try_store_exclusive_u32(&mut self, addr: u64, val: u32) -> Result<bool, ExecError> {
        let _ = (addr, val);
        Err(ExecError::ExclusiveMonitorNotSupported)
    }

    /// Attempt to store conditionally to an address (64-bit, e.g., on STXR).
    fn try_store_exclusive_u64(&mut self, addr: u64, val: u64) -> Result<bool, ExecError> {
        let _ = (addr, val);
        Err(ExecError::ExclusiveMonitorNotSupported)
    }

    /// Explicitly clear the local exclusive monitor (e.g., on CLREX).
    fn clear_exclusive(&mut self) {
        // Default no-op
    }
}

/// Marker trait guaranteeing that all operations exposed by the implementor
/// are reentrant, non-blocking, non-allocating, and safe to execute within
/// an asynchronous POSIX signal handler (e.g., SIGILL, SIGTRAP, SIGSEGV).
///
/// # Safety
///
/// An implementor of `AsyncSignalSafe` guarantees the following invariants:
/// 1. **Zero Heap Allocation:** Never invokes the global allocator or any dynamic heap subsystem.
/// 2. **No Non-Reentrant Locks:** Never acquires mutexes, rwlocks, non-reentrant spinlocks,
///    or sleeps/parks threads.
/// 3. **Bounded Stack Usage:** Guaranteed maximum call stack depth (< 1 KB) compatible with
///    constrained `sigaltstack` frames.
/// 4. **Safe Fault Handling:** Never causes unrecoverable secondary faults during memory probing.
/// 5. **Deterministic Termination:** All operations complete in bounded O(1) time.
pub unsafe trait AsyncSignalSafe {}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRegs {
        gpr: [u64; 31],
        sp: u64,
        pc: u64,
        flags: Nzcv,
    }

    impl MockRegs {
        fn new() -> Self {
            Self {
                gpr: [0; 31],
                sp: 0,
                pc: 0,
                flags: Nzcv::default(),
            }
        }
    }

    impl RegisterBank for MockRegs {
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

    #[test]
    fn test_write_core_w_zero_extension() {
        let mut regs = MockRegs::new();
        let gpr0 = Gpr::new(0).unwrap();
        let reg0 = CoreReg::Reg(gpr0);

        regs.write_x(gpr0, 0xFFFF_FFFF_FFFF_FFFF);
        assert_eq!(regs.read_x(gpr0), 0xFFFF_FFFF_FFFF_FFFF);

        regs.write_core_w(reg0, 0x1234_5678);
        assert_eq!(regs.read_x(gpr0), 0x0000_0000_1234_5678);
        assert_eq!(regs.read_core_w(reg0), 0x1234_5678);
        assert_eq!(regs.read_core_x(reg0), 0x0000_0000_1234_5678);
    }

    #[test]
    fn test_zero_and_stack_pointer() {
        let mut regs = MockRegs::new();

        assert_eq!(regs.read_core_x(CoreReg::Zr), 0);
        assert_eq!(regs.read_core_w(CoreReg::Zr), 0);
        regs.write_core_x(CoreReg::Zr, 0xDEAD_BEEF);
        assert_eq!(regs.read_core_x(CoreReg::Zr), 0);
        regs.write_core_w(CoreReg::Zr, 0x1234_5678);
        assert_eq!(regs.read_core_x(CoreReg::Zr), 0);

        regs.write_core_x(CoreReg::Sp, 0x1000_0000_0000_0000);
        assert_eq!(regs.read_core_x(CoreReg::Sp), 0x1000_0000_0000_0000);
        assert_eq!(regs.read_sp(), 0x1000_0000_0000_0000);

        regs.write_core_w(CoreReg::Sp, 0x8000_0000);
        assert_eq!(regs.read_core_x(CoreReg::Sp), 0x0000_0000_8000_0000);
    }

    #[test]
    fn test_advance_pc() {
        let mut regs = MockRegs::new();
        regs.set_pc(0x1000);
        regs.advance_pc(4);
        assert_eq!(regs.get_pc(), 0x1004);
        regs.advance_pc(-8);
        assert_eq!(regs.get_pc(), 0x0FFC);

        // Boundary cases for high bits / signed overflow protection
        regs.set_pc(0x8000_0000_0000_0000);
        regs.advance_pc(-4);
        assert_eq!(regs.get_pc(), 0x7FFF_FFFF_FFFF_FFFC);

        regs.set_pc(0xFFFF_FFFF_FFFF_FFFC);
        regs.advance_pc(4);
        assert_eq!(regs.get_pc(), 0x0000_0000_0000_0000);
    }
}
