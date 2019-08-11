use criterion::{criterion_group, Criterion};
use png;
use png_decoder::png as my_png;

fn bench(c: &mut Criterion) {
    let png_raw_data = std::fs::read("data/depth.png").unwrap();
    // let png_raw_data = std::fs::read("data/eye.png").unwrap();
    // let png_raw_data = std::fs::read("data/inkscape.png").unwrap();
    // let png_raw_data = std::fs::read("data/rgb.png").unwrap();
    // let png_raw_data = std::fs::read("data/screen.png").unwrap();
    // let png_raw_data = std::fs::read("data/texture_alpha.png").unwrap();
    // let png_raw_data = std::fs::read("data/transparent.png").unwrap();
    let png_raw_data_bis = png_raw_data.clone();
    let png_raw_data_clone = png_raw_data.clone();

    c.bench_function("decode_file, slice", move |b| {
        b.iter(|| my_png::decode_no_check(&png_raw_data))
    });

    c.bench_function("decode_file, bis", move |b| {
        b.iter(|| my_png::decode_no_check_bis(&png_raw_data_bis))
    });

    c.bench_function("decode_file, png crate", move |b| {
        b.iter(|| {
            let mut decoder = png::Decoder::new(png_raw_data_clone.as_slice());
            // Use the IDENTITY transformation because by default
            // it will use STRIP_16 which only keep 8 bits.
            // See also SWAP_ENDIAN that might be useful
            //   (but seems not possible to use according to documentation).
            decoder.set_transformations(png::Transformations::IDENTITY);
            let (info, mut reader) = decoder.read_info().unwrap();
            let mut buffer = vec![0; info.buffer_size()];
            reader.next_frame(&mut buffer).unwrap();
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench
}
