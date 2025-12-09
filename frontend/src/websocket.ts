interface WebSocketMessage {
  type: string;
  data?: any;
}

interface TerminalMessage extends WebSocketMessage {
  type: 'TerminalInput' | 'TerminalOutput' | 'ResizeTerminal' | 'CreateSession' | 'AttachSession' | 'SessionCreated' | 'SessionAttached' | 'Error';
}

interface CreateSessionMessage extends TerminalMessage {
  type: 'CreateSession';
  data: {
    user_id: string;
    container_image?: string;
  };
}

interface AttachSessionMessage extends TerminalMessage {
  type: 'AttachSession';
  data: {
    session_id: string;
  };
}

interface TerminalInputMessage extends TerminalMessage {
  type: 'TerminalInput';
  data: {
    data: number[]; // Raw bytes as array
  };
}

interface ResizeTerminalMessage extends TerminalMessage {
  type: 'ResizeTerminal';
  data: {
    rows: number;
    cols: number;
  };
}

interface TerminalOutputMessage extends TerminalMessage {
  type: 'TerminalOutput';
  data: {
    data: number[]; // Raw bytes as array
  };
}

interface SessionCreatedMessage extends TerminalMessage {
  type: 'SessionCreated';
  data: {
    session_id: string;
    websocket_url: string;
  };
}

interface SessionAttachedMessage extends TerminalMessage {
  type: 'SessionAttached';
  data: {
    session_id: string;
    container_id: string;
  };
}

interface ErrorMessage extends TerminalMessage {
  type: 'Error';
  data: {
    message: string;
  };
}

type ProtocolMessage = 
  | CreateSessionMessage 
  | AttachSessionMessage 
  | TerminalInputMessage 
  | ResizeTerminalMessage
  | TerminalOutputMessage 
  | SessionCreatedMessage 
  | SessionAttachedMessage 
  | ErrorMessage;

export class SecureWebSocketManager {
  private socket: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private connectionPromise: Promise<void> | null = null;
  private isDestroyed = false;
  private messageQueue: ProtocolMessage[] = [];
  private currentSessionId: string | null = null;

  constructor(
    private url: string,
    private onMessage: (message: ProtocolMessage) => void,
    private onConnect?: () => void,
    private onDisconnect?: () => void,
    private onError?: (error: Event) => void
  ) {}

  async connect(): Promise<void> {
    if (this.connectionPromise) {
      return this.connectionPromise;
    }

    this.connectionPromise = this.createConnection();
    return this.connectionPromise;
  }

  private async createConnection(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.isDestroyed) {
        reject(new Error('WebSocket manager destroyed'));
        return;
      }

      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      const wsUrl = `${protocol}//${host}${this.url}`;

      // Establishing WebSocket connection
      
      this.socket = new WebSocket(wsUrl);

      this.socket.onopen = () => {
        // WebSocket connection established
        this.reconnectAttempts = 0;
        this.connectionPromise = null;
        
        // Flush queued messages
        this.flushMessageQueue();
        
        this.onConnect?.();
        resolve();
      };

      this.socket.onmessage = (event) => {
        try {
          const message: ProtocolMessage = JSON.parse(event.data);
          this.onMessage(message);
        } catch (error) {
          // Failed to parse WebSocket message
        }
      };

      this.socket.onclose = (event) => {
        // WebSocket connection closed
        this.socket = null;
        this.connectionPromise = null;
        
        this.onDisconnect?.();
        
        // Auto-reconnect unless explicitly closed or destroyed
        if (!this.isDestroyed && event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
          this.scheduleReconnect();
        }
        
        resolve(); // Don't reject on close
      };

      this.socket.onerror = (error) => {
        // WebSocket connection error occurred
        this.onError?.(error);
        this.connectionPromise = null;
        reject(error);
      };
    });
  }

  private scheduleReconnect(): void {
    if (this.isDestroyed) return;
    
    this.reconnectAttempts++;
    const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);
    
    // Scheduling WebSocket reconnection
    
    setTimeout(() => {
      if (!this.isDestroyed) {
        this.connect().catch(() => {
          // WebSocket reconnection failed
        });
      }
    }, delay);
  }

  private flushMessageQueue(): void {
    if (this.socket?.readyState === WebSocket.OPEN && this.messageQueue.length > 0) {
      // Flushing queued WebSocket messages
      
      for (const message of this.messageQueue) {
        this.sendMessage(message);
      }
      
      this.messageQueue = [];
    }
  }

  sendMessage(message: ProtocolMessage): void {
    if (this.isDestroyed) {
      // Cannot send message - WebSocket manager destroyed
      return;
    }

    if (this.socket?.readyState === WebSocket.OPEN) {
      try {
        this.socket.send(JSON.stringify(message));
      } catch {
        // Failed to send WebSocket message
      }
    } else {
      // Queue message for later
      this.messageQueue.push(message);
      // WebSocket message queued for later sending
      
      // Try to establish connection if not already connecting
      if (!this.connectionPromise) {
        this.connect().catch(() => {
          // Failed to establish connection for queued message
        });
      }
    }
  }

  async createSession(userId: string, containerImage?: string): Promise<void> {
    const message: CreateSessionMessage = {
      type: 'CreateSession',
      data: {
        user_id: userId,
        container_image: containerImage,
      },
    };

    await this.connect();
    this.sendMessage(message);
  }

  async attachSession(sessionId: string): Promise<void> {
    this.currentSessionId = sessionId;
    
    const message: AttachSessionMessage = {
      type: 'AttachSession',
      data: {
        session_id: sessionId,
      },
    };

    await this.connect();
    this.sendMessage(message);
  }

  sendTerminalInput(data: string): void {
    if (!this.currentSessionId) {
      // Cannot send input - no active session
      return;
    }

    const bytes = new TextEncoder().encode(data);
    const message: TerminalInputMessage = {
      type: 'TerminalInput',
      data: {
        data: Array.from(bytes),
      },
    };

    this.sendMessage(message);
  }

  sendTerminalResize(rows: number, cols: number): void {
    if (!this.currentSessionId) {
      // Cannot resize terminal - no active session
      return;
    }

    const message: ResizeTerminalMessage = {
      type: 'ResizeTerminal',
      data: {
        rows,
        cols,
      },
    };

    this.sendMessage(message);
  }

  isConnected(): boolean {
    return this.socket?.readyState === WebSocket.OPEN;
  }

  getState(): string {
    if (!this.socket) return 'DISCONNECTED';
    
    switch (this.socket.readyState) {
      case WebSocket.CONNECTING: return 'CONNECTING';
      case WebSocket.OPEN: return 'CONNECTED';
      case WebSocket.CLOSING: return 'CLOSING';
      case WebSocket.CLOSED: return 'CLOSED';
      default: return 'UNKNOWN';
    }
  }

  disconnect(): void {
    this.isDestroyed = true;
    this.messageQueue = [];
    this.currentSessionId = null;
    
    if (this.socket) {
      this.socket.close(1000, 'Client disconnect');
      this.socket = null;
    }
    
    this.connectionPromise = null;
  }
}

// Utility function to convert byte array to string
export function bytesToString(bytes: number[]): string {
  return new TextDecoder().decode(new Uint8Array(bytes));
}

// Utility function to convert string to byte array
export function stringToBytes(str: string): number[] {
  return Array.from(new TextEncoder().encode(str));
}

export type {
  ProtocolMessage,
  TerminalMessage,
  TerminalOutputMessage,
  ErrorMessage,
  SessionCreatedMessage,
  SessionAttachedMessage,
};