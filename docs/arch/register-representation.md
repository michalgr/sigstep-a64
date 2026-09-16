# Architectural Register Representation Specification

This document formalizes the AArch64 architectural register types, indexing models, sizing semantics, dual architectural identities, and disassembly conventions in `sigstep-a64`.

---

## 1. Overview & Type Safety

In standard AArch64 instruction opcodes, general-purpose and SIMD/FP register fields are encoded as 5-bit integers ($0..=31$). Representing register indices as raw primitive integers (`u8` or `u32`) in internal APIs introduces unnecessary runtime error handling overhead and permits invalid index states ($>31$).

To ensure zero-cost abstraction, type safety, and signal safety, `sigstep-a64` introduces bounded register types (`Reg` and `VReg`) that guarantee indices are strictly within $0..=31$ by construction.

---

## 2. General-Purpose Registers: `Reg`

The `Reg` type encapsulates a 5-bit register selector ($0..=31$).

### 2.1 Representation & Type Definition

```rust
/// Bounded 5-bit general-purpose register index (0..=31).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Reg(u8);

impl Reg {
    /// Construct a `Reg` from a raw instruction opcode register field,
    /// masking with `0x1F` (bits 4:0) to guarantee the value is within `0..=31`.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self((raw & 0x1F) as u8)
    }

    /// Returns the underlying raw register index (0..=31).
    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Returns `true` if this register index represents register 31
    /// (which acts as XZR/WZR or SP/WSP depending on opcode context).
    #[inline]
    pub const fn is_zr_or_sp(self) -> bool {
        self.0 == 31
    }
}
```

### 2.2 Dual Identity of Register 31 ($XZR$ / $WZR$ vs $SP$ / $WSP$)

Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a), Section B1.2.1 "Registers in AArch64 execution state".

In AArch64 instruction encodings, register index `31` (`0b11111`) does not refer to a dedicated 32nd general-purpose register. Instead, its architectural identity depends entirely on the instruction context and opcode group:

1. **Zero Register ($XZR$ / $WZR$):**
   - **Contexts:** Data-processing instructions (e.g., `ADD`, `SUB`, `ORR`, `AND` flags-setting or standard register-register forms), logical instructions, shift operations, and store-source operands.
   - **Semantics:**
     - Reads from register 31 in zero-register contexts always evaluate to constant zero (`0`).
     - Writes to register 31 in zero-register contexts are discarded (no-op side effects).

2. **Stack Pointer ($SP$ / $WSP$):**
   - **Contexts:** Base address register in load/store instructions (`LDR`, `STR`, `LDP`, `STP`), non-flags data-processing instructions (e.g., `ADD` / `SUB` with immediate or extended register when referencing $SP$), and direct stack pointer manipulation.
   - **Semantics:**
     - Reads and writes access the architectural Stack Pointer ($SP$) register.

Because instruction opcodes statically determine whether index 31 represents $XZR$ or $SP$, the register type `Reg` itself remains context-agnostic, providing `is_zr_or_sp()` so the instruction interpreter or decoder helper can route access appropriately.

---

## 3. Register Widths & Zero-Extension Semantics

Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a), Section B1.2.1 "General-purpose registers".

AArch64 general-purpose registers can be accessed as either 64-bit $X$ registers ($X0 \dots X30$) or 32-bit $W$ registers ($W0 \dots W30$).

### 3.1 Width Views

- **$X0 \dots X30$ (64-bit):** Full 64-bit width register view used for 64-bit integer calculations and 64-bit memory addressing.
- **$W0 \dots W30$ (32-bit):** Lower 32 bits of the corresponding $X$ register ($X[31:0]$).

### 3.2 Architectural Zero-Extension Rule

Per Arm ARM DDI 0487K.a, Section B1.2.1:
> "When a 32-bit general-purpose register $Wn$ is written, the upper 32 bits of the corresponding 64-bit general-purpose register $Xn$ ($Xn[63:32]$) are set to zero."

This rule is critical for instruction interpretation:

```rust
// Writing W register zero-extends upper 32 bits into 64-bit X register state:
let updated_x_val = val_u32 as u64;
```

Read operations on $W$ registers return the lower 32 bits of the 64-bit $X$ register value (`x_val as u32`).

---

## 4. Vector Registers: `VReg`

The `VReg` type represents a 128-bit Advanced SIMD / Floating-Point register index ($V0 \dots V31$).

### 4.1 Representation & Type Definition

```rust
/// Bounded 5-bit SIMD / Floating-Point vector register index (0..=31).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VReg(u8);

impl VReg {
    /// Construct a `VReg` from a raw instruction opcode register field,
    /// masking with `0x1F` (bits 4:0) to guarantee the value is within `0..=31`.
    #[inline]
    pub const fn from_raw(raw: u32) -> Self {
        Self((raw & 0x1F) as u8)
    }

    /// Returns the underlying raw vector register index (0..=31).
    #[inline]
    pub const fn index(self) -> u8 {
        self.0
    }
}
```

### 4.2 Vector Register Semantics

- **Distinct Identity for Index 31:** Unlike integer registers, index 31 in vector instruction contexts always refers to $V31$ (or scalar arrangement $B31$, $H31$, $S31$, $D31$, $Q31$). It is **never** interpreted as $XZR$ or $SP$.
- **Width & Storage:** Each SIMD register is 128 bits wide (`u128`). Scalar operations access subsets ($B=8$, $H=16$, $S=32$, $D=64$, $Q=128$ bits). Per Arm ARM, scalar vector writes zero-extend to the remainder of the 128-bit vector register.

---

## 5. Special Registers & PSTATE Condition Flags

### 5.1 Program Counter ($PC$)

- **Role:** Holds the 64-bit architectural Program Counter.
- **Alignment Requirement:** AArch64 standard instructions are fixed 32-bit (4-byte) aligned words. Any branch or update setting $PC$ to an unaligned value ($PC[1:0] \neq 00$) triggers an `ExecError::UnalignedPc(pc)` fault.

### 5.2 Condition Flags ($PSTATE.NZCV$)

Reference: Arm ARM DDI 0487K.a, Section C5.2.10 "NZCV, Condition Flags".

The condition flags are stored in $PSTATE.NZCV$ (bits 31:28):
- **N (Bit 31):** Negative condition flag.
- **Z (Bit 30):** Zero condition flag.
- **C (Bit 29):** Carry condition flag.
- **V (Bit 28):** Overflow condition flag.

---

## 6. Disassembly & Display Conventions

When formatting or disassembling registers, standard Arm A64 assembly mnemonics must be used:

| Register Index | 64-bit $X$ View | 32-bit $W$ View | SIMD/FP Vector View | Context / Identity |
| :---: | :---: | :---: | :---: | :---: |
| `0` .. `30` | `x0` .. `x30` | `w0` .. `w30` | `v0` .. `v30` | General Purpose Registers / SIMD |
| `31` (ZR Context) | `xzr` | `wzr` | `v31` | Zero Register ($XZR$ / $WZR$) or $V31$ |
| `31` (SP Context) | `sp` | `wsp` | `v31` | Stack Pointer ($SP$ / $WSP$) or $V31$ |
| — | `pc` | — | — | Program Counter |

---

## 7. Cross-References & Specification Links

- `docs/arch/traits-and-safety.md` — `RegisterBank` trait implementation using `Reg` and `VReg`.
- `docs/arch/instruction-matrix.md` — Instruction decoding matrix and ISA roadmap.
