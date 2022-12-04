use super::{error, Decoder};
use std::io::{Read, Result};
use tracing::debug;

/// JPEG markers
///
/// <https://dev.exiv2.org/projects/exiv2/wiki/The_Metadata_in_JPEG_files#2-The-metadata-structure-in-JPEG>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum Marker {
    /// Start Of Image
    SOI,
    /// Start Of Frame (Baseline DCT)
    SOF0,
    /// Start Of Frame (Progressive DCT)
    SOF2,
    /// Define Huffman Table
    DHT,
    /// Define Quantization Table
    DQT,
    /// Define Restart Interval
    DRI,
    /// Start Of Scan
    SOS,
    /// Restart
    RST(u8),
    /// Application specific
    APP(u8),
    /// Comment
    COM,
    /// End Of Image
    EOI,
}

impl Marker {
    /// The prefix of a marker.
    const PREFIX: u8 = 0xFF;
}

impl TryFrom<u8> for Marker {
    type Error = ();

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0xC0 => Ok(Marker::SOF0),
            0xC2 => Ok(Marker::SOF2),
            0xC4 => Ok(Marker::DHT),
            0xD0..=0xD7 => Ok(Marker::RST(value - 0xD0)),
            0xD8 => Ok(Marker::SOI),
            0xD9 => Ok(Marker::EOI),
            0xDA => Ok(Marker::SOS),
            0xDB => Ok(Marker::DQT),
            0xDD => Ok(Marker::DRI),
            0xE0..=0xEF => Ok(Marker::APP(value - 0xE0)),
            0xFE => Ok(Marker::COM),
            _ => Err(()),
        }
    }
}

impl<R: Read> Decoder<R> {
    /// Read the next marker.
    pub fn next_marker(&mut self) -> Result<Marker> {
        let mut count = 0;
        loop {
            let byte = self.read_byte()?;
            count += 1;
            if byte != Marker::PREFIX {
                continue;
            }
            let byte = self.read_byte()?;
            count += 1;
            if byte == 0x00 {
                continue;
            }
            let marker = Marker::try_from(byte)
                .map_err(|_| error(format!("Invalid marker: 0x{:02X}", byte)))?;
            debug!(?marker, skip = count - 2, "read marker");
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
        #[rustfmt::skip]
        assert_eq!(
            markers,
            vec![SOI, APP(0), APP(0xC), DQT, DQT, SOF0, DHT, DHT, DHT, DHT, SOS, EOI]
        );
    }
}
