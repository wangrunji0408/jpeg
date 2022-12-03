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

    pub fn write_slice(&mut self, pixel: &[RGB]) -> Result<()> {
        let buf =
            unsafe { std::slice::from_raw_parts(pixel.as_ptr() as *const u8, pixel.len() * 3) };
        self.writer.write_all(buf)
    }
}
