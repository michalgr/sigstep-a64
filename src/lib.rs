//! # sigstep-a64
//!
//! A `no_std` ARM64 (AArch64) instruction decoder and execution engine
//! parameterized over register and memory semantics, with compile-time
//! async-signal-safety guarantees.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// Placeholder module skeleton for core abstractions
pub mod core {
    //! Foundational traits, error taxonomies, and register representations.
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_crate_bootstrap() {
        assert_eq!(2 + 2, 4);
    }
}
