use german_str::GermanStr;

use criterion::{criterion_group, criterion_main, Criterion};
use rand::Rng as _;
use rand::distributions::Alphanumeric;
use smol_str::SmolStr;

fn comparison_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("Comparing strings.");
    group.bench_function(
        "german strings, 4 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(4)).unwrap(), GermanStr::new(gen_random_string(4)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "smolstr, 4 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(4)), SmolStr::new(gen_random_string(4))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "String, 4 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(4), gen_random_string(4)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "german strings, 10 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(10)).unwrap(), GermanStr::new(gen_random_string(10)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "smolstr, 10 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(10)), SmolStr::new(gen_random_string(10))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "String, 10 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(10), gen_random_string(10)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "german strings, 20 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(20)).unwrap(), GermanStr::new(gen_random_string(20)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "smolstr, 20 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(20)), SmolStr::new(gen_random_string(20))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "String, 20 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(20), gen_random_string(20)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "german strings, 50 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(50)).unwrap(), GermanStr::new(gen_random_string(50)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "smolstr, 50 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(50)), SmolStr::new(gen_random_string(50))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "String, 50 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(50), gen_random_string(50)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
}

fn gen_random_string(len: usize) -> String {
    let mut char_gen = rand::thread_rng().sample_iter(Alphanumeric);
    let mut vec = Vec::new();
    for _ in 0..len {
        vec.push(char_gen.next().unwrap());
    }
    String::from_utf8(vec).unwrap()
}

criterion_group!(benches, comparison_benches);
criterion_main!(benches);
