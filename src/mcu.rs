use super::Decoder;
use crate::{
    error,
    huffman::{Code, HuffmanTable, HuffmanTree},
    start_of_frame_0::StartOfFrameInfo,
    start_of_scan::StartOfScanInfo,
};
use std::io::{Read, Result};
use tracing::debug;

/// Minimum Coded Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mcu {
    blocks: Vec<Block>,
}

/// 8x8 Block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block([i16; 64]);

impl<R: Read> Decoder<R> {
    /// Read minimum coded units (MCU).
    pub fn read_mcus(
        &mut self,
        sof: &StartOfFrameInfo,
        sos: &StartOfScanInfo,
        huffman: &[HuffmanTable],
    ) -> Result<Vec<Mcu>> {
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
            huffman_tables.push((&dc.map, &ac.map));
        }
        McuReader {
            reader: BitReader::new(self),
            sof,
            huffman_tables,
            last_dc: [0; 3],
        }
        .read_mcus()
    }
}

struct McuReader<'a, R: Read> {
    reader: BitReader<'a, R>,
    sof: &'a StartOfFrameInfo,
    huffman_tables: Vec<(&'a HuffmanTree, &'a HuffmanTree)>,
    last_dc: [i16; 3],
}

impl<R: Read> McuReader<'_, R> {
    /// Read minimum coded units (MCU).
    fn read_mcus(&mut self) -> Result<Vec<Mcu>> {
        debug!("read MCUs");

        let mut mcus = vec![];
        for i in 0..self.sof.mcu_height() {
            for j in 0..self.sof.mcu_width() {
                mcus.push(self.read_mcu()?);
            }
        }
        Ok(mcus)
    }

    /// Read a minimum coded unit (MCU).
    fn read_mcu(&mut self) -> Result<Mcu> {
        let mut blocks = Vec::with_capacity(6);
        for (id, component) in self.sof.component_infos.iter().enumerate() {
            for _ in 0..component.vertical_sampling {
                for _ in 0..component.horizontal_sampling {
                    let block = self.read_block(id)?;
                    blocks.push(block);
                }
            }
        }
        Ok(Mcu { blocks })
    }

    /// Read a minimum coded unit (MCU).
    fn read_block(&mut self, id: usize) -> Result<Block> {
        let (dc, ac) = self.huffman_tables[id];
        let mut x = [0; 64];
        x[0] = self.read_dc(dc, id)?;
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
    fn read_dc(&mut self, map: &HuffmanTree, id: usize) -> Result<i16> {
        let dc = &mut self.last_dc[id];
        let len = self.reader.read_decode_haffman(map)?;
        *dc += self.reader.read_value(len)?;
        Ok(*dc)
    }
}

struct BitReader<'a, R: Read> {
    reader: &'a mut Decoder<R>,
    buf: u8,
    count: u8,
}

impl<'a, R: Read> BitReader<'a, R> {
    fn new(reader: &'a mut Decoder<R>) -> Self {
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
