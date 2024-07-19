use german_str::GermanStr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng as _;
use rand::distributions::{Alphanumeric, Uniform};

fn gen_random_string(len: usize) -> String {
    let mut char_gen = rand::thread_rng().sample_iter(Alphanumeric);
    let mut vec = Vec::new();
    for _ in 0..len {
        vec.push(char_gen.next().unwrap());
    }
    String::from_utf8(vec).unwrap()
}

fn bench_mixed(c: &mut Criterion) {
    let mut string_lengths = rand::thread_rng().sample_iter(Uniform::new(2, 50));
    let mut strings = Vec::new();
    for _ in 0..1000 {
        let len = string_lengths.next().unwrap();
        strings.push(gen_random_string(len));
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


fn bench_compare_to_string(c: &mut Criterion) {
    c.bench_function(
        "compare two regular strings",
        |b| b.iter_batched(
            || (gen_random_string(10), gen_random_string(10)),
            |(a, b)| black_box(a == b),
            criterion::BatchSize::SmallInput,
        )
    );
    c.bench_function(
        "compare regular string to german string",
        |b| b.iter_batched(
            || (gen_random_string(10), GermanStr::new(gen_random_string(10)).unwrap()),
            |(a, b)| black_box(a == b),
            criterion::BatchSize::SmallInput,
        )
    );
    c.bench_function(
        "compare two german strings",
        |b| b.iter_batched(
            || (GermanStr::new(gen_random_string(10)).unwrap(), GermanStr::new(gen_random_string(10)).unwrap()),
            |(a, b)| black_box(a == b),
            criterion::BatchSize::SmallInput,
        )
    );
}

criterion_group!(benches, bench_mixed, bench_short, bench_compare_to_string);
criterion_main!(benches);
