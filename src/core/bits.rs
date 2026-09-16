//! Bitfield manipulation and sign-extension utilities.
//!
//! Provides `const fn` helpers for extracting instruction bitfields and sign-extending immediates.

/// Extracts a contiguous range of `width` bits starting at bit position `lsb` from a 32-bit word.
///
/// Returns `0` if `width == 0` or `lsb >= 32`.
#[inline]
pub const fn extract_bits(val: u32, lsb: u32, width: u32) -> u32 {
    if lsb >= 32 || width == 0 {
        return 0;
    }
    let shifted = val >> lsb;
    if width >= 32 {
        shifted
    } else {
        let mask = (1u32 << width) - 1;
        shifted & mask
    }
}

/// Sign-extends a `bit_width`-bit signed integer value contained in `val` to a 64-bit unsigned integer representation.
///
/// Returns `val` as-is if `bit_width == 0` or `bit_width >= 64`.
#[inline]
pub const fn sign_extend(val: u64, bit_width: u32) -> u64 {
    if bit_width == 0 || bit_width >= 64 {
        return val;
    }
    let shift = 64 - bit_width;
    let signed = (val << shift) as i64 >> shift;
    signed as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bits() {
        let val: u32 = 0b1101_1010_0101_0011_1110_0000_1111_0000;

        assert_eq!(extract_bits(val, 4, 4), 0b1111);
        assert_eq!(extract_bits(val, 0, 1), 0);
        assert_eq!(extract_bits(val, 28, 5), 0b1101);
        assert_eq!(extract_bits(val, 0, 32), val);

        assert_eq!(extract_bits(val, 0, 0), 0);
        assert_eq!(extract_bits(val, 32, 1), 0);
        assert_eq!(extract_bits(val, 40, 5), 0);
    }

    #[test]
    fn test_sign_extend() {
        assert_eq!(sign_extend(0b011, 3), 3);
        assert_eq!(sign_extend(0b111, 3), 0xFFFF_FFFF_FFFF_FFFF);
        assert_eq!(sign_extend(0x7FF, 12), 2047);
        assert_eq!(sign_extend(0x800, 12), 0xFFFF_FFFF_FFFF_F800);

        let val_64 = 0x8000_0000_0000_0000;
        assert_eq!(sign_extend(val_64, 64), val_64);

        assert_eq!(sign_extend(0b111, 0), 0b111);
    }
}
