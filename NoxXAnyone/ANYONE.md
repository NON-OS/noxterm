# NOXTERM 4 Anyone Protocol 

## Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   User Browser  │    │  Anyone Network │    │  NØXTERM Core   │    │ Docker Engine   │
│                 │    │                 │    │                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │   React     │◄├────┤►│   Onion     │◄├────┤►│    Rust     │◄├────┤►│ Containers  │ │
│ │  Frontend   │ │    │ │  Circuits   │ │    │ │   Backend   │ │    │ │ +Anon Proxy │ │
│ │ +AnonClient │ │    │ └─────────────┘ │    │ │ +AnonServer │ │    │ └─────────────┘ │
│ └─────────────┘ │    │                 │    │ └─────────────┘ │    │                 │
│                 │    │ ┌─────────────┐ │    │ ┌─────────────┐ │    │ ┌─────────────┐ │
│ Privacy Toggle  │    │ │   SOCKS5    │ │    │ │  WebSocket  │ │    │ │  Isolated   │ │
│ Circuit Status  │    │ │   Proxy     │ │    │ │  via Anon   │ │    │ │  Networking │ │
│ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
```

NOXTERM integrates Anyone Protocol SDK to provide onion network privacy protection for containerized terminal sessions. The integration consists of three main components:

### 1. Backend Privacy Service
- **Location**: `src/anyone_service.rs`
- **Purpose**: Manages Anyone Protocol client lifecycle and SOCKS proxy
- **Features**: 
  - Automatic Anyone client installation via npm
  - SOCKS5 proxy on port 9050, control port 9051
  - Service management with error handling
  - Status monitoring and graceful shutdown

### 2. Privacy Control API
- **Endpoints**:
  - `POST /api/privacy/enable` - Start Anyone network
  - `POST /api/privacy/disable` - Stop Anyone network  
  - `GET /api/privacy/status` - Check privacy status
- **Integration**: Rust backend with full async/await support

### 3. Frontend Privacy Controls
- **Location**: `src/components/PrivacyControls.tsx`
- **Features**: 
  - Real-time circuit status monitoring
  - One-click privacy toggle
  - Visual indicators for anonymous mode
  - Anonymous API routing when privacy enabled

## Network Flow

1. **Standard Mode**: Client → Backend → Docker Container
2. **Privacy Mode**: Client → Anyone Network → Backend → Docker Container

When privacy mode is enabled:
- Frontend API requests route through Anyone SOCKS proxy
- WebSocket connections maintain privacy through backend proxy
- All network traffic flows through onion routing

## Security Features

- Container isolation with resource limits
- No-new-privileges security profiles  
- Automatic cleanup of proxy processes
- Error handling and logging
