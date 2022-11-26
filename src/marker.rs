use crate::Decoder;
use num_enum::TryFromPrimitive;
use std::io::{Error, ErrorKind, Read, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum Marker {
    /// Start Of Image
    SOI = 0xD8,
    /// Application specific 0
    APP0 = 0xE0,
    /// Application specific C
    APPC = 0xEC,
    /// Define Quantization Table
    DQT = 0xDB,
    /// Define Huffman Table
    DHT = 0xC4,
    /// Start Of Frame (baseline)
    SOF0 = 0xC0,
    /// Start Of Scan
    SOS = 0xDA,
    /// End Of Image
    EOI = 0xD9,
}

impl Marker {
    /// The prefix of a marker.
    const PREFIX: u8 = 0xFF;
}

impl<R: Read> Decoder<R> {
    /// Read a byte.
    fn read_byte(&mut self) -> Result<u8> {
        let mut buf = [0u8];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Read the next marker.
    pub fn next_marker(&mut self) -> Result<Marker> {
        loop {
            let byte = self.read_byte()?;
            if byte != Marker::PREFIX {
                continue;
            }
            let byte = self.read_byte()?;
            if byte == 0x00 {
                continue;
            }
            let marker = Marker::try_from(byte).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid marker: 0x{:02X}", byte),
                )
            })?;
            return Ok(marker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_marker() {
        use Marker::*;
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let mut decoder = Decoder::new(file);
        let mut markers = vec![];
        loop {
            let marker = decoder.next_marker().expect("failed to read marker");
            markers.push(marker);
            if marker == EOI {
                break;
            }
        }
        assert_eq!(
            markers,
            vec![SOI, APP0, APPC, DQT, DQT, SOF0, DHT, DHT, DHT, DHT, SOS, EOI]
        );
    }
}
