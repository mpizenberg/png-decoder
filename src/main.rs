use nom::bytes::complete::{tag, take};
use nom::multi::many1;
use nom::number::complete::be_u32;
use nom::IResult;
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
            png.iter().for_each(|chunk| println!("{}", chunk));
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

struct Chunk {
    length: u32,
    chunk_type: ChunkType,
    data: Vec<u8>,
    crc: [u8; 4],
}

impl std::fmt::Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{length: {}, type: {:?}, crc: {:?}}}",
            self.length, self.chunk_type, self.crc
        )
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

fn parse_chunk(input: &[u8]) -> IResult<&[u8], Chunk> {
    let (input, length) = be_u32(input)?;
    let (input, t) = take(4usize)(input)?;
    let type_ = [t[0] as char, t[1] as char, t[2] as char, t[3] as char];
    let chunk_type = ChunkType::from(type_);
    let (input, data_) = take(length)(input)?;
    let data = data_.to_owned();
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
#[derive(Debug)]
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
            ['I','H','D','R'] => ChunkType::IHDR,
            ['P','L','T','E'] => ChunkType::PLTE,
            ['I','D','A','T'] => ChunkType::IDAT,
            ['I','E','N','D'] => ChunkType::IEND,
            ['t','R','N','S'] => ChunkType::tRNS,
            ['g','A','M','A'] => ChunkType::gAMA,
            ['c','H','R','M'] => ChunkType::cHRM,
            ['s','R','G','B'] => ChunkType::sRGB,
            ['i','C','C','P'] => ChunkType::iCCP,
            ['t','E','X','t'] => ChunkType::tEXt,
            ['z','T','X','t'] => ChunkType::zTXt,
            ['i','T','X','t'] => ChunkType::iTXt,
            ['b','K','G','D'] => ChunkType::bKGD,
            ['p','H','Y','s'] => ChunkType::pHYs,
            ['s','B','I','T'] => ChunkType::sBIT,
            ['s','P','L','T'] => ChunkType::sPLT,
            ['h','I','S','T'] => ChunkType::hIST,
            ['t','I','M','E'] => ChunkType::tIME,
            _ => ChunkType::Unknown(name),
        }
    }
}

fn validate_chunk_constraints(chunks: &[Chunk]) -> Result<&[Chunk], String> {
    Ok(chunks)
}

struct IHDRData {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: ColorType,
    compression_method: u8,
    filter_method: u8,
    interlace_method: u8,
}

enum ColorType {
    Gray,
    RGB,
    PLTE,
    GrayAlpha,
    RGBA,
}
