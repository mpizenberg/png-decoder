use nom::bytes::complete::tag;
use nom::multi::many1;
use nom::IResult;
use std::convert::TryFrom;
use std::error::Error;

// inner modules
use crate::chunk::{self, Chunk, ChunkType};
use crate::chunk_data::{self, ChunkData, IHDRData};
use crate::color::ColorType;
use crate::filter::{self, Filter};

// TYPES #######################################################################

pub struct Png {
    pub width: usize,
    pub height: usize,
    pub color_type: ColorType,
    pub bytes_per_pixel: usize,
    pub data: Vec<u8>,
}

impl Png {
    pub fn get(&self, x: usize, y: usize) -> &[u8] {
        let line_width = self.bytes_per_pixel * self.width;
        let start = y * line_width + x * self.bytes_per_pixel;
        let end = start + self.bytes_per_pixel;
        &self.data.as_slice()[start..end]
    }
}

const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

// const EXTENDED_SIGNATURE: [u8; 12] = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];

// FUNCTIONS ###################################################################

pub fn decode_no_check(input: &[u8]) -> Result<Png, Box<Error>> {
    match parse_chunks(input) {
        Ok((_, chunks)) => {
            let ihdr_chunk = &chunks[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            let inflated_idats = chunk_data::inflate_idats(idats.as_slice())?;
            let scanlines = lines_slices(&inflated_idats, ihdr_data.scanline_width());
            let png_img = unfilter(&ihdr_data, scanlines);
            Ok(png_img)
        }
        Err(e) => Err(format!("{:?}", e).into()),
    }
}

pub fn decode_verbose(data: &[u8]) -> Result<(), Box<Error>> {
    match parse_chunks(data) {
        Ok((_, chunks)) => {
            let chunks_valid = chunk::validate_chunk_constraints(&chunks)?;
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            let inflated_idats = chunk_data::inflate_idats(idats.as_slice())?;
            let ihdr_chunk = &chunks_valid[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            let scanlines = lines_slices(&inflated_idats, ihdr_data.scanline_width());
            println!("Inflate image data size: {}", inflated_idats.len());
            display_filters(&scanlines);
            let img = unfilter(&ihdr_data, scanlines);
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

pub fn decode_no_check_verbose(input: &[u8]) -> Result<Png, Box<Error>> {
    let mut now = std::time::Instant::now();
    match parse_chunks(input) {
        Ok((_, chunks)) => {
            println!("parse_chunks: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let ihdr_chunk = &chunks[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            println!("parse_ihdr_data: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            println!("filter idats: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let inflated_idats = chunk_data::inflate_idats(idats.as_slice())?;
            println!("inflate idats: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let scanlines = lines_slices(&inflated_idats, ihdr_data.scanline_width());
            println!("get_scanlines: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let png_img = unfilter(&ihdr_data, scanlines);
            println!("unfilter: {} us", now.elapsed().as_micros());
            Ok(png_img)
        }
        Err(e) => Err(format!("{:?}", e).into()),
    }
}

pub fn decode_no_check_verbose_bis(input: &[u8]) -> Result<Png, Box<Error>> {
    let mut now = std::time::Instant::now();
    match parse_chunks(input) {
        Ok((_, chunks)) => {
            println!("parse_chunks: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let ihdr_chunk = &chunks[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            println!("parse_ihdr_data: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            println!("filter idats: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let mut inflated_idats = chunk_data::inflate_idats(idats.as_slice())?;
            println!("inflate idats: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let scanlines = lines_num(&inflated_idats, ihdr_data.scanline_width());
            println!("get_scanlines: {} us", now.elapsed().as_micros());
            now = std::time::Instant::now();
            let png_img = unfilter_bis(&ihdr_data, scanlines, &mut inflated_idats);
            println!("unfilter: {} us", now.elapsed().as_micros());
            Ok(png_img)
        }
        Err(e) => Err(format!("{:?}", e).into()),
    }
}

pub fn decode_no_check_bis(input: &[u8]) -> Result<Png, Box<Error>> {
    match parse_chunks(input) {
        Ok((_, chunks)) => {
            let ihdr_chunk = &chunks[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            let idats: Vec<_> = chunks
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            let mut inflated_idats = chunk_data::inflate_idats(idats.as_slice())?;
            let scanlines = lines_num(&inflated_idats, ihdr_data.scanline_width());
            let png_img = unfilter_bis(&ihdr_data, scanlines, &mut inflated_idats);
            Ok(png_img)
        }
        Err(e) => Err(format!("{:?}", e).into()),
    }
}

pub fn parse_chunks(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
    let (input, _) = tag(SIGNATURE)(input)?;
    many1(Chunk::parse)(input)
}

pub fn unfilter(ihdr: &IHDRData, scanlines: Vec<(Filter, &[u8])>) -> Png {
    let width = ihdr.width as usize;
    let height = ihdr.height as usize;
    let bytes_per_channel = std::cmp::max(1, ihdr.bit_depth as usize / 8);
    let bpp = bytes_per_channel
        * match ihdr.color_type {
            ColorType::Gray => 1,
            ColorType::GrayAlpha => 2,
            ColorType::RGB => 3,
            ColorType::RGBA => 4,
            ColorType::PLTE => unimplemented!(),
        };
    let data = filter::unfilter(width, height, bpp, scanlines);
    Png {
        width,
        height,
        color_type: ihdr.color_type,
        bytes_per_pixel: bpp,
        data,
    }
}

pub fn unfilter_bis(ihdr: &IHDRData, scanlines: Vec<(Filter, usize)>, inflated: &mut [u8]) -> Png {
    let width = ihdr.width as usize;
    let height = ihdr.height as usize;
    let bytes_per_channel = std::cmp::max(1, ihdr.bit_depth as usize / 8);
    let bpp = bytes_per_channel
        * match ihdr.color_type {
            ColorType::Gray => 1,
            ColorType::GrayAlpha => 2,
            ColorType::RGB => 3,
            ColorType::RGBA => 4,
            ColorType::PLTE => unimplemented!(),
        };
    let data = filter::unfilter_bis(width, height, bpp, scanlines, inflated);
    Png {
        width,
        height,
        color_type: ihdr.color_type,
        bytes_per_pixel: bpp,
        data,
    }
}

// Helpers #####################################################################

pub fn lines_slices(data: &[u8], scanline_width: usize) -> Vec<(Filter, &[u8])> {
    let nb_scanlines = data.len() / scanline_width;
    (0..nb_scanlines)
        .map(|i| i * scanline_width)
        .map(|start| {
            (
                Filter::try_from(data[start]).expect("Incorrect filter type"),
                &data[start + 1..start + scanline_width],
            )
        })
        .collect()
}

pub fn lines_num(data: &[u8], scanline_width: usize) -> Vec<(Filter, usize)> {
    let nb_scanlines = data.len() / scanline_width;
    (0..nb_scanlines)
        .map(|i| i * scanline_width)
        .map(|start| {
            (
                Filter::try_from(data[start]).expect("Incorrect filter type"),
                start + 1,
            )
        })
        .collect()
}

fn display_filters(scanlines: &[(Filter, &[u8])]) -> () {
    scanlines
        .iter()
        .enumerate()
        .for_each(|(i, (filter, _))| print!("{} {:?}, ", i, filter));
    println!("");
}
