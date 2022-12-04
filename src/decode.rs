use crate::{
    mcu::{Block, Mcu},
    quantization_table::QuantizationTable,
    start_of_frame_0::StartOfFrameInfo,
};

/// Minimum Coded Unit in RGB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McuRGB {
    blocks: Vec<[RGB; 64]>,
    width_blocks: u8,
    height_blocks: u8,
}

impl McuRGB {
    pub fn line(&self, h: usize) -> impl Iterator<Item = &[RGB]> + '_ {
        let wb = self.width_blocks as usize;
        self.blocks[h / 8 * wb..(h / 8 + 1) * wb]
            .iter()
            .map(move |b| &b[h % 8 * 8..(h % 8 + 1) * 8])
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Mcu {
    pub fn itrans(&mut self, sof: &StartOfFrameInfo, qts: &[QuantizationTable]) {
        let mut i = 0;
        for component in &sof.component_infos {
            let qt = &qts[component.quant_table_id as usize].values;
            for _ in 0..component.horizontal_sampling * component.vertical_sampling {
                self.blocks[i] = self.blocks[i].dequantize(qt).zigzag().idct();
                i += 1;
            }
        }
    }

    pub fn to_rgb(&self, sof: &StartOfFrameInfo) -> McuRGB {
        let mut blocks = Vec::<[RGB; 64]>::with_capacity(
            (sof.max_horizontal_sampling * sof.max_vertical_sampling) as usize,
        );
        #[allow(clippy::uninit_vec)]
        unsafe {
            blocks.set_len(blocks.capacity());
        }

        let size = sof
            .component_infos
            .map(|c| c.horizontal_sampling * c.vertical_sampling);
        assert!(size[1] == 1 && size[2] == 1, "only support 4:4:4 or 4:1:1");
        let offset = [0, size[0] as usize, (size[0] + size[1]) as usize];
        let mut i = 0;
        for v in 0..sof.max_vertical_sampling {
            for h in 0..sof.max_horizontal_sampling {
                let y = self.blocks[i];
                let cb = if size[1] == 1 && sof.max_vertical_sampling == 2 {
                    self.blocks[offset[1]].upsample_2x2(v as usize, h as usize)
                } else {
                    self.blocks[offset[1]]
                };
                let cr = if size[2] == 1 && sof.max_vertical_sampling == 2 {
                    self.blocks[offset[2]].upsample_2x2(v as usize, h as usize)
                } else {
                    self.blocks[offset[2]]
                };
                let rgb = &mut blocks[i];
                for i in 0..64 {
                    fn chomp(x: i32) -> u8 {
                        (((x >> 10) as i16).clamp(i8::MIN as _, i8::MAX as _) as i8 as u8) ^ 0x80
                    }
                    fn fixed(x: f32) -> i32 {
                        (x * 1024.0) as i32
                    }
                    let y = (y.0[i] as i32) << 10;
                    let cb = cb.0[i] as i32;
                    let cr = cr.0[i] as i32;
                    let r = chomp(y + fixed(1.402) * cr);
                    let g = chomp(y - fixed(0.344) * cb - fixed(0.714) * cr);
                    let b = chomp(y + fixed(1.772) * cb);
                    rgb[i] = RGB { r, g, b };
                }
                i += 1;
            }
        }
        McuRGB {
            blocks,
            width_blocks: sof.max_horizontal_sampling,
            height_blocks: sof.max_vertical_sampling,
        }
    }
}

impl Block {
    pub fn dequantize(&self, qt: &[i16; 64]) -> Self {
        let mut block = Block::uninit();
        for i in 0..64 {
            block.0[i] = self.0[i] * qt[i];
        }
        block
    }

    pub fn zigzag(&self) -> Self {
        #[rustfmt::skip]
        const ZIGZAG: [usize; 64] = [
             0,  1,  5,  6, 14, 15, 27, 28,
             2,  4,  7, 13, 16, 26, 29, 42,
             3,  8, 12, 17, 25, 30, 41, 43,
             9, 11, 18, 24, 31, 40, 44, 53,
            10, 19, 23, 32, 39, 45, 52, 54,
            20, 22, 33, 38, 46, 51, 55, 60,
            21, 34, 37, 47, 50, 56, 59, 61,
            35, 36, 48, 49, 57, 58, 62, 63,
        ];

        let mut x = Block::uninit();
        for i in 0..8 {
            for j in 0..8 {
                x.0[i * 8 + j] = self.0[ZIGZAG[i * 8 + j]];
            }
        }
        x
    }

    pub fn idct(&self) -> Self {
        lazy_static::lazy_static! {
            // 10bit fixed point
            static ref IDCT: [[i16; 8]; 8] = {
                use std::f32::consts::PI;
                let mut m = [[0.0; 8]; 8];
                for i in 0..8 {
                    for j in 0..8 {
                        m[i][j] = ((2 * i + 1) as f32 * j as f32 * PI / 16.0).cos();
                    }
                    m[i][0] *= 1.0 / 2_f32.sqrt();
                }
                m.map(|m| m.map(|f| (f * 1024.0).round() as i16))
            };
        }

        let idct = &*IDCT;
        // 1D IDCT
        #[allow(invalid_value)]
        #[allow(clippy::uninit_assumed_init)]
        let mut res1: [i32; 64] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        for i in 0..8 {
            for j in 0..8 {
                // 10bit fixed point
                let mut v = 0;
                for x in 0..8 {
                    v += self.0[i * 8 + x] as i32 * idct[j][x] as i32;
                }
                res1[j * 8 + i] = v;
            }
        }
        // 1D IDCT
        let mut res2 = Block::uninit();
        for j in 0..8 {
            for i in 0..8 {
                // 20bit fixed point
                let mut v = 0;
                for x in 0..8 {
                    v += res1[j * 8 + x] * idct[i][x] as i32;
                }
                res2.0[i * 8 + j] = ((v / 4) >> 20) as i16;
            }
        }
        res2
    }

    pub fn upsample_2x2(&self, oh: usize, ow: usize) -> Self {
        match (oh, ow) {
            (0, 0) => self.upsample_2x2_inline::<0, 0>(),
            (0, 1) => self.upsample_2x2_inline::<0, 1>(),
            (1, 0) => self.upsample_2x2_inline::<1, 0>(),
            (1, 1) => self.upsample_2x2_inline::<1, 1>(),
            _ => unreachable!(),
        }
    }

    fn upsample_2x2_inline<const I: usize, const J: usize>(&self) -> Self {
        let mut x = Block::uninit();
        for i in 0..8 {
            for j in 0..8 {
                x.0[i * 8 + j] = self.0[(I * 8 + i) / 2 * 8 + (J * 8 + j) / 2];
            }
        }
        x
    }

    #[allow(invalid_value)]
    #[allow(clippy::uninit_assumed_init)]
    #[inline]
    fn uninit() -> Self {
        unsafe { std::mem::MaybeUninit::uninit().assume_init() }
    }
}
