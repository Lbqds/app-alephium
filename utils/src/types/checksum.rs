use crate::buffer::Buffer;
use crate::decode::*;
use crate::djb_hash_with_prefix;
use crate::fixed_size_bytes;

fixed_size_bytes!(Checksum, 4);

impl Checksum {
    pub fn check(self: &Self, prefix: u8, bytes: &[u8]) -> bool {
        let hash: [u8; 4] = djb_hash_with_prefix(prefix, bytes).to_be_bytes();
        self.0 == hash
    }
}

#[cfg(test)]
mod tests {
    use super::Checksum;
    use crate::types::{byte32::tests::gen_bytes, u256::tests::hex_to_bytes};

    #[test]
    fn test_checksum() {
        let test_vectors = [
            (
                hex_to_bytes(
                    "038652277ddfd3b919e0d4415fffa9cc0e05838541ecc9bd0007bf4716604fdcdd17",
                )
                .unwrap(),
                hex_to_bytes("e7bc6f40").unwrap(),
            ),
            (
                hex_to_bytes(
                    "00e7e379a63b39fef2cb42e5cf88bef4e78c9bd2ec0de6f28a0a0ba91937e7260624",
                )
                .unwrap(),
                hex_to_bytes("dfe0ed4d").unwrap(),
            ),
            (
                hex_to_bytes(
                    "011c6465f5fb36f036b9915699fe467b14bcbb260dc004f3f68fdedb37101b48edc7",
                )
                .unwrap(),
                hex_to_bytes("48875be0").unwrap(),
            ),
            (
                hex_to_bytes("0262e9a4736b18738ccb1dbe6d0b05872c6019faf21d269f00d1744363a86a8aed")
                    .unwrap(),
                hex_to_bytes("b76dc73c").unwrap(),
            ),
        ];
        for case in test_vectors {
            let prefix = case.0[0];
            let bytes = &case.0[1..];
            let checksum = Checksum(case.1.try_into().unwrap());
            assert_eq!(checksum.check(prefix, &bytes), true);
            let invalid_checksum = Checksum(gen_bytes(4, 4).try_into().unwrap());
            assert_eq!(invalid_checksum.check(prefix, &bytes), false);
        }
    }
}
