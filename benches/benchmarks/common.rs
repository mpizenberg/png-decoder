use png_decoder::filter::Filter;
use rand::{self, Rng};

pub fn gen_idat_inflated(scanline_width: usize, nb_scanlines: usize, filter: Filter) -> Vec<u8> {
    let data_length = scanline_width * nb_scanlines;
    let mut rng = rand::thread_rng();
    let mut data: Vec<u8> = (0..data_length).map(|_| rng.gen()).collect();
    for f in data.iter_mut().step_by(scanline_width) {
        *f = filter.into();
    }
    data
}
