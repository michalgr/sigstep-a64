//! Core error taxonomies and execution classifications.
//!
//! Defines error types for instruction decoding (`DecodeError`) and execution (`ExecError`),
//! as well as memory access classifications (`AccessKind`) and memory ordering semantics (`MemoryOrdering`).

use core::fmt;

/// Memory access permissions and classification.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccessKind {
    /// Read access.
    Read,
    /// Write access.
    Write,
    /// Instruction execution/fetch access.
    Execute,
}

impl fmt::Display for AccessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessKind::Read => write!(f, "Read"),
            AccessKind::Write => write!(f, "Write"),
            AccessKind::Execute => write!(f, "Execute"),
        }
    }
}

/// Standard atomic memory ordering semantics (aligned with C11 / Rust `core::sync::atomic`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemoryOrdering {
    /// Relaxed ordering.
    Relaxed,
    /// Acquire ordering.
    Acquire,
    /// Release ordering.
    Release,
    /// Acquire-Release ordering.
    AcqRel,
    /// Sequentially consistent ordering.
    SeqCst,
}

impl fmt::Display for MemoryOrdering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryOrdering::Relaxed => write!(f, "Relaxed"),
            MemoryOrdering::Acquire => write!(f, "Acquire"),
            MemoryOrdering::Release => write!(f, "Release"),
            MemoryOrdering::AcqRel => write!(f, "AcqRel"),
            MemoryOrdering::SeqCst => write!(f, "SeqCst"),
        }
    }
}

/// Errors occurring during instruction decoding.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Undefined(op) => write!(f, "Undefined instruction opcode: {:#010x}", op),
            DecodeError::Reserved(op) => write!(f, "Reserved instruction opcode: {:#010x}", op),
            DecodeError::Unallocated(op) => {
                write!(f, "Unallocated instruction opcode: {:#010x}", op)
            }
            DecodeError::Unpredictable(op) => {
                write!(f, "Unpredictable instruction encoding: {:#010x}", op)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

/// Errors occurring during instruction execution.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExecError {
    /// Failed memory access due to unmapped memory, page fault, or protection violation.
    MemoryFault {
        /// Faulting memory address.
        address: u64,
        /// Access type (Read, Write, or Execute).
        kind: AccessKind,
    },
    /// Memory access violated hardware or software alignment restrictions.
    AlignmentFault {
        /// Faulting memory address.
        address: u64,
        /// Required alignment in bytes.
        required_alignment: usize,
    },
    /// Program Counter was set to an unaligned address (AArch64 requires 4-byte alignment).
    UnalignedPc(u64),
    /// Instruction decoding failed.
    Decode(DecodeError),
    /// Instruction is architecturally undefined.
    UndefinedInstruction(u32),
    /// Valid instruction encoding that is not yet implemented by the interpreter.
    UnimplementedInstruction(u32),
    /// Encountered a breakpoint instruction (BRK / HLT).
    Breakpoint {
        /// Immediate comment field in breakpoint encoding.
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

impl From<DecodeError> for ExecError {
    #[inline]
    fn from(err: DecodeError) -> Self {
        ExecError::Decode(err)
    }
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::MemoryFault { address, kind } => {
                write!(
                    f,
                    "Memory fault at address {:#018x} during {}",
                    address, kind
                )
            }
            ExecError::AlignmentFault {
                address,
                required_alignment,
            } => write!(
                f,
                "Alignment fault at address {:#018x} (required alignment: {} bytes)",
                address, required_alignment
            ),
            ExecError::UnalignedPc(pc) => write!(f, "Unaligned Program Counter: {:#018x}", pc),
            ExecError::Decode(err) => write!(f, "Decode error: {}", err),
            ExecError::UndefinedInstruction(op) => {
                write!(f, "Undefined instruction opcode: {:#010x}", op)
            }
            ExecError::UnimplementedInstruction(op) => {
                write!(f, "Unimplemented instruction opcode: {:#010x}", op)
            }
            ExecError::Breakpoint { comment } => {
                write!(
                    f,
                    "Breakpoint instruction executed (comment: {:#06x})",
                    comment
                )
            }
            ExecError::StepComplete => write!(f, "Step completed"),
            ExecError::AtomicNotSupported => write!(f, "Atomic operations not supported"),
            ExecError::ExclusiveMonitorNotSupported => {
                write!(f, "Exclusive monitor operations not supported")
            }
            ExecError::SimdNotSupported => write!(f, "SIMD/FP operations not supported"),
            ExecError::InternalInvariantViolated => {
                write!(f, "Internal execution invariant violated")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecError::Decode(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_kind_display() {
        use core::fmt::Write;
        struct Buffer([u8; 64], usize);
        impl Write for Buffer {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                let bytes = s.as_bytes();
                let rem = self.0.len() - self.1;
                let to_copy = bytes.len().min(rem);
                self.0[self.1..self.1 + to_copy].copy_from_slice(&bytes[..to_copy]);
                self.1 += to_copy;
                Ok(())
            }
        }

        let mut buf = Buffer([0; 64], 0);
        let _ = write!(buf, "{}", AccessKind::Read);
        let s = core::str::from_utf8(&buf.0[..buf.1]).unwrap();
        assert_eq!(s, "Read");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_error_display_and_source() {
        use std::error::Error;
        use std::string::ToString;

        let dec_err = DecodeError::Undefined(0xD503201F);
        let dec_str = std::format!("{}", dec_err);
        assert!(dec_str.contains("0xd503201f"));

        let exec_err = ExecError::Decode(dec_err);
        assert_eq!(exec_err.source().map(|e| e.to_string()), Some(dec_str));

        let mem_err = ExecError::MemoryFault {
            address: 0x1000,
            kind: AccessKind::Read,
        };
        assert!(std::format!("{}", mem_err).contains("0x0000000000001000"));
        assert!(std::format!("{}", mem_err).contains("Read"));

        let align_err = ExecError::AlignmentFault {
            address: 0x1001,
            required_alignment: 8,
        };
        assert!(std::format!("{}", align_err).contains("8 bytes"));
    }

    #[test]
    fn test_memory_ordering_display() {
        use std::string::ToString;
        assert_eq!(MemoryOrdering::Relaxed.to_string(), "Relaxed");
        assert_eq!(MemoryOrdering::Acquire.to_string(), "Acquire");
        assert_eq!(MemoryOrdering::Release.to_string(), "Release");
        assert_eq!(MemoryOrdering::AcqRel.to_string(), "AcqRel");
        assert_eq!(MemoryOrdering::SeqCst.to_string(), "SeqCst");
    }

    #[test]
    fn test_decode_error_try_propagation() {
        fn decode_op(op: u32) -> Result<(), DecodeError> {
            Err(DecodeError::Undefined(op))
        }

        fn execute_op(op: u32) -> Result<(), ExecError> {
            decode_op(op)?;
            Ok(())
        }

        let err = execute_op(0x12345678).unwrap_err();
        assert_eq!(err, ExecError::Decode(DecodeError::Undefined(0x12345678)));
    }

    #[test]
    fn test_error_hash_and_sets() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(AccessKind::Read);
        set.insert(AccessKind::Write);
        assert!(set.contains(&AccessKind::Read));

        let mut mo_set = HashSet::new();
        mo_set.insert(MemoryOrdering::Acquire);
        assert!(mo_set.contains(&MemoryOrdering::Acquire));

        let mut decode_set = HashSet::new();
        decode_set.insert(DecodeError::Undefined(0));
        assert!(decode_set.contains(&DecodeError::Undefined(0)));

        let mut exec_set = HashSet::new();
        exec_set.insert(ExecError::AtomicNotSupported);
        assert!(exec_set.contains(&ExecError::AtomicNotSupported));
    }
}
