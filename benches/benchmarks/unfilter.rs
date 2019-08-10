use super::common;
use criterion::{criterion_group, Criterion};
use lazy_static::lazy_static;
use png_decoder::filter::{self, Filter};

const BPP: usize = 3;
const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const SCANLINE_WIDTH: usize = 1 + BPP * WIDTH;
lazy_static! {
    static ref DATA_NONE: Vec<u8> = common::gen_idat_inflated(SCANLINE_WIDTH, HEIGHT, Filter::None);
    static ref DATA_SUB: Vec<u8> = common::gen_idat_inflated(SCANLINE_WIDTH, HEIGHT, Filter::Sub);
    static ref DATA_UP: Vec<u8> = common::gen_idat_inflated(SCANLINE_WIDTH, HEIGHT, Filter::Up);
    static ref DATA_AVERAGE: Vec<u8> =
        common::gen_idat_inflated(SCANLINE_WIDTH, HEIGHT, Filter::Average);
    static ref DATA_PAETH: Vec<u8> =
        common::gen_idat_inflated(SCANLINE_WIDTH, HEIGHT, Filter::Paeth);
}

fn unfilter_none(c: &mut Criterion) {
    c.bench_function("unfilter, none, clone overhead", |b| {
        b.iter(|| DATA_NONE.clone())
    });
    c.bench_function("unfilter, none, slice", |b| {
        b.iter(|| unfilter_slice(&DATA_NONE))
    });
    c.bench_function("unfilter, none, mut", |b| {
        b.iter(|| unfilter_mut(&DATA_NONE))
    });
}

fn unfilter_sub(c: &mut Criterion) {
    c.bench_function("unfilter, sub, clone overhead", |b| {
        b.iter(|| DATA_SUB.clone())
    });
    c.bench_function("unfilter, sub, slice", |b| {
        b.iter(|| unfilter_slice(&DATA_SUB))
    });
    c.bench_function("unfilter, sub, mut", |b| b.iter(|| unfilter_mut(&DATA_SUB)));
}

fn unfilter_up(c: &mut Criterion) {
    c.bench_function("unfilter, up, clone overhead", |b| {
        b.iter(|| DATA_UP.clone())
    });
    c.bench_function("unfilter, up, slice", |b| {
        b.iter(|| unfilter_slice(&DATA_UP))
    });
    c.bench_function("unfilter, up, mut", |b| b.iter(|| unfilter_mut(&DATA_UP)));
}

fn unfilter_average(c: &mut Criterion) {
    c.bench_function("unfilter, average, clone overhead", |b| {
        b.iter(|| DATA_AVERAGE.clone())
    });
    c.bench_function("unfilter, average, slice", |b| {
        b.iter(|| unfilter_slice(&DATA_AVERAGE))
    });
    c.bench_function("unfilter, average, mut", |b| {
        b.iter(|| unfilter_mut(&DATA_AVERAGE))
    });
}

fn unfilter_paeth(c: &mut Criterion) {
    c.bench_function("unfilter, paeth, clone overhead", |b| {
        b.iter(|| DATA_PAETH.clone())
    });
    c.bench_function("unfilter, paeth, slice", |b| {
        b.iter(|| unfilter_slice(&DATA_PAETH))
    });
    c.bench_function("unfilter, paeth, mut", |b| {
        b.iter(|| unfilter_mut(&DATA_PAETH))
    });
}

// Helpers

fn unfilter_slice(data: &[u8]) {
    let data = data.to_vec();
    let scanlines = common::lines_slices(&data, SCANLINE_WIDTH);
    filter::unfilter(WIDTH, HEIGHT, BPP, scanlines);
}

fn unfilter_mut(data: &[u8]) {
    let mut data = data.to_vec();
    let scanlines = common::lines_num(&data, SCANLINE_WIDTH);
    filter::unfilter_bis(WIDTH, HEIGHT, BPP, scanlines, &mut data);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = unfilter_none, unfilter_sub, unfilter_up, unfilter_average, unfilter_paeth
}
