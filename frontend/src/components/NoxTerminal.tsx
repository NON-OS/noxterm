import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import 'xterm/css/xterm.css';

interface NoxTerminalProps {
  sessionId: string;
  userId: string;
  containerImage?: string;
}

export const NoxTerminal: React.FC<NoxTerminalProps> = ({ 
  sessionId, 
  userId, 
  containerImage = 'ubuntu:22.04' 
}) => {
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const fitAddon = useRef<FitAddon | null>(null);
  const socket = useRef<WebSocket | null>(null);
  const [status, setStatus] = useState<'connecting' | 'connected' | 'error' | 'disconnected'>('connecting');
  const [usePtyMode, setUsePtyMode] = useState(true); // PTY mode by default for full editor support

  useEffect(() => {
    if (!terminalRef.current || terminal.current) return;
    
    
    terminal.current = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      theme: {
        background: '#000000',
        foreground: '#ffffff',
        cursor: '#ffffff',
        cursorAccent: '#000000',
        selectionBackground: 'rgba(255, 255, 255, 0.3)',
      },
      fontSize: 14,
      fontFamily: 'Monaco, Menlo, "Ubuntu Mono", "Consolas", monospace',
      convertEol: false,
      scrollback: 10000,
      allowTransparency: false,
      disableStdin: false,
      cols: 80,
      rows: 24,
      // PTY mode settings - critical for editor support
      windowsMode: false,
      macOptionIsMeta: true, // Use Option as Meta key on macOS
      macOptionClickForcesSelection: true,
      rightClickSelectsWord: true,
      // Allow all terminal escape sequences through
      allowProposedApi: true,
    });

    fitAddon.current = new FitAddon();
    terminal.current.loadAddon(fitAddon.current);
    terminal.current.loadAddon(new WebLinksAddon());

    terminal.current.open(terminalRef.current);
    
    // Initial fit
    setTimeout(() => {
      fitAddon.current?.fit();
    }, 100);

    // Handle window resize
    const handleResize = () => {
      fitAddon.current?.fit();
    };
    
    window.addEventListener('resize', handleResize);
    
    // Cleanup resize listener
    const cleanup = () => {
      window.removeEventListener('resize', handleResize);
    };

    // Welcome message - clean, professional NON-OS branding
    terminal.current.clear();
    terminal.current.writeln('\x1b[36m┌─────────────────────────────────────────────────────────────┐\x1b[0m');
    terminal.current.writeln('\x1b[36m│\x1b[0m  \x1b[1;37mNØXTERM\x1b[0m \x1b[36m- Privacy-First Terminal\x1b[0m                           \x1b[36m│\x1b[0m');
    terminal.current.writeln('\x1b[36m│\x1b[0m  \x1b[90mBy NON-OS Systems\x1b[0m                                           \x1b[36m│\x1b[0m');
    terminal.current.writeln('\x1b[36m└─────────────────────────────────────────────────────────────┘\x1b[0m');
    terminal.current.writeln('');
    terminal.current.writeln('\x1b[90m  nonos.systems | github.com/NON-OS/noxterm\x1b[0m');
    terminal.current.writeln('');
    terminal.current.writeln('\x1b[36m  Container:\x1b[0m ' + containerImage);
    terminal.current.writeln('\x1b[36m  User:\x1b[0m      ' + userId);
    terminal.current.writeln('\x1b[36m  Session:\x1b[0m   ' + sessionId.slice(0, 8) + '...');
    terminal.current.writeln('');
    terminal.current.writeln('\x1b[90m  Containerized isolation | Zero data retention | Encrypted\x1b[0m');
    terminal.current.writeln('');
    terminal.current.writeln('\x1b[90m  Support Development:\x1b[0m');
    terminal.current.writeln('\x1b[90m  ETH:\x1b[0m \x1b[36m0x23EEF7121705C90cfa01474736D0645c1eEB21ac\x1b[0m');
    terminal.current.writeln('\x1b[90m  XMR:\x1b[0m \x1b[36m427RJHzQCGXUogwDcqbQNaYY5aUQ8e9B712FAvdQSt4c9PuUbX9kS9wZXz4AmT95fzMbdRfZuE3Wm43ekCRM1GYZ9Mdib2z\x1b[0m');
    terminal.current.writeln('');
    
    // Connect immediately
    connectToBackend();

    return () => {
      cleanup();
      if (socket.current) {
        socket.current.close();
      }
      if (terminal.current) {
        terminal.current.dispose();
      }
    };
  }, []);

  const connectToBackend = () => {
    // Connecting to backend
    setStatus('connecting');
    terminal.current?.writeln('\x1b[36m  Establishing connection...\x1b[0m');
    
    // Fixed to use current host for WebSocket connection **Community Feedback Robert**
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.hostname;
    const port = window.location.port || (window.location.protocol === 'https:' ? '443' : '3001');
    const wsUrl = usePtyMode ?
      `${protocol}//${host}:${port}/pty/${sessionId}` :
      `${protocol}//${host}:${port}/ws/${sessionId}`;
    
    socket.current = new WebSocket(wsUrl);

    // Set binary type for PTY mode - required for proper binary data handling
    if (usePtyMode) {
      socket.current.binaryType = 'arraybuffer';
    }

    socket.current.onopen = () => {
      setStatus('connected');
      terminal.current?.writeln('\x1b[32m  Connected\x1b[0m');
      terminal.current?.writeln('');
      if (usePtyMode) {
        setupPtyInput();
      } else {
        setupTerminalInput();
      }
    };

    socket.current.onmessage = async (event) => {
      if (usePtyMode) {
        // PTY mode - handle raw terminal output
        if (event.data instanceof ArrayBuffer) {
          // Binary data from backend - write directly to terminal
          const uint8Array = new Uint8Array(event.data);
          terminal.current?.write(uint8Array);
        } else if (event.data instanceof Blob) {
          // Handle Blob data (fallback for some browsers)
          const arrayBuffer = await event.data.arrayBuffer();
          const uint8Array = new Uint8Array(arrayBuffer);
          terminal.current?.write(uint8Array);
        } else if (typeof event.data === 'string') {
          // Text data - could be JSON control message or raw terminal output
          if (event.data.startsWith('{')) {
            try {
              const message = JSON.parse(event.data);
              if (message.type === 'pty_output') {
                terminal.current?.write(message.data);
              } else if (message.type === 'exit_interactive') {
                (window as any).setInteractiveMode?.(false);
              }
              return;
            } catch {
              // Not JSON, write as raw output
            }
          }
          // Raw terminal output string
          terminal.current?.write(event.data);
        }
      } else {
        // Command mode - handle structured messages
        try {
          const message = JSON.parse(event.data);
          handleBackendMessage(message);
        } catch {
          // Fallback to raw output
          terminal.current?.write(event.data);
        }
      }
    };

    socket.current.onclose = () => {
      setStatus('disconnected');
      terminal.current?.writeln('\r\n\x1b[31mConnection lost\x1b[0m');
    };

    socket.current.onerror = () => {
      setStatus('error');
      terminal.current?.writeln('\r\n\x1b[31mConnection error\x1b[0m');
    };
  };

  const handleBackendMessage = (message: any) => {
    switch (message.type) {
      case 'container_ready':
        terminal.current?.writeln('\x1b[32mContainer ready\x1b[0m');
        terminal.current?.write('\r\n$ ');
        break;

      case 'ready':
        terminal.current?.writeln('\x1b[32mReady\x1b[0m');
        terminal.current?.write('\r\n$ ');
        break;

      case 'command_output':
        const output = message.output || '';
        if (output.trim()) {
          terminal.current?.writeln('\r\n' + output.replace(/\n$/, ''));
        }
        terminal.current?.write('\r\n$ ');
        break;

      case 'command_error':
        terminal.current?.writeln(`\r\n\x1b[31mError: ${message.error}\x1b[0m`);
        terminal.current?.write('\r\n$ ');
        break;

      case 'error':
        terminal.current?.writeln(`\r\n\x1b[31mSystem error: ${message.message}\x1b[0m`);
        break;

      default:
        break;
    }
  };

  const setupPtyInput = () => {
    if (!terminal.current) return;

    // Send raw terminal data as text - xterm gives us strings with escape sequences
    // The backend handles this correctly as UTF-8
    terminal.current.onData((data) => {
      if (!socket.current || socket.current.readyState !== WebSocket.OPEN) return;

      // Send as text - the string contains all escape sequences (Ctrl+X = \x18, etc.)
      // Using text preserves UTF-8 encoding and control characters
      socket.current.send(data);
    });

    // Handle terminal resize - send resize command to backend
    terminal.current.onResize(({ cols, rows }) => {
      if (!socket.current || socket.current.readyState !== WebSocket.OPEN) return;

      // Send resize command as JSON text
      const resizeMsg = JSON.stringify({ resize: [cols, rows] });
      socket.current.send(resizeMsg);
    });

    // Trigger initial resize after connection
    setTimeout(() => {
      if (terminal.current && socket.current?.readyState === WebSocket.OPEN) {
        const dims = fitAddon.current?.proposeDimensions();
        if (dims) {
          const resizeMsg = JSON.stringify({ resize: [dims.cols, dims.rows] });
          socket.current.send(resizeMsg);
        }
      }
    }, 500);
  };

  const setupTerminalInput = () => {
    if (!terminal.current) return;

    let currentCommand = '';
    let inInteractiveMode = false;

    terminal.current.onData((data) => {
      const code = data.charCodeAt(0);

      if (inInteractiveMode) {
        executeCommand('\x1B[raw]' + data);
        return;
      }

      if (code < 32 && code !== 13 && code !== 8) {
        if (code === 3) {
          terminal.current?.write('^C\r\n$ ');
          currentCommand = '';
          return;
        } else if (code === 26) {
          terminal.current?.write('^Z\r\n$ ');
          currentCommand = '';
          return;
        }
        return;
      }

      if (code === 13) {
        terminal.current?.write('\r\n');
        
        if (currentCommand.trim()) {
          const cmd = currentCommand.trim();
          if (cmd === 'nano' || cmd.startsWith('nano ') || cmd === 'vim' || cmd.startsWith('vim ') || cmd === 'htop') {
            inInteractiveMode = true;
          }
          executeCommand(cmd);
        } else {
          terminal.current?.write('$ ');
        }
        currentCommand = '';
        
      } else if (code === 127 || code === 8) {
        if (currentCommand.length > 0) {
          currentCommand = currentCommand.slice(0, -1);
          terminal.current?.write('\b \b');
        }
        
      } else if (data >= ' ') {
        if (!inInteractiveMode) {
          currentCommand += data;
          terminal.current?.write(data);
        }
      }
    });
  };

  const executeCommand = (command: string) => {
    if (!socket.current || socket.current.readyState !== WebSocket.OPEN) {
      terminal.current?.writeln('\x1b[31mNot connected\x1b[0m');
      terminal.current?.write('$ ');
      return;
    }

    socket.current.send(command);
  };

  const togglePtyMode = useCallback(() => {
    if (socket.current) {
      socket.current.close();
    }
    setUsePtyMode(!usePtyMode);
    setTimeout(connectToBackend, 100);
  }, [usePtyMode]);

  const reconnect = () => {
    if (socket.current) {
      socket.current.close();
    }
    socket.current = null;
    connectToBackend();
  };

  const getStatusColor = () => {
    switch (status) {
      case 'connected': return 'text-[#66FFFF]';
      case 'connecting': return 'text-[#FFB366]';
      case 'error': return 'text-[#FF6666]';
      case 'disconnected': return 'text-gray-500';
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-black overflow-hidden">
      {/* Status bar */}
      <div className="bg-[#0a0a0a] px-4 py-2 flex justify-between items-center text-sm border-b border-[rgba(102,255,255,0.1)] flex-shrink-0">
        <div className={`flex items-center space-x-2 ${getStatusColor()}`}>
          <span className="w-2 h-2 rounded-full bg-current"></span>
          <span className="capitalize font-mono">{status}</span>
          <span className="text-gray-600">|</span>
          <span className="text-gray-500 font-mono">NØXTERM</span>
          <button
            onClick={togglePtyMode}
            className={`ml-2 px-3 py-1 text-xs font-mono rounded border transition-all ${
              usePtyMode
                ? 'bg-[#2E5C5C] text-[#66FFFF] border-[#66FFFF]/30'
                : 'bg-[#111] text-gray-400 border-gray-700 hover:border-gray-600'
            }`}
            title={usePtyMode ? 'PTY Mode: Full terminal emulation' : 'CMD Mode: Command-based interaction'}
          >
            {usePtyMode ? 'PTY' : 'CMD'}
          </button>
        </div>
        <div className="flex items-center space-x-4">
          {/* Project Links */}
          <div className="flex items-center space-x-3 text-xs">
            <a href="https://nonos.systems" target="_blank" rel="noopener noreferrer"
               className="text-[#66FFFF] hover:text-white font-mono transition-colors">
              nonos.systems
            </a>
            <a href="https://github.com/NON-OS/noxterm" target="_blank" rel="noopener noreferrer"
               className="text-gray-500 hover:text-[#66FFFF] font-mono flex items-center space-x-1 transition-colors">
               <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                 <path fillRule="evenodd" d="M10 0C4.477 0 0 4.484 0 10.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0110 4.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.203 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.942.359.31.678.921.678 1.856 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0020 10.017C20 4.484 15.522 0 10 0z"/>
               </svg>
               <span>GitHub</span>
            </a>
          </div>
          {status !== 'connected' && (
            <button
              onClick={reconnect}
              className="px-3 py-1 bg-[#2E5C5C] text-[#66FFFF] rounded text-xs hover:bg-[#3a7575] font-mono transition-colors"
            >
              RECONNECT
            </button>
          )}
          <span className="text-gray-600 font-mono">{sessionId.slice(0, 8)}</span>
        </div>
      </div>
      
      {/* Terminal - Full screen */}
      <div className="flex-1 overflow-hidden">
        <div 
          ref={terminalRef}
          className="w-full h-full bg-black"
        />
      </div>

      {/* Footer */}
      <div className="bg-[#0a0a0a] px-4 py-2 border-t border-[rgba(102,255,255,0.1)] flex-shrink-0">
        <div className="flex justify-between items-center text-xs">
          <div className="flex items-center space-x-4 text-gray-600">
            <span className="font-mono">NØXTERM</span>
            <span className="text-gray-700">|</span>
            <span>Privacy-First Terminal</span>
          </div>

          {/* Support Section */}
          <div className="flex items-center space-x-4">
            <span className="text-gray-700 font-mono">Support:</span>
            <div className="flex items-center space-x-3">
              <div className="group relative">
                <button className="text-[#66FFFF]/60 hover:text-[#66FFFF] font-mono text-xs px-2 py-1 border border-[rgba(102,255,255,0.2)] rounded flex items-center space-x-1 transition-colors">
                  <svg className="w-3 h-3" viewBox="0 0 256 417" fill="currentColor">
                    <path d="M127.961 0l-2.795 9.5v275.668l2.795 2.79 127.962-75.638z"/>
                    <path d="M127.962 0L0 212.32l127.962 75.639V154.158z" opacity="0.6"/>
                    <path d="M127.961 312.187l-1.575 1.92v98.199l1.575 4.6L256 236.587z"/>
                    <path d="M127.962 416.905v-104.72L0 236.585z" opacity="0.6"/>
                    <path d="M127.961 287.958l127.96-75.637-127.96-58.162z" opacity="0.2"/>
                    <path d="M0 212.32l127.96 75.638v-133.8z" opacity="0.6"/>
                  </svg>
                  <span>ETH</span>
                </button>
                <div className="absolute bottom-full right-0 mb-2 hidden group-hover:block bg-[#0d0d0d] text-white text-xs p-3 rounded-lg shadow-lg border border-[rgba(102,255,255,0.2)] w-80">
                  <div className="font-mono break-all text-[#66FFFF]">
                    0x23EEF7121705C90cfa01474736D0645c1eEB21ac
                  </div>
                  <div className="text-gray-500 mt-2">ERC-20 Donation Address</div>
                </div>
              </div>

              <div className="group relative">
                <button className="text-[#66FFFF]/60 hover:text-[#66FFFF] font-mono text-xs px-2 py-1 border border-[rgba(102,255,255,0.2)] rounded flex items-center space-x-1 transition-colors">
                  <svg className="w-3 h-3" viewBox="0 0 256 256" fill="currentColor">
                    <path d="M128 0C57.3 0 0 57.3 0 128s57.3 128 128 128 128-57.3 128-128S198.7 0 128 0zm0 240c-61.9 0-112-50.1-112-112S66.1 16 128 16s112 50.1 112 112-50.1 112-112 112z"/>
                    <circle cx="128" cy="128" r="64"/>
                  </svg>
                  <span>XMR</span>
                </button>
                <div className="absolute bottom-full right-0 mb-2 hidden group-hover:block bg-[#0d0d0d] text-white text-xs p-3 rounded-lg shadow-lg border border-[rgba(102,255,255,0.2)] w-96">
                  <div className="font-mono break-all text-[#66FFFF] text-xs">
                    427RJHzQCGXUogwDcqbQNaYY5aUQ8e9B712FAvdQSt4c9PuUbX9kS9wZXz4AmT95fzMbdRfZuE3Wm43ekCRM1GYZ9Mdib2z
                  </div>
                  <div className="text-gray-500 mt-2">Monero Donation Address</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
