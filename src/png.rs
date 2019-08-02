use nom::bytes::complete::tag;
use nom::multi::many1;
use nom::IResult;
use std::convert::TryFrom;
use std::error::Error;

// inner modules
use crate::chunk::{Chunk, ChunkType};
use crate::chunk_data::{self, IHDRData};
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
            let scanlines = get_scanlines(&ihdr_data, &inflated_idats);
            Ok(unfilter(&ihdr_data, scanlines))
        }
        Err(e) => Err(format!("{:?}", e).into()),
    }
}

pub fn parse_chunks(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
    let (input, _) = tag(SIGNATURE)(input)?;
    many1(Chunk::parse)(input)
}

pub fn get_scanlines<'a>(ihdr: &IHDRData, image_data: &'a [u8]) -> Vec<(Filter, &'a [u8])> {
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

pub fn unfilter(ihdr: &IHDRData, scanlines: Vec<(Filter, &[u8])>) -> Png {
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
    Png {
        width,
        height,
        color_type: ColorType::RGBA,
        bytes_per_pixel: bpp,
        data,
    }
}
