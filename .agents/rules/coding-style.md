# Agent Rule: Coding Style & Architectural Specifications

**Target Agents:** Jules, Antigravity, and all automated/human contributors.  
**Enforcement Scope:** Entire `sigstep-a64` codebase.

---

## 1. `#![no_std]` Idiomatic Standards

`sigstep-a64` is designed for low-level systems, bare-metal kernels, and signal handlers. The core library must remain strictly `#![no_std]`.

1. **Standard Library Imports:**
   - Always import from `core::` instead of `std::` (e.g., `core::fmt`, `core::mem`, `core::sync::atomic`, `core::ops`).
   - Gating standard library features must be strictly restricted to testing or optional utility modules using `#[cfg(feature = "std")]`.
2. **Panic Policy:**
   - Production execution paths must never panic. Avoid `unwrap()`, `expect()`, `unreachable!()`, or index bounds panics (prefer `.get()` or pattern matching).
   - If a condition is mathematically or architecturally impossible, return an explicit error variant (`ExecError::InternalInvariantViolated` or `DecodeError::Undefined`) rather than invoking panic machinery.
   - Core modules must be buildable with `panic = "abort"`.

---

## 2. Mandatory Arm Architecture Documentation Citations

To facilitate rapid human review, formal auditing, and golden-model verification against the hardware reference, **every instruction definition, bitfield extractor, decoder branch, and execution handler MUST cite the official Arm Architecture Reference Manual (Arm ARM)**.

### 2.1 Citation Format
Every instruction decode and emulation function must include a standardized header doc comment:

```rust
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
/// Section:   C6.2.26 "B.cond - Branch conditionally"
/// Canonical: https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/B-cond--Branch-conditionally-
///
/// Encoding:
///   31  30 29 28   24 23                 5 4    3   0
/// +----+-----+-------+--------------------+---+------+
/// | 0  | 1 0 | 1 0 1 | 0 0 |     imm19    | 0 | cond |
/// +----+-----+-------+--------------------+---+------+
///
/// Operational Pseudocode:
///   if ConditionHolds(cond) then
///       BranchTo(PC[] + SignExtend(imm19:'00', 64), BranchType_DIR);
```

### 2.2 Citation Elements
1. **Manual Edition & Section:** Specify the document and section number (e.g., `Arm ARM DDI 0487K.a, Section C6.2.x`).
2. **Instruction Title & Mnemonic:** Exact mnemonic and description matching the specification.
3. **Canonical Developer Link:** Link to the Arm Developer A64 ISA reference entry (`https://developer.arm.com/documentation/...`).
4. **Binary Layout Diagram:** Visual bit diagram showing field positions (`sf`, `op`, `imm`, `Rn`, `Rd`, `cond`).
5. **Specification Pseudocode:** Key operational lines from the Arm ARM pseudocode describing the exact side effects on registers, memory, and condition flags.

---

## 3. Bitfield Manipulation & Const Utilities

To maintain readability, debuggability, and zero-cost abstraction without macro bloat:

1. **Prefer `const` Bitwise Functions Over Heavy Macros:**
   - Do NOT use heavy procedural macro crates (e.g., complex bitfield DSLs that obscure binary layouts or bloat compile times).
   - Implement bit manipulation using small, transparent `const fn` utilities:
     - `extract_bits(value: u32, lsb: u32, width: u32) -> u32`
     - `sign_extend_64(value: u64, num_bits: u32) -> u64`
     - `sign_extend_32(value: u32, num_bits: u32) -> i32`
2. **Explicit Masks and Shifts:**
   - Define named constants for bit positions and bit patterns matching the Arm ARM naming (e.g., `const COND_MASK: u32 = 0x0F;`, `const IMM19_SHIFT: u32 = 5;`).
3. **No Unchecked Arithmetic Shifts:**
   - Always cast unsigned raw bitfields to signed types explicitly or utilize the validated `sign_extend` helpers to prevent unexpected zero-extension or truncation bugs.

---

## 4. Data Type Guarantees & Memory Footprint

1. **Trait Derivations:**
   - Every instruction representation, operand structure, condition code enum, and register descriptor must implement:
     ```rust
     #[derive(Copy, Clone, Debug, PartialEq, Eq)]
     ```
2. **Compact Representation:**
   - Enums representing decoded instructions must be structured efficiently to prevent bloated enum discriminant sizing. Where applicable, keep decoded instruction structures $\le 32$ bytes.
3. **Value Semantics:**
   - Use plain value copies rather than references for decoded instructions and operands. References introduce lifetimes that needlessly complicate signal handler state machines.

---

## 5. Target Platforms & Linter Hygiene

1. **Required Compilation Targets:**
   All PRs and builds must compile cleanly across these targets:
   - `aarch64-unknown-none` (Bare-metal verification, pure `#![no_std]`)
   - `aarch64-unknown-linux-gnu` (Linux signal handler backend target)
   - Host target (e.g., `aarch64-apple-darwin` or `x86_64-unknown-linux-gnu` for cross-compilation testing)

2. **Clippy & Compiler Warnings:**
   - Zero tolerance for warnings:
     ```bash
     cargo clippy --all-targets -- -D warnings
     ```
   - All `unsafe` blocks must have an accompanying `// SAFETY:` comment detailing the invariants upheld.

3. **Formatting:**
   - Format with standard `rustfmt`:
     ```bash
     cargo fmt --check
     ```
