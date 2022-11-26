use super::{error, Decoder};
use num_enum::TryFromPrimitive;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Error, Read, Result},
    str::FromStr,
};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTable {
    pub class: HuffmanTableClass,
    pub map: HuffmanTree,
}

pub type HuffmanTree = HashMap<Code, u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum HuffmanTableClass {
    DC0 = 0x00,
    DC1 = 0x01,
    AC0 = 0x10,
    AC1 = 0x11,
}

/// A Huffman code.
///
/// The inner value is the bit string as integer with an prefix 1.
/// e.g. "011" => Code(0b1011)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(u32);

impl Default for Code {
    fn default() -> Self {
        Code(1)
    }
}

impl Code {
    fn value(self) -> u32 {
        if self.0.is_power_of_two() {
            0
        } else {
            self.0 & !(self.0.next_power_of_two() >> 1)
        }
    }
    fn len(self) -> u32 {
        u32::BITS - 1 - self.0.leading_zeros()
    }

    pub fn push(&mut self, x: bool) {
        self.0 = (self.0 << 1) | (x as u32);
    }
}

impl Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0len$b}", self.value(), len = self.len() as usize)
    }
}

impl FromStr for Code {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let value = u32::from_str_radix(s, 2).map_err(|_| error("Invalid code"))?;
        Ok(Code(value | (1 << s.len() as u32)))
    }
}

impl Code {
    fn inc(&mut self) {
        self.0 += 1;
    }
    fn double(&mut self) {
        self.0 *= 2;
    }
}

impl<R: Read> Decoder<R> {
    /// Read the next marker.
    pub fn read_huffman_table(&mut self) -> Result<Vec<HuffmanTable>> {
        let mut len = self.read_u16()?;
        debug!(len, "read section DHT");
        len -= 2;
        let mut tables = vec![];
        while len != 0 {
            let byte = self.read_byte()?;
            let class = HuffmanTableClass::try_from(byte)
                .map_err(|_| error(format!("invalid huffman table class: 0x{byte:02x}")))?;
            debug!(?class, "read huffman table");
            let mut counts = [0; 16];
            self.reader.read_exact(&mut counts)?;
            len -= 1 + 16;

            let mut code = Code::default();
            let mut map = HashMap::new();
            for count in counts {
                code.double();
                for _ in 0..count {
                    let value = self.read_byte()?;
                    map.insert(code, value);
                    code.inc();
                }
                len -= count as u16;
            }
            tables.push(HuffmanTable { class, map });
        }
        Ok(tables)
    }
}

#[cfg(test)]
mod tests {
    use crate::marker::Marker;

    use super::*;

    #[test]
    fn test_huffman_table() {
        // tracing_subscriber::fmt::init();
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let mut decoder = Decoder::new(file);
        while decoder.next_marker().expect("failed to read marker") != Marker::DHT {}
        let dhts = decoder.read_huffman_table().expect("failed to read DHT");
        assert_eq!(
            dhts,
            vec![HuffmanTable {
                class: HuffmanTableClass::DC0,
                map: [
                    ("00".parse().unwrap(), 0),
                    ("010".parse().unwrap(), 1),
                    ("011".parse().unwrap(), 2),
                    ("100".parse().unwrap(), 3),
                    ("101".parse().unwrap(), 4),
                    ("110".parse().unwrap(), 5),
                    ("1110".parse().unwrap(), 6),
                    ("11110".parse().unwrap(), 7),
                    ("111110".parse().unwrap(), 8),
                    ("1111110".parse().unwrap(), 9),
                    ("11111110".parse().unwrap(), 10),
                    ("111111110".parse().unwrap(), 11),
                ]
                .into_iter()
                .collect(),
            }]
        );
    }
}
