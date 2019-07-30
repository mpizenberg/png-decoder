use inflate::core::inflate_flags::{
    TINFL_FLAG_PARSE_ZLIB_HEADER, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use lazy_static::lazy_static;
use miniz_oxide::inflate;
use nom::bytes::complete::{tag, take, take_till};
use nom::combinator::{map, map_res, rest};
use nom::multi::many1;
use nom::number::complete::{be_u16, be_u32, be_u8};
use nom::IResult;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::io::Cursor;
use std::{env, error::Error, fs};

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
            let png_valid = validate_chunk_constraints(&png)?;
            png_valid.iter().for_each(|chunk| {
                match parse_chunk_data(chunk) {
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

struct Chunk<'a> {
    length: u32,
    chunk_type: ChunkType,
    data: &'a [u8],
    crc: [u8; 4],
}

impl std::fmt::Display for Chunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{length: {}, type: {:?}, crc: {:?}}}",
            self.length, self.chunk_type, self.crc
        )
    }
}

impl std::fmt::Debug for Chunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

fn parse_chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    let (input, length) = be_u32(input)?;
    let (input, t) = take(4usize)(input)?;
    let type_ = [t[0] as char, t[1] as char, t[2] as char, t[3] as char];
    let chunk_type = ChunkType::from(type_);
    let (input, data) = take(length)(input)?;
    let (input, crc_) = take(4usize)(input)?;
    let crc = [crc_[0], crc_[1], crc_[2], crc_[3]];
    Ok((
        input,
        Chunk {
            length,
            chunk_type,
            data,
            crc,
        },
    ))
}

fn parse_png(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
    let (input, _) = tag(SIGNATURE)(input)?;
    many1(parse_chunk)(input)
}

// 22 juillet ---------------------------------------------------------------------

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum ChunkType {
    // Critical chunks
    IHDR, // image header
    PLTE, // palette
    IDAT, // image data
    IEND, // image trailer
    // Ancillary chunks
    tRNS, // transparency
    gAMA, // image gamma
    cHRM, // primary chromaticities
    sRGB, // standard RGB color space
    iCCP, // embedded ICC profile
    tEXt, // textual data
    zTXt, // compressed textual data
    iTXt, // international textual data
    bKGD, // background color
    pHYs, // physical pixel dimensions
    sBIT, // significant bits
    sPLT, // suggested palette
    hIST, // palette histogram
    tIME, // image last-modification time
    // Unknown
    Unknown([char; 4]),
}

impl From<[char; 4]> for ChunkType {
    fn from(name: [char; 4]) -> Self {
        match name {
            ['I', 'H', 'D', 'R'] => ChunkType::IHDR,
            ['P', 'L', 'T', 'E'] => ChunkType::PLTE,
            ['I', 'D', 'A', 'T'] => ChunkType::IDAT,
            ['I', 'E', 'N', 'D'] => ChunkType::IEND,
            ['t', 'R', 'N', 'S'] => ChunkType::tRNS,
            ['g', 'A', 'M', 'A'] => ChunkType::gAMA,
            ['c', 'H', 'R', 'M'] => ChunkType::cHRM,
            ['s', 'R', 'G', 'B'] => ChunkType::sRGB,
            ['i', 'C', 'C', 'P'] => ChunkType::iCCP,
            ['t', 'E', 'X', 't'] => ChunkType::tEXt,
            ['z', 'T', 'X', 't'] => ChunkType::zTXt,
            ['i', 'T', 'X', 't'] => ChunkType::iTXt,
            ['b', 'K', 'G', 'D'] => ChunkType::bKGD,
            ['p', 'H', 'Y', 's'] => ChunkType::pHYs,
            ['s', 'B', 'I', 'T'] => ChunkType::sBIT,
            ['s', 'P', 'L', 'T'] => ChunkType::sPLT,
            ['h', 'I', 'S', 'T'] => ChunkType::hIST,
            ['t', 'I', 'M', 'E'] => ChunkType::tIME,
            _ => ChunkType::Unknown(name),
        }
    }
}

fn validate_chunk_constraints<'a, 'c>(chunks: &'a [Chunk<'c>]) -> Result<&'a [Chunk<'c>], String> {
    // let inner_chunks = ihdr_first(chunks).and_then(iend_last)?;
    // let authorized_set = START_CHUNKS.clone();
    let mut authorized_set = HashSet::new();
    authorized_set.insert(ChunkType::IHDR);
    let _ = chunks
        .iter()
        .try_fold((HashSet::new(), authorized_set), validate_chunk)?;
    Ok(chunks)
}

type ValidationSets = (HashSet<ChunkType>, HashSet<ChunkType>);

lazy_static! {
    // Chunks that can only happen before PLTE
    static ref BEFORE_PLTE_CHUNKS: HashSet<ChunkType> = [
        ChunkType::PLTE, // palette
        ChunkType::gAMA, // image gamma
        ChunkType::cHRM, // primary chromaticities
        ChunkType::sRGB, // standard RGB color space
        ChunkType::iCCP, // embedded ICC profile
        ChunkType::sBIT, // significant bits
    ]
    .iter()
    .cloned()
    .collect();

    // Chunks that can only happen before IDAT
    static ref BEFORE_IDAT_CHUNKS: HashSet<ChunkType> = [
        ChunkType::PLTE, // palette
        ChunkType::tRNS, // transparency
        ChunkType::gAMA, // image gamma
        ChunkType::cHRM, // primary chromaticities
        ChunkType::sRGB, // standard RGB color space
        ChunkType::iCCP, // embedded ICC profile
        ChunkType::bKGD, // background color
        ChunkType::pHYs, // physical pixel dimensions
        ChunkType::sBIT, // significant bits
        ChunkType::sPLT, // suggested palette
        ChunkType::hIST, // palette histogram
    ]
    .iter()
    .cloned()
    .collect();

    // Authorized chunks after IHDR,
    // basically everything except IHDR and IEND
    static ref START_CHUNKS: HashSet<ChunkType> = [
        ChunkType::PLTE, // palette
        ChunkType::IDAT, // image data
        ChunkType::tRNS, // transparency
        ChunkType::gAMA, // image gamma
        ChunkType::cHRM, // primary chromaticities
        ChunkType::sRGB, // standard RGB color space
        ChunkType::iCCP, // embedded ICC profile
        ChunkType::tEXt, // textual data
        ChunkType::zTXt, // compressed textual data
        ChunkType::iTXt, // international textual data
        ChunkType::bKGD, // background color
        ChunkType::pHYs, // physical pixel dimensions
        ChunkType::sBIT, // significant bits
        ChunkType::sPLT, // suggested palette
        ChunkType::hIST, // palette histogram
        ChunkType::tIME, // image last-modification time
    ]
    .iter()
    .cloned()
    .collect();
}

// Spec: http://www.libpng.org/pub/png/spec/1.2/png-1.2-pdg.html
// Critical chunks (must appear in this order, except PLTE
//                  is optional):
//
//         Name  Multiple  Ordering constraints
//                 OK?
//
//         IHDR    No      Must be first
//         PLTE    No      Before IDAT
//         IDAT    Yes     Multiple IDATs must be consecutive
//         IEND    No      Must be last
//
// Ancillary chunks (need not appear in this order):
//
//         Name  Multiple  Ordering constraints
//                 OK?
//
//         cHRM    No      Before PLTE and IDAT
//         gAMA    No      Before PLTE and IDAT
//         iCCP    No      Before PLTE and IDAT
//         sBIT    No      Before PLTE and IDAT
//         sRGB    No      Before PLTE and IDAT
//         bKGD    No      After PLTE; before IDAT
//         hIST    No      After PLTE; before IDAT
//         tRNS    No      After PLTE; before IDAT
//         pHYs    No      Before IDAT
//         sPLT    Yes     Before IDAT
//         tIME    No      None
//         iTXt    Yes     None
//         tEXt    Yes     None
//         zTXt    Yes     None
fn validate_chunk(acc: ValidationSets, chunk: &Chunk) -> Result<ValidationSets, String> {
    let (mut present, mut authorized) = acc;
    if authorized.contains(&chunk.chunk_type) {
        match chunk.chunk_type {
            // --- Critical chunks ---
            ChunkType::IHDR => {
                authorized = START_CHUNKS.clone();
                present.insert(ChunkType::IHDR);
                Ok((present, authorized))
            }
            ChunkType::PLTE => {
                authorized = authorized
                    .difference(&BEFORE_PLTE_CHUNKS)
                    .cloned()
                    .collect();
                present.insert(ChunkType::PLTE);
                Ok((present, authorized))
            }
            ChunkType::IDAT => {
                if !present.contains(&ChunkType::IDAT) {
                    // When encountering the first IDAT chunk
                    authorized = authorized
                        .difference(&BEFORE_IDAT_CHUNKS)
                        .cloned()
                        .collect();
                    authorized.insert(ChunkType::IEND);
                    present.insert(ChunkType::IDAT);
                }
                Ok((present, authorized))
            }
            ChunkType::IEND => {
                authorized = HashSet::new();
                present.insert(ChunkType::IEND);
                Ok((present, authorized))
            }
            // --- Ancillary chunks ---
            // Before PLTE and IDAT
            ChunkType::cHRM => {
                authorized.remove(&ChunkType::cHRM);
                present.insert(ChunkType::cHRM);
                Ok((present, authorized))
            }
            ChunkType::gAMA => {
                authorized.remove(&ChunkType::gAMA);
                present.insert(ChunkType::gAMA);
                Ok((present, authorized))
            }
            ChunkType::iCCP => {
                authorized.remove(&ChunkType::iCCP);
                present.insert(ChunkType::iCCP);
                Ok((present, authorized))
            }
            ChunkType::sBIT => {
                authorized.remove(&ChunkType::sBIT);
                present.insert(ChunkType::sBIT);
                Ok((present, authorized))
            }
            ChunkType::sRGB => {
                authorized.remove(&ChunkType::sRGB);
                present.insert(ChunkType::sRGB);
                Ok((present, authorized))
            }
            // After PLTE; before IDAT
            ChunkType::bKGD => {
                authorized.remove(&ChunkType::PLTE);
                authorized.remove(&ChunkType::bKGD);
                present.insert(ChunkType::bKGD);
                Ok((present, authorized))
            }
            ChunkType::hIST => {
                authorized.remove(&ChunkType::PLTE);
                authorized.remove(&ChunkType::hIST);
                present.insert(ChunkType::hIST);
                Ok((present, authorized))
            }
            ChunkType::tRNS => {
                authorized.remove(&ChunkType::PLTE);
                authorized.remove(&ChunkType::tRNS);
                present.insert(ChunkType::tRNS);
                Ok((present, authorized))
            }
            // before IDAT
            ChunkType::pHYs => {
                authorized.remove(&ChunkType::pHYs);
                present.insert(ChunkType::pHYs);
                Ok((present, authorized))
            }
            // before IDAT, multiple possible
            ChunkType::sPLT => {
                present.insert(ChunkType::sPLT);
                Ok((present, authorized))
            }
            // anywhere
            ChunkType::tIME => {
                authorized.remove(&ChunkType::tIME);
                present.insert(ChunkType::tIME);
                if present.contains(&ChunkType::IDAT) {
                    // IDAT chunks must be consecutive
                    authorized.remove(&ChunkType::IDAT);
                }
                Ok((present, authorized))
            }
            // anywhere, multiple possible
            ChunkType::iTXt => {
                present.insert(ChunkType::iTXt);
                if present.contains(&ChunkType::IDAT) {
                    // IDAT chunks must be consecutive
                    authorized.remove(&ChunkType::IDAT);
                }
                Ok((present, authorized))
            }
            ChunkType::tEXt => {
                present.insert(ChunkType::tEXt);
                if present.contains(&ChunkType::IDAT) {
                    // IDAT chunks must be consecutive
                    authorized.remove(&ChunkType::IDAT);
                }
                Ok((present, authorized))
            }
            ChunkType::zTXt => {
                present.insert(ChunkType::zTXt);
                if present.contains(&ChunkType::IDAT) {
                    // IDAT chunks must be consecutive
                    authorized.remove(&ChunkType::IDAT);
                }
                Ok((present, authorized))
            }
            ChunkType::Unknown(name) => Err(format!("{:?} chunks are not handled for now", name)),
        }
    } else {
        Err(format!("Unauthorized chunk: {:?}", &chunk.chunk_type))
    }
}

// 23 juillet ----------- journée pastaciutta + pot = LOL -------------------------

fn parse_ihdr_data(input: &[u8]) -> IResult<&[u8], IHDRData> {
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

#[derive(Debug)]
struct IHDRData {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: ColorType,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

#[derive(Debug)]
enum ColorType {
    Gray,
    RGB,
    PLTE,
    GrayAlpha,
    RGBA,
}

impl TryFrom<u8> for ColorType {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ColorType::Gray),
            2 => Ok(ColorType::RGB),
            3 => Ok(ColorType::PLTE),
            4 => Ok(ColorType::GrayAlpha),
            6 => Ok(ColorType::RGBA),
            _ => Err(format!("Color type {} is not valid", value)),
        }
    }
}

// 24 juillet ---------------------------------------------------------------------

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

// TODO
fn parse_idat_data(input: &[u8]) -> IResult<&[u8], IDATData> {
    let flags = TINFL_FLAG_PARSE_ZLIB_HEADER | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let mut decomp = inflate::core::DecompressorOxide::new();
    decomp.init();
    let mut ret: Vec<u8> = vec![0; input.len() * 2];

    let mut in_pos = 0;
    let mut out_pos = 0;
    loop {
        let (status, in_consumed, out_consumed) = {
            // Wrap the whole output slice so we know we have enough of the
            // decompressed data for matches.
            let mut c = Cursor::new(ret.as_mut_slice());
            c.set_position(out_pos as u64);
            inflate::core::decompress(&mut decomp, &input[in_pos..], &mut c, flags)
        };
        in_pos += in_consumed;
        out_pos += out_consumed;

        match status {
            inflate::TINFLStatus::Done => {
                ret.truncate(out_pos);
                return Ok((
                    &[],
                    IDATData {
                        length: input.len(),
                        data: ret,
                    },
                ));
            }

            inflate::TINFLStatus::HasMoreOutput => {
                // We need more space so extend the buffer.
                ret.extend(&vec![0; out_pos]);
            }

            _ => return map_res(rest, |_| Err(status))(input),
        }
    }
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

#[derive(Debug)]
enum SignificantBits {
    Gray(u8),
    GrayAlpha([u8; 2]),
    RGB([u8; 3]),
    RGBA([u8; 4]),
}

#[derive(Debug)]
struct PhysicalPixelDimension {
    x: u32,
    y: u32,
    unit: DimensionUnit,
}

#[derive(Debug)]
enum DimensionUnit {
    Unknown,
    Meter,
}

#[derive(Debug)]
struct Text {
    keyword: String,
    text: String,
}

#[derive(Debug)]
struct CompressedText {
    keyword: String,
    method: u8,
    text: String,
}

#[derive(Debug)]
struct IDATData {
    length: usize,
    data: Vec<u8>,
}

#[derive(Debug)]
enum Background {
    Palette(u8),
    Gray(u16),
    RGB([u16; 3]),
}

#[derive(Debug)]
struct LastModificationTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
enum ChunkData<'a> {
    // Critical chunks
    IHDR(IHDRData), // image header
    // PLTE, // palette
    IDAT(IDATData), // image data
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

// TODO
fn parse_chunk_data<'a>(chunk: &'a Chunk<'a>) -> IResult<&'a [u8], ChunkData<'a>> {
    match chunk.chunk_type {
        // --- Critical chunks ---
        ChunkType::IHDR => map(parse_ihdr_data, ChunkData::IHDR)(&chunk.data),
        ChunkType::PLTE => map(take(0u8), ChunkData::Unknown)(&chunk.data),
        ChunkType::IDAT => map(parse_idat_data, ChunkData::IDAT)(&chunk.data),
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
