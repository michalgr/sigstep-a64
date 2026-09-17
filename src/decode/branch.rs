//! Phase 1 unconditional branch decoders (B, BL, BR, BLR, RET).
//!
//! Provides decoding logic for control flow instructions strictly following
//! Arm Architecture Reference Manual (Arm ARM DDI 0487K.a).

use crate::core::bits::{extract_bits, sign_extend};
use crate::core::error::DecodeError;
use crate::core::reg::Gpr;
use crate::decode::Instruction;

/// Decodes Phase 1 unconditional branch instructions.
///
/// Returns `Ok(Some(Instruction))` if `raw` matches an unconditional branch encoding.
/// Returns `Ok(None)` if `raw` does not match any unconditional branch opcode.
/// Returns `Err(DecodeError::Reserved)` if register operand is reserved (e.g. Rn == 31).
///
/// # Instruction Citations
///
/// ## B — Branch (Immediate)
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
/// Section:   C6.2.25 "B - Branch"
/// Canonical: https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/B--Branch-
/// Encoding:
///   31 30 29 28 27 26 25                          0
///  +--+--+--+--+--+--+-----------------------------+
///  |0 |0 |0 |1 |0 |1 |            imm26            |
///  +--+--+--+--+--+--+-----------------------------+
/// Operational Pseudocode:
///   bits(64) offset = SignExtend(imm26:'00', 64);
///   BranchTo(PC[] + offset, BranchType_DIR);
///
/// ## BL — Branch with Link (Immediate)
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
/// Section:   C6.2.33 "BL - Branch with Link"
/// Canonical: https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BL--Branch-with-Link-
/// Encoding:
///   31 30 29 28 27 26 25                          0
///  +--+--+--+--+--+--+-----------------------------+
///  |1 |0 |0 |1 |0 |1 |            imm26            |
///  +--+--+--+--+--+--+-----------------------------+
/// Operational Pseudocode:
///   X[30, 64] = PC[] + 4;
///   bits(64) offset = SignExtend(imm26:'00', 64);
///   BranchTo(PC[] + offset, BranchType_CALL);
///
/// ## BR — Branch to Register
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
/// Section:   C6.2.34 "BR - Branch to Register"
/// Canonical: https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BR--Branch-to-Register-
/// Encoding:
///   31          25 24 21 20   16 15   10 9     5 4   0
///  +--------------+-----+-------+-------+-------+-----+
///  |   1101011    |0000 | 11111 |000000 |  Rn   |00000|
///  +--------------+-----+-------+-------+-------+-----+
/// Operational Pseudocode:
///   bits(64) target = X[n, 64];
///   BranchTo(target, BranchType_INDIR);
///
/// ## BLR — Branch with Link to Register
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
/// Section:   C6.2.35 "BLR - Branch with Link to Register"
/// Canonical: https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/BLR--Branch-with-Link-to-Register-
/// Encoding:
///   31          25 24 21 20   16 15   10 9     5 4   0
///  +--------------+-----+-------+-------+-------+-----+
///  |   1101011    |0001 | 11111 |000000 |  Rn   |00000|
///  +--------------+-----+-------+-------+-------+-----+
/// Operational Pseudocode:
///   X[30, 64] = PC[] + 4;
///   bits(64) target = X[n, 64];
///   BranchTo(target, BranchType_INDIRCALL);
///
/// ## RET — Return from Subroutine
/// Reference: Arm Architecture Reference Manual (Arm ARM DDI 0487K.a)
/// Section:   C6.2.235 "RET - Return from subroutine"
/// Canonical: https://developer.arm.com/documentation/ddi0596/2021-12/Base-Instructions/RET--Return-from-subroutine-
/// Encoding:
///   31          25 24 21 20   16 15   10 9     5 4   0
///  +--------------+-----+-------+-------+-------+-----+
///  |   1101011    |0010 | 11111 |000000 |  Rn   |00000|
///  +--------------+-----+-------+-------+-------+-----+
/// Operational Pseudocode:
///   bits(64) target = X[n, 64];
///   BranchTo(target, BranchType_RET);
pub fn decode_branch(raw: u32) -> Result<Option<Instruction>, DecodeError> {
    // B: (raw & 0xFC00_0000) == 0x1400_0000
    if (raw & 0xFC00_0000) == 0x1400_0000 {
        let imm26_raw = extract_bits(raw, 0, 26) as u64;
        let sign_extended = sign_extend(imm26_raw, 26) as i64;
        let offset = sign_extended << 2;
        return Ok(Some(Instruction::B { offset }));
    }

    // BL: (raw & 0xFC00_0000) == 0x9400_0000
    if (raw & 0xFC00_0000) == 0x9400_0000 {
        let imm26_raw = extract_bits(raw, 0, 26) as u64;
        let sign_extended = sign_extend(imm26_raw, 26) as i64;
        let offset = sign_extended << 2;
        return Ok(Some(Instruction::Bl { offset }));
    }

    // BR: (raw & 0xFFFF_FC1F) == 0xD61F_0000
    if (raw & 0xFFFF_FC1F) == 0xD61F_0000 {
        let rn_raw = (raw >> 5) & 0x1F;
        if rn_raw == 31 {
            return Err(DecodeError::Reserved(raw));
        }
        let rn = Gpr::new(rn_raw as u8).ok_or(DecodeError::Undefined(raw))?;
        return Ok(Some(Instruction::Br { rn }));
    }

    // BLR: (raw & 0xFFFF_FC1F) == 0xD63F_0000
    if (raw & 0xFFFF_FC1F) == 0xD63F_0000 {
        let rn_raw = (raw >> 5) & 0x1F;
        if rn_raw == 31 {
            return Err(DecodeError::Reserved(raw));
        }
        let rn = Gpr::new(rn_raw as u8).ok_or(DecodeError::Undefined(raw))?;
        return Ok(Some(Instruction::Blr { rn }));
    }

    // RET: (raw & 0xFFFF_FC1F) == 0xD65F_0000
    if (raw & 0xFFFF_FC1F) == 0xD65F_0000 {
        let rn_raw = (raw >> 5) & 0x1F;
        if rn_raw == 31 {
            return Err(DecodeError::Reserved(raw));
        }
        let rn = Gpr::new(rn_raw as u8).ok_or(DecodeError::Undefined(raw))?;
        return Ok(Some(Instruction::Ret { rn }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::Decoder;

    #[test]
    fn test_decode_b_immediates() {
        // B +8 (imm26 = 2)
        let raw_fwd = 0x1400_0002;
        assert_eq!(Decoder::decode(raw_fwd), Ok(Instruction::B { offset: 8 }));

        // B -8 (imm26 = -2 = 0x03FF_FFFE)
        let raw_back = 0x17FF_FFFE;
        assert_eq!(Decoder::decode(raw_back), Ok(Instruction::B { offset: -8 }));

        // B max positive immediate (+128 MB - 4 B = +134,217,724 B, imm26 = 0x01FF_FFFF)
        let raw_max_pos = 0x15FF_FFFF;
        assert_eq!(
            Decoder::decode(raw_max_pos),
            Ok(Instruction::B {
                offset: 134_217_724
            })
        );

        // B max negative immediate (-128 MB = -134,217,728 B, imm26 = 0x0200_0000)
        let raw_max_neg = 0x1600_0000;
        assert_eq!(
            Decoder::decode(raw_max_neg),
            Ok(Instruction::B {
                offset: -134_217_728
            })
        );
    }

    #[test]
    fn test_decode_bl_immediates() {
        // BL +16 (imm26 = 4)
        let raw_fwd = 0x9400_0004;
        assert_eq!(Decoder::decode(raw_fwd), Ok(Instruction::Bl { offset: 16 }));

        // BL -16 (imm26 = -4 = 0x03FF_FFFC)
        let raw_back = 0x97FF_FFFC;
        assert_eq!(
            Decoder::decode(raw_back),
            Ok(Instruction::Bl { offset: -16 })
        );
    }

    #[test]
    fn test_decode_register_branches() {
        let x0 = Gpr::new(0).unwrap();
        let x15 = Gpr::new(15).unwrap();
        let x30 = Gpr::new(30).unwrap();

        // BR X0
        assert_eq!(Decoder::decode(0xD61F_0000), Ok(Instruction::Br { rn: x0 }));
        // BR X15
        assert_eq!(
            Decoder::decode(0xD61F_0000 | (15 << 5)),
            Ok(Instruction::Br { rn: x15 })
        );

        // BLR X30
        assert_eq!(
            Decoder::decode(0xD63F_0000 | (30 << 5)),
            Ok(Instruction::Blr { rn: x30 })
        );

        // RET X30 (standard ret: 0xD65F03C0)
        assert_eq!(
            Decoder::decode(0xD65F_03C0),
            Ok(Instruction::Ret { rn: x30 })
        );
        // RET X15
        assert_eq!(
            Decoder::decode(0xD65F_0000 | (15 << 5)),
            Ok(Instruction::Ret { rn: x15 })
        );
    }

    #[test]
    fn test_decode_reserved_rn31() {
        // BR with Rn = 31
        let raw_br_rn31 = 0xD61F_0000 | (31 << 5);
        assert_eq!(
            Decoder::decode(raw_br_rn31),
            Err(DecodeError::Reserved(raw_br_rn31))
        );

        // BLR with Rn = 31
        let raw_blr_rn31 = 0xD63F_0000 | (31 << 5);
        assert_eq!(
            Decoder::decode(raw_blr_rn31),
            Err(DecodeError::Reserved(raw_blr_rn31))
        );

        // RET with Rn = 31
        let raw_ret_rn31 = 0xD65F_0000 | (31 << 5);
        assert_eq!(
            Decoder::decode(raw_ret_rn31),
            Err(DecodeError::Reserved(raw_ret_rn31))
        );
    }

    #[test]
    fn test_decode_unmatched_opcode() {
        let raw_unmatched = 0x0000_0000;
        assert_eq!(
            Decoder::decode(raw_unmatched),
            Err(DecodeError::Undefined(raw_unmatched))
        );
    }
}
