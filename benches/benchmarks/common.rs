use png_decoder::filter::Filter;
use rand::{self, Rng};
use std::convert::TryFrom;

pub fn gen_idat_inflated(scanline_width: usize, nb_scanlines: usize, filter: Filter) -> Vec<u8> {
    let data_length = scanline_width * nb_scanlines;
    let mut rng = rand::thread_rng();
    let mut data: Vec<u8> = (0..data_length).map(|_| rng.gen()).collect();
    for f in data.iter_mut().step_by(scanline_width) {
        *f = filter.into();
    }
    data
}

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
