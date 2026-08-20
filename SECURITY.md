# PChome Security Guidelines

## P2P Encryption

### WebRTC SRTP Encryption
All data transmitted between PChome Desktop and PChome Mobile uses WebRTC DataChannels, which provide built-in SRTP (Secure Real-time Transport Protocol) encryption.

**Encryption Guarantees**:
- AES-128 or AES-256 GCM encryption by default
- Key exchange happens via DTLS handshake during WebRTC setup
- No plaintext data leaves the source device

### Signal Server Security
The Go Signal server operates as a relay only:
- **No media relay**: WebRTC DataChannels are peer-to-peer (P2P)
- **No session storage**: PINs are transient (300s TTL), not persisted
- **Minimal metadata**: Only room PIN mapping and ICE candidate relay

### PIN Authentication Flow
1. Desktop generates cryptographically secure 6-digit PIN
2. PIN registered to Signal server via authenticated WebSockets
3. User inputs PIN in Mobile app
4. Signal server validates PIN match
5. WebRTC connection established only after successful authentication
6. PIN expired after 300 seconds (automatic TTL cleanup)

### Linux udev Security Rules

#### udev Rule for /dev/uinput
```bash
# Allow only pchome user to access uinput
KERNEL=="uinput", GROUP="pchome", MODE="0660"
RUN+="/usr/bin/chmod 0660 /dev/%k"
RUN+="/usr/bin/chown root:pchome /dev/%k"
```

#### Required Group
- Create `pchome` group
- Add authorized users to `pchome` group
- Ensure proper permissions on `/dev/uinput`

#### Verification
```bash
# Check uinput permissions
ls -la /dev/uinput
# Should show: crw-rw---- root pchome /dev/uinput

# Verify user group membership
groups <username>
# Should include pchome
```

### Data Protection

#### Screen Capture Security
- DMA-BUF zero-copy avoids unnecessary memory copies
- Encoded H.264 streams are transmitted via encrypted WebRTC
- No screen content stored to disk unless explicitly configured

#### Input Injection Security
- /dev/uinput events are injected at kernel level
- Only authorized users (pchome group) can create uinput devices
- Events are scoped to the virtual input device only

### Threat Model

#### Mitigated Threats
- **Eavesdropping**: WebRTC SRTP encryption prevents traffic interception
- **Session hijacking**: 300s PIN TTL limits exposure window
- **Unauthorized input**: udev rules restrict /dev/uinput access
- **Data leakage**: No persistent storage of captured content

#### Remaining Risks
- **Physical access**: If attacker has physical access to either device, security boundaries are reduced
- **Network compromise**: Endpoints on compromised networks may be vulnerable
- **Signal server MITM**: Relies on WebSocket TLS; certificate validation required

### Compliance

#### Required Configurations
- [ ] WebSocket TLS certificates valid and auto-renewed
- [ ] udev rules deployed on all Linux systems running PChome Desktop
- [ ] PIN generation uses CSPRNG (ChaCha20 or similar)
- [ ] 300s PIN TTL enforced server-side and client-side
- [ ] /dev/uinput group permissions verified periodically

#### Audit Checklist
- [ ] WebRTC implementation uses DTLS/SRTP
- [ ] Signal server does not store PINs beyond TTL
- [ ] udev rules present and correct
- [ ] Color palette enforcement in all UI code
- [ ] Async architecture prevents blocking vulnerabilities

### Secret Scanning

#### Git History Scanning
- **gitleaks**: Pre-commit hook and CI integration scan git history for secrets
- **truffleHog**: Periodic full history scans for high-entropy strings
- **GitHub Secret Scanning**: Enabled for repository; alerts pushed to security tab

#### Pre-commit Prevention
- All commits are scanned via `gitleaks` before entering the repository
- Large binary blobs are rejected via `pre-commit` check-added-large-files hook
- Private key patterns are detected via `detect-private-key` hook

#### Runtime Protection
- No hardcoded credentials in source code
- Environment variables and secret managers used for runtime configuration
- `.env` and `.env.local` files are gitignored
- Rotation policy for any test credentials

#### Incident Response
1. Immediately rotate exposed credentials
2. Audit access logs for the affected period
3. Add regex patterns to gitleaks config to prevent recurrence
4. Force-prewrite git history if secrets are found in committed history
5. Notify security team and affected service providers

### Reporting Security Issues

- **DO NOT** open public issues for security vulnerabilities
- Email security issues to the maintainers directly
- Include detailed reproduction steps and impact assessment
- Allow 90 days for remediation before public disclosure