use lazy_static::lazy_static;
use nom::bytes::complete::{tag, take};
use nom::multi::many1;
use nom::number::complete::be_u32;
use nom::IResult;
use std::collections::HashSet;
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
            png_valid.iter().for_each(|chunk| println!("{}", chunk));
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

fn validate_chunk_constraints(chunks: &[Chunk]) -> Result<&[Chunk], String> {
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
