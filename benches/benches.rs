use german_str::GermanStr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng as _;
use rand::distributions::Alphanumeric;
use smol_str::SmolStr;

fn comparison_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("Ord::cmp");
    group.bench_function(
        "01: Comparing 2 u32",
        |b| b.iter_batched(
            || (rand::thread_rng().gen::<u32>(), rand::thread_rng().gen()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "02: GermanStr, 4 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(4)).unwrap(), GermanStr::new(gen_random_string(4)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "03: GermanStr, 10 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(10)).unwrap(), GermanStr::new(gen_random_string(10)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "04: GermanStr, 20 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(20)).unwrap(), GermanStr::new(gen_random_string(20)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "05: GermanStr, 50 chars",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_random_string(50)).unwrap(), GermanStr::new(gen_random_string(50)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "06: String, 4 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(4), gen_random_string(4)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "07: String, 10 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(10), gen_random_string(10)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "08: String, 20 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(20), gen_random_string(20)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "09: String, 50 chars",
        |b| b.iter_batched_ref(
            || (gen_random_string(50), gen_random_string(50)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "10: SmolStr, 4 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(4)), SmolStr::new(gen_random_string(4))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "11: SmolStr, 10 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(10)), SmolStr::new(gen_random_string(10))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "12: SmolStr, 20 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(20)), SmolStr::new(gen_random_string(20))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "13: SmolStr, 50 chars",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_random_string(50)), SmolStr::new(gen_random_string(50))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "14: GermanStr, 4 chars, worst case",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_empty_string(4)).unwrap(), GermanStr::new(gen_empty_string(4)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "15: GermanStr, 10 chars, worst case",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_empty_string(10)).unwrap(), GermanStr::new(gen_empty_string(10)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "16: GermanStr, 20 chars, worst case",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_empty_string(20)).unwrap(), GermanStr::new(gen_empty_string(20)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "17: GermanStr, 50 chars, worst case",
        |b| b.iter_batched_ref(
            || (GermanStr::new(gen_empty_string(50)).unwrap(), GermanStr::new(gen_empty_string(50)).unwrap()),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "18: String, 4 chars, worst case",
        |b| b.iter_batched_ref(
            || (gen_empty_string(4), gen_empty_string(4)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "19: String, 10 chars, worst case",
        |b| b.iter_batched_ref(
            || (gen_empty_string(10), gen_empty_string(10)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "20: String, 20 chars, worst case",
        |b| b.iter_batched_ref(
            || (gen_empty_string(20), gen_empty_string(20)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "21: String, 50 chars, worst case",
        |b| b.iter_batched_ref(
            || (gen_empty_string(50), gen_empty_string(50)),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "22: SmolStr, 4 chars, worst case",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_empty_string(4)), SmolStr::new(gen_empty_string(4))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "23: SmolStr, 10 chars, worst case",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_empty_string(10)), SmolStr::new(gen_empty_string(10))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "24: SmolStr, 20 chars, worst case",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_empty_string(20)), SmolStr::new(gen_empty_string(20))),
            |(a, b)| a.cmp(&b),
            criterion::BatchSize::SmallInput,
        )
    );
    group.bench_function(
        "25: SmolStr, 50 chars, worst case",
        |b| b.iter_batched_ref(
            || (SmolStr::new(gen_empty_string(50)), SmolStr::new(gen_empty_string(50))),
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

fn gen_empty_string(len: usize) -> String {
    black_box(
        (0..len).map(|_| ' ').collect()
    )
}


criterion_group!(benches, comparison_benches);
criterion_main!(benches);
