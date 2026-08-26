# PChome Signal Server (Rust)

Lightweight WebSocket signaling relay for PChome P2P WebRTC handshake and 6-digit PIN authentication.

## Features

- **6-Digit PIN Rooms**: Maps `?pin=<digits>&role=desktop|mobile` peers into rooms; stale rooms are swept after a 300s TTL.
- **WebSocket Relay**: Relays WebRTC SDP offers/answers and ICE candidates to the other peer only (no echo).
- **Health & Metrics**: `GET /health` (port `HEALTH_PORT`, default 8081) plus Prometheus-style `/metrics` gauges.
- **Rate Limiting**: Per-IP sliding-window cap on new connections via `RATE_LIMIT` (default 20/min).
- **Docker Ready**: Prepared for 24/7 cloud deployment (Railway/Render terminate TLS).

## Getting Started

```bash
cd pchome-signal
cargo build --release
./target/release/pchome-signal
# WS: ws://0.0.0.0:8080/ws   Health: http://0.0.0.0:8081/health
```

## Docker Deployment

```bash
docker build -t pchome-signal .
docker run -d -p 8080:8080 -p 8081:8081 pchome-signal
```
