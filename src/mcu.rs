use super::Decoder;
use crate::{
    error,
    huffman::{Code, HuffmanTable, HuffmanTree},
    start_of_frame_0::StartOfFrameInfo,
    start_of_scan::StartOfScanInfo,
};
use std::{
    fmt::Debug,
    io::{Read, Result},
};

/// Minimum Coded Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mcu {
    blocks: Vec<Block>,
}

/// 8x8 Block.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
    pub(super) fn from_decoder(
        decoder: Decoder<R>,
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
            total: sof.mcu_height() as usize * sof.mcu_width() as usize,
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
    reader: Decoder<R>,
    buf: u8,
    count: u8,
}

impl<R: Read> BitReader<R> {
    fn new(reader: Decoder<R>) -> Self {
        Self {
            reader,
            buf: 0,
            count: 0,
        }
    }

    fn read_decode_haffman(&mut self, map: &HuffmanTree) -> Result<u8> {
        let mut code = Code::default();
        loop {
            code.push(self.read_bit()?);
            if let Some(&value) = map.get(&code) {
                return Ok(value);
            }
        }
    }

    /// Read an encoded value in length.
    fn read_value(&mut self, len: u8) -> Result<i16> {
        if len == 0 {
            return Ok(0);
        }
        let mut ret: i16 = 1;
        let first = self.read_bit()?;
        for _ in 1..len {
            let b = self.read_bit()?;
            ret = (ret << 1) + (first == b) as i16;
        }
        ret = if first { ret } else { -ret };
        Ok(ret)
    }

    /// Read a bit.
    fn read_bit(&mut self) -> Result<bool> {
        if self.count == 0 {
            self.buf = self.reader.read_byte()?;
            if self.buf == 0xFF {
                let check = self.reader.read_byte()?;
                if check != 0 {
                    return Err(error("0xFF must be followed with 0x00 in compressed data"));
                }
            }
        }
        let ret = (self.buf & (1 << (7 - self.count))) != 0;
        self.count = if self.count == 7 { 0 } else { self.count + 1 };
        Ok(ret)
    }
}
