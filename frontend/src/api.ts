import axios from 'axios';
import {
  Session,
  SessionResponse,
  CreateSessionRequest,
  SessionSummary,
  HealthResponse,
} from './types';

const API_BASE_URL = (import.meta as any).env?.VITE_API_URL || 'http://localhost:3001/api';

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

export const apiClient = {
  // Health check
  getHealth: (): Promise<HealthResponse> =>
    api.get('/health').then(res => res.data),

  // Sessions
  createSession: (data: CreateSessionRequest): Promise<SessionResponse> =>
    api.post('/sessions', data).then(res => res.data),

  listSessions: (userId: string): Promise<SessionSummary[]> =>
    api.get('/sessions', {
      params: { user_id: userId }
    }).then(res => res.data),

  getSession: (sessionId: string): Promise<Session> =>
    api.get(`/sessions/${sessionId}`).then(res => res.data),
};

export default apiClient;