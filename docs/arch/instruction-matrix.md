# Architecture Specification: Instruction Support Matrix & Phased Roadmap

This document outlines the phased development roadmap for AArch64 instruction decoding and interpretation in `sigstep-a64`.

Every instruction entry includes:
- **Arm ARM Citation:** Specific section and pseudocode reference from the official Arm Architecture Reference Manual (DDI 0487K.a).
- **Canonical ISA Link:** Direct link to Arm Developer A64 Instruction Set documentation.
- **Binary Bitfield Format:** Opcode mask, fixed bits, and variable field positions.
- **Operational Semantics:** Architectural state updates ($PC$, general-purpose registers, flags, memory).

---

## Phase Overview

| Phase | Category | Purpose | Status |
| :--- | :--- | :--- | :--- |
| **Phase 1** | Unconditional Branches & Control Flow | Basic execution redirect, trampoline jumping, function returns | Planned |
| **Phase 2** | Conditional & Test Branches | Conditional execution flow, loop branches, bit testing | Planned |
| **Phase 3** | PC-Relative Addressing | Position-independent data resolution, literal pool loading | Planned |
| **Phase 4** | Hook Emulation Basics | Register initialization, pointer arithmetic, flags setting, NOP sleds | Planned |
| **Phase 5** | Memory Access | Register loads/stores, struct access, stack pushing/popping | Planned |

---

## Phase 1: Control Flow & Unconditional Branches

Unconditional control flow instructions are the core building blocks for instrumenting jump trampolines and executing function returns.

### 1.1 `B` — Branch (Immediate)
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.25
- **Canonical:** [Arm Developer A64: B](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/B--Branch-)
- **Format:**
  ```text
   31 30 29 28 27 26 25                          0
  +--+--+--+--+--+--+-----------------------------+
  |0 |0 |0 |1 |0 |1 |            imm26            |
  +--+--+--+--+--+--+-----------------------------+
  ```
- **Mask / Match:** `(raw & 0xFC00_0000) == 0x1400_0000`
- **Operands:** `imm26`: 26-bit signed immediate offset (word-aligned).
- **Semantics:**
  ```text
  offset = SignExtend(imm26 : '00', 64)   // Range: +/- 128MB
  PC = PC + offset
  ```

### 1.2 `BL` — Branch with Link (Immediate)
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.33
- **Canonical:** [Arm Developer A64: BL](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BL--Branch-with-Link-)
- **Format:**
  ```text
   31 30 29 28 27 26 25                          0
  +--+--+--+--+--+--+-----------------------------+
  |1 |0 |0 |1 |0 |1 |            imm26            |
  +--+--+--+--+--+--+-----------------------------+
  ```
- **Mask / Match:** `(raw & 0xFC00_0000) == 0x9400_0000`
- **Operands:** `imm26`: 26-bit signed immediate offset.
- **Semantics:**
  ```text
  X[30] = PC + 4
  offset = SignExtend(imm26 : '00', 64)
  PC = PC + offset
  ```

### 1.3 `BR` — Branch to Register
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.34
- **Canonical:** [Arm Developer A64: BR](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BR--Branch-to-Register-)
- **Format:**
  ```text
   31          25 24 21 20   16 15   10 9     5 4   0
  +--------------+-----+-------+-------+-------+-----+
  |   1101011    |0000 | 11111 |000000 |  Rn   |00000|
  +--------------+-----+-------+-------+-------+-----+
  ```
- **Mask / Match:** `(raw & 0xFFFF_FC1F) == 0xD61F_0000`
- **Operands:** `Rn`: 5-bit general-purpose source register (0–30).
- **Semantics:**
  ```text
  target = X[Rn]
  if target & 0x3 != 0: return UnalignedPc(target)
  PC = target
  ```

### 1.4 `BLR` — Branch with Link to Register
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.35
- **Canonical:** [Arm Developer A64: BLR](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BLR--Branch-with-Link-to-Register-)
- **Format:**
  ```text
   31          25 24 21 20   16 15   10 9     5 4   0
  +--------------+-----+-------+-------+-------+-----+
  |   1101011    |0001 | 11111 |000000 |  Rn   |00000|
  +--------------+-----+-------+-------+-------+-----+
  ```
- **Mask / Match:** `(raw & 0xFFFF_FC1F) == 0xD63F_0000`
- **Operands:** `Rn`: 5-bit register (0–30).
- **Semantics:**
  ```text
  target = X[Rn]
  X[30] = PC + 4
  if target & 0x3 != 0: return UnalignedPc(target)
  PC = target
  ```

### 1.5 `RET` — Return from Subroutine
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.235
- **Canonical:** [Arm Developer A64: RET](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/RET--Return-from-subroutine-)
- **Format:**
  ```text
   31          25 24 21 20   16 15   10 9     5 4   0
  +--------------+-----+-------+-------+-------+-----+
  |   1101011    |0010 | 11111 |000000 |  Rn   |00000|
  +--------------+-----+-------+-------+-------+-----+
  ```
- **Mask / Match:** `(raw & 0xFFFF_FC1F) == 0xD65F_0000`
- **Operands:** `Rn`: 5-bit register (defaults to `X30` if `Rn == 30`).
- **Semantics:**
  ```text
  target = X[Rn]
  if target & 0x3 != 0: return UnalignedPc(target)
  PC = target
  ```

---

## Phase 2: Conditional & Test Branches

Conditional branches evaluate condition flags ($NZCV$) or specific register contents to alter the flow of execution.

### 2.1 `B.cond` — Branch Conditionally
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.26
- **Canonical:** [Arm Developer A64: B.cond](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/B-cond--Branch-conditionally-)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23                 5 4 3   0
  +--+--+--+--+--+--+--+--+--------------------+-+----+
  |0 |1 |0 |1 |0 |1 |0 |0 |       imm19        |0|cond|
  +--+--+--+--+--+--+--+--+--------------------+-+----+
  ```
- **Mask / Match:** `(raw & 0xFF00_0010) == 0x5400_0000`
- **Operands:**
  - `imm19`: 19-bit signed immediate offset (range $\pm 1\text{ MB}$).
  - `cond`: 4-bit condition code (`EQ`, `NE`, `CS`, `CC`, `MI`, `PL`, `VS`, `VC`, `HI`, `LS`, `GE`, `LT`, `GT`, `LE`, `AL`, `NV`).
- **Semantics:**
  ```text
  if ConditionHolds(cond, flags):
      PC = PC + SignExtend(imm19 : '00', 64)
  else:
      PC = PC + 4
  ```

### 2.2 `CBZ` / `CBNZ` — Compare and Branch on Zero / Non-Zero
- **Reference:** Arm ARM DDI 0487K.a, Sections C6.2.45 / C6.2.46
- **Canonical:** [Arm Developer A64: CBZ](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/CBZ--Compare-and-Branch-on-Zero-)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23                 5 4    0
  +--+--+--+--+--+--+--+--+--------------------+------+
  |sf|0 |1 |1 |0 |1 |0 |op|       imm19        |  Rt  |
  +--+--+--+--+--+--+--+--+--------------------+------+
  ```
- **Mask / Match:** `(raw & 0x7E00_0000) == 0x3400_0000`
- **Operands:**
  - `sf`: 0 = 32-bit (`W`), 1 = 64-bit (`X`).
  - `op`: 0 = `CBZ`, 1 = `CBNZ`.
  - `imm19`: 19-bit signed immediate offset ($\pm 1\text{ MB}$).
  - `Rt`: Register to test.
- **Semantics:**
  ```text
  val = if sf == 1 { X[Rt] } else { W[Rt] as u64 }
  condition = if op == 0 { val == 0 } else { val != 0 }
  if condition:
      PC = PC + SignExtend(imm19 : '00', 64)
  else:
      PC = PC + 4
  ```

### 2.3 `TBZ` / `TBNZ` — Test Bit and Branch on Zero / Non-Zero
- **Reference:** Arm ARM DDI 0487K.a, Sections C6.2.336 / C6.2.337
- **Canonical:** [Arm Developer A64: TBZ](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/TBZ--Test-Bit-and-Branch-if-Zero-)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23 19 18           5 4    0
  +--+--+--+--+--+--+--+--+-----+--------------+------+
  |b5|0 |1 |1 |0 |1 |1 |op| b40 |    imm14     |  Rt  |
  +--+--+--+--+--+--+--+--+-----+--------------+------+
  ```
- **Mask / Match:** `(raw & 0x7E00_0000) == 0x3600_0000`
- **Operands:**
  - `bit_pos = (b5 << 5) | b40`: Bit index (0–63).
  - `op`: 0 = `TBZ`, 1 = `TBNZ`.
  - `imm14`: 14-bit signed immediate offset ($\pm 32\text{ KB}$).
  - `Rt`: Register to test.
- **Semantics:**
  ```text
  bit_val = (X[Rt] >> bit_pos) & 1
  condition = if op == 0 { bit_val == 0 } else { bit_val != 0 }
  if condition:
      PC = PC + SignExtend(imm14 : '00', 64)
  else:
      PC = PC + 4
  ```

---

## Phase 3: PC-Relative Addressing

Crucial for relocated hook trampolines and position-independent code (PIC) to compute absolute addresses of data and global variables.

### 3.1 `ADR` — Form PC-Relative Address
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.10
- **Canonical:** [Arm Developer A64: ADR](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/ADR--Form-PC-relative-address-)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23                 5 4    0
  +--+-----+--+--+--+--+--+--------------------+------+
  |0 | immlo|1 |0 |0 |0 |0 |       immhi        |  Rd  |
  +--+-----+--+--+--+--+--+--------------------+------+
  ```
- **Mask / Match:** `(raw & 0x9F00_0000) == 0x1000_0000`
- **Operands:**
  - `imm21 = (immhi << 2) | immlo`: Signed 21-bit offset ($\pm 1\text{ MB}$).
  - `Rd`: Destination register.
- **Semantics:**
  ```text
  X[Rd] = PC + SignExtend(imm21, 64)
  PC = PC + 4
  ```

### 3.2 `ADRP` — Form PC-Relative Page Address
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.11
- **Canonical:** [Arm Developer A64: ADRP](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/ADRP--Form-PC-relative-address-to-4KB-page-)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23                 5 4    0
  +--+-----+--+--+--+--+--+--------------------+------+
  |1 | immlo|1 |0 |0 |0 |0 |       immhi        |  Rd  |
  +--+-----+--+--+--+--+--+--------------------+------+
  ```
- **Mask / Match:** `(raw & 0x9F00_0000) == 0x9000_0000`
- **Operands:**
  - `imm21 = (immhi << 2) | immlo`: Signed 21-bit page offset ($\pm 4\text{ GB}$).
  - `Rd`: Destination register.
- **Semantics:**
  ```text
  base_page = PC & !0xFFF
  page_offset = SignExtend(imm21 : '000000000000', 64)
  X[Rd] = base_page + page_offset
  PC = PC + 4
  ```

### 3.3 `LDR (literal)` — Load Register (Literal)
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.148
- **Canonical:** [Arm Developer A64: LDR literal](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/LDR--literal---Load-Register--literal--)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23                 5 4    0
  +-----+--+--+--+--+--+--+--------------------+------+
  | opc |0 |1 |1 |0 |0 |0 |       imm19        |  Rt  |
  +-----+--+--+--+--+--+--+--------------------+------+
  ```
- **Mask / Match:** `(raw & 0x3F00_0000) == 0x1800_0000`
- **Operands:**
  - `opc`: `00` = 32-bit `W`, `01` = 64-bit `X`.
  - `imm19`: 19-bit signed word offset ($\pm 1\text{ MB}$).
  - `Rt`: Destination register.
- **Semantics:**
  ```text
  target_addr = PC + SignExtend(imm19 : '00', 64)
  if opc == 00:
      W[Rt] = mem.read_u32(target_addr)?
  else:
      X[Rt] = mem.read_u64(target_addr)?
  PC = PC + 4
  ```

---

## Phase 4: Hook Emulation Basics

Basic ALU, register moves, and NOPs needed to execute rewritten prologues and epilogues.

### 4.1 `MOV (wide immediate)` — `MOVZ`, `MOVN`, `MOVK`
- **Reference:** Arm ARM DDI 0487K.a, Sections C6.2.186, C6.2.185, C6.2.184
- **Canonical:** [Arm Developer A64: MOVZ](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/MOVZ--Move-wide-with-zero-)
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23 22 21 20         5 4    0
  +--+-----+--+--+--+--+--+--+-----+------------+------+
  |sf| opc |1 |0 |0 |1 |0 |1 |  hw |   imm16    |  Rd  |
  +--+-----+--+--+--+--+--+--+-----+------------+------+
  ```
- **Mask / Match:** `(raw & 0x1F80_0000) == 0x1280_0000`
- **Semantics:**
  - `MOVZ` (`opc = 10`): Sets `Rd = imm16 << (hw * 16)`.
  - `MOVN` (`opc = 00`): Sets `Rd = !(imm16 << (hw * 16))`.
  - `MOVK` (`opc = 11`): Keeps existing bits in `Rd` and overwrites bits `[(hw*16 + 15) : (hw*16)]` with `imm16`.

### 4.2 `ADD` / `SUB` (Immediate) & Flags-Setting (`ADDS` / `SUBS`)
- **Reference:** Arm ARM DDI 0487K.a, Sections C6.2.4, C6.2.308, C6.2.5, C6.2.309
- **Format:**
  ```text
   31 30 29 28 27 26 25 24 23 22 21           10 9   5 4    0
  +--+--+--+--+--+--+--+--+--+--+---------------+-----+------+
  |sf|op| S|1 |0 |0 |0 |1 |0 |sh|     imm12     | Rn  |  Rd  |
  +--+--+--+--+--+--+--+--+--+--+---------------+-----+------+
  ```
- **Operands:**
  - `sf`: 32/64-bit mode.
  - `op`: 0 = `ADD`, 1 = `SUB`.
  - `S`: 1 = update condition flags $NZCV$.
  - `sh`: 0 = unshifted, 1 = shifted left 12 bits.
  - `imm12`: 12-bit unsigned immediate.

### 4.3 `NOP` & Architectural Hints
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.203
- **Canonical:** [Arm Developer A64: NOP](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/NOP--No-Operation-)
- **Encoding:** `0xD503_201F`
- **Semantics:** Advance $PC$ by 4 with zero side effects on registers or flags.

---

## Phase 5: Memory Access

Emulating load and store instructions commonly relocated during hook installation.

> **Transactional Execution & Alignment Invariants:**
> - **Atomic Commit & Faulting PC:** If address resolution, memory read/write, or alignment check faults during execution of any load/store instruction, no architectural state modifications (including base register writeback) are committed, and $PC$ remains strictly pointing to the faulting instruction per [Traits & Safety Specification Section 1.3](traits-and-safety.md#13-transactional-execution--faulting-pc-invariant).
> - **Two-Tier Alignment:** Scalar `LDR`/`STR` instructions support unaligned access on Normal memory (passed to `MemoryInterface` per Tier 2), whereas pair `LDP`/`STP` instructions strictly require natural element alignment (4-byte for 32-bit, 8-byte for 64-bit) verified by the interpreter prior to memory access per Tier 1 of [Traits & Safety Specification Section 3.3](traits-and-safety.md#33-two-tier-memory-alignment-contract).

### 5.1 `LDR` / `STR` (Immediate Offset & Pre/Post-Indexed)
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.149 / C6.2.304
- **Size Encodings:**
  - `size = 00`: 8-bit (`u8`)
  - `size = 01`: 16-bit (`u16`)
  - `size = 10`: 32-bit (`u32`)
  - `size = 11`: 64-bit (`u64`)
- **Indexing Modes:**
  - Unsigned offset: `[Rn, #offset]`
  - Pre-indexed: `[Rn, #offset]!` (updates $Rn = Rn + offset$)
  - Post-indexed: `[Rn], #offset` (updates $Rn = Rn + offset$ after access)
- **Alignment & Transactional Semantics:** Scalar unaligned accesses are permitted on Normal memory per Tier 2 alignment contract. If memory access or base register calculation encounters a fault (e.g. `ExecError::MemoryFault`), no registers or memory are modified, and $PC$ retains the address of the faulting instruction.

### 5.2 `LDP` / `STP` (Load and Store Pair)
- **Reference:** Arm ARM DDI 0487K.a, Section C6.2.146 / C6.2.301
- **Canonical:** [Arm Developer A64: LDP](https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/LDP--Load-Pair-of-Registers-)
- **Operands:** `Rt`, `Rt2`, `Rn`, signed 7-bit immediate offset.
- **Semantics:** Reads or writes two consecutive words/doublewords from memory into `Rt` and `Rt2`, supporting pre-indexed, post-indexed, and signed offset modes.
- **Alignment & Transactional Semantics:** Requires natural element alignment (4-byte alignment for 32-bit transfers, 8-byte alignment for 64-bit transfers) enforced by the Interpreter before memory access (Tier 1 alignment contract). On alignment violation (`ExecError::AlignmentFault`) or memory fault (`ExecError::MemoryFault`), execution halts atomically without modifying `Rt`, `Rt2`, `Rn`, or $PC$.
