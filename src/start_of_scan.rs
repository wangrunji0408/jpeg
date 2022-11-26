use crate::{
    error,
    huffman::HuffmanTableClass::{self, *},
    start_of_frame_0::Component,
    Decoder,
};
use std::io::{Read, Result};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartOfScanInfo {
    pub table_mapping: [HuffmanTableId; 3], // [Y, Cb, Cr]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuffmanTableId {
    pub dc: HuffmanTableClass,
    pub ac: HuffmanTableClass,
}

impl<R: Read> Decoder<R> {
    /// Read the [`StartOfScanInfo`].
    pub fn read_start_of_scan(&mut self) -> Result<StartOfScanInfo> {
        let len = self.read_u16()?;
        debug!(len, "read section SOS");

        let mut table_mapping = [HuffmanTableId { dc: DC0, ac: AC0 }; 3];

        let component_number = self.read_byte()?;
        assert_eq!(component_number, 3);
        for _ in 0..component_number {
            let component_id = self.read_byte()?;
            Component::try_from(component_id)
                .map_err(|_| error(format!("invalid component id: {}", component_id)))?;
            let id = self.read_byte()?;
            table_mapping[component_id as usize - 1] = HuffmanTableId {
                dc: match id >> 4 {
                    0 => DC0,
                    1 => DC1,
                    dc => return Err(error(format!("invalid DC table: {dc}"))),
                },
                ac: match id & 0x0F {
                    0 => AC0,
                    1 => AC1,
                    ac => return Err(error(format!("invalid AC table: {ac}"))),
                },
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
                    HuffmanTableId { dc: DC0, ac: AC0 },
                    HuffmanTableId { dc: DC1, ac: AC1 },
                    HuffmanTableId { dc: DC1, ac: AC1 },
                ]
            }
        );
    }
}
