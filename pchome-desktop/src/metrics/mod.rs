use lazy_static::lazy_static;
use prometheus::{Counter, Histogram, HistogramOpts, Opts, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref CAPTURE_LATENCY: Histogram = {
        let opts = HistogramOpts::new("pchome_capture_latency_ms", "Capture roundtrip latency in milliseconds");
        let h = Histogram::with_opts(opts).expect("metric init failed");
        REGISTRY.register(Box::new(h.clone())).expect("register failed");
        h
    };

    pub static ref ENCODE_LATENCY: Histogram = {
        let opts = HistogramOpts::new("pchome_encode_latency_ms", "Encode roundtrip latency in milliseconds");
        let h = Histogram::with_opts(opts).expect("metric init failed");
        REGISTRY.register(Box::new(h.clone())).expect("register failed");
        h
    };

    pub static ref FRAMES_ENCODED: Counter = {
        let opts = Opts::new("pchome_frames_encoded_total", "Total number of frames encoded");
        let c = Counter::with_opts(opts).expect("metric init failed");
        REGISTRY.register(Box::new(c.clone())).expect("register failed");
        c
    };

    pub static ref ERRORS_TOTAL: Counter = {
        let opts = Opts::new("pchome_errors_total", "Total number of errors by type");
        let c = Counter::with_opts(opts).expect("metric init failed");
        REGISTRY.register(Box::new(c.clone())).expect("register failed");
        c
    };

    pub static ref BITRATE_BYTES_TOTAL: Counter = {
        let opts = Opts::new("pchome_bitrate_bytes_total", "Total bytes encoded for bitrate calculation");
        let c = Counter::with_opts(opts).expect("metric init failed");
        REGISTRY.register(Box::new(c.clone())).expect("register failed");
        c
    };
}
