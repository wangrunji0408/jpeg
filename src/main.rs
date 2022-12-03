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
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let file = std::fs::File::open(args.file).expect("failed to open file");
    let out = std::fs::File::create(args.output).expect("failed to create file");
    let decoder = Decoder::new(file);
    let mut decoder = decoder.read().unwrap();
    let mut writer = PpmWriter::new(out, decoder.width() as _, decoder.height() as _).unwrap();
    let mut mcus = Vec::with_capacity(decoder.mcu_width_num() as usize);
    let mut height = decoder.height();
    while let Some(mcu) = decoder.next().unwrap() {
        mcus.push(mcu);
        if mcus.len() == decoder.mcu_width_num() as usize {
            for h in 0..decoder.mcu_height() {
                let mut width = decoder.width() as usize;
                for mcu in mcus.iter().flat_map(|mcu| mcu.line(h as usize)) {
                    let len = mcu.len().min(width);
                    writer.write_slice(&mcu[..len]).unwrap();
                    width -= len;
                }
                height -= 1;
                if height == 0 {
                    break;
                }
            }
            mcus.clear();
        }
    }
}
