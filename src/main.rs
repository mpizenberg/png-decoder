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
    match png::parse_chunks(&data) {
        Ok((_, chunks)) => {
            let chunks_valid = chunk::validate_chunk_constraints(&chunks)?;
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            let pixel_data = chunk_data::parse_idats(idats.as_slice())?;
            let ihdr_chunk = &chunks_valid[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            let scanlines = png::get_scanlines(&ihdr_data, &pixel_data);
            println!("There are {} pixel values", pixel_data.len());
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
    println!("All done!");
    Ok(())
}

fn display_filters(scanlines: &[(Filter, &[u8])]) -> () {
    scanlines
        .iter()
        .enumerate()
        .for_each(|(i, (filter, _))| print!("{} {:?}, ", i, filter));
    println!("");
}
