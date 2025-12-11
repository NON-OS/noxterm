import React, { useState } from 'react';
import { NoxTerminal } from './NoxTerminal';
import { PrivacyControls } from './PrivacyControls';
import { anonymousApi } from '../services/anonymousApi';

interface Session {
  id: string;
  user_id: string;
  status: string;
  container_image: string;
  created_at: string;
}

export const NoxApp: React.FC = () => {
  const [currentView, setCurrentView] = useState<'home' | 'terminal'>('home');
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [userId, setUserId] = useState('');
  const [selectedImage, setSelectedImage] = useState('ubuntu:22.04');
  const [isLoading, setIsLoading] = useState(false);
  const [privacyEnabled, setPrivacyEnabled] = useState(false);

  const containerImages = [
    { name: 'Ubuntu 22.04', value: 'ubuntu:22.04', description: 'Latest Ubuntu LTS with full package support' },
    { name: 'Ubuntu 20.04', value: 'ubuntu:20.04', description: 'Stable Ubuntu LTS release' },
    { name: 'Alpine Linux', value: 'alpine:latest', description: 'Lightweight Linux distribution' },
    { name: 'Debian', value: 'debian:latest', description: 'Stable Debian Linux' },
    { name: 'CentOS', value: 'centos:7', description: 'Enterprise Linux distribution' },
    { name: 'Node.js', value: 'node:18-alpine', description: 'Node.js development environment' },
    { name: 'Python', value: 'python:3.11-slim', description: 'Python development environment' },
    { name: 'Rust', value: 'rust:latest', description: 'Rust development environment' },
  ];

  const handlePrivacyChange = (enabled: boolean) => {
    setPrivacyEnabled(enabled);
  };

  const createSession = async () => {
    if (!userId.trim()) {
      alert('Please enter a user ID');
      return;
    }

    if (isLoading) {
      return; // Prevent double-clicking
    }

    setIsLoading(true);
    try {
      // Use anonymous API client
      const sessionData = await anonymousApi.createSession({
        user_id: userId,
        container_image: selectedImage,
      });

      const session = sessionData;
      
      // Add to sessions list  
      const newSession = {
        id: session.session_id,
        user_id: userId,
        status: 'created',
        container_image: selectedImage,
        created_at: new Date().toISOString(),
      };
      
      setSessions(prev => {
        // Check if session already exists
        if (prev.find(s => s.id === session.session_id)) {
          return prev;
        }
        return [...prev, newSession];
      });
      
      setActiveSessionId(session.session_id);
      setCurrentView('terminal');
    } catch (error) {
      alert(`Failed to create session: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setIsLoading(false);
    }
  };

  const loadSessions = async () => {
    if (!userId.trim()) return;
    
    try {
      const response = await fetch(`http://localhost:3001/api/sessions?user_id=${userId}`);
      if (response.ok) {
        const data = await response.json();
        setSessions(data.sessions || []);
      }
    } catch (error) {
      // Silently handle session loading errors
    }
  };

  const openSession = (sessionId: string) => {
    setActiveSessionId(sessionId);
    setCurrentView('terminal');
  };

  const goHome = () => {
    setCurrentView('home');
    setActiveSessionId(null);
  };

  if (currentView === 'terminal' && activeSessionId) {
    return (
      <div className="h-screen flex flex-col bg-black">
        {/* Navigation Bar */}
        <div className="bg-[#0a0a0a] border-b border-[rgba(102,255,255,0.1)] px-6 py-3 flex justify-between items-center">
          <button
            onClick={goHome}
            className="flex items-center space-x-2 text-[#66FFFF] hover:text-white transition-colors"
          >
            <span>←</span>
            <span>Back to Sessions</span>
          </button>
          <div className="flex items-center space-x-4">
            <div className="text-gray-400 font-mono text-sm">
              Session: {activeSessionId.slice(0, 8)}...
            </div>
            {privacyEnabled && (
              <div className="text-[#66FFFF] bg-[rgba(102,255,255,0.1)] border border-[rgba(102,255,255,0.3)] px-3 py-1 rounded-full text-sm flex items-center gap-2">
                <span className="w-2 h-2 bg-[#66FFFF] rounded-full"></span>
                Anonymous Mode Active
              </div>
            )}
          </div>
        </div>

        {/* Terminal */}
        <div className="flex-1">
          <NoxTerminal
            sessionId={activeSessionId}
            userId={userId}
            containerImage={selectedImage}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black">
      {/* Header */}
      <div className="bg-[#0a0a0a] border-b border-[rgba(102,255,255,0.1)]">
        <div className="max-w-6xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <h1 className="text-3xl font-bold">
                <span className="text-white">NØX</span>
                <span className="text-[#66FFFF]">TERM</span>
              </h1>
            </div>
            <div className="text-gray-500 text-sm font-mono">
              Privacy-First Terminal
            </div>
          </div>
        </div>
      </div>

      <div className="max-w-6xl mx-auto px-6 py-8">
        {/* Welcome Section */}
        <div className="text-center mb-12">
          <h2 className="text-4xl font-bold mb-4">
            <span className="text-white">Privacy-First </span>
            <span className="text-[#66FFFF]">Terminal Access</span>
          </h2>
          <p className="text-xl text-gray-400 mb-8 max-w-2xl mx-auto">
            Secure, containerized terminal environments with built-in anonymity.
            Your commands. Your privacy. No compromises.
          </p>

          {/* Privacy Controls */}
          <div className="mb-8">
            <PrivacyControls onPrivacyChange={handlePrivacyChange} />
          </div>

          {/* Features */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
            <div className="glass-card p-6">
              <div className="w-10 h-10 rounded-lg bg-[rgba(102,255,255,0.1)] flex items-center justify-center mb-4 mx-auto">
                <svg className="w-5 h-5 text-[#66FFFF]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
                </svg>
              </div>
              <h3 className="text-lg font-semibold text-white mb-2">Isolated Containers</h3>
              <p className="text-gray-500 text-sm">Sandboxed environments with multiple Linux distributions</p>
            </div>
            <div className="glass-card p-6">
              <div className="w-10 h-10 rounded-lg bg-[rgba(102,255,255,0.1)] flex items-center justify-center mb-4 mx-auto">
                <svg className="w-5 h-5 text-[#66FFFF]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
              </div>
              <h3 className="text-lg font-semibold text-white mb-2">Security Hardened</h3>
              <p className="text-gray-500 text-sm">Container isolation, resource limits, network restrictions</p>
            </div>
            <div className="glass-card p-6">
              <div className="w-10 h-10 rounded-lg bg-[rgba(102,255,255,0.1)] flex items-center justify-center mb-4 mx-auto">
                <svg className="w-5 h-5 text-[#66FFFF]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
                </svg>
              </div>
              <h3 className="text-lg font-semibold text-white mb-2">Anonymous Routing</h3>
              <p className="text-gray-500 text-sm">Optional onion routing via the Anyone network</p>
            </div>
          </div>
        </div>

        {/* Session Creation */}
        <div className="glass-card p-8 mb-8">
          <h3 className="text-2xl font-semibold text-white mb-6">Launch Terminal</h3>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            {/* User Configuration */}
            <div className="space-y-6">
              <div>
                <label className="block text-gray-400 text-sm font-medium mb-2">
                  User ID
                </label>
                <input
                  type="text"
                  value={userId}
                  onChange={(e) => setUserId(e.target.value)}
                  placeholder="Enter your username"
                  className="w-full px-4 py-3 bg-black border border-[rgba(102,255,255,0.2)] rounded-lg text-white placeholder-gray-600 focus:border-[#66FFFF] focus:outline-none transition-colors"
                />
              </div>

              <div>
                <label className="block text-gray-400 text-sm font-medium mb-2">
                  Environment
                </label>
                <select
                  value={selectedImage}
                  onChange={(e) => setSelectedImage(e.target.value)}
                  className="w-full px-4 py-3 bg-black border border-[rgba(102,255,255,0.2)] rounded-lg text-white focus:border-[#66FFFF] focus:outline-none transition-colors"
                >
                  {containerImages.map((image) => (
                    <option key={image.value} value={image.value}>
                      {image.name}
                    </option>
                  ))}
                </select>
                <p className="text-gray-600 text-sm mt-2">
                  {containerImages.find(img => img.value === selectedImage)?.description}
                </p>
              </div>

              <button
                onClick={createSession}
                disabled={isLoading || !userId.trim()}
                className="w-full btn-nox px-6 py-3 rounded-lg font-semibold disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isLoading ? 'Initializing...' : 'Launch Session'}
              </button>
            </div>

            {/* Container Images Preview */}
            <div className="bg-black/50 p-6 rounded-lg border border-[rgba(102,255,255,0.1)]">
              <h4 className="text-lg font-medium text-white mb-4">Available Environments</h4>
              <div className="space-y-2 max-h-64 overflow-y-auto">
                {containerImages.map((image) => (
                  <div
                    key={image.value}
                    className={`p-3 rounded-lg border cursor-pointer transition-all ${
                      selectedImage === image.value
                        ? 'border-[#66FFFF] bg-[rgba(102,255,255,0.1)]'
                        : 'border-[rgba(102,255,255,0.1)] hover:border-[rgba(102,255,255,0.3)]'
                    }`}
                    onClick={() => setSelectedImage(image.value)}
                  >
                    <div className="text-white font-medium">{image.name}</div>
                    <div className="text-gray-500 text-sm">{image.description}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Existing Sessions */}
        {userId && (
          <div className="glass-card p-8">
            <div className="flex justify-between items-center mb-6">
              <h3 className="text-2xl font-semibold text-white">Active Sessions</h3>
              <button
                onClick={loadSessions}
                className="btn-nox-outline px-4 py-2 rounded-lg text-sm"
              >
                Refresh
              </button>
            </div>

            {sessions.length === 0 ? (
              <p className="text-gray-500 text-center py-8">
                No active sessions. Launch your first terminal above.
              </p>
            ) : (
              <div className="grid gap-4">
                {sessions.map((session) => (
                  <div
                    key={session.id}
                    className="flex items-center justify-between p-4 bg-black/50 rounded-lg border border-[rgba(102,255,255,0.1)] hover:border-[rgba(102,255,255,0.3)] transition-all"
                  >
                    <div>
                      <div className="text-white font-medium">
                        {session.container_image}
                      </div>
                      <div className="text-gray-500 text-sm font-mono">
                        {session.id.slice(0, 8)}... • {new Date(session.created_at).toLocaleString()}
                      </div>
                    </div>
                    <button
                      onClick={() => openSession(session.id)}
                      className="btn-nox px-6 py-2 rounded-lg text-sm"
                    >
                      Connect
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Footer */}
        <div className="text-center mt-12 text-gray-600 text-sm">
          <p className="font-mono">NØXTERM • Privacy-First Terminal Access</p>
          <p className="text-gray-700 mt-1">
            <a href="https://nonos.systems" target="_blank" rel="noopener noreferrer" className="text-[#66FFFF] hover:text-white transition-colors">
              nonos.systems
            </a>
          </p>
        </div>
      </div>
    </div>
  );
};