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

    // Welcome message
    terminal.current.clear();
    terminal.current.writeln('🚀 NØXTERM - Privacy-First Containerized Terminal');
    terminal.current.writeln('================================================');
    terminal.current.writeln('🌟 Open Source Project by NØNOS Team');
    terminal.current.writeln('🌐 Website: https://nonos.systems');
    terminal.current.writeln('📦 GitHub: https://github.com/NON-OS/noxterm');
    terminal.current.writeln('📧 Contact: team@nonos.systems');
    terminal.current.writeln('🛡️  Security: security@nonos.systems');
    terminal.current.writeln('');
    terminal.current.writeln('🔒 Security Hardened:');
    terminal.current.writeln('   • Containerized isolation • Privacy-first design');
    terminal.current.writeln('   • Zero data retention • Encrypted communications');
    terminal.current.writeln('   • See SECURITY.md for full security documentation');
    terminal.current.writeln('');
    terminal.current.writeln('💎 Support Development:');
    terminal.current.writeln('   ETH/ERC-20: 0x23EEF7121705C90cfa01474736D0645c1eEB21ac');
    terminal.current.writeln('   XMR: 427RJHzQCGXUogwDcqbQNaYY5aUQ8e9B712FAvdQSt4c9PuUbX9kS9wZXz4AmT95fzMbdRfZuE3Wm43ekCRM1GYZ9Mdib2z');
    terminal.current.writeln('');
    terminal.current.writeln('================================================');
    terminal.current.writeln(`🐳 Container: ${containerImage}`);
    terminal.current.writeln(`👤 User: ${userId}`);
    terminal.current.writeln(`🔑 Session: ${sessionId.slice(0, 8)}...`);
    terminal.current.writeln('');
    terminal.current.writeln('✨ Privacy-enhanced terminal ready! Type commands below.');
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
    terminal.current?.writeln('🔌 Connecting to PTY backend...');

    const wsUrl = usePtyMode ? 
      `ws://localhost:3001/pty/${sessionId}` : 
      `ws://localhost:3001/ws/${sessionId}`;
    
    socket.current = new WebSocket(wsUrl);

    // Set binary type for PTY mode - required for proper binary data handling
    if (usePtyMode) {
      socket.current.binaryType = 'arraybuffer';
    }

    socket.current.onopen = () => {
      setStatus('connected');
      terminal.current?.writeln('✅ Connected to NOXTERM backend');
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
      terminal.current?.writeln('\r\n❌ Connection lost');
    };

    socket.current.onerror = () => {
      setStatus('error');
      terminal.current?.writeln('\r\n❌ Connection error');
    };
  };

  const handleBackendMessage = (message: any) => {
    switch (message.type) {
      case 'container_ready':
        terminal.current?.writeln('📦 Container is ready!');
        terminal.current?.write('\r\n$ ');
        break;
        
      case 'ready':
        terminal.current?.writeln('✅ System ready! Type commands below.');
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
        terminal.current?.writeln(`\r\n❌ Error: ${message.error}`);
        terminal.current?.write('\r\n$ ');
        break;

      case 'error':
        terminal.current?.writeln(`\r\n🚨 System error: ${message.message}`);
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
      terminal.current?.writeln('❌ Not connected to backend');
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
      case 'connected': return 'text-green-400';
      case 'connecting': return 'text-yellow-400';
      case 'error': return 'text-red-400';
      case 'disconnected': return 'text-gray-400';
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-black overflow-hidden">
      {/* Status bar */}
      <div className="bg-gray-900 px-4 py-2 flex justify-between items-center text-sm border-b border-gray-700 flex-shrink-0">
        <div className={`flex items-center space-x-2 ${getStatusColor()}`}>
          <span>●</span>
          <span className="capitalize font-mono">{status}</span>
          <span className="text-gray-400">| NOXTERM v1.0 Open Source</span>
          <button 
            onClick={togglePtyMode}
            className={`ml-2 px-3 py-1 text-xs font-mono rounded border transition-all ${
              usePtyMode 
                ? 'bg-green-600 text-white border-green-500 shadow-lg shadow-green-500/20' 
                : 'bg-gray-700 text-gray-300 border-gray-600 hover:bg-gray-600'
            }`}
            title={usePtyMode ? 'PTY Mode: Full terminal emulation' : 'CMD Mode: Command-based interaction'}
          >
            {usePtyMode ? '🔧 PTY' : '💻 CMD'}
          </button>
        </div>
        <div className="flex items-center space-x-4">
          {/* Project Links */}
          <div className="flex items-center space-x-2 text-xs">
            <a href="https://nonos.systems" target="_blank" rel="noopener noreferrer" 
               className="text-blue-400 hover:text-blue-300 font-mono flex items-center space-x-1">
              <span>🌐</span><span>nonos.systems</span>
            </a>
            <a href="https://github.com/NON-OS/noxterm" target="_blank" rel="noopener noreferrer" 
               className="text-green-400 hover:text-green-300 font-mono flex items-center space-x-1">
               <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                 <path fillRule="evenodd" d="M10 0C4.477 0 0 4.484 0 10.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0110 4.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.203 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.942.359.31.678.921.678 1.856 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0020 10.017C20 4.484 15.522 0 10 0z"/>
               </svg>
               <span>GitHub</span>
            </a>
            <a href="mailto:team@nonos.systems" 
               className="text-yellow-400 hover:text-yellow-300 font-mono flex items-center space-x-1">
              <span>📧</span><span>Contact</span>
            </a>
          </div>
          {status !== 'connected' && (
            <button 
              onClick={reconnect}
              className="px-3 py-1 bg-blue-600 text-white rounded text-xs hover:bg-blue-700 font-mono"
            >
              RECONNECT
            </button>
          )}
          <span className="text-gray-500 font-mono">ID: {sessionId.slice(0, 8)}</span>
        </div>
      </div>
      
      {/* Terminal - Full screen */}
      <div className="flex-1 overflow-hidden">
        <div 
          ref={terminalRef}
          className="w-full h-full bg-black"
        />
      </div>

      {/* Professional Footer with Donations */}
      <div className="bg-gray-900 px-4 py-2 border-t border-gray-700 flex-shrink-0">
        <div className="flex justify-between items-center text-xs">
          <div className="flex items-center space-x-4 text-gray-400">
            <span className="font-mono">🚀 NOXTERM - Secure Containerized Terminal</span>
            <span>•</span>
            <span>Enterprise-Ready Terminal Solution</span>
          </div>
          
          {/* Donation Section */}
          <div className="flex items-center space-x-4">
            <span className="text-gray-500 font-mono">Support Development:</span>
            <div className="flex items-center space-x-3">
              <div className="group relative">
                <button className="text-purple-400 hover:text-purple-300 font-mono text-xs px-2 py-1 border border-purple-600 rounded flex items-center space-x-1">
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
                <div className="absolute bottom-full right-0 mb-2 hidden group-hover:block bg-gray-800 text-white text-xs p-2 rounded shadow-lg border border-gray-600 w-80">
                  <div className="font-mono break-all">
                    0x23EEF7121705C90cfa01474736D0645c1eEB21ac
                  </div>
                  <div className="text-gray-400 mt-1">ERC-20 Donation Address</div>
                </div>
              </div>
              
              <div className="group relative">
                <button className="text-orange-400 hover:text-orange-300 font-mono text-xs px-2 py-1 border border-orange-600 rounded flex items-center space-x-1">
                  <svg className="w-3 h-3" viewBox="0 0 256 256" fill="currentColor">
                    <path d="M128 0C57.3 0 0 57.3 0 128s57.3 128 128 128 128-57.3 128-128S198.7 0 128 0zm0 240c-61.9 0-112-50.1-112-112S66.1 16 128 16s112 50.1 112 112-50.1 112-112 112z"/>
                    <circle cx="128" cy="128" r="64"/>
                  </svg>
                  <span>XMR</span>
                </button>
                <div className="absolute bottom-full right-0 mb-2 hidden group-hover:block bg-gray-800 text-white text-xs p-2 rounded shadow-lg border border-gray-600 w-96">
                  <div className="font-mono break-all text-xs">
                    427RJHzQCGXUogwDcqbQNaYY5aUQ8e9B712FAvdQSt4c9PuUbX9kS9wZXz4AmT95fzMbdRfZuE3Wm43ekCRM1GYZ9Mdib2z
                  </div>
                  <div className="text-gray-400 mt-1">Monero Donation Address</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};