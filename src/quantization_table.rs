use super::Decoder;
use std::io::{Error, ErrorKind, Read, Result};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizationTable {
    pub id: u8,
    pub values: [u16; 64],
}

impl<R: Read> Decoder<R> {
    /// Read the [`QuantizationTable`].
    pub fn read_quantization_table(&mut self) -> Result<Vec<QuantizationTable>> {
        let mut len = self.read_u16()?;
        debug!(len, "read section DQT");

        len -= 2;
        let mut tables = vec![];
        while len != 0 {
            let byte = self.read_byte()?;
            let precision = byte >> 4;
            let id = byte & 0x0F;
            debug!(id, precision, "read quantization table");
            match precision {
                0 => {
                    let mut values = [0u16; 64];
                    for i in 0..64 {
                        values[i] = self.read_byte()? as u16;
                    }
                    tables.push(QuantizationTable { id, values });
                    len -= 1 + 64;
                }
                1 => {
                    let mut values = [0u16; 64];
                    for i in 0..64 {
                        values[i] = self.read_u16()?;
                    }
                    tables.push(QuantizationTable { id, values });
                    len -= 1 + 128;
                }
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Invalid precision: {}", precision),
                    ))
                }
            }
        }
        Ok(tables)
    }
}

#[cfg(test)]
mod tests {
    use crate::marker::Marker;

    use super::*;

    #[test]
    fn test_read_quantization_table() {
        // tracing_subscriber::fmt::init();
        let file = std::fs::File::open("data/autumn.jpg").expect("failed to read file");
        let mut decoder = Decoder::new(file);
        while decoder.next_marker().expect("failed to read marker") != Marker::DQT {}
        let dqts = decoder.read_quantization_table().expect("failed to read DQT");
        assert_eq!(
            dqts,
            vec![QuantizationTable {
                id: 0,
                #[rustfmt::skip]
                values: [
                     3,  2,  2,  2,  2,  2,  3,  2, 
                     2,  2,  3,  3,  3,  3,  4,  6, 
                     4,  4,  4,  4,  4,  8,  6,  6, 
                     5,  6,  9,  8, 10, 10,  9,  8, 
                     9,  9, 10, 12, 15, 12, 10, 11,
                    14, 11,  9,  9, 13, 17, 13, 14,
                    15, 16, 16, 17, 16, 10, 12, 18, 
                    19, 18, 16, 19, 15, 16, 16, 16,
                ]
            }]
        );
    }
}
