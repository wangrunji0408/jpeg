use super::{error, Decoder};
use num_enum::TryFromPrimitive;
use std::{
    fmt::Debug,
    io::{Read, Result},
};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTable {
    pub class: HuffmanTableClass,
    pub map: HuffmanTree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, TryFromPrimitive)]
#[repr(u8)]
pub enum HuffmanTableClass {
    DC0 = 0x00,
    DC1 = 0x01,
    AC0 = 0x10,
    AC1 = 0x11,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTree {
    len: [u8; 1 << 16],
    val: [u8; 1 << 16],
}

impl HuffmanTree {
    pub const fn new() -> Self {
        HuffmanTree {
            len: [0; 1 << 16],
            val: [0; 1 << 16],
        }
    }

    pub fn insert(&mut self, code: u16, len: u8, val: u8) {
        assert!(len <= 16);
        let base = (code << (16 - len)) as usize;
        let range = base..=(base | ((1 << (16 - len)) - 1));
        self.len[range.clone()].fill(len);
        self.val[range].fill(val);
    }

    /// Decode a value from the stream. Return (len, val).
    pub fn get(&self, code: u16) -> (u8, u8) {
        let len = self.len[code as usize];
        let val = self.val[code as usize];
        (len, val)
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
            debug!(?counts, "leaf nodes");
            len -= 1 + 16;

            let mut code = 0;
            let mut h = 0;
            let mut map = HuffmanTree::new();
            for count in counts {
                code *= 2;
                h += 1;
                for _ in 0..count {
                    let value = self.read_byte()?;
                    map.insert(code, h, value);
                    code += 1;
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
                map: {
                    let mut map = HuffmanTree::new();
                    map.insert(0b00, 2, 0);
                    map.insert(0b010, 3, 1);
                    map.insert(0b011, 3, 2);
                    map.insert(0b100, 3, 3);
                    map.insert(0b101, 3, 4);
                    map.insert(0b110, 3, 5);
                    map.insert(0b1110, 4, 6);
                    map.insert(0b11110, 5, 7);
                    map.insert(0b111110, 6, 8);
                    map.insert(0b1111110, 7, 9);
                    map.insert(0b11111110, 8, 10);
                    map.insert(0b111111110, 9, 11);

                    assert_eq!(map.get(0), (2, 0));
                    assert_eq!(map.get(0b010 << 13), (3, 1));
                    map
                }
            }]
        );
    }
}
