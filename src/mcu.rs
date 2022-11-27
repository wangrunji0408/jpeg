use crate::{
    error,
    huffman::{HuffmanTable, HuffmanTree},
    start_of_frame_0::StartOfFrameInfo,
    start_of_scan::StartOfScanInfo,
};
use std::{
    fmt::Debug,
    io::{BufReader, Read, Result},
};

/// Minimum Coded Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mcu {
    pub blocks: Vec<Block>,
}

/// 8x8 Block.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(align(32))] // optimize
pub struct Block(pub [i16; 64]);

impl Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..8 {
            for j in 0..8 {
                write!(f, " {}", self.0[i * 8 + j])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub struct McuReader<R: Read> {
    reader: BitReader<R>,
    sof: StartOfFrameInfo,
    huffman_tables: Vec<(HuffmanTree, HuffmanTree)>,
    last_dc: [i16; 3],
    i: usize,
    total: usize,
}

impl<R: Read> McuReader<R> {
    /// Read minimum coded units (MCU).
    pub(super) fn new(
        decoder: BufReader<R>,
        sof: StartOfFrameInfo,
        sos: StartOfScanInfo,
        huffman: &[HuffmanTable],
    ) -> Result<Self> {
        let mut huffman_tables = Vec::with_capacity(3);
        for id in sos.table_mapping {
            let dc = huffman
                .iter()
                .find(|h| h.class == id.dc)
                .ok_or_else(|| error(format!("huffman table not found: {:?}", id.dc)))?;
            let ac = huffman
                .iter()
                .find(|h| h.class == id.ac)
                .ok_or_else(|| error(format!("huffman table not found: {:?}", id.ac)))?;
            huffman_tables.push((dc.map.clone(), ac.map.clone()));
        }
        Ok(McuReader {
            reader: BitReader::new(decoder),
            total: sof.mcu_height_num() as usize * sof.mcu_width_num() as usize,
            sof,
            huffman_tables,
            last_dc: [0; 3],
            i: 0,
        })
    }

    /// Read a minimum coded unit (MCU).
    pub fn next(&mut self) -> Result<Option<Mcu>> {
        if self.i == self.total {
            return Ok(None);
        }
        if self.reader.reset {
            self.reader.reset();
            self.last_dc = [0; 3];
        }
        let mut blocks = Vec::with_capacity(6);
        for (id, component) in self.sof.component_infos.clone().iter().enumerate() {
            for _ in 0..component.vertical_sampling {
                for _ in 0..component.horizontal_sampling {
                    let block = self.read_block(id)?;
                    blocks.push(block);
                }
            }
        }
        self.i += 1;
        Ok(Some(Mcu { blocks }))
    }

    /// Read a minimum coded unit (MCU).
    fn read_block(&mut self, id: usize) -> Result<Block> {
        let mut x = [0; 64];
        x[0] = self.read_dc(id)?;
        let (_, ac) = &self.huffman_tables[id];
        let mut i = 1;
        while i < 64 {
            match self.reader.read_decode_haffman(ac)? {
                0x00 => break,
                0xF0 => i += 16,
                code => {
                    let zeros = (code >> 4) as usize;
                    let value = self.reader.read_value(code & 0x0F)?;
                    x[i + zeros] = value;
                    i += zeros + 1;
                }
            }
        }
        Ok(Block(x))
    }

    /// Read a DC value.
    fn read_dc(&mut self, id: usize) -> Result<i16> {
        let (map, _) = &self.huffman_tables[id];
        let dc = &mut self.last_dc[id];
        let len = self.reader.read_decode_haffman(map)?;
        *dc += self.reader.read_value(len)?;
        Ok(*dc)
    }
}

struct BitReader<R: Read> {
    reader: BufReader<R>,
    buf: u32,
    /// The lower `count` bits of `buf` is valid.
    count: u8,
    /// Meet a RST or EOI marker.
    /// If set, should not read from `reader` any more and always return 0.
    reset: bool,
}

impl<R: Read> BitReader<R> {
    fn new(reader: BufReader<R>) -> Self {
        Self {
            reader,
            buf: 0,
            count: 0,
            reset: false,
        }
    }

    fn reset(&mut self) {
        self.buf = 0;
        self.count = 0;
        self.reset = false;
    }

    fn read_decode_haffman(&mut self, map: &HuffmanTree) -> Result<u8> {
        let x = self.peek(16)?;
        let (len, val) = map.get(x);
        assert_ne!(len, 0);
        self.consume(len);
        // tracing::debug!("haffman: {len} {val}");
        Ok(val)
    }

    /// Read an encoded value in length.
    fn read_value(&mut self, len: u8) -> Result<i16> {
        if len == 0 {
            return Ok(0);
        }
        let mut v = self.peek(len)? as i16;
        if v >> (len - 1) == 0 {
            v -= (1 << len) - 1;
        }
        self.consume(len);
        // tracing::debug!("value: {len} {v}");
        Ok(v)
    }

    /// Peek the next `n` bits.
    fn peek(&mut self, n: u8) -> Result<u16> {
        debug_assert!(n <= 16);
        while self.count < n {
            if self.reset {
                self.buf <<= 8;
                self.count += 8;
                continue;
            }
            let mut b = 0;
            self.reader.read(std::slice::from_mut(&mut b))?;
            if b == 0xFF {
                let mut c = 0;
                self.reader.read_exact(std::slice::from_mut(&mut c))?;
                // skip RSTn (0xDn) or EOI (0xD9)
                if c != 0 {
                    self.reset = true;
                    continue;
                }
            }
            self.buf = (self.buf << 8) | b as u32;
            self.count += 8;
        }
        Ok((self.buf >> (self.count - n)) as u16)
    }

    /// Consume `n` bits.
    fn consume(&mut self, n: u8) {
        self.count -= n;
        self.buf &= (1 << self.count) - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Decoder;

    #[test]
    fn test_read_mcu() {
        // tracing_subscriber::fmt::init();
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let decoder = Decoder::new(file);
        let (mut reader, _) = decoder.read().unwrap();
        while let Some(_mcu) = reader.next().unwrap() {}
    }

    #[test]
    fn bit_reader() {
        let buf = [0xFF, 0x00, 0b10101010, 0b00000000, 0xFF, 0xAA];
        let mut reader = BitReader::new(BufReader::new(&buf[..]));
        assert_eq!(reader.peek(7).unwrap(), 0b1111111);
        assert_eq!(reader.peek(16).unwrap(), 0b11111111_10101010);
        reader.consume(4);
        assert_eq!(reader.peek(16).unwrap(), 0b1111_10101010_0000);
        reader.consume(4);
        assert_eq!(reader.peek(16).unwrap(), 0b10101010_00000000);
        assert_eq!(reader.read_value(3).unwrap(), 5);
        assert_eq!(reader.read_value(2).unwrap(), -2);
        assert_eq!(reader.peek(11).unwrap(), 0b010_00000000);
    }
}
