// Core session types for NØXTERM
export interface Session {
  id: string;
  user_id: string;
  container_id: string | null;
  container_image: string;
  status: SessionStatus;
  created_at: string;
}

export type SessionStatus = 'starting' | 'active' | 'ended' | 'failed';

export interface SessionResponse {
  id: string;
  session_id: string;
  status: SessionStatus;
  container_id: string | null;
  created_at: string;
}

export interface CreateSessionRequest {
  user_id: string;
  container_image?: string;
}

export interface SessionSummary {
  id: string;
  status: SessionStatus;
  created_at: string;
  container_image: string;
}

export interface HealthResponse {
  status: string;
  version: string;
}

// WebSocket message types
export interface WebSocketMessage {
  type: 'container_ready' | 'command_output' | 'command_error' | 'session_ended';
  session_id: string;
  data?: string;
  error?: string;
}