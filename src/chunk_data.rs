use inflate::core::inflate_flags::{
    TINFL_FLAG_HAS_MORE_INPUT, TINFL_FLAG_PARSE_ZLIB_HEADER,
    TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use miniz_oxide::inflate;
use nom::bytes::complete::{take, take_till};
use nom::combinator::{map, map_res, rest};
use nom::number::complete::{be_u16, be_u32, be_u8};
use nom::IResult;
use std::convert::TryFrom;
use std::io::Cursor;

// Internal imports
use crate::chunk::{Chunk, ChunkType};
use crate::color::ColorType;

// TYPES #######################################################################

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum ChunkData<'a> {
    // Critical chunks
    IHDR(IHDRData), // image header
    // PLTE, // palette
    // IDAT(IDATData), // image data
    IEND, // image trailer
    // Ancillary chunks
    // tRNS, // transparency
    // gAMA, // image gamma
    // cHRM, // primary chromaticities
    // sRGB, // standard RGB color space
    // iCCP, // embedded ICC profile
    tEXt(Text),           // textual data
    zTXt(CompressedText), // compressed textual data
    // iTXt, // international textual data
    bKGD(Background),             // background color
    pHYs(PhysicalPixelDimension), // physical pixel dimensions
    sBIT(SignificantBits),        // significant bits
    // sPLT, // suggested palette
    // hIST, // palette histogram
    tIME(LastModificationTime), // image last-modification time
    // Unknown
    Unknown(&'a [u8]),
}

#[derive(Debug)]
pub struct IHDRData {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: ColorType,
    pub compression_method: u8,
    pub filter_method: u8,
    pub interlace_method: u8,
}

#[derive(Debug)]
pub enum SignificantBits {
    Gray(u8),
    GrayAlpha([u8; 2]),
    RGB([u8; 3]),
    RGBA([u8; 4]),
}

#[derive(Debug)]
pub struct PhysicalPixelDimension {
    x: u32,
    y: u32,
    unit: DimensionUnit,
}

#[derive(Debug)]
pub enum DimensionUnit {
    Unknown,
    Meter,
}

#[derive(Debug)]
pub struct Text {
    keyword: String,
    text: String,
}

#[derive(Debug)]
pub struct CompressedText {
    keyword: String,
    method: u8,
    text: String,
}

// #[derive(Debug)]
// struct IDATData {
//     length: usize,
//     data: Vec<u8>,
// }

#[derive(Debug)]
pub enum Background {
    Palette(u8),
    Gray(u16),
    RGB([u16; 3]),
}

#[derive(Debug)]
pub struct LastModificationTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

// FUNCTIONS ###################################################################

impl IHDRData {
    pub fn scanline_width(&self) -> usize {
        let nb_chanels = match self.color_type {
            ColorType::Gray => 1,
            ColorType::GrayAlpha => 2,
            ColorType::RGB => 3,
            ColorType::RGBA => 4,
            ColorType::PLTE => panic!("Palette type not handled"),
        };
        let bytes_per_channel = std::cmp::max(1, self.bit_depth as u32 / 8);
        (1 + self.width * nb_chanels * bytes_per_channel) as usize
    }
}

pub fn parse_chunk_data<'a>(chunk: &'a Chunk<'a>) -> IResult<&'a [u8], ChunkData<'a>> {
    match chunk.chunk_type {
        // --- Critical chunks ---
        ChunkType::IHDR => map(parse_ihdr_data, ChunkData::IHDR)(&chunk.data),
        ChunkType::PLTE => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::IDAT => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::IEND => map(take(0u8), |_| ChunkData::IEND)(&chunk.data),
        // --- Ancillary chunks ---
        ChunkType::cHRM => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::gAMA => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::iCCP => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::sBIT => map(|d| parse_sbit_data(d, chunk.length), ChunkData::sBIT)(&chunk.data),
        ChunkType::sRGB => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::bKGD => map(|d| parse_bkgd_data(d, chunk.length), ChunkData::bKGD)(&chunk.data),
        ChunkType::hIST => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::tRNS => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::pHYs => map(parse_phys_data, ChunkData::pHYs)(&chunk.data),
        ChunkType::sPLT => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::tIME => map(parse_time_data, ChunkData::tIME)(&chunk.data),
        ChunkType::iTXt => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::tEXt => map(parse_text_data, ChunkData::tEXt)(&chunk.data),
        ChunkType::zTXt => map(parse_ztxt_data, ChunkData::zTXt)(&chunk.data),
        ChunkType::Unknown(_) => map(take(0u8), ChunkData::Unknown)(&chunk.data),
    }
}

pub fn parse_ihdr_data(input: &[u8]) -> IResult<&[u8], IHDRData> {
    let (input, width) = be_u32(input)?;
    let (input, height) = be_u32(input)?;
    let (input, bit_depth) = be_u8(input)?;
    let (input, color_type) = map_res(be_u8, ColorType::try_from)(input)?;
    let (input, compression_method) = be_u8(input)?;
    let (input, filter_method) = be_u8(input)?;
    let (input, interlace_method) = be_u8(input)?;
    Ok((
        input,
        IHDRData {
            width,
            height,
            bit_depth,
            color_type,
            compression_method,
            filter_method,
            interlace_method,
        },
    ))
}

fn parse_sbit_data(input: &[u8], length: u32) -> IResult<&[u8], SignificantBits> {
    match length {
        1 => map(be_u8, SignificantBits::Gray)(input),
        2 => map(take(2u8), |sb: &[u8]| {
            SignificantBits::GrayAlpha([sb[0], sb[1]])
        })(input),
        3 => map(take(3u8), |sb: &[u8]| {
            SignificantBits::RGB([sb[0], sb[1], sb[2]])
        })(input),
        4 => map(take(4u8), |sb: &[u8]| {
            SignificantBits::RGBA([sb[0], sb[1], sb[2], sb[3]])
        })(input),
        _ => map_res(take(0u8), |_| {
            Err("There must be 1 to 4 bytes in the sBIT data chunk")
        })(input),
    }
}

fn parse_bkgd_data(input: &[u8], length: u32) -> IResult<&[u8], Background> {
    match length {
        1 => map(be_u8, Background::Palette)(input),
        2 => map(be_u16, Background::Gray)(input),
        6 => {
            let (input, red) = be_u16(input)?;
            let (input, green) = be_u16(input)?;
            let (input, blue) = be_u16(input)?;
            Ok((input, Background::RGB([red, green, blue])))
        }
        _ => map_res(take(0u8), |_| {
            Err("There must be 1, 2 or 6 bytes in the bKGD data chunk")
        })(input),
    }
}

fn parse_phys_data(input: &[u8]) -> IResult<&[u8], PhysicalPixelDimension> {
    let (input, x) = be_u32(input)?;
    let (input, y) = be_u32(input)?;
    let (input, unit) = map_res(be_u8, |n| match n {
        0 => Ok(DimensionUnit::Unknown),
        1 => Ok(DimensionUnit::Meter),
        _ => Err("pHYs unit specifier can only be 0 or 1"),
    })(input)?;
    Ok((input, PhysicalPixelDimension { x, y, unit }))
}

fn parse_text_data(input: &[u8]) -> IResult<&[u8], Text> {
    let (input, keyword) = map(str_till_null, String::from)(input)?;
    let (input, _) = take(1_u8)(input)?;
    let (input, text) = map(str_till_null, String::from)(input)?;
    Ok((input, Text { keyword, text }))
}

fn parse_ztxt_data(input: &[u8]) -> IResult<&[u8], CompressedText> {
    let (input, keyword) = map(str_till_null, String::from)(input)?;
    let (input, _) = take(1_u8)(input)?;
    let (input, method) = be_u8(input)?;
    let (input, text) = map_res(
        map_res(rest, miniz_oxide::inflate::decompress_to_vec_zlib),
        String::from_utf8,
    )(input)?;
    Ok((
        input,
        CompressedText {
            keyword,
            method,
            text,
        },
    ))
}

fn str_till_null(input: &[u8]) -> IResult<&[u8], &str> {
    map_res(till_null, std::str::from_utf8)(input)
}

fn till_null(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_till(|c| c == 0)(input)
}

pub fn inflate_idats(idats: &[&Chunk]) -> Result<Vec<u8>, String> {
    let flags = TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let mut decomp = inflate::core::DecompressorOxide::new();
    decomp.init();
    let compressed_length: u32 = idats.iter().map(|c| c.length).sum();
    let mut ret: Vec<u8> = vec![0; 2 * compressed_length as usize];

    let nb_chunk = idats.len();
    let mut out_pos = 0;
    for (id, chunk) in idats.iter().enumerate() {
        let mut in_pos = 0;
        let input = chunk.data;
        let chunk_flags = if id == nb_chunk - 1 {
            flags
        } else {
            flags | TINFL_FLAG_HAS_MORE_INPUT
        };
        loop {
            let (status, in_consumed, out_consumed) = {
                // Wrap the whole output slice so we know we have enough of the
                // decompressed data for matches.
                let mut c = Cursor::new(ret.as_mut_slice());
                c.set_position(out_pos as u64);
                inflate::core::decompress(&mut decomp, &input[in_pos..], &mut c, chunk_flags)
            };
            in_pos += in_consumed;
            out_pos += out_consumed;

            match status {
                inflate::TINFLStatus::Done => {
                    ret.truncate(out_pos);
                    return Ok(ret);
                }

                inflate::TINFLStatus::HasMoreOutput => {
                    // We need more space so extend the buffer.
                    ret.extend(&vec![0; out_pos]);
                }

                inflate::TINFLStatus::NeedsMoreInput => {
                    // normal if we are not at the last chunk.
                    if id == nb_chunk - 1 {
                        return Err(format!("{:?}", status));
                    } else {
                        break;
                    }
                }

                _ => return Err(format!("{:?}", status)),
            }
        }
    }
    Ok(ret)
}

fn parse_time_data(input: &[u8]) -> IResult<&[u8], LastModificationTime> {
    let (input, year) = be_u16(input)?;
    let (input, month) = be_u8(input)?;
    let (input, day) = be_u8(input)?;
    let (input, hour) = be_u8(input)?;
    let (input, minute) = be_u8(input)?;
    let (input, second) = be_u8(input)?;
    Ok((
        input,
        LastModificationTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        },
    ))
}
