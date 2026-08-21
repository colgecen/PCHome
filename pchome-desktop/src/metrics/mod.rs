use lazy_static::lazy_static;
use prometheus::{Counter, Histogram, HistogramOpts, Opts, Registry, TextEncoder, Encoder};
use std::io::Write;
use std::net::SocketAddr;

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

/// Expose the Prometheus registry over a minimal HTTP endpoint. Spawns a
/// blocking listener on a dedicated thread so it never blocks the async runtime.
pub fn serve(addr: SocketAddr) {
    std::thread::spawn(move || {
        if let Ok(listener) = std::net::TcpListener::bind(addr) {
            log::info!("Metrics endpoint listening on http://{}", addr);
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut body = Vec::new();
                if TextEncoder::new().encode(&REGISTRY.gather(), &mut body).is_ok() {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(&body);
                }
            }
        } else {
            log::warn!("Failed to bind metrics endpoint on {}", addr);
        }
    });
}
