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

#### Complete udev Rule File
Create `/etc/udev/rules.d/99-pchome-uinput.rules`:
```udev
# PChome uinput access control
KERNEL=="uinput", GROUP="pchome", MODE="0660", TAG+="uaccess"

# Optional: Restrict to specific device node
SUBSYSTEM=="misc", KERNEL=="uinput", GROUP="pchome", MODE="0660"
```

Apply rules:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
sudo systemctl restart udev
```

#### Runtime Permission Check
```rust
// Desktop daemon verifies permissions at startup
if nix::unistd::geteuid().is_root() {
    warn!("Running as root is not recommended");
}
match std::fs::metadata("/dev/uinput") {
    Ok(meta) => {
        let mode = meta.permissions().mode();
        if mode & 0o0660 != 0o0660 {
            error!("Incorrect /dev/uinput permissions: {:o}", mode);
        }
    }
    Err(e) => error!("Cannot stat /dev/uinput: {}", e),
}
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

### Sandboxing and Capability Limits

#### Process Sandboxing
PChome Desktop daemon should run with minimal privileges:

```bash
# Create dedicated system user
sudo useradd -r -s /usr/sbin/nologin pchome

# Run daemon as non-root
sudo -u pchome pchome-desktop

# Optional: systemd service with sandboxing
```

#### systemd Service with Hardening
Create `/etc/systemd/system/pchome-desktop.service`:
```ini
[Unit]
Description=PChome Desktop Daemon
After=network.target pipewire.service

[Service]
Type=simple
User=pchome
Group=pchome
ExecStart=/usr/bin/pchome-desktop
Restart=on-failure
RestartSec=5

# Sandboxing directives
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
NoNewPrivileges=true
PrivateDevices=true
RestrictSUIDSGID=true
RestrictNamespaces=true
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK
RestrictRealtime=true
RestrictFileSystems=ext4 xfs btrfs tmpfs

# Capability limits
CapabilityBoundingSet=CAP_SYS_ADMIN CAP_NET_ADMIN CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_SYS_ADMIN CAP_NET_ADMIN CAP_NET_BIND_SERVICE

# Resource limits
LimitNOFILE=65536
MemoryMax=512M
CPUQuota=50%

[Install]
WantedBy=multi-user.target
```

#### Linux Capabilities
Required capabilities for `/dev/uinput` access:
- `CAP_SYS_ADMIN`: Required for uinput device creation
- `CAP_NET_ADMIN`: Required for STUN/NAT traversal
- `CAP_NET_BIND_SERVICE`: Required for WebSocket binding on privileged ports

Grant capabilities without full root:
```bash
sudo setcap cap_sys_admin,cap_net_admin,cap_net_bind_service+ep /usr/bin/pchome-desktop
```

#### Runtime Verification
```bash
# Check capabilities
getcap /usr/bin/pchome-desktop

# Verify no unnecessary privileges
sudo pchome-desktop --check-permissions

# Audit open file descriptors
ls -la /proc/$(pgrep pchome-desktop)/fd
```

#### Additional Sandboxing Options
- **seccomp**: Filter syscalls to reduce attack surface
- **AppArmor/SELinux**: Mandatory access control policies
- **namespace isolation**: Run in separate PID, network, mount namespaces
- **cgroups**: Resource isolation and limits
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