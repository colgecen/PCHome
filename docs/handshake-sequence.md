# Handshake Sequence

```mermaid
sequenceDiagram
    autonumber
    participant D as Desktop (Rust)
    participant S as Signal Server (Go)
    participant M as Mobile (Android)

    D->>D: generate crypto-secure 6-digit PIN
    D->>S: GET /ws?pin=123456&role=desktop (Reserve room)
    S-->>D: 200 OK (room created, TTL 300s)

    M->>M: user enters PIN "123-456" (stripped to 123456)
    M->>S: GET /ws?pin=123456&role=mobile
    S-->>M: 200 OK (hub linked to desktop)

    D->>S: { type: "offer", sdp }
    S->>M: relay { from: desktop, data: offer }
    M->>S: { type: "answer", sdp }
    S->>D: relay { from: mobile, data: answer }

    D->>S: { type: "ice-candidate", candidate }
    S->>M: relay { from: desktop, data: ice-candidate }
    M->>S: { type: "ice-candidate", candidate }
    S->>D: relay { from: mobile, data: ice-candidate }

    Note over D,M: WebRTC PeerConnection established (P2P)

    D->>M: screen frames (MediaStream) + input (DataChannel)
    M->>D: touch/key events (DataChannel)
```

## Notes

- The Signal server only relays; it never echoes a message back to its sender.
- Each relayed message refreshes the room TTL, so an active session does not
  expire.
- Once the `PeerConnection` is up, all media and input traffic is P2P and no
  longer touches the Signal server.
