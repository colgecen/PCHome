use criterion::{criterion_group, criterion_main, Criterion};
use pchome_desktop::encoder::Encoder;
use pchome_desktop::pipewire::{CaptureStream, Frame};
use pchome_desktop::pin::PinManager;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

fn bench_pin_generation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(async { PinManager::new() });

    c.bench_function("pin_generate_and_register", |b| {
        b.iter(|| {
            let code = rt.block_on(async { manager.generate_and_register().await.unwrap() });
            assert!(code < 1_000_000);
        });
    });
}

fn bench_encoder_software_fallback(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut encoder = rt.block_on(async {
        Encoder::init_encoder().unwrap()
    });

    let frame_data = vec![0u8; 1920 * 1080 * 4];

    c.bench_function("encoder_software_fallback_1080p", |b| {
        b.iter(|| {
            let result = rt.block_on(async {
                encoder.encode(&frame_data).await.unwrap()
            });
            assert!(!result.is_empty());
        });
    });
}

fn bench_capture_stream_init(c: &mut Criterion) {
    c.bench_function("capture_stream_init", |b| {
        b.iter(|| {
            let stream = CaptureStream::new(1920, 1080, 60);
            assert_eq!(stream.width, 1920);
            assert_eq!(stream.height, 1080);
            assert_eq!(stream.framerate, 60);
        });
    });
}

fn bench_roundtrip_simulated(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut encoder = rt.block_on(async {
        Encoder::init_encoder().unwrap()
    });

    let frame_data = vec![0u8; 1920 * 1080 * 4];

    c.bench_function("roundtrip_capture_encode_1080p", |b| {
        b.iter(|| {
            let start = Instant::now();
            let encoded = rt.block_on(async {
                encoder.encode(&frame_data).await.unwrap()
            });
            let encode_duration = start.elapsed();

            assert!(!encoded.is_empty());
            assert!(encode_duration < Duration::from_millis(40),
                "encode took {:?}, expected <40ms", encode_duration);
        });
    });
}

criterion_group!(
    benches,
    bench_pin_generation,
    bench_encoder_software_fallback,
    bench_capture_stream_init,
    bench_roundtrip_simulated
);
criterion_main!(benches);
