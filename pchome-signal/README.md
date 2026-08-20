# PChome Signal Server (Go)

Lightweight WebSocket signaling server for PChome P2P WebRTC handshake and 6-digit PIN authentication.

## Features

- **6-Digit PIN Room Allocation**: Manages active PIN sessions with 300s TTL.
- **WebSocket Relay**: Relays WebRTC SDP offers/answers and ICE candidates for NAT traversal.
- **Docker Ready**: Prepared for 24/7 cloud deployment.

## Getting Started

```bash
cd pchome-signal
go mod tidy
go run main.go
```

## Docker Deployment

```bash
docker build -t pchome-signal .
docker run -d -p 8080:8080 pchome-signal
```
