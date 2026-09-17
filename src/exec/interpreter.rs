//! AArch64 transactional instruction execution engine (Interpreter).
//!
//! Enforces transactional state modifications and the faulting PC invariant:
//! state updates are committed if and only if execution completes with `Ok(())`.

use crate::core::error::ExecError;
use crate::core::reg::Gpr;
use crate::core::traits::{AsyncSignalSafe, MemoryInterface, RegisterBank};
use crate::decode::{Decoder, Instruction};

/// Transactional instruction interpreter parameterized over register and memory semantics.
pub struct Interpreter<R: RegisterBank, M: MemoryInterface> {
    /// Register bank state.
    pub regs: R,
    /// Memory interface state.
    pub mem: M,
}

impl<R: RegisterBank, M: MemoryInterface> Interpreter<R, M> {
    /// Constructs a new `Interpreter` with given register bank and memory interface.
    pub fn new(regs: R, mem: M) -> Self {
        Self { regs, mem }
    }

    /// Executes a single decoded `Instruction`.
    ///
    /// # Transactional Invariants & Faulting PC
    ///
    /// Execution adheres strictly to atomic commit semantics:
    /// - If target address validation fails (e.g. `ExecError::UnalignedPc`),
    ///   no architectural state modifications (PC or general-purpose registers including X30)
    ///   are committed to `regs` or `mem`.
    /// - On `Ok(())`, state changes (e.g., link register writeback and target PC) commit.
    pub fn step(&mut self, inst: &Instruction) -> Result<(), ExecError> {
        match *inst {
            Instruction::B { offset } => {
                let pc = self.regs.get_pc();
                let target = pc.wrapping_add(offset as u64);
                if target & 0x3 != 0 {
                    return Err(ExecError::UnalignedPc(target));
                }
                self.regs.set_pc(target);
                Ok(())
            }
            Instruction::Bl { offset } => {
                let pc = self.regs.get_pc();
                let target = pc.wrapping_add(offset as u64);
                if target & 0x3 != 0 {
                    return Err(ExecError::UnalignedPc(target));
                }
                let lr = Gpr::new(30).ok_or(ExecError::InternalInvariantViolated)?;
                self.regs.write_x(lr, pc.wrapping_add(4));
                self.regs.set_pc(target);
                Ok(())
            }
            Instruction::Br { rn } => {
                let target = self.regs.read_x(rn);
                if target & 0x3 != 0 {
                    return Err(ExecError::UnalignedPc(target));
                }
                self.regs.set_pc(target);
                Ok(())
            }
            Instruction::Blr { rn } => {
                let target = self.regs.read_x(rn);
                if target & 0x3 != 0 {
                    return Err(ExecError::UnalignedPc(target));
                }
                let pc = self.regs.get_pc();
                let lr = Gpr::new(30).ok_or(ExecError::InternalInvariantViolated)?;
                self.regs.write_x(lr, pc.wrapping_add(4));
                self.regs.set_pc(target);
                Ok(())
            }
            Instruction::Ret { rn } => {
                let target = self.regs.read_x(rn);
                if target & 0x3 != 0 {
                    return Err(ExecError::UnalignedPc(target));
                }
                self.regs.set_pc(target);
                Ok(())
            }
        }
    }

    /// Decodes a raw 32-bit AArch64 opcode word and executes the decoded instruction.
    pub fn step_raw(&mut self, raw: u32) -> Result<(), ExecError> {
        let inst = Decoder::decode(raw)?;
        self.step(&inst)
    }
}

unsafe impl<R, M> AsyncSignalSafe for Interpreter<R, M>
where
    R: RegisterBank + AsyncSignalSafe,
    M: MemoryInterface + AsyncSignalSafe,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{NoopMem, VirtualRegs};

    #[test]
    fn test_exec_b() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        interp.regs.set_pc(0x1000);

        // B +8
        interp.step(&Instruction::B { offset: 8 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x1008);

        // B -16
        interp.step(&Instruction::B { offset: -16 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x0FF8);
    }

    #[test]
    fn test_exec_bl() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        interp.regs.set_pc(0x2000);
        let x30 = Gpr::new(30).unwrap();

        // BL +32
        interp.step(&Instruction::Bl { offset: 32 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x2020);
        assert_eq!(interp.regs.read_x(x30), 0x2004);
    }

    #[test]
    fn test_exec_br() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        interp.regs.set_pc(0x1000);
        let x1 = Gpr::new(1).unwrap();
        interp.regs.write_x(x1, 0x4000);

        // BR X1
        interp.step(&Instruction::Br { rn: x1 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x4000);
    }

    #[test]
    fn test_exec_blr() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        interp.regs.set_pc(0x1000);
        let x2 = Gpr::new(2).unwrap();
        let x30 = Gpr::new(30).unwrap();
        interp.regs.write_x(x2, 0x5000);

        // BLR X2
        interp.step(&Instruction::Blr { rn: x2 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x5000);
        assert_eq!(interp.regs.read_x(x30), 0x1004);
    }

    #[test]
    fn test_exec_ret() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        interp.regs.set_pc(0x1000);
        let x30 = Gpr::new(30).unwrap();
        let x5 = Gpr::new(5).unwrap();

        // RET X30
        interp.regs.write_x(x30, 0x8000);
        interp.step(&Instruction::Ret { rn: x30 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x8000);

        // RET X5
        interp.regs.write_x(x5, 0x9000);
        interp.step(&Instruction::Ret { rn: x5 }).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x9000);
    }

    #[test]
    fn test_exec_step_raw() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        interp.regs.set_pc(0x1000);

        // B +8 raw (0x1400_0002)
        interp.step_raw(0x1400_0002).unwrap();
        assert_eq!(interp.regs.get_pc(), 0x1008);
    }

    #[test]
    fn test_unaligned_pc_faulting_pc_invariant() {
        let mut interp = Interpreter::new(VirtualRegs::new(), NoopMem);
        let x30 = Gpr::new(30).unwrap();
        let x1 = Gpr::new(1).unwrap();

        // 1. Unaligned B target
        interp.regs.set_pc(0x1000);
        let res_b = interp.step(&Instruction::B { offset: 3 }); // 0x1003
        assert_eq!(res_b, Err(ExecError::UnalignedPc(0x1003)));
        assert_eq!(interp.regs.get_pc(), 0x1000); // PC unchanged

        // 2. Unaligned BL target - verify X30 and PC unchanged
        interp.regs.set_pc(0x2000);
        interp.regs.write_x(x30, 0x1111);
        let res_bl = interp.step(&Instruction::Bl { offset: 1 }); // 0x2001
        assert_eq!(res_bl, Err(ExecError::UnalignedPc(0x2001)));
        assert_eq!(interp.regs.get_pc(), 0x2000); // PC unchanged
        assert_eq!(interp.regs.read_x(x30), 0x1111); // X30 unchanged

        // 3. Unaligned BR target
        interp.regs.set_pc(0x3000);
        interp.regs.write_x(x1, 0x3002);
        let res_br = interp.step(&Instruction::Br { rn: x1 });
        assert_eq!(res_br, Err(ExecError::UnalignedPc(0x3002)));
        assert_eq!(interp.regs.get_pc(), 0x3000); // PC unchanged

        // 4. Unaligned BLR target - verify X30 and PC unchanged
        interp.regs.set_pc(0x4000);
        interp.regs.write_x(x1, 0x5003);
        interp.regs.write_x(x30, 0x2222);
        let res_blr = interp.step(&Instruction::Blr { rn: x1 });
        assert_eq!(res_blr, Err(ExecError::UnalignedPc(0x5003)));
        assert_eq!(interp.regs.get_pc(), 0x4000); // PC unchanged
        assert_eq!(interp.regs.read_x(x30), 0x2222); // X30 unchanged

        // 5. Unaligned RET target
        interp.regs.set_pc(0x6000);
        interp.regs.write_x(x30, 0x7001);
        let res_ret = interp.step(&Instruction::Ret { rn: x30 });
        assert_eq!(res_ret, Err(ExecError::UnalignedPc(0x7001)));
        assert_eq!(interp.regs.get_pc(), 0x6000); // PC unchanged
    }
}
