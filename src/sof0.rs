use super::Decoder;
use std::io::{Read, Result};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SofInfo {
    pub precision: u8,
    pub height: u16,
    pub width: u16,
    pub component_infos: [ComponentInfo; 3], // [Y, Cb, Cr]
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ComponentInfo {
    pub horizontal_sampling: u8,
    pub vertical_sampling: u8,
    pub quant_table_id: u8,
}

impl<R: Read> Decoder<R> {
    /// Read the Start Of Frame 0 (baseline) info.
    pub fn read_sof0(&mut self) -> Result<SofInfo> {
        let len = self.read_u16()?;
        debug!(len, "read section SOF0");

        let precision = self.read_byte()?;
        let height = self.read_u16()?;
        let width = self.read_u16()?;
        let number_of_component = self.read_byte()?;

        let mut component_infos = [ComponentInfo::default(); 3];
        for _ in 0..number_of_component {
            // Y => 1, Cb => 2, Cr => 3
            let component_id = self.read_byte()?;
            let sampling = self.read_byte()?;
            let quant_table_id = self.read_byte()?;
            component_infos[component_id as usize - 1] = ComponentInfo {
                horizontal_sampling: sampling >> 4,
                vertical_sampling: sampling & 0x0f,
                quant_table_id,
            };
        }

        Ok(SofInfo {
            precision,
            height,
            width,
            component_infos,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::marker::Marker;

    use super::*;

    #[test]
    fn test_sof0() {
        tracing_subscriber::fmt::init();
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let mut decoder = Decoder::new(file);
        while decoder.next_marker().expect("failed to read marker") != Marker::SOF0 {}
        let sof0 = decoder.read_sof0().expect("failed to read SOF0");
        assert_eq!(
            sof0,
            SofInfo {
                precision: 8,
                height: 1080,
                width: 1920,
                component_infos: [
                    ComponentInfo {
                        horizontal_sampling: 2,
                        vertical_sampling: 2,
                        quant_table_id: 0,
                    },
                    ComponentInfo {
                        horizontal_sampling: 1,
                        vertical_sampling: 1,
                        quant_table_id: 1,
                    },
                    ComponentInfo {
                        horizontal_sampling: 1,
                        vertical_sampling: 1,
                        quant_table_id: 1,
                    },
                ],
            }
        );
    }
}
