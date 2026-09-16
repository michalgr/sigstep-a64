//! Foundational traits, error taxonomies, and register representations.

pub mod bits;
pub mod error;
pub mod reg;
pub mod traits;

pub use bits::{extract_bits, sign_extend};
pub use error::{AccessKind, DecodeError, ExecError, MemoryOrdering};
pub use reg::{CoreReg, Gpr, Nzcv, VReg};
pub use traits::{AsyncSignalSafe, MemoryInterface, RegisterBank};
