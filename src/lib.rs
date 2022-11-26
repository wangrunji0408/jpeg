use std::io::{BufReader, Read, Result};

mod huffman;
mod marker;
mod mcu;
mod quantization_table;
mod start_of_frame_0;
mod start_of_scan;

use self::marker::Marker;

pub struct Decoder<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> Decoder<R> {
    pub fn new(reader: R) -> Self {
        Decoder {
            reader: BufReader::new(reader),
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let mut quantization_tables = vec![];
        let mut huffman_tables = vec![];
        let mut sof = None;
        loop {
            match self.next_marker()? {
                Marker::SOI => {}
                Marker::APP0 => {}
                Marker::APPC => {}
                Marker::DQT => quantization_tables.extend(self.read_quantization_table()?),
                Marker::DHT => huffman_tables.extend(self.read_huffman_table()?),
                Marker::SOF0 => sof = Some(self.read_start_of_frame_0()?),
                Marker::SOS => {
                    let sos = self.read_start_of_scan()?;
                    let sof = sof.as_ref().expect("SOF not found");
                    self.read_mcus(sof, &sos, &huffman_tables)?;
                }
                Marker::EOI => return Ok(()),
            }
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

fn error(msg: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read() {
        tracing_subscriber::fmt::init();
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let mut decoder = Decoder::new(file);
        decoder.read().unwrap();
    }
}
