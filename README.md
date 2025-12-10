# NOXTERM

**Privacy-First Web Terminal with Isolated Container Execution**

<div align="center">

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD--3--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue.svg)](https://www.typescriptlang.org/)
[![Docker](https://img.shields.io/badge/Docker-Required-blue.svg)](https://www.docker.com/)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)](#cross-platform-support)

[Features](#features) | [Quick Start](#quick-start) | [Documentation](#documentation) | [Contributing](#contributing)

</div>

---

## Overview

NOXTERM is a web-based terminal that executes commands in completely isolated Docker containers. Each session runs in a fresh, ephemeral environment that is destroyed the moment you disconnect—leaving no traces behind.

Built with a Rust backend for performance and reliability, React frontend with xterm.js for terminal emulation, and optional privacy routing through the Anyone Protocol.

## Features

### Core Capabilities

- **Complete Session Isolation** — Each terminal session runs in its own Docker container with no persistence
- **Full PTY Support** — Native pseudo-terminal with support for interactive applications (nano, vim, htop)
- **Real-Time Terminal** — WebSocket-based communication with full terminal emulation
- **Multiple Environments** — Ubuntu, Alpine, Debian, Node.js, Python, and custom images
- **Cross-Platform** — Native support for macOS, Linux, and Windows

### Privacy & Security

- **Zero Data Persistence** — All session data is destroyed on disconnect
- **Container Sandboxing** — Complete process and filesystem isolation
- **Optional Anonymous Routing** — Anyone Protocol integration for network privacy
- **No Activity Logging** — Commands and outputs are never stored

### Developer Experience

- **Auto-Setup** — Automatic Docker and Node.js installation on all platforms
- **Hot Reload** — Frontend development with instant updates
- **Configurable** — Environment variables for all deployment scenarios
- **API Access** — RESTful endpoints for programmatic control

---

## Quick Start

### Prerequisites

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Docker | 20.10+ | 24.0+ |
| Rust | 1.70+ | Latest stable |
| Node.js | 18.0+ | 20.0+ LTS |

> NOXTERM can automatically install Docker and Node.js if they're not present. See [Auto-Setup](#auto-setup).

### Installation

```bash
# Clone the repository
git clone https://github.com/NON-OS/noxterm.git
cd noxterm

# Start the backend
cd nox-backend
cargo run --bin noxterm-backend

# In a new terminal, start the frontend
cd frontend
npm install
npm run dev
```

Open http://localhost:5173 in your browser.

### First Session

1. Enter any username
2. Select an environment (Ubuntu 22.04 recommended for beginners)
3. Click "Create Terminal Session"
4. Start running commands

```bash
# Verify your environment
uname -a

# Install packages (they won't persist after session ends)
apt update && apt install -y curl htop

# Test network
curl https://httpbin.org/ip
```

---

## Documentation

### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Browser   │    │  NOXTERM Core   │    │  Docker Engine  │
│                 │    │                 │    │                 │
│  React + xterm  │◄──►│  Rust Backend   │◄──►│   Containers    │
│                 │    │                 │    │                 │
│    WebSocket    │    │  Session Mgmt   │    │   Isolation     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Data Flow:**
1. Browser connects via WebSocket to Rust backend
2. Backend creates isolated Docker container
3. PTY session established with container shell
4. All I/O streams through WebSocket in real-time
5. Container destroyed on session close

### Cross-Platform Support

NOXTERM automatically detects and configures Docker on all platforms:

| Platform | Docker Runtime | Auto-Detection |
|----------|---------------|----------------|
| **macOS** | Docker Desktop, Colima, OrbStack | Socket paths auto-detected |
| **Linux** | Docker Engine, Podman | System service integration |
| **Windows** | Docker Desktop, WSL2 | Named pipe connection |

### Auto-Setup

When dependencies are missing, NOXTERM attempts automatic installation:

**Docker Installation:**
- macOS: Homebrew (`brew install --cask docker`) or Colima
- Linux: Official Docker script or package managers (apt, dnf, yum, pacman)
- Windows: winget, Chocolatey, or Scoop

**Node.js Installation (for Privacy Mode):**
- macOS: Homebrew or nvm
- Linux: NodeSource, package managers, or nvm
- Windows: winget, Chocolatey, or Scoop

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_HOST` | `0.0.0.0` | Backend bind address |
| `SERVER_PORT` | `3001` | Backend port |
| `ENVIRONMENT` | `development` | Environment name |
| `DOCKER_HOST` | auto-detect | Docker socket path |
| `RUST_LOG` | `noxterm=info` | Log level |

### Privacy Mode (Anyone Protocol)

Enable anonymous network routing through the Anyone Protocol:

```bash
# Enable via API
curl -X POST http://localhost:3001/api/privacy/enable

# Check status
curl http://localhost:3001/api/privacy/status

# Disable
curl -X POST http://localhost:3001/api/privacy/disable
```

When enabled:
- Container traffic routes through decentralized network
- Your IP is hidden from destination services
- SOCKS5 proxy available on port 9050

---

## Development

### Project Structure

```
noxterm/
├── nox-backend/
│   ├── src/
│   │   ├── noxterm.rs        # Main backend with WebSocket handlers
│   │   ├── anyone_service.rs # Privacy mode service manager
│   │   └── lib.rs            # Library exports
│   └── Cargo.toml
├── frontend/
│   ├── src/
│   │   ├── components/
│   │   │   └── NoxTerminal.tsx  # Terminal component
│   │   └── App.tsx
│   └── package.json
└── README.md
```

### Building for Production

```bash
# Backend
cd nox-backend
cargo build --release
# Binary: target/release/noxterm-backend

# Frontend
cd frontend
npm run build
# Static files: dist/
```

### Code Quality

```bash
# Backend
cargo test
cargo clippy
cargo fmt --check

# Frontend
npm run lint
npm run type-check
```

---

## Roadmap

### Completed

- [x] Cross-platform Docker auto-detection (macOS, Linux, Windows)
- [x] Cross-platform Docker auto-installation
- [x] Cross-platform Node.js auto-installation for Anyone SDK
- [x] Full PTY mode with interactive application support
- [x] Terminal resize handling
- [x] Multiple container environment support
- [x] WebSocket-based real-time terminal
- [x] Session isolation and cleanup
- [x] Anyone Protocol privacy integration
- [x] Copy/paste support in terminal

### In Progress

- [ ] Mobile/touch interface improvements
- [ ] File upload/download support
- [ ] Session reconnection after network drops

### Planned

- [ ] Multi-user session sharing
- [ ] Custom container image support via UI
- [ ] Session recording and playback
- [ ] Kubernetes backend option
- [ ] Plugin system for extensions

---

## Contributing

We welcome contributions of all kinds.

### Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Run tests: `cargo test && npm run lint`
5. Commit with conventional format: `feat: add feature`
6. Open a pull request

### Commit Convention

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Test additions or changes
- `chore:` Build/tooling changes

---

## License

BSD 3-Clause License. See [LICENSE](LICENSE) for details.

```
Copyright (c) 2024, NON-OS
All rights reserved.
```

---

## Support

- [Report Issues](https://github.com/NON-OS/noxterm/issues)
- [Discussions](https://github.com/NON-OS/noxterm/discussions)

Built by the NONOS Team.
