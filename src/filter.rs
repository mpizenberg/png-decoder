use std::convert::TryFrom;

#[derive(Debug)]
pub enum Filter {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl TryFrom<u8> for Filter {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Filter::None),
            1 => Ok(Filter::Sub),
            2 => Ok(Filter::Up),
            3 => Ok(Filter::Average),
            4 => Ok(Filter::Paeth),
            _ => Err(format!("Filter type {} is not valid", value)),
        }
    }
}

pub fn decode_none(line: &[u8], line_start: usize, data: &mut Vec<u8>) -> usize {
    let next_line_start = line_start + line.len();
    data[line_start..next_line_start].copy_from_slice(line);
    next_line_start
}

pub fn decode_sub(bpp: usize, line: &[u8], line_start: usize, data: &mut Vec<u8>) -> usize {
    let data_line = &mut data.as_mut_slice()[line_start..];
    line.iter().enumerate().for_each(|(i, p)| {
        let left = if i >= bpp { data_line[i - bpp] } else { 0 };
        data_line[i] = p.wrapping_add(left);
    });
    line_start + line.len()
}

pub fn decode_up(line: &[u8], line_start: usize, data: &mut Vec<u8>) -> usize {
    if line_start == 0 {
        line.iter().enumerate().for_each(|(i, p)| {
            data[line_start + i] = *p;
        });
    } else {
        let previous_line_start = line_start - line.len();
        line.iter().enumerate().for_each(|(i, p)| {
            let up = data[previous_line_start + i];
            data[line_start + i] = p.wrapping_add(up);
        });
    }
    line_start + line.len()
}

pub fn decode_average(bpp: usize, line: &[u8], line_start: usize, data: &mut Vec<u8>) -> usize {
    if line_start == 0 {
        line.iter().enumerate().for_each(|(i, p)| {
            let left = if i >= bpp {
                data[line_start + i - bpp]
            } else {
                0
            };
            data[line_start + i] = p.wrapping_add(left);
        });
    } else {
        let previous_line_start = line_start - line.len();
        line.iter().enumerate().for_each(|(i, p)| {
            let up = data[previous_line_start + i] as u16;
            let left = if i >= bpp {
                data[line_start + i - bpp]
            } else {
                0
            } as u16;
            data[line_start + i] = p.wrapping_add(((up + left) / 2) as u8);
        });
    }
    line_start + line.len()
}

pub fn decode_paeth(bpp: usize, line: &[u8], line_start: usize, data: &mut Vec<u8>) -> usize {
    if line_start == 0 {
        line.iter().enumerate().for_each(|(i, p)| {
            let left = if i >= bpp {
                data[line_start + i - bpp]
            } else {
                0
            };
            data[line_start + i] = p.wrapping_add(paeth_predictor(left, 0, 0));
        });
    } else {
        let previous_line_start = line_start - line.len();
        line.iter().enumerate().for_each(|(i, p)| {
            let (up_left, up, left) = if i >= bpp {
                (
                    data[previous_line_start + i - bpp],
                    data[previous_line_start + i],
                    data[line_start + i - bpp],
                )
            } else {
                (0, data[previous_line_start + i], 0)
            };
            data[line_start + i] = p.wrapping_add(paeth_predictor(left, up, up_left));
        });
    }
    line_start + line.len()
}

// http://www.libpng.org/pub/png/spec/1.2/png-1.2-pdg.html#Filters
// ; a = left, b = above, c = upper left
// p := a + b - c        ; initial estimate
// pa := abs(p - a)      ; distances to a, b, c
// pb := abs(p - b)
// pc := abs(p - c)
// ; return nearest of a,b,c,
// ; breaking ties in order a,b,c.
// if pa <= pb AND pa <= pc then return a
// else if pb <= pc then return b
// else return c
fn paeth_predictor(left: u8, up: u8, up_left: u8) -> u8 {
    let (a, b, c) = (left as i16, up as i16, up_left as i16);
    let p = a + b - c; // initial estimate
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        left // a
    } else if pb <= pc {
        up // b
    } else {
        up_left // c
    }
}
