use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{Rng, thread_rng};

fn mix_benchmark(c: &mut Criterion) {
    println!("bench!");

    let mut arr = [0; 0x100_000];
    thread_rng().fill(arr.as_mut());

    c.bench_function("mix", |b| b.iter(|| {
        let mut m = gesist::mixer::Mixer::new_from_padder(gesist::padder::Padder::new(black_box(arr.as_slice())));
        m.mix();
    }));
}

criterion_group!(mix_benches, mix_benchmark);
criterion_main!(mix_benches);
