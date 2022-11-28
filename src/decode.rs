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
    pub fn line(&self, h: usize) -> impl Iterator<Item = RGB> + '_ {
        let wb = self.width_blocks as usize;
        self.blocks[h / 8 * wb..(h / 8 + 1) * wb]
            .iter()
            .flat_map(move |b| b[h % 8 * 8..(h % 8 + 1) * 8].iter().cloned())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub struct McuDecoder {
    sof: StartOfFrameInfo,
    qts: Vec<QuantizationTable>,
}

impl McuDecoder {
    pub fn new(sof: StartOfFrameInfo, qts: Vec<QuantizationTable>) -> Self {
        McuDecoder { sof, qts }
    }

    pub fn width(&self) -> u16 {
        self.sof.width
    }

    pub fn height(&self) -> u16 {
        self.sof.height
    }

    pub fn mcu_width_num(&self) -> u16 {
        self.sof.mcu_width_num()
    }

    pub fn mcu_height(&self) -> u16 {
        self.sof.mcu_height()
    }

    pub fn decode(&self, mut mcu: Mcu) -> McuRGB {
        let mut i = 0;
        for component in &self.sof.component_infos {
            let qt = &self.qts[component.quant_table_id as usize].values;
            for _ in 0..component.horizontal_sampling * component.vertical_sampling {
                mcu.blocks[i] = mcu.blocks[i].dequantize(qt).zigzag().idct();
                i += 1;
            }
        }
        mcu.to_rgb(&self.sof)
    }
}

impl Mcu {
    pub fn to_rgb(&self, sof: &StartOfFrameInfo) -> McuRGB {
        let mut blocks =
            Vec::with_capacity((sof.max_horizontal_sampling * sof.max_vertical_sampling) as usize);

        let size = sof
            .component_infos
            .map(|c| c.horizontal_sampling * c.vertical_sampling);
        assert!(size[1] == 1 && size[2] == 1, "only support 4:4:4 or 4:1:1");
        let offset = [0, size[0] as usize, (size[0] + size[1]) as usize];
        for v in 0..sof.max_vertical_sampling {
            for h in 0..sof.max_horizontal_sampling {
                let y = self.blocks[(v * sof.max_horizontal_sampling + h) as usize];
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
                let mut rgb = [RGB::default(); 64];
                for i in 0..64 {
                    fn chomp(x: i32) -> u8 {
                        ((x >> 10).clamp(i8::MIN as _, i8::MAX as _) as i8 as u8) ^ 0x80
                    }
                    fn fixed(x: f32) -> i32 {
                        (x * 1024.0) as i32
                    }
                    let y = y.0[i] as i32;
                    let cb = cb.0[i] as i32;
                    let cr = cr.0[i] as i32;
                    let r = chomp(fixed(1.0) * y + fixed(1.402) * cr);
                    let g = chomp(fixed(1.0) * y - fixed(0.344) * cb - fixed(0.714) * cr);
                    let b = chomp(fixed(1.0) * y + fixed(1.772) * cb);
                    rgb[i] = RGB { r, g, b };
                }
                blocks.push(rgb)
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
        let mut block = *self;
        for i in 0..64 {
            block.0[i] *= qt[i];
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
        for i in 0..64 {
            x.0[i] = self.0[ZIGZAG[i]];
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
                m.map(|m| m.map(|f| (f * 1024.0) as i16))
            };
        }

        let mut res = Block::uninit();
        let idct = &*IDCT;
        for i in 0..8 {
            for j in 0..8 {
                // 20bit fixed point
                let mut v = 0;
                for x in 0..8 {
                    for y in 0..8 {
                        v += self.0[x * 8 + y] as i32 * idct[i][x] as i32 * idct[j][y] as i32;
                    }
                }
                res.0[i * 8 + j] = ((v / 4) >> 20) as i16;
            }
        }
        res
    }

    pub fn upsample_2x2(&self, oh: usize, ow: usize) -> Self {
        let mut x = Block::uninit();
        for i in 0..64 {
            x.0[i] = self.0[(oh * 8 + i / 8) / 2 * 8 + (ow * 8 + i % 8) / 2];
        }
        x
    }

    #[allow(invalid_value)]
    #[inline]
    fn uninit() -> Self {
        unsafe { std::mem::MaybeUninit::uninit().assume_init() }
    }
}
