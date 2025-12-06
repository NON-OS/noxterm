# NØXTERM with love by NØNOS Team.

**A privacy-focused web terminal that runs in isolated containers**

<div align="center">

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD--3--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0+-blue.svg)](https://www.typescriptlang.org/)
[![Docker](https://img.shields.io/badge/Docker-Required-blue.svg)](https://www.docker.com/)
[![Status](https://img.shields.io/badge/Status-Working%20Beta-orange.svg)](#beta-status)

**🚧 This is beta software - expect rough edges and breaking changes 🚧**

[🥷🏻 Try It](#quick-start) • [🤝 Get Involved](#contributing) • [🐛 Report Issues](https://github.com/NON-OS/noxterm/issues)

</div>

---

## What is NØXTERM?

NØXTERM is a web-based terminal that lets you run commands in completely isolated Docker containers. Think of it as a terminal that forgets everything the moment you close it - perfect for testing, learning, or when you need a clean environment.

**Key features:**
- 🔒 **Zero data persistence** - nothing survives after your session ends
- 🐳 **Container isolation** - each session runs in its own Docker container
- 🌐 **Browser-based** - no installation needed, just open a web page
- 🥷🏻 **Multiple environments** - Ubuntu, Alpine, Node.js, and more
- 🔄 **Real-time terminal** - full terminal emulation with copy/paste support

**Perfect for:**
- Testing code without cluttering your system
- Learning Linux commands safely
- Demonstrating software to others
- Quick debugging in a clean environment
- Privacy-sensitive tasks that shouldn't leave traces

## Table of Contents

- [Quick Start](#quick-start)
- [Beta Status](#beta-status)
- [How It Works](#how-it-works)
- [Getting Involved](#getting-involved)
- [Contributing](#contributing)
- [Development Setup](#development-setup)
- [License](#license)

---

## Quick Start

Want to try NØXTERM? Here's how to get it running in about 5 minutes:

### Prerequisites

You'll need:
- **Docker** - [Install Docker](https://docs.docker.com/get-docker/)
- **Rust** - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js** - [Download Node.js](https://nodejs.org/) (version 18 or later)

### Running NØXTERM

1. **Clone the repo:**
   ```bash
   git clone https://github.com/NON-OS/noxterm.git
   cd noxterm
   ```

2. **Start the backend:**
   ```bash
   cd nox-backend
   SERVER_PORT=3001 cargo run --bin noxterm-terminal
   ```
   Wait until you see "Server running on port 3001" (this might take a few minutes on first run)

3. **In a new terminal, start the frontend:**
   ```bash
   cd frontend
   npm install
   npm run dev
   ```

4. **Open your browser** and go to http://localhost:5173

5. **Create a session:**
   - Enter any username
   - Pick an environment (try "Ubuntu 22.04" for starters)
   - Click "Create Terminal Session"
   - Start typing commands!

### Try These Commands
```bash
# See what system you're running
uname -a

# Check available tools
ls /usr/bin | head -20

# Install something (it'll be gone when you close the session)
apt update && apt install -y curl

# Test network access
curl https://httpbin.org/ip
```

## Beta Status

**This is beta software!** Here's what that means:

### ✅ What works well:
- Basic terminal functionality
- Multiple container environments  
- Session isolation and cleanup
- Real-time terminal streaming
- Copy/paste support

### ⚠️ What's still rough:
- Mobile/touch support is limited
- Large file uploads can be slow
- Sessions don't survive server restarts (Intentional on phase 1)
- Error messages could be clearer
- Documentation is still growing

### 🚧 Known issues:
- Not all terminal features work perfectly as today, but is in heavy development.
- Performance isn't optimized yet

**We're actively working on these!** If you hit issues, please [report them](https://github.com/NON-OS/noxterm/issues).

## How It Works

Here's the simple version:

1. **You open NØXTERM in your browser** - it's just a web page
2. **You create a session** - this spins up a fresh Docker container
3. **You get a terminal** - connected directly to that container via WebSocket
4. **You run commands** - everything happens inside the isolated container
5. **You close the session** - the container gets completely destroyed

### Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Browser   │    │  NØXTERM Core   │    │ Docker Engine   │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │    React    │◄├────┤►│    Rust     │◄├────┤►│ Containers  │ │
│ │  Frontend   │ │    │ │  Backend    │ │    │ │             │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │  XTerm.js   │◄├────┤►│  WebSocket  │ │    │ │  Isolation  │ │
│ │  Terminal   │ │    │ │   Handler   │ │    │ │ & Cleanup   │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**Privacy by design:**
- No session data is stored on servers
- Containers are destroyed immediately when sessions end
- Each session is completely isolated from others
- We don't log your commands or activities

---

## Installation

### System Requirements

| Component | Minimum Version | Recommended | Purpose |
|-----------|----------------|-------------|---------|
| **Docker** | 20.10+ | 24.0+ | Container runtime |
| **Rust** | 1.70+ | Latest stable | Backend compilation |
| **Node.js** | 18.0+ | 20.0+ LTS | Frontend development |
| **Memory** | 4GB | 8GB+ | Runtime performance |
| **Storage** | 10GB | 20GB+ | Images and containers |

### Prerequisites Installation

#### Ubuntu/Debian

```bash
# Install Docker
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker $USER
newgrp docker

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Node.js (via NodeSource)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Verify installations
docker --version
rustc --version
node --version
npm --version
```

#### macOS

```bash
# Install via Homebrew
brew install docker rust node

# Start Docker Desktop
open -a Docker

# Verify installations
docker --version
rustc --version
node --version
```

#### Windows (PowerShell)

```powershell
# Install via Chocolatey
choco install docker-desktop rust nodejs

# Verify installations
docker --version
rustc --version
node --version
```

---

## Getting Involved

**Want to help make NØXTERM better?** We'd love to have you! Here are ways you can contribute:

### 🐛 Found a bug?
- [Open an issue](https://github.com/NON-OS/noxterm/issues/new) with details
- Include your OS, browser, and steps to reproduce
- Screenshots help a lot!

### 💡 Have an idea?
- Check [existing issues](https://github.com/NON-OS/noxterm/issues) first
- Open a new issue to discuss your idea
- We love feature requests!

### 👩‍💻 Want to code?
- Check out [good first issues](https://github.com/NON-OS/noxterm/labels/good%20first%20issue)
- Fork the repo and make your changes
- Open a pull request

### 📖 Improve documentation?
- Fix typos in this README
- Add examples or clarify instructions
- Write guides for specific use cases

### 🧪 Help with testing?
- Try NØXTERM on different systems
- Test new features before releases
- Report what works (and what doesn't)

### 💬 Spread the word?
- Star the repo if you find it useful
- Share it with friends who might be interested
- Write about your experience using it

---

## Development Setup

Want to hack on NØXTERM? Here's how to get started:

### Prerequisites Installation

#### Ubuntu/Debian

```bash
# Install Docker
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker $USER
newgrp docker

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Verify everything is working
docker --version && rustc --version && node --version
```

#### macOS

```bash
# Install with Homebrew
brew install docker rust node

# Start Docker Desktop
open -a Docker

# Verify everything is working
docker --version && rustc --version && node --version
```

#### Windows

Use [Docker Desktop](https://docs.docker.com/desktop/windows/install/), [rustup.rs](https://rustup.rs/), and [Node.js installer](https://nodejs.org/).

### Run NØXTERM Locally

```bash
# 1. Clone and enter the repo
git clone https://github.com/NON-OS/noxterm.git
cd noxterm

# 2. Start the backend (in one terminal)
cd nox-backend
SERVER_PORT=3001 cargo run --bin noxterm-terminal

# 3. Start the frontend (in another terminal) 
cd frontend
npm install && npm run dev

# 4. Open http://localhost:5173 in your browser
```

### Development Tips

- Backend changes require restarting `cargo run`
- Frontend has hot-reload, just save your changes
- Check browser dev tools for errors
- Look at `nox-backend/src/main.rs` for backend logic
- Check `frontend/src/components/` for React components

---

## Contributing

Ready to contribute? Awesome! Here's what you need to know:

### Code Style
- **Rust**: We use `cargo fmt` and `cargo clippy` 
- **TypeScript**: We use Prettier and ESLint
- **Git**: Use conventional commits (like `feat:`, `fix:`, `docs:`)

### Before Submitting
```bash
# Backend checks
cd nox-backend
cargo test
cargo clippy
cargo fmt --check

# Frontend checks  
cd frontend
npm run lint
npm run type-check
```

### Pull Request Process
1. Fork the repo
2. Create a feature branch (`git checkout -b feature/cool-thing`)
3. Make your changes
4. Run the checks above
5. Commit and push
6. Open a pull request

We'll review it as soon as we can! Don't worry if it's not perfect - we're happy to help you improve it.

---

## License

NØXTERM is licensed under the [BSD-3-Clause License](https://opensource.org/licenses/BSD-3-Clause).

```
BSD 3-Clause License

Copyright (c) 2024, NON-OS
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

---

## Support

- 🐛 **Report bugs**: [GitHub Issues](https://github.com/NON-OS/noxterm/issues)
- 💬 **Ask questions**: [GitHub Discussions](https://github.com/NON-OS/noxterm/discussions)  
- ⭐ **Star the repo**: If you find NØXTERM useful!

**Remember: This is beta software!** Things will break, change and hopefully get better. Thanks for being part of the journey. 

