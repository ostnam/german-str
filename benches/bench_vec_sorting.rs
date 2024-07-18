use criterion::{black_box, criterion_group, criterion_main, Criterion};
use german_str::GermanStr;
use rand::{distributions::{Alphanumeric, Uniform}, Rng};

fn bench_mixed(c: &mut Criterion) {
    let mut char_gen = rand::thread_rng().sample_iter(Alphanumeric);
    let mut string_lengths = rand::thread_rng().sample_iter(Uniform::new(2, 50));
    let mut strings = Vec::new();
    for _ in 0..1000 {
        let len = string_lengths.next().unwrap();
        let mut vec = Vec::new();
        for _ in 0..len {
            vec.push(char_gen.next().unwrap());
        }
        let string = String::from_utf8(vec).unwrap();
        strings.push(string);
    }
    let german_strings: Vec<_> = strings.iter()
        .map(|s| GermanStr::new(s).unwrap())
        .collect();
    c.bench_function(
        "sort regular strings",
        |b| b.iter_batched(
            || strings.clone(),
            |mut data| black_box(data.sort()),
            criterion::BatchSize::SmallInput,
        )
    );
    c.bench_function(
        "sort german strings",
        |b| b.iter_batched(
            || german_strings.clone(),
            |mut data| black_box(data.sort()),
            criterion::BatchSize::SmallInput,
        )
    );
}

fn bench_short(c: &mut Criterion) {
    let mut char_gen = rand::thread_rng().sample_iter(Alphanumeric);
    let mut string_lengths = rand::thread_rng().sample_iter(Uniform::new(0, 12));
    let mut strings = Vec::new();
    for _ in 0..1000 {
        let len = string_lengths.next().unwrap();
        let mut vec = Vec::new();
        for _ in 0..len {
            vec.push(char_gen.next().unwrap());
        }
        let string = String::from_utf8(vec).unwrap();
        strings.push(string);
    }
    let german_strings: Vec<_> = strings.iter()
        .map(|s| GermanStr::new(s).unwrap())
        .collect();
    c.bench_function(
        "sort short regular strings",
        |b| b.iter_batched(
            || strings.clone(),
            |mut data| black_box(data.sort()),
            criterion::BatchSize::SmallInput,
        )
    );
    c.bench_function(
        "sort short german strings",
        |b| b.iter_batched(
            || german_strings.clone(),
            |mut data| black_box(data.sort()),
            criterion::BatchSize::SmallInput,
        )
    );
}


criterion_group!(benches, bench_mixed, bench_short);
criterion_main!(benches);
