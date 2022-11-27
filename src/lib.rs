use std::io::{BufReader, Read, Result};

mod decode;
mod huffman;
mod marker;
pub mod mcu;
pub mod ppm;
mod quantization_table;
pub mod start_of_frame_0;
mod start_of_scan;

use tracing::debug;

use self::decode::McuDecoder;
use self::marker::Marker;
use self::mcu::McuReader;

pub struct Decoder<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> Decoder<R> {
    pub fn new(reader: R) -> Self {
        Decoder {
            reader: BufReader::new(reader),
        }
    }

    pub fn read(mut self) -> Result<(McuReader<R>, McuDecoder)> {
        let mut quantization_tables = vec![];
        let mut huffman_tables = vec![];
        let mut sof = None;
        let mut restart_interval = None;
        loop {
            match self.next_marker()? {
                Marker::EOI => return Err(error("unexpected EOI")),
                Marker::DQT => quantization_tables.extend(self.read_quantization_table()?),
                Marker::DHT => huffman_tables.extend(self.read_huffman_table()?),
                Marker::SOF0 => sof = Some(self.read_start_of_frame_0()?),
                Marker::DRI => restart_interval = Some(self.read_restart_interval()?),
                Marker::SOS => break,
                _ => {}
            }
        }
        let sos = self.read_start_of_scan()?;
        let sof = sof.take().expect("SOF not found");
        let reader = McuReader::new(self.reader, sof.clone(), sos, &huffman_tables)?;
        let decoder = McuDecoder::new(sof, quantization_tables);
        Ok((reader, decoder))
    }

    fn read_restart_interval(&mut self) -> Result<u16> {
        let len = self.read_u16()?;
        debug!(len, "read section DRI");
        if len != 4 {
            return Err(error(format!("invalid DRI length: {len}")));
        }
        let interval = self.read_u16()?;
        debug!(interval, "restart interval");
        Ok(interval)
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
