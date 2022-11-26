use super::Decoder;
use crate::start_of_frame_0::Component;
use std::io::{Read, Result};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartOfScanInfo {
    pub table_mapping: [HuffmanTableId; 3], // [Y, Cb, Cr]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuffmanTableId {
    pub dc: u8,
    pub ac: u8,
}

impl<R: Read> Decoder<R> {
    /// Read the [`StartOfScanInfo`].
    pub fn read_start_of_scan(&mut self) -> Result<StartOfScanInfo> {
        let len = self.read_u16()?;
        debug!(len, "read section SOS");

        let mut table_mapping = [HuffmanTableId { dc: 0, ac: 0 }; 3];

        let component_number = self.read_byte()?;
        assert_eq!(component_number, 3);
        for _ in 0..component_number {
            let component = self.read_byte()?;
            Component::try_from(component).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid component: {}", component),
                )
            })?;
            let id = self.read_byte()?;
            table_mapping[component as usize - 1] = HuffmanTableId {
                dc: id >> 4,
                ac: id & 0x0f,
            };
        }
        // skip 3 bytes
        assert_eq!(self.read_byte()?, 0x00);
        assert_eq!(self.read_byte()?, 0x3F);
        assert_eq!(self.read_byte()?, 0x00);

        Ok(StartOfScanInfo { table_mapping })
    }
}

#[cfg(test)]
mod tests {
    use crate::marker::Marker;

    use super::*;

    #[test]
    fn test_start_of_scan() {
        // tracing_subscriber::fmt::init();
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let mut decoder = Decoder::new(file);
        while decoder.next_marker().expect("failed to read marker") != Marker::SOS {}
        let sos = decoder.read_start_of_scan().expect("failed to read SOS");
        assert_eq!(
            sos,
            StartOfScanInfo {
                table_mapping: [
                    HuffmanTableId { dc: 0, ac: 0 },
                    HuffmanTableId { dc: 1, ac: 1 },
                    HuffmanTableId { dc: 1, ac: 1 },
                ]
            }
        );
    }
}
