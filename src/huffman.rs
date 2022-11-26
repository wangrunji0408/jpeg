use super::Decoder;
use num_enum::TryFromPrimitive;
use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Read, Result},
    str::FromStr,
};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTable {
    pub class: HuffmanTableClass,
    pub map: Vec<(Code, u8)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, TryFromPrimitive)]
#[repr(u8)]

pub enum HuffmanTableClass {
    DC0 = 0x00,
    DC1 = 0x01,
    AC0 = 0x10,
    AC1 = 0x11,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code {
    pub value: u8,
    pub len: u8,
}

impl Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:0len$b}", self.value, len = self.len as usize)
    }
}

impl FromStr for Code {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Code {
            value: u8::from_str_radix(s, 2)
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid code"))?,
            len: s.len() as u8,
        })
    }
}

impl Code {
    fn inc(&mut self) {
        self.value += 1;
    }
    fn double(&mut self) {
        self.value <<= 1;
        self.len += 1;
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
            let class = HuffmanTableClass::try_from(byte).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid huffman table class: 0x{byte:02x}"),
                )
            })?;
            debug!(?class, "read huffman table");
            let mut counts = [0; 16];
            self.reader.read_exact(&mut counts)?;
            len -= 1 + 16;

            let mut code = Code::default();
            let mut map = vec![];
            for count in counts {
                code.double();
                for _ in 0..count {
                    let value = self.read_byte()?;
                    map.push((code, value));
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
                map: vec![
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
                    ("011111110".parse().unwrap(), 11)
                ]
            }]
        );
    }
}
