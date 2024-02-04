use crate::buffer::{Buffer, Writable};
use crate::decode::*;
use crate::types::compact_integer::*;

use super::reset;

#[cfg_attr(test, derive(Debug))]
pub struct U256 {
    pub bytes: [u8; 33],
}

impl Default for U256 {
    fn default() -> Self {
        Self { bytes: [0; 33] }
    }
}

impl Reset for U256 {
    fn reset(&mut self) {
        self.bytes = [0; 33];
    }
}

impl PartialEq for U256 {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

fn trim(dest: &[u8]) -> &[u8] {
    let mut index = dest.len() - 1;
    while index != 0 {
        if dest[index] == b'0' {
            index -= 1;
        } else {
            break;
        }
    }
    if dest[index] == b'.' {
        &dest[..index]
    } else {
        &dest[..index + 1]
    }
}

impl U256 {
    const ALPH_DECIMALS: usize = 18;
    const DECIMAL_PLACES: usize = 6;
    const _1000_NANO_ALPH: u64 =
        (10 as u64).pow((Self::ALPH_DECIMALS - Self::DECIMAL_PLACES) as u32);

    #[inline]
    pub fn get_length(&self) -> usize {
        decode_length(self.bytes[0])
    }

    #[inline]
    pub fn is_fixed_size(&self) -> bool {
        is_fixed_size(self.bytes[0])
    }

    #[cfg(test)]
    pub fn from_encoded_bytes(bytes: &[u8]) -> Self {
        let mut bs = [0u8; 33];
        bs[..bytes.len()].copy_from_slice(bytes);
        Self { bytes: bs }
    }

    pub fn is_zero(&self) -> bool {
        self.get_length() == 1 && self.bytes.iter().all(|v| *v == 0)
    }

    fn decode_fixed_size(bytes: &[u8]) -> u32 {
        assert!(bytes.len() <= 4);
        let mut result: u32 = ((bytes[0] as u32) & MASK_MODE) << ((bytes.len() - 1) * 8);
        let mut index = 1;
        while index < bytes.len() {
            let byte = bytes[index];
            result |= ((byte & 0xff) as u32) << ((bytes.len() - index - 1) * 8);
            index += 1;
        }
        result
    }

    pub fn to_str<'a>(&self, output: &'a mut [u8]) -> Option<&'a [u8]> {
        if output.len() == 0 {
            return None;
        }
        if self.is_zero() {
            output[0] = b'0';
            return Some(&output[..1]);
        }

        let length = self.get_length();
        let mut bytes = [0u8; 32];
        if self.is_fixed_size() {
            let value = Self::decode_fixed_size(&self.bytes[..length]);
            bytes[28..].copy_from_slice(&value.to_be_bytes());
        } else {
            bytes[(33 - length)..].copy_from_slice(&self.bytes[1..length])
        }
        let mut index = output.len();
        while !bytes.into_iter().all(|v| v == 0) {
            if index == 0 {
                return None;
            }
            index -= 1;
            let mut carry = 0u16;
            for i in 0..32 {
                let v = (carry << 8) | (bytes[i] as u16);
                let rem = v % 10;
                bytes[i] = (v / 10) as u8;
                carry = rem;
            }
            output[index] = b'0' + (carry as u8);
        }
        output.copy_within(index..output.len(), 0);
        Some(&output[..(output.len() - index)])
    }

    fn to_str_with_decimals<'a>(
        &self,
        output: &'a mut [u8],
        decimals: usize,
        decimal_places: usize,
    ) -> Option<&'a [u8]> {
        reset(output);
        let str = self.to_str(output)?;
        let str_length = str.len();
        if decimals == 0 {
            return Some(&output[..str_length]);
        }

        if str_length > decimals {
            let decimal_index = str_length - decimals;
            output.copy_within(decimal_index..str_length, decimal_index + 1);
            output[decimal_index] = b'.';
            return Some(trim(&output[..(decimal_index + decimal_places + 1)]));
        }

        let pad_size = decimals - str_length;
        output.copy_within(0..str_length, 2 + pad_size);
        for i in 0..(2 + pad_size) {
            if i == 1 {
                output[i] = b'.';
            } else {
                output[i] = b'0';
            }
        }
        return Some(trim(&output[..(2 + decimal_places)]));
    }

    fn is_less_than_1000_nano(&self) -> bool {
        if self.is_fixed_size() {
            return true;
        }
        let length = self.get_length();
        if length > 8 {
            return false;
        }
        let mut value: u64 = 0;
        let mut index = 1;
        while index < length {
            let byte = self.bytes[index];
            value = (value << 8) | ((byte & 0xff) as u64);
            if value >= Self::_1000_NANO_ALPH {
                return false;
            }
            index += 1
        }
        return true;
    }

    pub fn to_alph<'a>(&self, output: &'a mut [u8]) -> Option<&'a [u8]> {
        reset(output);
        let postfix = b" ALPH";
        if self.is_zero() {
            output[0] = b'0';
            let total_size = 1 + postfix.len();
            output[1..total_size].copy_from_slice(postfix);
            return Some(&output[..total_size]);
        }

        if self.is_less_than_1000_nano() {
            let str = b"<0.000001";
            let total_size = str.len() + postfix.len();
            if output.len() < total_size {
                return None;
            }
            output[..str.len()].copy_from_slice(str);
            output[str.len()..total_size].copy_from_slice(postfix);
            return Some(&output[..total_size]);
        }

        if output.len() < 28 + postfix.len() {
            // max ALPH amount
            return None;
        }

        let str = self.to_str_with_decimals(output, Self::ALPH_DECIMALS, Self::DECIMAL_PLACES)?;
        let str_length = str.len();
        let total_size = str_length + postfix.len();
        output[str_length..total_size].copy_from_slice(postfix);
        return Some(&output[..total_size]);
    }
}

impl RawDecoder for U256 {
    fn step_size(&self) -> u16 {
        1
    }

    fn decode<'a, W: Writable>(
        &mut self,
        buffer: &mut Buffer<'a, W>,
        stage: &DecodeStage,
    ) -> DecodeResult<DecodeStage> {
        if buffer.is_empty() {
            return Ok(DecodeStage { ..*stage });
        }
        let from_index = if stage.index == 0 {
            self.bytes[0] = buffer.next_byte().unwrap();
            1
        } else {
            stage.index
        };
        let length = self.get_length();
        let mut idx = 0;
        while !buffer.is_empty() && idx < (length - (from_index as usize)) {
            self.bytes[(from_index as usize) + idx] = buffer.next_byte().unwrap();
            idx += 1;
        }
        let new_index = (from_index as usize) + idx;
        if new_index == length {
            Ok(DecodeStage::COMPLETE)
        } else {
            Ok(DecodeStage {
                step: stage.step,
                index: new_index as u16,
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    extern crate std;

    use crate::buffer::Buffer;
    use crate::types::u256::U256;
    use crate::{decode::*, TempData};
    use core::str::from_utf8;
    use rand::Rng;
    use std::string::String;
    use std::vec::Vec;

    pub fn hex_to_bytes(hex_string: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
        (0..hex_string.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_string[i..i + 2], 16))
            .collect()
    }

    fn random_usize(from: usize, to: usize) -> usize {
        let mut rng = rand::thread_rng();
        rng.gen_range(from..=to)
    }

    pub struct TestCase<'a>(pub Vec<u8>, pub &'a str);
    pub fn get_test_vector<'a>() -> [TestCase<'a>; 31] {
        [
            TestCase(hex_to_bytes("00").unwrap(), "0"),
            TestCase(hex_to_bytes("01").unwrap(), "1"),
            TestCase(hex_to_bytes("02").unwrap(), "2"),
            TestCase(hex_to_bytes("3e").unwrap(), "62"),
            TestCase(hex_to_bytes("3f").unwrap(), "63"),
            TestCase(hex_to_bytes("4040").unwrap(), "64"),
            TestCase(hex_to_bytes("4041").unwrap(), "65"),
            TestCase(hex_to_bytes("4042").unwrap(), "66"),
            TestCase(hex_to_bytes("7ffe").unwrap(), "16382"),
            TestCase(hex_to_bytes("7fff").unwrap(), "16383"),
            TestCase(hex_to_bytes("80004000").unwrap(), "16384"),
            TestCase(hex_to_bytes("80004001").unwrap(), "16385"),
            TestCase(hex_to_bytes("80004002").unwrap(), "16386"),
            TestCase(hex_to_bytes("bffffffe").unwrap(), "1073741822"),
            TestCase(hex_to_bytes("bfffffff").unwrap(), "1073741823"),
            TestCase(hex_to_bytes("c040000000").unwrap(), "1073741824"),
            TestCase(hex_to_bytes("c040000001").unwrap(), "1073741825"),
            TestCase(hex_to_bytes("c040000002").unwrap(), "1073741826"),
            TestCase(
                hex_to_bytes("c5010000000000000000").unwrap(),
                "18446744073709551616",
            ),
            TestCase(
                hex_to_bytes("c5010000000000000001").unwrap(),
                "18446744073709551617",
            ),
            TestCase(
                hex_to_bytes("c4ffffffffffffffff").unwrap(),
                "18446744073709551615",
            ),
            TestCase(
                hex_to_bytes("cd00000000000000ff00000000000000ff00").unwrap(),
                "1204203453131759529557760",
            ),
            TestCase(
                hex_to_bytes("cd0100000000000000000000000000000000").unwrap(),
                "340282366920938463463374607431768211456",
            ),
            TestCase(
                hex_to_bytes("cd0100000000000000000000000000000001").unwrap(),
                "340282366920938463463374607431768211457",
            ),
            TestCase(
                hex_to_bytes("ccffffffffffffffffffffffffffffffff").unwrap(),
                "340282366920938463463374607431768211455",
            ),
            TestCase(
                hex_to_bytes("d501000000000000000000000000000000000000000000000000").unwrap(),
                "6277101735386680763835789423207666416102355444464034512896",
            ),
            TestCase(
                hex_to_bytes("d501000000000000000000000000000000000000000000000001").unwrap(),
                "6277101735386680763835789423207666416102355444464034512897",
            ),
            TestCase(
                hex_to_bytes("d4ffffffffffffffffffffffffffffffffffffffffffffffff").unwrap(),
                "6277101735386680763835789423207666416102355444464034512895",
            ),
            TestCase(
                hex_to_bytes("dcffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                    .unwrap(),
                "115792089237316195423570985008687907853269984665640564039457584007913129639935",
            ),
            TestCase(
                hex_to_bytes("dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe")
                    .unwrap(),
                "115792089237316195423570985008687907853269984665640564039457584007913129639934",
            ),
            TestCase(
                hex_to_bytes("dcfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd")
                    .unwrap(),
                "115792089237316195423570985008687907853269984665640564039457584007913129639933",
            ),
        ]
    }

    #[test]
    fn test_decode_u256() {
        let arrays = get_test_vector();
        let mut temp_data = TempData::new();
        for item in arrays {
            let bytes = item.0.as_slice();

            {
                let mut decoder = new_decoder::<U256>();
                let mut buffer = Buffer::new(bytes, &mut temp_data).unwrap();
                let result = decoder.decode(&mut buffer).unwrap();
                assert!(result.is_some());
                let result = result.unwrap();
                let length = result.get_length();
                assert_eq!(bytes, &result.bytes[..length]);
                assert!(decoder.stage.is_complete());
            }

            let mut length: usize = 0;
            let mut decoder = new_decoder::<U256>();

            while length < bytes.len() {
                let remain = bytes.len() - length;
                let size = random_usize(0, remain);
                let mut buffer =
                    Buffer::new(&bytes[length..(length + size)], &mut temp_data).unwrap();
                length += size;

                let result = decoder.decode(&mut buffer).unwrap();
                if length == bytes.len() {
                    assert!(result.is_some());
                    let result = result.unwrap();
                    let length = result.get_length();
                    assert_eq!(bytes, &result.bytes[..length]);
                    assert!(decoder.stage.is_complete());
                } else {
                    assert_eq!(result, None);
                    assert_eq!(decoder.stage.index as usize, length);
                }
            }
        }
    }

    const MAX_OF_4_BYTES_ENCODED: u128 = 1073741823;
    fn encode_fixed_bytes(n: u32) -> U256 {
        if n < 0x40 {
            U256::from_encoded_bytes(&[n as u8])
        } else if n < (0x40 << 8) {
            U256::from_encoded_bytes(&[((n >> 8) + 0x40) as u8, n as u8])
        } else if n < (0x40 << 24) {
            U256::from_encoded_bytes(&[
                ((n >> 24) + 0x40) as u8,
                (n >> 16) as u8,
                (n >> 8) as u8,
                n as u8,
            ])
        } else {
            panic!()
        }
    }

    fn encode_u128(value: u128) -> U256 {
        if value <= MAX_OF_4_BYTES_ENCODED {
            encode_fixed_bytes(value as u32)
        } else {
            let mut bytes: Vec<u8> = value
                .to_be_bytes()
                .iter()
                .cloned()
                .skip_while(|&b| b == 0)
                .collect();
            let header: u8 = ((bytes.len() - 4) as u8) | 0xc0;
            bytes.insert(0, header);
            U256::from_encoded_bytes(&bytes)
        }
    }

    #[test]
    fn test_is_less_than_1000_nano_alph() {
        let u2560 = encode_u128((U256::_1000_NANO_ALPH - 1) as u128);
        let u2561 = encode_u128((U256::_1000_NANO_ALPH) as u128);
        let u2562 = encode_u128((U256::_1000_NANO_ALPH + 1) as u128);

        assert!(u2560.is_less_than_1000_nano());
        assert!(!u2561.is_less_than_1000_nano());
        assert!(!u2562.is_less_than_1000_nano());
        assert!(!encode_u128(u128::MAX).is_less_than_1000_nano())
    }

    #[test]
    fn test_to_alph() {
        let alph = |str: &str| {
            let index_opt = str.find('.');
            let mut result_str = String::new();
            if index_opt.is_none() {
                result_str.extend(str.chars());
                let pad: String = std::iter::repeat('0').take(U256::ALPH_DECIMALS).collect();
                result_str.extend(pad.chars());
            } else {
                let index = index_opt.unwrap();
                let pad_size = U256::ALPH_DECIMALS - (str.len() - index_opt.unwrap()) + 1;
                result_str.extend(str[0..index].chars());
                result_str.extend(str[(index + 1)..].chars());
                let pad: String = std::iter::repeat('0').take(pad_size).collect();
                result_str.extend(pad.chars());
            }
            result_str.parse::<u128>().unwrap()
        };

        let cases = [
            (0, "0"),
            (U256::_1000_NANO_ALPH as u128, "0.000001"),
            ((10 as u128).pow(12), "0.000001"),
            ((U256::_1000_NANO_ALPH as u128) - 1, "<0.000001"),
            ((10 as u128).pow(13), "0.00001"),
            ((10 as u128).pow(14), "0.0001"),
            ((10 as u128).pow(17), "0.1"),
            ((10 as u128).pow(17), "0.1"),
            ((10 as u128).pow(18), "1"),
            (alph("0.11111111111"), "0.111111"),
            (alph("111111.11111111"), "111111.111111"),
            (alph("1.010101"), "1.010101"),
            (alph("1.101010"), "1.10101"),
            (alph("1.9999999"), "1.999999"),
        ];
        for (number, str) in cases {
            let u256 = encode_u128(number);
            let mut output = [0u8; 33];
            let result = u256.to_alph(&mut output);
            assert!(result.is_some());
            let expected = from_utf8(result.unwrap()).unwrap();
            let amount_str = String::from(str) + " ALPH";
            assert_eq!(amount_str, String::from(expected));
        }

        let test_vector = get_test_vector();
        let u256 = U256::from_encoded_bytes(&test_vector[test_vector.len() - 1].0);
        let mut output = [0u8; 33];
        assert!(u256.to_alph(&mut output).is_none());
    }

    #[test]
    fn test_to_str() {
        let test_vector = get_test_vector();

        for case in test_vector.iter() {
            let u256 = U256::from_encoded_bytes(&case.0);
            let mut output = [0u8; 78];
            let result = u256.to_str(&mut output).unwrap();
            let expected = from_utf8(&result).unwrap();
            assert_eq!(expected, case.1);
        }

        let case = &test_vector[test_vector.len() - 1];
        let u256 = U256::from_encoded_bytes(&case.0);
        let mut output = [0u8; 19];
        let result = u256.to_str(&mut output);
        assert!(result.is_none());
    }
}
