use std::io::{BufReader, Read, Result};

mod huffman;
mod marker;
mod quantization_table;

pub struct Decoder<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> Decoder<R> {
    pub fn new(reader: R) -> Self {
        Decoder {
            reader: BufReader::new(reader),
        }
    }

    /// Read a byte.
    fn read_byte(&mut self) -> Result<u8> {
        let mut buf = [0u8];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Read a u16.
    fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.reader.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }
}
