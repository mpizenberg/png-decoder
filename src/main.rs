use inflate::core::inflate_flags::{
    TINFL_FLAG_HAS_MORE_INPUT, TINFL_FLAG_PARSE_ZLIB_HEADER,
    TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use miniz_oxide::inflate;
use nom::bytes::complete::{tag, take, take_till};
use nom::combinator::{map, map_res, rest};
use nom::multi::many1;
use nom::number::complete::{be_u16, be_u32, be_u8};
use nom::IResult;
use std::convert::TryFrom;
use std::io::Cursor;
use std::{env, error::Error, fs};

// inner modules
use png_decoder::chunk::{self, Chunk, ChunkType};
use png_decoder::chunk_data::{self, ChunkData, IHDRData};
use png_decoder::color::ColorType;
use png_decoder::filter::{self, Filter};

fn main() {
    let args: Vec<String> = env::args().collect();
    if let Err(error) = run(&args) {
        eprintln!("{:?}", error);
    }
}

fn run(args: &[String]) -> Result<(), Box<Error>> {
    let data = fs::read(&args[1])?;
    match parse_png(&data) {
        Ok((_, png)) => {
            let png_valid = chunk::validate_chunk_constraints(&png)?;
            let idats: Vec<_> = png
                .iter()
                .filter(|c| c.chunk_type == ChunkType::IDAT)
                .collect();
            let pixel_data = chunk_data::parse_idats(idats.as_slice())?;
            let ihdr_chunk = &png_valid[0];
            let ihdr_data = chunk_data::parse_ihdr_data(ihdr_chunk.data).unwrap().1;
            let scanlines = get_scanlines(&ihdr_data, &pixel_data);
            println!("There are {} pixel values", pixel_data.len());
            // println!("Scanlines:\n{:?}", scanlines);
            display_filters(&scanlines);
            let img = unfilter(&ihdr_data, scanlines);
            println!("{:?}", img.get(77, 21));
            println!("{:?}", img.get(78, 21));
            println!("{:?}", img.get(79, 21));
            println!("{:?}", img.get(80, 21));
            println!("{:?}", img.get(81, 21));
            println!("{:?}", img.get(82, 21));
            // println!("{:?}", &img.data.as_slice()[0..10]);
            png_valid.iter().for_each(|chunk| {
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

const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

// const EXTENDED_SIGNATURE: [u8; 12] = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];

fn parse_png(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
    let (input, _) = tag(SIGNATURE)(input)?;
    many1(Chunk::parse)(input)
}

// 22 juillet ---------------------------------------------------------------------

// 23 juillet ----------- journée pastaciutta + pot = LOL -------------------------

fn get_scanlines<'a>(ihdr: &IHDRData, image_data: &'a [u8]) -> Vec<(Filter, &'a [u8])> {
    let nb_chanels = match ihdr.color_type {
        ColorType::Gray => 1,
        ColorType::GrayAlpha => 2,
        ColorType::RGB => 3,
        ColorType::RGBA => 4,
        ColorType::PLTE => panic!("Palette type not handled"),
    };
    let bytes_per_channel = std::cmp::max(1, ihdr.bit_depth as u32 / 8);
    let full_line_length = (1 + ihdr.width * nb_chanels * bytes_per_channel) as usize;
    assert_eq!(image_data.len(), ihdr.height as usize * full_line_length);
    let lines_starts = (0..ihdr.height as usize).map(|l| l * full_line_length);
    lines_starts
        .map(|start| {
            (
                Filter::try_from(image_data[start]).expect("Incorrect filter type"),
                &image_data[(start + 1)..(start + full_line_length)],
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

fn unfilter(ihdr: &IHDRData, scanlines: Vec<(Filter, &[u8])>) -> Img {
    let width = ihdr.width as usize;
    let height = ihdr.height as usize;
    let bytes_per_channel = std::cmp::max(1, ihdr.bit_depth as usize / 8);
    let bpp = bytes_per_channel
        * match &ihdr.color_type {
            ColorType::Gray => 1,
            ColorType::GrayAlpha => 2,
            ColorType::RGB => 3,
            ColorType::RGBA => 4,
            ColorType::PLTE => unimplemented!(),
        };
    println!("bytes_per_pixel: {}", bpp);
    // let mut data = Vec::with_capacity(bpp * width * height);
    let mut data = vec![0; bpp * width * height];
    let line_start = 0;
    scanlines
        .iter()
        .fold(line_start, |line_start, (filter, line)| match filter {
            Filter::None => filter::decode_none(line, line_start, &mut data),
            Filter::Sub => filter::decode_sub(bpp, line, line_start, &mut data),
            Filter::Up => filter::decode_up(line, line_start, &mut data),
            Filter::Average => filter::decode_average(bpp, line, line_start, &mut data),
            Filter::Paeth => filter::decode_paeth(bpp, line, line_start, &mut data),
        });
    assert_eq!(height, scanlines.len());
    assert_eq!(data.len(), bpp * width * height);
    Img {
        width,
        height,
        color_type: ColorType::RGBA,
        bytes_per_pixel: bpp,
        data,
    }
}

struct Img {
    width: usize,
    height: usize,
    color_type: ColorType,
    bytes_per_pixel: usize,
    data: Vec<u8>,
}

impl Img {
    fn get(&self, x: usize, y: usize) -> &[u8] {
        let line_width = self.bytes_per_pixel * self.width;
        let start = y * line_width + x * self.bytes_per_pixel;
        let end = start + self.bytes_per_pixel;
        &self.data.as_slice()[start..end]
    }
}

// 24 juillet ---------------------------------------------------------------------
