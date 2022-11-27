use clap::Parser;
use jpeg_labs::{ppm::PpmWriter, Decoder};

/// JPEG to PPM.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap()]
    file: String,

    #[clap(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();

    let file = std::fs::File::open(args.file).expect("failed to open file");
    let out = std::fs::File::create(args.output).expect("failed to create file");
    let decoder = Decoder::new(file);
    let (mut reader, decoder) = decoder.read().unwrap();
    let mut writer = PpmWriter::new(out, decoder.width() as _, decoder.height() as _).unwrap();
    let mut mcus = Vec::with_capacity(decoder.mcu_width_num() as usize);
    while let Some(mcu) = reader.next().unwrap() {
        let rgb = decoder.decode(mcu);
        mcus.push(rgb);
        if mcus.len() == decoder.mcu_width_num() as usize {
            for h in 0..decoder.mcu_height() {
                for mcu in &mcus {
                    for rgb in mcu.line(h as usize) {
                        writer.write(rgb).unwrap();
                    }
                }
            }
            mcus.clear();
        }
    }
}
