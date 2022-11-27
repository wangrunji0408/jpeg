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
    fn to_rgb(&self, sof: &StartOfFrameInfo) -> McuRGB {
        let mut blocks =
            Vec::with_capacity((sof.max_horizontal_sampling * sof.max_vertical_sampling) as usize);

        fn chomp(x: f32) -> u8 {
            if x >= 255.0 {
                return 255;
            } else if x <= 0.0 {
                return 0;
            } else {
                return x.round() as u8;
            }
        }
        let size = sof
            .component_infos
            .map(|c| c.horizontal_sampling * c.vertical_sampling);
        let offset = [0, size[0] as usize, (size[0] + size[1]) as usize];
        for v in 0..sof.max_vertical_sampling {
            for h in 0..sof.max_horizontal_sampling {
                let y = self.blocks[(v * sof.max_horizontal_sampling + h) as usize].to_f32();
                let cb = if size[1] == 1 && sof.max_vertical_sampling == 2 {
                    self.blocks[offset[1]].upsample_2x2(v as usize, h as usize)
                } else {
                    todo!("select Cb")
                }
                .to_f32();
                let cr = if size[2] == 1 && sof.max_vertical_sampling == 2 {
                    self.blocks[offset[2]].upsample_2x2(v as usize, h as usize)
                } else {
                    todo!("select Cr")
                }
                .to_f32();
                let mut rgb = [RGB::default(); 64];
                for i in 0..64 {
                    let r = chomp(y[i] + 1.402 * cr[i] + 128.0);
                    let g = chomp(y[i] - 0.34414 * cb[i] - 0.71414 * cr[i] + 128.0);
                    let b = chomp(y[i] + 1.772 * cb[i] + 128.0);
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
    fn dequantize(&self, qt: &[i16; 64]) -> Self {
        let mut block = *self;
        for i in 0..64 {
            block.0[i] *= qt[i];
        }
        block
    }

    fn zigzag(&self) -> Self {
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

        let mut x = [0; 64];
        for i in 0..64 {
            x[i] = self.0[ZIGZAG[i]];
        }
        Block(x)
    }

    fn idct(&self) -> Self {
        use std::f32::consts::PI;

        fn cc(i: usize, j: usize) -> f32 {
            match (i, j) {
                (0, 0) => 0.5,
                (0, _) | (_, 0) => 0.5_f32.sqrt(),
                _ => 1.0,
            }
        }

        let mut tmp: [i16; 64] = [0; 64];
        for i in 0..8 {
            for j in 0..8 {
                let mut v = 0.0;
                for x in 0..8 {
                    for y in 0..8 {
                        let i_cos = ((2 * i + 1) as f32 * PI / 16.0 * x as f32).cos();
                        let j_cos = ((2 * j + 1) as f32 * PI / 16.0 * y as f32).cos();
                        v += cc(x, y) * self.0[x * 8 + y] as f32 * i_cos * j_cos;
                    }
                }
                tmp[i * 8 + j] = (v / 4.0) as i16;
            }
        }
        Block(tmp)
    }

    fn upsample_2x2(&self, oh: usize, ow: usize) -> Self {
        let mut x = [0; 64];
        for i in 0..64 {
            x[i] = self.0[(oh * 8 + i / 8) / 2 * 8 + (ow * 8 + i % 8) / 2];
        }
        Block(x)
    }

    fn to_f32(&self) -> [f32; 64] {
        self.0.map(f32::from)
    }
}
