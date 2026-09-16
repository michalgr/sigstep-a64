# Architecture Specification: Register Representation & Disambiguation

This document formalizes the AArch64 architectural register representation, decoder disambiguation semantics, typed accessors, and width-agnostic design for `sigstep-a64`.

---

## 1. Official Arm Architecture Citations & Links

All register structures and semantics in `sigstep-a64` strictly adhere to the official Arm architecture and ABI specifications:

* **Arm Architecture Reference Manual for A-profile architecture (Arm ARM DDI 0487K.a):**
  * Section B1.2.1 "General-purpose registers"
  * Section B1.2.2 "SIMD and floating-point registers"
  * Section C5.2.10 "NZCV, Condition Flags"
* **Arm Developer Architecture Guide:**
  * [Registers in AArch64 — General-purpose registers](https://developer.arm.com/documentation/102374/latest/Registers-in-AArch64---general-purpose-registers)
* **Arm Architecture Procedure Call Standard for AArch64 (AAPCS64):**
  * Section 6.1.1 "Core Registers": [AAPCS64 Specification](https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst)
* **Arm Developer A64 Base Instruction Reference (DDI 0596):**
  * [A64 Base Instructions Overview](https://developer.arm.com/documentation/ddi0596/latest)

---

## 2. Physical Storage vs. Decoded Operands

In the Armv8-A architecture, processing elements feature 31 general-purpose registers ($X0 \dots X30$), a dedicated Stack Pointer ($SP$), a Zero Register ($XZR$), and 32 vector/SIMD registers ($V0 \dots V31$).

The 5-bit register fields (`Rn`, `Rm`, `Rd`, `Rt`) in A64 instruction encodings range from `0` to `31`. Register field value `31` has a dual architectural identity depending on the instruction encoding:
* In contexts requiring a stack pointer or destination operand for stack-oriented operations, encoding `31` represents $SP$.
* In general integer ALU and memory operands, encoding `31` represents the Zero Register ($XZR$ / $WZR$).

To decouple physical storage from instruction decoding and prevent error-prone runtime branching in hot execution paths, `sigstep-a64` explicitly separates physical register storage indices from decoded core operands.

### 2.1 Bounded Physical Register Index: `Gpr`

Physical register storage maintains exactly 31 64-bit storage locations ($X0 \dots X30$). `Gpr` represents a range-checked index bounded to $0..=30$.

```rust
/// Bounded general-purpose register index strictly in 0..=30 (X0–X30 / W0–W30).
///
/// Range-checked to ensure index is strictly in 0..=30.
///
/// Reference: Arm ARM DDI 0487K.a, Section B1.2.1
/// Canonical: https://developer.arm.com/documentation/102374/latest/Registers-in-AArch64---general-purpose-registers
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gpr(u8);

impl Gpr {
    /// Creates a new `Gpr` if `idx` is in the valid range `0..=30`.
    pub const fn new(idx: u8) -> Option<Self> {
        if idx <= 30 { Some(Self(idx)) } else { None }
    }

    /// Creates a new `Gpr` without range checking.
    ///
    /// # Safety
    /// Caller must guarantee `idx <= 30`.
    pub const unsafe fn new_unchecked(idx: u8) -> Self {
        Self(idx)
    }

    /// Returns the raw zero-based physical index (0..=30).
    pub const fn index(self) -> u8 {
        self.0
    }
}
```

### 2.2 Decoded Core Operand: `CoreReg`

`CoreReg` represents a decoded core integer register operand, resolving field 31 during instruction decoding per AAPCS64 Section 6.1.1.

```rust
/// Decoded core integer register operand.
///
/// Resolves the architectural identity of register field 31 during decoding.
///
/// Reference: AAPCS64, Section 6.1.1 "Core Registers"
/// Canonical: https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CoreReg {
    /// Architectural general-purpose register X0..X30 (or W0..W30).
    Reg(Gpr),
    /// Zero Register (XZR / WZR): reads return 0, writes are discarded.
    Zr,
    /// Special Register: Stack Pointer (SP / WSP).
    Sp,
}
```

---

## 3. Decoder Disambiguation Helpers

Register 31 disambiguation is handled strictly within the instruction decoder. The decoder inspects the opcode structure and uses specialized constructor functions to yield a disambiguated `CoreReg`:

```rust
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
```

### Interpreter Impact
Because the instruction decoder resolves encoding 31 into `CoreReg::Zr` or `CoreReg::Sp` upfront:
1. The interpreter never checks opcode rules or opcode tables to determine register 31 behavior.
2. The interpreter interacts with `CoreReg` uniformly using default trait methods on `RegisterBank`.
3. Execution hot paths eliminate dynamic branching on opcode classes for register access.

### Infallible Register Extraction Invariant
Register operand extraction in the instruction decoder is architecturally infallible by construction:
1. **5-Bit Masking:** Register fields in A64 instructions (`Rn`, `Rm`, `Rd`, `Rt`) are 5 bits wide. Masking with `0x1F` (`(raw >> shift) & 0x1F`) mathematically bounds the value to $0 \le \text{raw\_5bit} \le 31$.
2. **Index 31 Resolution:** When $\text{raw\_5bit} = 31$, the constructor helper constructs `CoreReg::Zr` or `CoreReg::Sp` based on instruction context.
3. **Index 0..=30 Resolution:** When $\text{raw\_5bit} \le 30$, the constructor helper constructs `CoreReg::Reg(Gpr)` safely via `Gpr::new_unchecked(idx)`.
4. **Zero Error Branches:** Consequently, register field extraction in `Decoder::decode` is completely infallible, requiring zero error branches or `DecodeError` variants.

---

## 4. Width-Agnostic Design Rationale

`CoreReg` represents register identity only, intentionally omitting operand width (`32-bit W` vs. `64-bit X`).

### Key Design Motives:
1. **Instruction-Level Width Scoping:** In AArch64, homogeneous integer instructions share an instruction-level width flag (`sf` bit, where `sf = 0` indicates 32-bit $W$ variant and `sf = 1` indicates 64-bit $X$ variant). Modeling width on individual register operands would allow representing invalid states (e.g., `ADD W0, X1, W2`), complicating decoder verification.
2. **Flag and Arithmetic Alignment:** Homogeneous ALU operations compute NZCV flags based on operation width (`sf`). A single width flag at the instruction level matches AArch64 operational pseudocode.
3. **Explicit Handling of Heterogeneous Operands:** For instructions with mixed operand widths (e.g., `SMULL Xd, Wn, Wm`, `LDRSW Rt, [Xn, #imm]`, `LDR Xt, [Xn, Wm, SXTW]`), individual fields in the decoded instruction descriptor explicitly declare operand extension modes or register widths.

---

## 5. Typed Accessors & Zero-Extension Semantics

The `RegisterBank` trait provides uniform 64-bit ($X$) and 32-bit ($W$) accessors for `CoreReg`.

### 5.1 Accessor Interface

```rust
fn read_core_x(&self, reg: CoreReg) -> u64;
fn read_core_w(&self, reg: CoreReg) -> u32;
fn write_core_x(&mut self, reg: CoreReg, val: u64);
fn write_core_w(&mut self, reg: CoreReg, val: u32);
```

### 5.2 Architectural Zero-Extension Rule

Per Arm ARM DDI 0487K.a, Section B1.2.1:
> "A write to a 32-bit general-purpose register (W0–W30) zero-extends the value into the upper 32 bits of the corresponding 64-bit register (X0–X30)."

`RegisterBank` enforces this invariant via default accessor implementations:

```rust
fn write_core_w(&mut self, reg: CoreReg, val: u32) {
    self.write_core_x(reg, val as u64);
}
```

When writing to `CoreReg::Reg(gpr)`, `val as u64` zero-extends bits [31:0] and sets bits [63:32] to `0`. Writing to `CoreReg::Zr` is a no-op, while writing to `CoreReg::Sp` writes the zero-extended 64-bit value to $SP$.

---

## 6. Vector Registers (`VReg`)

SIMD and Floating-Point instructions operate on 32 128-bit vector registers ($V0 \dots V31$).

```rust
/// Bounded vector register index strictly in 0..=31 (V0–V31).
///
/// Reference: Arm ARM DDI 0487K.a, Section B1.2.2
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VReg(u8);

impl VReg {
    /// Creates a new `VReg` if `idx` is in the valid range `0..=31`.
    pub const fn new(idx: u8) -> Option<Self> {
        if idx <= 31 { Some(Self(idx)) } else { None }
    }

    /// Creates a new `VReg` without range checking.
    ///
    /// # Safety
    /// Caller must guarantee `idx <= 31`.
    pub const unsafe fn new_unchecked(idx: u8) -> Self {
        Self(idx)
    }

    /// Returns the raw zero-based physical index (0..=31).
    pub const fn index(self) -> u8 {
        self.0
    }
}
```

Note: In SIMD/FP vector instruction encodings, index 31 always identifies physical vector register $V31$—it is never interpreted as $XZR$ or $SP$.

---

## 7. Program Counter ($PC$) & Condition Flags ($PSTATE.NZCV$)

### 7.1 Program Counter ($PC$)
* $PC$ is a 64-bit register holding the memory address of the current instruction.
* All AArch64 instructions are 32 bits wide and must be 4-byte aligned.
* Setting $PC$ to an unaligned address (where `PC & 3 != 0`) triggers an `ExecError::UnalignedPc(addr)` fault during fetch/execution.

### 7.2 Condition Flags ($PSTATE.NZCV$)
* NZCV flags reflect arithmetic/logical execution outcomes in bits 31:28 of `PSTATE` (Arm ARM DDI 0487K.a, Section C5.2.10):
  * **N (Bit 31):** Negative result flag.
  * **Z (Bit 30):** Zero result flag.
  * **C (Bit 29):** Carry flag.
  * **V (Bit 28):** Overflow flag.

---

## 8. Disassembly & Formatting Reference Table

The table below illustrates the mnemonic formatting across register identities and operand widths:

| Register Identity (`CoreReg` / `VReg`) | 64-bit Integer | 32-bit Integer | 128-bit Vector | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `CoreReg::Reg(Gpr(0))` | `x0` | `w0` | — | Physical register index 0 |
| `CoreReg::Reg(Gpr(30))` | `x30` | `w30` | — | Link Register ($LR$) |
| `CoreReg::Zr` | `xzr` | `wzr` | — | Zero Register (reads 0, writes ignored) |
| `CoreReg::Sp` | `sp` | `wsp` | — | Stack Pointer |
| `VReg(0)` | — | — | `v0` | SIMD / FP register index 0 |
| `VReg(31)` | — | — | `v31` | SIMD / FP register index 31 |
| Program Counter | `pc` | — | — | 64-bit instruction address |
