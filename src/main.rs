use std::{env, error::Error, fs};

// inner modules
use png_decoder::chunk::{self, ChunkType};
use png_decoder::chunk_data::{self, ChunkData};
use png_decoder::filter::Filter;
use png_decoder::png;

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(error) = run(&args) {
        eprintln!("{:?}", error);
    }
}

fn run(args: &[String]) -> Result<(), Box<Error>> {
    let data = fs::read(&args[1])?;
    let _ = png::decode_no_check_verbose_bis(&data)?;
    // let _ = png::decode_no_check_verbose(&data)?;
    // let _ = decode_verbose(&data)?;
    println!("All done!");
    Ok(())
}

pub fn decode_verbose(data: &[u8]) -> Result<(), Box<Error>> {
    match png::parse_chunks(data) {
        Ok((_, chunks)) => {
            let chunks_valid = chunk::validate_chunk_constraints(&chunks)?;
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            let inflated_idats = chunk_data::inflate_idats(idats.as_slice())?;
            let ihdr_chunk = &chunks_valid[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            let scanlines = png::lines_slices(&inflated_idats, ihdr_data.scanline_width());
            println!("Inflate image data size: {}", inflated_idats.len());
            // println!("Scanlines:\n{:?}", scanlines);
            display_filters(&scanlines);
            let img = png::unfilter(&ihdr_data, scanlines);
            println!("{:?}", img.get(77, 21));
            println!("{:?}", img.get(78, 21));
            println!("{:?}", img.get(79, 21));
            println!("{:?}", img.get(80, 21));
            println!("{:?}", img.get(81, 21));
            println!("{:?}", img.get(82, 21));
            // println!("{:?}", &img.data.as_slice()[0..10]);
            chunks_valid.iter().for_each(|chunk| {
                match chunk_data::parse_chunk_data(chunk) {
                    Ok((_, ChunkData::Unknown(_))) => println!("{}", chunk),
                    Ok((_, chunk_data)) => println!("{:?}", chunk_data),
                    Err(e) => eprintln!("{:?}", e),
                };
            });
        }
        Err(e) => {
            eprintln!("{:?}", e);
        }
    }
    Ok(())
}

fn display_filters(scanlines: &[(Filter, &[u8])]) -> () {
    scanlines
        .iter()
        .enumerate()
        .for_each(|(i, (filter, _))| print!("{} {:?}, ", i, filter));
    println!("");
}
