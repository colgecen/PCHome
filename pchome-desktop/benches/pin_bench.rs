use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn format_pin(value: u32) -> String {
    format!("{:06}", value % 1_000_000)
}

fn bench_pin_format(c: &mut Criterion) {
    c.bench_function("format_pin", |b| {
        b.iter(|| format_pin(black_box(849204)))
    });
}

criterion_group!(benches, bench_pin_format);
criterion_main!(benches);
