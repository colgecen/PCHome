# Handshake Sequence

```mermaid
sequenceDiagram
    autonumber
    participant D as Desktop (Rust)
    participant S as Signal Server (Rust)
    participant M as Mobile (Android)

    D->>D: generate crypto-secure 6-digit PIN
    D->>S: WS ws://signal/ws?pin=123456&role=desktop (room created, TTL 300s)
    S-->>D: 101 Switching Protocols

    M->>M: user enters PIN "123-456" (stripped to 123456)
    M->>S: WS ws://signal/ws?pin=123456&role=mobile
    S-->>M: 101 Switching Protocols (linked to desktop)

    M->>S: { type: "hello" }
    S->>D: relay hello
    D->>S: { type: "offer", sdp }
    S->>M: relay offer
    M->>S: { type: "answer", sdp }
    S->>D: relay answer

    D->>S: { type: "ice-candidate", candidate }
    S->>M: relay ice-candidate
    M->>S: { type: "ice-candidate", candidate }
    S->>D: relay ice-candidate

    Note over D,M: WebRTC PeerConnection established (P2P)

    D->>M: screen frames (video track) + ping (DataChannel)
    M->>D: touch/key events + pong (DataChannel)
```

## Notes

- The Signal server only relays; it never echoes a message back to its sender.
- The desktop is the WebRTC **offerer**: it waits for the mobile `hello`
  before creating the offer.
- Rooms carry a TTL of 300 seconds from the first join and are swept by a
  background cleaner; a room is removed when both peers disconnect or the
  TTL elapses.
- Once the `PeerConnection` is up, all media and input traffic is P2P and no
  longer touches the Signal server.
