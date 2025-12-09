import axios, { AxiosInstance } from 'axios';
import { CreateSessionRequest, SessionResponse, SessionSummary, HealthResponse } from '../types';

class AnonymousApiClient {
  private baseURL: string;
  private privacyEnabled: boolean = false;
  private standardClient: AxiosInstance;
  
  constructor() {
    this.baseURL = (import.meta as any).env?.VITE_API_URL || '/api';
    this.standardClient = axios.create({
      baseURL: this.baseURL,
      headers: {
        'Content-Type': 'application/json',
      },
    });
  }

  enablePrivacy() {
    this.privacyEnabled = true;
  }

  disablePrivacy() {
    this.privacyEnabled = false;
  }

  private getClient(): AxiosInstance {
    // When privacy is enabled, requests will be routed through backend proxy
    if (this.privacyEnabled) {
      // Add privacy headers to indicate anonymous routing
      return axios.create({
        baseURL: this.baseURL,
        headers: {
          'Content-Type': 'application/json',
          'X-Privacy-Mode': 'anonymous',
          'X-Route-Via': 'anyone-network',
        },
      });
    }
    
    return this.standardClient;
  }

  // Health check
  async getHealth(): Promise<HealthResponse> {
    const response = await this.getClient().get('/health');
    return response.data;
  }

  // Privacy control endpoints
  async enablePrivacyMode(): Promise<{ status: string; socks_port: number }> {
    const response = await this.standardClient.post('/privacy/enable');
    this.enablePrivacy();
    return response.data;
  }

  async disablePrivacyMode(): Promise<{ status: string }> {
    const response = await this.standardClient.post('/privacy/disable');
    this.disablePrivacy();
    return response.data;
  }

  async getPrivacyStatus(): Promise<{ enabled: boolean; socks_port?: number }> {
    const response = await this.standardClient.get('/privacy/status');
    this.privacyEnabled = response.data.enabled;
    return response.data;
  }

  // Session management
  async createSession(data: CreateSessionRequest): Promise<SessionResponse> {
    const response = await this.getClient().post('/sessions', data);
    return response.data;
  }

  async listSessions(userId: string): Promise<SessionSummary[]> {
    const response = await this.getClient().get('/sessions', {
      params: { user_id: userId }
    });
    return response.data;
  }

  async getSession(sessionId: string): Promise<any> {
    const response = await this.getClient().get(`/sessions/${sessionId}`);
    return response.data;
  }

  // Get WebSocket URL with privacy consideration
  getWebSocketUrl(sessionId: string): string {
    const wsBaseUrl = (import.meta as any).env?.VITE_WS_URL || 'ws://localhost:3001';
    const wsUrl = `${wsBaseUrl}/api/sessions/${sessionId}/ws`;
    
    // Note: WebSocket privacy routing handled by backend
    return wsUrl;
  }
}

export const anonymousApi = new AnonymousApiClient();