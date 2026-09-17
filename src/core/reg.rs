//! Physical register representation and decoded core operands.
//!
//! Defines physical general-purpose register indices (`Gpr`), vector registers (`VReg`),
//! decoded core operand enums (`CoreReg`), and condition flags (`Nzcv`).

/// Bounded general-purpose register index strictly in 0..=30 (X0–X30 / W0–W30).
///
/// Range-checked to ensure index is strictly in 0..=30.
///
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a), Section B1.2.1
/// Canonical: https://developer.arm.com/documentation/102374/latest/Registers-in-AArch64---general-purpose-registers
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gpr(u8);

impl Gpr {
    /// Creates a new `Gpr` if `idx` is in the valid range `0..=30`.
    #[inline]
    pub const fn new(idx: u8) -> Option<Self> {
        if idx <= 30 {
            Some(Self(idx))
        } else {
            None
        }
    }

    /// Creates a new `Gpr` without range checking.
    ///
    /// # Safety
    /// Caller must guarantee `idx <= 30`.
    #[inline]
    pub const unsafe fn new_unchecked(idx: u8) -> Self {
        Self(idx)
    }

    /// Returns the raw zero-based physical index (0..=30).
    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Decoded core integer register operand.
///
/// Resolves the architectural identity of register field 31 during decoding.
///
/// Reference: Arm Architecture Procedure Call Standard for AArch64 (AAPCS64), Section 6.1.1
/// Canonical: https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreReg {
    /// Architectural general-purpose register X0..X30 (or W0..W30).
    Reg(Gpr),
    /// Zero Register (XZR / WZR): reads return 0, writes are discarded.
    Zr,
    /// Special Register: Stack Pointer (SP / WSP).
    Sp,
}

impl CoreReg {
    /// Resolves a 5-bit register field where index 31 represents XZR / WZR.
    #[inline]
    pub const fn from_raw_zr(raw_5bit: u32) -> Self {
        let idx = (raw_5bit & 0x1F) as u8;
        if idx == 31 {
            CoreReg::Zr
        } else {
            // SAFETY: idx & 0x1F is <= 31, and idx != 31 guarantees idx <= 30.
            CoreReg::Reg(unsafe { Gpr::new_unchecked(idx) })
        }
    }

    /// Resolves a 5-bit register field where index 31 represents SP / WSP.
    #[inline]
    pub const fn from_raw_sp(raw_5bit: u32) -> Self {
        let idx = (raw_5bit & 0x1F) as u8;
        if idx == 31 {
            CoreReg::Sp
        } else {
            // SAFETY: idx & 0x1F is <= 31, and idx != 31 guarantees idx <= 30.
            CoreReg::Reg(unsafe { Gpr::new_unchecked(idx) })
        }
    }
}

/// Bounded vector register index strictly in 0..=31 (V0–V31).
///
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a), Section B1.2.2
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VReg(u8);

impl VReg {
    /// Creates a new `VReg` if `idx` is in the valid range `0..=31`.
    #[inline]
    pub const fn new(idx: u8) -> Option<Self> {
        if idx <= 31 {
            Some(Self(idx))
        } else {
            None
        }
    }

    /// Creates a new `VReg` without range checking.
    ///
    /// # Safety
    /// Caller must guarantee `idx <= 31`.
    #[inline]
    pub const unsafe fn new_unchecked(idx: u8) -> Self {
        Self(idx)
    }

    /// Returns the raw zero-based physical index (0..=31).
    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }
}

/// Condition flags from PSTATE (NZCV bits 31:28).
///
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a), Section C5.2.10
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
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
    /// Constructs `Nzcv` flags from raw 32-bit PSTATE/NZCV word.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self {
            n: (raw & (1 << 31)) != 0,
            z: (raw & (1 << 30)) != 0,
            c: (raw & (1 << 29)) != 0,
            v: (raw & (1 << 28)) != 0,
        }
    }

    /// Serializes `Nzcv` flags into raw 32-bit PSTATE/NZCV word.
    #[inline]
    pub const fn to_raw(self) -> u32 {
        ((self.n as u32) << 31)
            | ((self.z as u32) << 30)
            | ((self.c as u32) << 29)
            | ((self.v as u32) << 28)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpr_bounds() {
        for i in 0..=30 {
            let gpr = Gpr::new(i).expect("valid Gpr index");
            assert_eq!(gpr.index(), i);
            let gpr_unchecked = unsafe { Gpr::new_unchecked(i) };
            assert_eq!(gpr_unchecked.index(), i);
        }
        for i in 31..=255 {
            assert_eq!(Gpr::new(i), None);
        }
    }

    #[test]
    fn test_vreg_bounds() {
        for i in 0..=31 {
            let vreg = VReg::new(i).expect("valid VReg index");
            assert_eq!(vreg.index(), i);
            let vreg_unchecked = unsafe { VReg::new_unchecked(i) };
            assert_eq!(vreg_unchecked.index(), i);
        }
        for i in 32..=255 {
            assert_eq!(VReg::new(i), None);
        }
    }

    #[test]
    fn test_corereg_from_raw() {
        for i in 0..30 {
            assert_eq!(
                CoreReg::from_raw_zr(i),
                CoreReg::Reg(Gpr::new(i as u8).unwrap())
            );
            assert_eq!(
                CoreReg::from_raw_sp(i),
                CoreReg::Reg(Gpr::new(i as u8).unwrap())
            );
        }
        assert_eq!(
            CoreReg::from_raw_zr(30),
            CoreReg::Reg(Gpr::new(30).unwrap())
        );
        assert_eq!(
            CoreReg::from_raw_sp(30),
            CoreReg::Reg(Gpr::new(30).unwrap())
        );

        assert_eq!(CoreReg::from_raw_zr(31), CoreReg::Zr);
        assert_eq!(CoreReg::from_raw_sp(31), CoreReg::Sp);

        assert_eq!(
            CoreReg::from_raw_zr(0x20),
            CoreReg::Reg(Gpr::new(0).unwrap())
        );
        assert_eq!(CoreReg::from_raw_zr(0x3F), CoreReg::Zr);
        assert_eq!(CoreReg::from_raw_sp(0x3F), CoreReg::Sp);
    }

    #[test]
    fn test_nzcv_roundtrip() {
        for n in [false, true] {
            for z in [false, true] {
                for c in [false, true] {
                    for v in [false, true] {
                        let flags = Nzcv { n, z, c, v };
                        let raw = flags.to_raw();
                        let roundtrip = Nzcv::from_raw(raw);
                        assert_eq!(flags, roundtrip);
                    }
                }
            }
        }
    }

    #[test]
    fn test_corereg_ord_and_hash() {
        use std::collections::BTreeSet;
        use std::collections::HashSet;
        use std::vec::Vec;

        let reg0 = CoreReg::Reg(Gpr::new(0).unwrap());
        let reg1 = CoreReg::Reg(Gpr::new(1).unwrap());
        let zr = CoreReg::Zr;
        let sp = CoreReg::Sp;

        assert!(reg0 < reg1);
        assert!(reg1 < zr);
        assert!(zr < sp);

        let mut set = HashSet::new();
        set.insert(reg0);
        set.insert(zr);
        set.insert(sp);
        assert!(set.contains(&reg0));

        let mut btree = BTreeSet::new();
        btree.insert(sp);
        btree.insert(zr);
        btree.insert(reg1);
        btree.insert(reg0);

        let vec: Vec<_> = btree.into_iter().collect();
        assert_eq!(vec, std::vec![reg0, reg1, zr, sp]);
    }

    #[test]
    fn test_nzcv_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        let flags = Nzcv {
            n: true,
            z: false,
            c: true,
            v: false,
        };
        set.insert(flags);
        assert!(set.contains(&flags));
    }
}
