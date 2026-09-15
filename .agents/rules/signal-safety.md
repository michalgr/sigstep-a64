# Agent Rule: Async-Signal Safety & Reentrancy Mandate

**Target Agents:** Jules, Antigravity, and all automated/human contributors.  
**Enforcement Scope:** Mandatory for `sigstep-a64` core engine (`src/core/*`, `src/decode/*`, `src/interp/*`), traits, and any component implementing `AsyncSignalSafe`.

---

## 1. Context and Problem Statement

Code executing inside a POSIX signal handler (e.g., handling `SIGILL`, `SIGTRAP`, `SIGSEGV`, or `SIGBUS`) operates in an asynchronous, interrupted context:
1. The interrupted thread may have been paused mid-execution inside any library, runtime routine, or system call.
2. If the interrupted thread was holding an internal lock (such as a memory allocator heap lock or a logging lock), acquiring that same lock inside the signal handler leads to an immediate, irrecoverable **deadlock**.
3. If the signal handler allocates from or alters a shared heap in an inconsistent state, it causes **heap corruption**.
4. Signal handlers frequently run on an alternate signal stack (`sigaltstack`), configured with minimal size (often `SIGSTKSZ`, typically 8 KB to 64 KB). Any recursive function, large stack-allocated buffer, or frame bloat triggers fatal **stack overflow**.

Therefore, every core module in `sigstep-a64` must adhere to strict async-signal safety constraints.

---

## 2. Hard Invariants (Zero Tolerance)

### 2.1 Hard Ban on Dynamic Heap Allocation
- **Prohibited Items:**
  - `extern crate alloc;` in core modules.
  - Any heap-allocated container: `Vec`, `String`, `Box`, `BTreeMap`, `BTreeSet`, `LinkedList`, `Arc`, `Rc`.
  - Dynamic string formatting routines: `format!`, `format_args!`, `.to_string()`, `.to_owned()`.
  - Formatting macros with variable buffers in panics (e.g., `panic!("Failed with code {}", code)`). Only static string literal panics (e.g., `panic!("unreachable")`) are permissible, though panics themselves must be avoided in production paths.
- **Permitted Alternatives:**
  - Fixed-capacity array buffers (e.g., `[u8; N]`, `[Option<T>; N]`) on the stack, where `N` is small and bounded.
  - Compile-time constants (`const`) and static slices (`&'static str`).
  - Pass-by-value or small reference types implementing `Copy`.

### 2.2 Hard Ban on Non-Reentrant Synchronization
- **Prohibited Items:**
  - Standard blocking synchronization: `std::sync::Mutex`, `std::sync::RwLock`, `std::sync::Condvar`.
  - Spinlocks (e.g., `spin::Mutex`, hand-rolled atomic test-and-set spin loops): if the interrupted thread held the spinlock, spinning on the same core deadlocks indefinitely.
  - Futexes, semaphores, barrier primitives, thread parking (`std::thread::park`).
- **Permitted Alternatives:**
  - Reentrancy-aware single-threaded state machine designs.
  - Lock-free, wait-free atomic read/write/CAS operations (`core::sync::atomic::Atomic*`) using bounded memory ordering (e.g., `Ordering::Relaxed`, `Ordering::Acquire`, `Ordering::Release`).

### 2.3 Strict Stack Depth & Recursion Limits
- **Prohibited Items:**
  - Any form of recursion (direct or mutual recursion). All decoding, dispatching, and emulation loops must be iterative with provably finite loop bounds.
  - Large stack allocations: do not allocate large arrays (e.g., buffers $> 256$ bytes) on the stack frame.
- **Invariant:**
  - The maximum stack usage for any public API entrypoint (`Decoder::decode`, `Interpreter::step`) must not exceed **1 KB** across all combined stack frames to remain fully safe inside `sigaltstack`.

### 2.4 Allocation-Free, Static Error Handling
- **Prohibited Items:**
  - Dynamic error trait objects (`Box<dyn Error>`).
  - Error types holding dynamically formatted strings.
- **Mandatory Requirements:**
  - All fallible operations must return a static, `Copy`-able result: `Result<T, ExecError>` or `Result<T, DecodeError>`.
  - Every error variant must be self-contained, lightweight, and expressible as pure numerical/enum data.

---

## 3. Memory Access Policy for Emulated Instructions

When emulating memory instructions (`LDR`, `STR`, `LDP`, `STP`, etc.):

1. **Alignment Verification First:**
   - Memory access must check alignment invariants *before* attempting access if the architectural state or access mode requires strict alignment. Misaligned accesses must return `ExecError::AlignmentFault { address, required_alignment }` rather than hardware-faulting.
2. **Signal-Safe Memory Probing:**
   - A signal backend must never perform unchecked raw pointer dereferencing (`*mut T` / `*const T`) directly if the target address can point to unmapped, protected, or invalid pages. Dereferencing an invalid pointer inside a signal handler triggers a nested `SIGSEGV`, causing process termination.
   - Signal-safe backends must employ safe probing mechanisms:
     - Pre-validating address ranges against known mapped segments, OR
     - Using safe kernel primitives (e.g., `process_vm_readv` / `process_vm_writev`), OR
     - Architecture-specific fault recovery routines (such as `sigsetjmp` / `siglongjmp` harnesses or specialized page table inspection).
3. **No Unbounded Memory Iterations:**
   - Bulk memory operations or register-pair transfers must have fixed, small upper bounds (e.g., 16 bytes for `LDP`/`STP`).

---

## 4. Atomic Operations & Exclusive Monitors

1. **Hardware Atomics Safety:**
   - Native AArch64 atomic operations (LSE instructions such as `CAS`, `SWP`, `LDADD`) are executed as single machine instructions that do not allocate memory or acquire non-reentrant kernel locks. They are naturally async-signal safe when backed by direct memory on AArch64 hardware.
2. **Exclusive Monitors (LL/SC):**
   - Emulating `LDXR` / `STXR` requires tracking the local exclusive monitor.
   - The monitor state must be held entirely within the `MemoryInterface` or interpreter state without allocation.
   - If an intervening signal or context switch clears the monitor, `STXR` must cleanly return a status code of 1 (failed store) per Arm architecture specification, without panicking or blocking.

---

## 5. Automated CI & Code Review Checklist

Before approving or merging any PR affecting core modules:
- [ ] Ensure `#![no_std]` is enforced and no `extern crate alloc;` is introduced in `core`.
- [ ] Verify that `cargo clippy --all-targets -- -D warnings` passes without suppression of allocation lints.
- [ ] Audit all functions for stack frame sizes; verify no stack buffer exceeds 256 bytes.
- [ ] Confirm no loops exist without a static, deterministic iteration bound.
- [ ] Confirm no locks (`Mutex`, `RwLock`, `Spinlock`) are referenced.
- [ ] Ensure all errors derive `Copy, Clone, Debug, PartialEq, Eq`.
