use std::io::{BufWriter, Result, Write};

use crate::decode::RGB;

pub struct PpmWriter<W: Write> {
    writer: BufWriter<W>,
}

impl<W: Write> PpmWriter<W> {
    pub fn new(writer: W, width: u32, height: u32) -> Result<Self> {
        let mut writer = BufWriter::new(writer);
        write!(writer, "P6\n{} {}\n255\n", width, height)?;
        Ok(PpmWriter { writer })
    }

    pub fn write(&mut self, pixel: RGB) -> Result<()> {
        self.writer.write_all(&[pixel.r, pixel.g, pixel.b])
    }
}
