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
    type_: [char; 4],
    data: Vec<u8>,
    crc: [u8; 4],
}

impl std::fmt::Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{length: {}, type: {:?}, crc: {:?}}}",
            self.length, self.type_, self.crc
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
    let (input, data_) = take(length)(input)?;
    let data = data_.to_owned();
    let (input, crc_) = take(4usize)(input)?;
    let crc = [crc_[0], crc_[1], crc_[2], crc_[3]];
    Ok((
        input,
        Chunk {
            length,
            type_,
            data,
            crc,
        },
    ))
}

fn parse_png(input: &[u8]) -> IResult<&[u8], Vec<Chunk>> {
    let (input, _) = tag(SIGNATURE)(input)?;
    many1(parse_chunk)(input)
}
