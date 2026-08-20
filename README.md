# PChome - Next-Gen Remote Control System

## Overview

PChome is a high-performance, low-latency (<40ms) cross-platform remote control and display mirroring ecosystem designed for Linux and Android. The project operates as a monorepo consisting of three decoupled yet seamlessly integrated modules:

1. **PChome Desktop** - Linux Daemon + GUI written in Rust
2. **PChome Mobile** - Android Native application written in Java
3. **PChome Signal** - P2P Handshake Server written in Go

## Quick Start

### Prerequisites

- Linux system with PipeWire support
- Android device with MediaProjection support
- Go 1.21+ for Signal server
- Rust 1.70+ for Desktop client
- Java JDK 17+ for Mobile client

### Building the Signal Server

```bash
cd pchome-signal
go run main.go
```

### Building the Desktop Client

```bash
cd pchome-desktop
# Follow BUILD.md for compilation instructions
```

### Building the Mobile Client

```bash
cd pchome-mobile
# Follow PERMISSIONS.md for setup instructions
```

## Color Palette

- **Primary Background**: `#090C10` (Deep Obsidian)
- **Card/Container**: `#0D1117`
- **Primary Accent**: `#00F4FF` (Cyber Neon Cyan)
- **Secondary Accent**: `#0A84FF` (Electric Blue)
- **Primary Text**: `#FFFFFF`
- **Subtle UI Borders**: `#30363D`
- **Error/Alert**: `#FF2A55` (Neon Red)

## Authentication

- 6-digit cryptographic PIN generated per session
- Valid for 300 seconds (TTL)
- Registered via WebSockets to Signal server
- PIN format: e.g., `849-204`