//! Memory interface mock returning memory faults for all operations.

use crate::core::error::{AccessKind, ExecError};
use crate::core::traits::{AsyncSignalSafe, MemoryInterface};

/// Mock MemoryInterface that returns `ExecError::MemoryFault` for all memory reads and writes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NoopMem;

impl MemoryInterface for NoopMem {
    fn read_u8(&mut self, addr: u64) -> Result<u8, ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Read,
        })
    }

    fn read_u16(&mut self, addr: u64) -> Result<u16, ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Read,
        })
    }

    fn read_u32(&mut self, addr: u64) -> Result<u32, ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Read,
        })
    }

    fn read_u64(&mut self, addr: u64) -> Result<u64, ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Read,
        })
    }

    fn read_u128(&mut self, addr: u64) -> Result<u128, ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Read,
        })
    }

    fn write_u8(&mut self, addr: u64, _val: u8) -> Result<(), ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Write,
        })
    }

    fn write_u16(&mut self, addr: u64, _val: u16) -> Result<(), ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Write,
        })
    }

    fn write_u32(&mut self, addr: u64, _val: u32) -> Result<(), ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Write,
        })
    }

    fn write_u64(&mut self, addr: u64, _val: u64) -> Result<(), ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Write,
        })
    }

    fn write_u128(&mut self, addr: u64, _val: u128) -> Result<(), ExecError> {
        Err(ExecError::MemoryFault {
            address: addr,
            kind: AccessKind::Write,
        })
    }
}

unsafe impl AsyncSignalSafe for NoopMem {}
