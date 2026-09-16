# Architecture Specification: Traits, Memory Semantics & Safety Invariants

This document formalizes the interface contracts for `sigstep-a64`, defining the register bank abstraction, memory interface semantics (including atomic operations and exclusive monitors), error taxonomies, and the `AsyncSignalSafe` contract.

---

## 1. Safety Abstraction: `AsyncSignalSafe`

POSIX signal handlers operate asynchronously in an arbitrary thread context. They can interrupt a thread at any point in its execution, including within the critical sections of libc allocators or internal runtime synchronization primitives.

### 1.1 The Unsafe Marker Trait Contract

```rust
/// Marker trait guaranteeing that all operations exposed by the implementor
/// are reentrant, non-blocking, non-allocating, and safe to execute within
/// an asynchronous POSIX signal handler (e.g., SIGILL, SIGTRAP, SIGSEGV).
///
/// # Safety
///
/// An implementor of `AsyncSignalSafe` guarantees the following invariants:
/// 1. **Zero Heap Allocation:** Never invokes the global allocator or any
///    dynamic heap subsystem.
/// 2. **No Non-Reentrant Locks:** Never acquires mutexes, rwlocks, non-reentrant
///    spinlocks, or sleeps/parks threads.
/// 3. **Bounded Stack Usage:** Guaranteed maximum call stack depth (< 1 KB)
///    compatible with constrained `sigaltstack` frames.
/// 4. **Safe Fault Handling:** Never causes unrecoverable secondary faults
///    (e.g., recursive SIGSEGV without recovery) during memory probing.
/// 5. **Deterministic Termination:** All operations complete in bounded O(1) time.
pub unsafe trait AsyncSignalSafe {}
```

### 1.2 Blanket Composition

The execution engine is an induction over its constituent dependencies:

```rust
pub struct Interpreter<R: RegisterBank, M: MemoryInterface> {
    pub regs: R,
    pub mem: M,
}

unsafe impl<R, M> AsyncSignalSafe for Interpreter<R, M>
where
    R: RegisterBank + AsyncSignalSafe,
    M: MemoryInterface + AsyncSignalSafe,
{}
```

This ensures that passing a test backend (such as a mock using `std::collections::HashMap` or `Vec<u8>`) to a signal handler is rejected at **compile time**.

---

## 2. Register Bank Interface: `RegisterBank`

The `RegisterBank` trait encapsulates architectural state for AArch64 processing elements. For complete register representation details, physical storage bounded types, decoded operand disambiguation, and width-agnostic design, see [Register Representation & Disambiguation](register-representation.md).

### 2.1 PSTATE NZCV Condition Flags

```rust
/// Condition flags from PSTATE (NZCV bits 31:28).
///
/// Reference: Arm ARM DDI 0487K.a, Section C5.2.10 "NZCV, Condition Flags"
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Nzcv {
    /// Negative condition flag (Bit 31)
    pub n: bool,
    /// Zero condition flag (Bit 30)
    pub z: bool,
    /// Carry condition flag (Bit 29)
    pub c: bool,
    /// Overflow condition flag (Bit 28)
    pub v: bool,
}

impl Nzcv {
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            n: (raw & (1 << 31)) != 0,
            z: (raw & (1 << 30)) != 0,
            c: (raw & (1 << 29)) != 0,
            v: (raw & (1 << 28)) != 0,
        }
    }

    pub const fn to_raw(self) -> u32 {
        ((self.n as u32) << 31)
            | ((self.z as u32) << 30)
            | ((self.c as u32) << 29)
            | ((self.v as u32) << 28)
    }
}
```

### 2.2 Register Bank Trait Definition

```rust
/// Abstract view of AArch64 architectural registers.
pub trait RegisterBank {
    /// Read a 64-bit general-purpose physical register (X0–X30).
    fn read_x(&self, reg: Gpr) -> u64;

    /// Write a 64-bit general-purpose physical register (X0–X30).
    fn write_x(&mut self, reg: Gpr, val: u64);

    /// Read the current Stack Pointer (SP).
    fn read_sp(&self) -> u64;

    /// Write the current Stack Pointer (SP).
    fn write_sp(&mut self, val: u64);

    /// Read 64-bit value from decoded CoreReg operand.
    fn read_core_x(&self, reg: CoreReg) -> u64 {
        match reg {
            CoreReg::Reg(r) => self.read_x(r),
            CoreReg::Zr => 0,
            CoreReg::Sp => self.read_sp(),
        }
    }

    /// Read 32-bit value from decoded CoreReg operand.
    fn read_core_w(&self, reg: CoreReg) -> u32 {
        self.read_core_x(reg) as u32
    }

    /// Write 64-bit value to decoded CoreReg operand.
    fn write_core_x(&mut self, reg: CoreReg, val: u64) {
        match reg {
            CoreReg::Reg(r) => self.write_x(r, val),
            CoreReg::Zr => {},
            CoreReg::Sp => self.write_sp(val),
        }
    }

    /// Write 32-bit value to decoded CoreReg operand (zero-extending per Arm ARM B1.2.1).
    fn write_core_w(&mut self, reg: CoreReg, val: u32) {
        self.write_core_x(reg, val as u64);
    }

    /// Read the current Program Counter (PC).
    fn get_pc(&self) -> u64;

    /// Write the current Program Counter (PC).
    fn set_pc(&mut self, val: u64);

    /// Advance the Program Counter by standard instruction offset (defaults to +4).
    fn advance_pc(&mut self, offset: i64) {
        self.set_pc((self.get_pc() as i64 + offset) as u64);
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
```

---

## 3. Memory Interface: `MemoryInterface`

The `MemoryInterface` trait abstracts byte-level and atomic memory accesses. AArch64 standard user-space operates with **little-endian** byte ordering.

### 3.1 Memory Ordering & Access Kinds

```rust
/// Memory access permissions and classification.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
    Execute,
}

/// Standard atomic memory ordering semantics (aligned with C11 / Rust core::sync::atomic).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryOrdering {
    Relaxed,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}
```

### 3.2 Trait Definition with Atomics and Exclusive Monitors

```rust
/// Abstraction for address space inspection and modification.
pub trait MemoryInterface {
    // --- Primitive Byte / Word Reads (Little-Endian) ---

    fn read_u8(&mut self, addr: u64) -> Result<u8, ExecError>;
    fn read_u16(&mut self, addr: u64) -> Result<u16, ExecError>;
    fn read_u32(&mut self, addr: u64) -> Result<u32, ExecError>;
    fn read_u64(&mut self, addr: u64) -> Result<u64, ExecError>;
    fn read_u128(&mut self, addr: u64) -> Result<u128, ExecError>;

    // --- Primitive Byte / Word Writes (Little-Endian) ---

    fn write_u8(&mut self, addr: u64, val: u8) -> Result<(), ExecError>;
    fn write_u16(&mut self, addr: u64, val: u16) -> Result<(), ExecError>;
    fn write_u32(&mut self, addr: u64, val: u32) -> Result<(), ExecError>;
    fn write_u64(&mut self, addr: u64, val: u64) -> Result<(), ExecError>;
    fn write_u128(&mut self, addr: u64, val: u128) -> Result<(), ExecError>;

    // --- Atomic Operations (LSE Atomics - Armv8.1-A) ---
    //
    // On native AArch64 hardware inside signal handlers, these map directly
    // to hardware atomic instructions which are lock-free and async-signal safe.

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
    //
    // Emulates local exclusive monitor state without heap allocation.

    /// Tag an address in the local exclusive monitor (e.g., on LDXR).
    fn mark_exclusive(&mut self, addr: u64, size: usize) -> Result<(), ExecError> {
        let _ = (addr, size);
        Ok(())
    }

    /// Attempt to store conditionally to an address (e.g., on STXR).
    ///
    /// Returns `Ok(true)` if the store succeeded (monitor held and cleared).
    /// Returns `Ok(false)` if the reservation was lost (store did not occur).
    fn try_store_exclusive_u32(
        &mut self,
        addr: u64,
        val: u32,
    ) -> Result<bool, ExecError> {
        let _ = (addr, val);
        Err(ExecError::ExclusiveMonitorNotSupported)
    }

    /// Attempt to store conditionally to an address (64-bit).
    fn try_store_exclusive_u64(
        &mut self,
        addr: u64,
        val: u64,
    ) -> Result<bool, ExecError> {
        let _ = (addr, val);
        Err(ExecError::ExclusiveMonitorNotSupported)
    }

    /// Explicitly clear the local exclusive monitor (e.g., on CLREX).
    fn clear_exclusive(&mut self) {
        // Default no-op
    }
}
```

---

## 4. Error Taxonomies

To comply with the zero-allocation invariant, all errors are static, small, and derive `Copy, Clone, Debug, PartialEq, Eq`.

### 4.1 Execution Errors: `ExecError`

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExecError {
    /// Failed memory access due to unmapped memory, page fault, or protection violation.
    MemoryFault {
        address: u64,
        kind: AccessKind,
    },

    /// Memory access violated hardware or software alignment restrictions.
    AlignmentFault {
        address: u64,
        required_alignment: usize,
    },

    /// Program Counter was set to an unaligned address (AArch64 requires 4-byte alignment).
    UnalignedPc(u64),

    /// Attempted to read or write an illegal or out-of-range register identifier.
    IllegalRegister(u8),

    /// Instruction decoding failed.
    Decode(DecodeError),

    /// Instruction is architecturally undefined.
    UndefinedInstruction(u32),

    /// Valid instruction encoding that is not yet implemented by the interpreter.
    UnimplementedInstruction(u32),

    /// Encountered a breakpoint instruction (BRK / HLT).
    Breakpoint {
        comment: u16,
    },

    /// Software single-step completed successfully.
    StepComplete,

    /// Atomic operation is unsupported on this backend.
    AtomicNotSupported,

    /// Exclusive monitor operations (LDXR/STXR) unsupported on this backend.
    ExclusiveMonitorNotSupported,

    /// SIMD / Floating-point operations unsupported on this backend.
    SimdNotSupported,

    /// Internal logical invariant violated (replaces panics).
    InternalInvariantViolated,
}
```

### 4.2 Decoding Errors: `DecodeError`

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Raw 32-bit opcode does not match any valid AArch64 instruction encoding.
    Undefined(u32),

    /// Opcode matches an architectural reserved encoding space.
    Reserved(u32),

    /// Opcode matches an unallocated encoding space within an instruction class.
    Unallocated(u32),

    /// Instruction fields contain an illegal or unpredictable combination of operands.
    Unpredictable(u32),
}
```
