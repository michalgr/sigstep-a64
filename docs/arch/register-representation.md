# Architectural Register Representation Specification

Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
Section: B1.2 "Register interfaces and general-purpose registers"

---

## 1. Overview & Physical vs. Decoded Operands Separation

In AArch64 hardware and OS execution contexts (such as Linux `user_pt_regs.regs[31]`), physical state consists of exactly 31 general-purpose 64-bit registers ($X0..X30$) plus a distinct physical Stack Pointer ($SP$).

Key architectural distinction:
- **Physical Register Storage:** Register index 31 does *not* exist as a physical general-purpose register slot in memory or state structures.
- **5-bit Field Encoding:** In instruction encodings (e.g. `Rn`, `Rd`, `Rm`, `Rt`), a 5-bit register field can hold values `0` through `31`. Depending on the opcode, field value 31 represents either the Zero Register ($XZR$/$WZR$) or the Stack Pointer ($SP$/$WSP$).

To eliminate ambiguity, prevent error-handling branches on hot execution paths, and preserve `#![no_std]` async-signal safety, `sigstep-a64` strictly separates physical register identifiers (`RegId`) from decoded register operands (`Gpr`).

---

## 2. Register Identifiers and Decoded Operands

### 2.1 Physical Register Identifier: `RegId`

`RegId` represents a verified physical general-purpose register index, bounded strictly to $0..=30$. Because it is range-checked upon construction, physical register accesses on `RegisterBank` are infallible.

```rust
/// Physical general-purpose register identifier (X0..X30 / W0..W30).
///
/// Range-checked to ensure index is strictly in 0..=30.
///
/// Reference: Arm ARM DDI 0487K.a, Section B1.2.1
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegId(u8);

impl RegId {
    /// Creates a new `RegId` if `idx` is in the valid range `0..=30`.
    pub const fn new(idx: u8) -> Option<Self> {
        if idx <= 30 {
            Some(Self(idx))
        } else {
            None
        }
    }

    /// Creates a new `RegId` without range checking.
    ///
    /// # Safety
    ///
    /// `idx` must be in `0..=30`.
    pub const unsafe fn new_unchecked(idx: u8) -> Self {
        Self(idx)
    }

    /// Returns the raw zero-based physical index (0..=30).
    pub const fn index(self) -> u8 {
        self.0
    }
}
```

### 2.2 Decoded General-Purpose Register Operand: `Gpr`

`Gpr` is a tagged union representing the decoded operand of an instruction. It resolves the dual architectural identity of register field value 31 during decoding.

```rust
/// Decoded general-purpose register operand.
///
/// Reference: Arm ARM DDI 0487K.a, Section B1.2.1
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Gpr {
    /// Architectural general-purpose register X0..X30 (or W0..W30).
    Reg(RegId),
    /// Zero Register (XZR / WZR): reads return 0, writes are discarded.
    Zr,
    /// Stack Pointer (SP / WSP): accesses the architectural stack pointer.
    Sp,
}
```

---

## 3. Decoder Disambiguation Helpers

To keep the interpreter loop branch-free and free of opcode-specific register 31 checks, instruction decoders use disambiguation helpers to convert raw 5-bit instruction fields directly into `Gpr` operands.

```rust
impl Gpr {
    /// Disambiguates a raw 5-bit register field where value 31 maps to Zero Register (XZR/WZR).
    ///
    /// - `0..=30` => `Gpr::Reg(RegId)`
    /// - `31` => `Gpr::Zr`
    pub const fn from_raw_zr(raw_5bit: u32) -> Self {
        if raw_5bit < 31 {
            // SAFETY: raw_5bit < 31 guarantees bounds 0..=30.
            Self::Reg(unsafe { RegId::new_unchecked(raw_5bit as u8) })
        } else {
            Self::Zr
        }
    }

    /// Disambiguates a raw 5-bit register field where value 31 maps to Stack Pointer (SP/WSP).
    ///
    /// - `0..=30` => `Gpr::Reg(RegId)`
    /// - `31` => `Gpr::Sp`
    pub const fn from_raw_sp(raw_5bit: u32) -> Self {
        if raw_5bit < 31 {
            // SAFETY: raw_5bit < 31 guarantees bounds 0..=30.
            Self::Reg(unsafe { RegId::new_unchecked(raw_5bit as u8) })
        } else {
            Self::Sp
        }
    }
}
```

With these helpers, the interpreter treats all `Gpr` operands uniformly without branching on instruction-specific register 31 semantics.

---

## 4. Width-Agnostic Operand Design & Semantics

### 4.1 Rationale for Width-Agnostic `Gpr`

`Gpr` specifies register identity only, detached from operation width (32-bit $W$ vs. 64-bit $X$).

1. **Homogeneous Instructions:** Standard ALU instructions (e.g. `ADD`, `SUB`, `AND`, `ORR`) enforce matching operand widths across destination and sources. Storing width in `Gpr` would allow architecturally invalid states (such as `ADD W0, X1, W2`). Moving width to the instruction level (`sf: bool` or a `Width` enum) ensures homogeneous operand width by construction and aligns with arithmetic flag ($NZCV$) calculation rules.
2. **Heterogeneous Instructions:** Instructions with mixed operand widths (e.g., sign/zero-extending multiplies like `SMULL Xd, Wn, Wm`, load signed word `LDRSW Xt, [Xn]`, or register-offset addressing `LDR Xt, [Xn, Wm, SXTW]`) explicitly encode individual operand widths in their instruction descriptor fields.

### 4.2 Typed Accessors and Zero-Extension Rules

The interpreter and register bank use uniform, typed accessors for 64-bit and 32-bit operations:

```rust
fn read_gpr_x(&self, gpr: Gpr) -> u64;
fn read_gpr_w(&self, gpr: Gpr) -> u32;
fn write_gpr_x(&mut self, gpr: Gpr, val: u64);
fn write_gpr_w(&mut self, gpr: Gpr, val: u32);
```

#### Zero-Extension Rule (Arm ARM DDI 0487K.a, Section B1.2.1)
> When reading a 32-bit $W$ register, the upper 32 bits [63:32] of the corresponding $X$ register are ignored.
> When writing a 32-bit $W$ register, the upper 32 bits [63:32] of the corresponding $X$ register are set to zero.

The default implementation of `write_gpr_w` enforces this rule by casting the 32-bit value to `u64` and delegating to `write_gpr_x`:

```rust
fn write_gpr_w(&mut self, gpr: Gpr, val: u32) {
    self.write_gpr_x(gpr, val as u64);
}
```

---

## 5. Vector Registers, PC, and PSTATE.NZCV

### 5.1 Vector / SIMD Registers: `VReg`

Armv8-A FP/SIMD vector registers ($V0..V31$) are 128-bit wide. Unlike general-purpose registers, index 31 for vector registers always refers to physical vector register $V31$ (never $XZR$ or $SP$).

```rust
/// Bounded 128-bit SIMD/FP register identifier (V0..V31).
///
/// Reference: Arm ARM DDI 0487K.a, Section B1.2.2
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VReg(u8);

impl VReg {
    /// Creates a new `VReg` if `idx` is in the valid range `0..=31`.
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
    ///
    /// `idx` must be in `0..=31`.
    pub const unsafe fn new_unchecked(idx: u8) -> Self {
        Self(idx)
    }

    /// Returns the raw zero-based index (0..=31).
    pub const fn index(self) -> u8 {
        self.0
    }
}
```

### 5.2 Program Counter ($PC$)

- **Width:** 64-bit address.
- **Alignment Requirement:** AArch64 instructions are 32-bit fixed width and must be 4-byte aligned.
- **Fault Handling:** Attempting to execute from an unaligned PC (where `pc & 0b11 != 0`) must return `ExecError::UnalignedPc(pc)`.

### 5.3 Condition Flags ($PSTATE.NZCV$)

Condition flags reside in bits 31:28 of `PSTATE` / `NZCV` system register:
- **Bit 31 ($N$):** Negative condition flag.
- **Bit 30 ($Z$):** Zero condition flag.
- **Bit 29 ($C$):** Carry condition flag.
- **Bit 28 ($V$):** Overflow condition flag.

---

## 6. Disassembly & Formatting Reference Table

| `Gpr` Variant / Reg | Width / Context | Mnemonic String |
| :--- | :--- | :--- |
| `Gpr::Reg(RegId(0..=30))` | 64-bit ($X$) | `x0` .. `x30` |
| `Gpr::Reg(RegId(0..=30))` | 32-bit ($W$) | `w0` .. `w30` |
| `Gpr::Zr` | 64-bit ($X$) | `xzr` |
| `Gpr::Zr` | 32-bit ($W$) | `wzr` |
| `Gpr::Sp` | 64-bit ($X$) | `sp` |
| `Gpr::Sp` | 32-bit ($W$) | `wsp` |
| Program Counter | 64-bit ($PC$) | `pc` |
| `VReg(0..=31)` | 128-bit SIMD/FP | `v0` .. `v31` |
