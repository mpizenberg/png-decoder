use std::convert::TryFrom;

#[derive(Debug, Copy, Clone)]
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

impl Into<u8> for Filter {
    fn into(self) -> u8 {
        match self {
            Filter::None => 0,
            Filter::Sub => 1,
            Filter::Up => 2,
            Filter::Average => 3,
            Filter::Paeth => 4,
        }
    }
}

pub fn unfilter(
    width: usize,
    height: usize,
    bpp: usize,
    scanlines: Vec<(Filter, &[u8])>,
) -> Vec<u8> {
    let len = bpp * width;
    let mut data = vec![0; len * height];
    let mut prev = vec![0; len];
    let mut line_start = 0;
    for (filter, line) in scanlines.into_iter() {
        match filter {
            Filter::None => decode_none(line, line_start, &mut data),
            Filter::Sub => decode_sub(bpp, line, line_start, &mut data),
            Filter::Up => decode_up(line, line_start, &mut data, &mut prev),
            Filter::Average => decode_average(bpp, line, line_start, &mut data, &mut prev),
            Filter::Paeth => decode_paeth(bpp, line, line_start, &mut data, &mut prev),
        };
        line_start += len;
    }
    data
}

pub fn unfilter_bis(
    width: usize,
    height: usize,
    bpp: usize,
    scanlines: Vec<(Filter, usize)>,
    inflated: &mut [u8],
) -> Vec<u8> {
    let len = bpp * width;
    let mut data = vec![0; height * len];
    let mut prev_buff = vec![0; len];
    let mut data_start = 0;
    for (filter, start) in scanlines.into_iter() {
        match filter {
            Filter::None => decode_none_bis(len, start, inflated, data_start, &mut data),
            Filter::Sub => decode_sub_bis(bpp, len, start, inflated, data_start, &mut data),
            Filter::Up => {
                decode_up_bis(len, start, inflated, data_start, &mut data, &mut prev_buff)
            }
            Filter::Average => decode_average_bis(
                bpp,
                len,
                start,
                inflated,
                data_start,
                &mut data,
                &mut prev_buff,
            ),
            Filter::Paeth => decode_paeth_bis(
                bpp,
                len,
                start,
                inflated,
                data_start,
                &mut data,
                &mut prev_buff,
            ),
        };
        data_start += len;
    }
    data
}

#[inline]
pub fn decode_none(line: &[u8], line_start: usize, data: &mut [u8]) {
    let next_line_start = line_start + line.len();
    data[line_start..next_line_start].copy_from_slice(line);
}

#[inline]
pub fn decode_none_bis(
    len: usize,
    start: usize,
    inflated: &mut [u8],
    data_start: usize,
    data: &mut [u8],
) -> () {
    data[data_start..data_start + len].copy_from_slice(&inflated[start..start + len]);
}

#[inline]
pub fn decode_sub_bis(
    bpp: usize,
    len: usize,
    start: usize,
    inflated: &mut [u8],
    data_start: usize,
    data: &mut [u8],
) -> () {
    let current = &inflated[start..start + len];
    let data_line = &mut data[data_start..];
    data_line[..bpp].copy_from_slice(&current[..bpp]);
    for i in bpp..len {
        data_line[i] = current[i].wrapping_add(data_line[i - bpp]);
    }
}

pub fn decode_up_bis(
    len: usize,
    start: usize,
    inflated: &mut [u8],
    data_start: usize,
    data: &mut [u8],
    prev_buff: &mut [u8],
) -> () {
    if data_start == 0 {
        decode_none_bis(len, start, inflated, data_start, data);
    } else {
        prev_buff.copy_from_slice(&data[data_start - len..data_start]);
        let current = &mut inflated[start..start + len];
        for i in 0..len {
            current[i] = current[i].wrapping_add(prev_buff[i]);
        }
        data[data_start..data_start + len].copy_from_slice(&current);
    }
}

pub fn decode_average_bis(
    bpp: usize,
    len: usize,
    start: usize,
    inflated: &mut [u8],
    data_start: usize,
    data: &mut [u8],
    prev_buff: &mut [u8],
) -> () {
    if data_start == 0 {
        decode_sub_bis(bpp, len, start, inflated, data_start, data);
    } else {
        prev_buff.copy_from_slice(&data[data_start - len..data_start]);
        let current = &mut inflated[start..start + len];
        for i in 0..bpp {
            current[i] = current[i].wrapping_add(prev_buff[i] / 2);
        }
        for i in bpp..len {
            let up = prev_buff[i] as u16;
            let left = current[i - bpp] as u16;
            current[i] = current[i].wrapping_add(((up + left) / 2) as u8);
        }
        data[data_start..data_start + len].copy_from_slice(&current);
    }
}

pub fn decode_paeth_bis(
    bpp: usize,
    len: usize,
    start: usize,
    inflated: &mut [u8],
    data_start: usize,
    data: &mut [u8],
    prev_buff: &mut [u8],
) -> () {
    if data_start == 0 {
        decode_sub_bis(bpp, len, start, inflated, data_start, data);
    } else {
        prev_buff.copy_from_slice(&data[data_start - len..data_start]);
        let current = &mut inflated[start..start + len];
        for i in 0..bpp {
            current[i] = current[i].wrapping_add(prev_buff[i]);
        }
        for i in bpp..len {
            let up_left = prev_buff[i - bpp];
            let up = prev_buff[i];
            let left = current[i - bpp];
            current[i] = current[i].wrapping_add(paeth_predictor(left, up, up_left));
        }
        data[data_start..data_start + len].copy_from_slice(&current);
    }
}

#[inline]
pub fn decode_sub(bpp: usize, line: &[u8], line_start: usize, data: &mut [u8]) {
    let data_line = &mut data[line_start..];
    data_line[..bpp].copy_from_slice(&line[..bpp]);
    for i in bpp..line.len() {
        data_line[i] = line[i].wrapping_add(data_line[i - bpp]);
    }
}

pub fn decode_up(line: &[u8], line_start: usize, data: &mut [u8], previous: &mut [u8]) {
    if line_start == 0 {
        decode_none(line, line_start, data)
    } else {
        previous.copy_from_slice(&data[line_start - line.len()..line_start]);
        let data_line = &mut data[line_start..line_start + line.len()];
        line.iter().enumerate().for_each(|(i, p)| {
            data_line[i] = p.wrapping_add(previous[i]);
        });
    }
}

pub fn decode_average(
    bpp: usize,
    line: &[u8],
    line_start: usize,
    data: &mut [u8],
    previous: &mut [u8],
) {
    if line_start == 0 {
        decode_sub(bpp, line, line_start, data)
    } else {
        previous.copy_from_slice(&data[line_start - line.len()..line_start]);
        let data_line = &mut data[line_start..line_start + line.len()];
        line.iter().take(bpp).enumerate().for_each(|(i, p)| {
            data_line[i] = p.wrapping_add(previous[i] / 2);
        });
        line.iter().enumerate().skip(bpp).for_each(|(i, p)| {
            let up = previous[i] as u16;
            let left = data_line[i - bpp] as u16;
            data_line[i] = p.wrapping_add(((up + left) / 2) as u8);
        });
    }
}

pub fn decode_paeth(
    bpp: usize,
    line: &[u8],
    line_start: usize,
    data: &mut [u8],
    previous: &mut [u8],
) {
    if line_start == 0 {
        decode_sub(bpp, line, line_start, data)
    } else {
        previous.copy_from_slice(&data[line_start - line.len()..line_start]);
        let data_line = &mut data[line_start..line_start + line.len()];
        line.iter().take(bpp).enumerate().for_each(|(i, p)| {
            data_line[i] = p.wrapping_add(previous[i]);
        });
        line.iter().enumerate().skip(bpp).for_each(|(i, p)| {
            let up_left = previous[i - bpp];
            let up = previous[i];
            let left = data_line[i - bpp];
            data_line[i] = p.wrapping_add(paeth_predictor(left, up, up_left));
        });
    }
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
#[inline]
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
