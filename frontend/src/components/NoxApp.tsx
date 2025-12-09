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
        <div className="bg-gray-900 border-b border-gray-700 px-6 py-3 flex justify-between items-center">
          <button
            onClick={goHome}
            className="flex items-center space-x-2 text-blue-400 hover:text-blue-300"
          >
            <span>←</span>
            <span>Back to Sessions</span>
          </button>
          <div className="flex items-center space-x-4">
            <div className="text-gray-400">
              Session: {activeSessionId.slice(0, 8)}...
            </div>
            {privacyEnabled && (
              <div className="text-green-400 bg-green-900/30 px-3 py-1 rounded-full text-sm flex items-center gap-2">
                🛡️ Anonymous Mode Active
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
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-black">
      {/* Header */}
      <div className="bg-black/50 backdrop-blur-sm border-b border-gray-700">
        <div className="max-w-6xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <h1 className="text-3xl font-bold text-white">NOXTERM</h1>
              <div className="bg-green-500 text-black px-2 py-1 rounded text-sm font-bold">
                BETA
              </div>
            </div>
            <div className="text-gray-400">
              NOXTERM • Full Functionality
            </div>
          </div>
        </div>
      </div>

      <div className="max-w-6xl mx-auto px-6 py-8">
        {/* Welcome Section */}
        <div className="text-center mb-12">
          <h2 className="text-4xl font-bold text-white mb-4">
            Secure Containerized Terminal
          </h2>
          <p className="text-xl text-gray-300 mb-8">
            NOXTERM containerized terminal with Docker integration
          </p>
          
          {/* Privacy Controls */}
          <div className="mb-8">
            <PrivacyControls onPrivacyChange={handlePrivacyChange} />
          </div>

          {/* Features */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
            <div className="bg-gray-800/50 p-6 rounded-lg border border-gray-700">
              <div className="text-green-400 text-3xl mb-3">🐳</div>
              <h3 className="text-xl font-semibold text-white mb-2">Docker Integration</h3>
              <p className="text-gray-400">Real containers with multiple Linux distributions</p>
            </div>
            <div className="bg-gray-800/50 p-6 rounded-lg border border-gray-700">
              <div className="text-blue-400 text-3xl mb-3">🛡️</div>
              <h3 className="text-xl font-semibold text-white mb-2">Secure by Default</h3>
              <p className="text-gray-400">Container isolation, resource limits, network restrictions</p>
            </div>
            <div className="bg-gray-800/50 p-6 rounded-lg border border-gray-700">
              <div className="text-purple-400 text-3xl mb-3">⚡</div>
              <h3 className="text-xl font-semibold text-white mb-2">Real Terminal</h3>
              <p className="text-gray-400">Full command execution with command history</p>
            </div>
          </div>
        </div>

        {/* Session Creation */}
        <div className="bg-gray-800/30 rounded-lg p-8 border border-gray-700 mb-8">
          <h3 className="text-2xl font-bold text-white mb-6">Create New Session</h3>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            {/* User Configuration */}
            <div className="space-y-6">
              <div>
                <label className="block text-gray-300 text-sm font-medium mb-2">
                  User ID
                </label>
                <input
                  type="text"
                  value={userId}
                  onChange={(e) => setUserId(e.target.value)}
                  placeholder="Enter your username"
                  className="w-full px-4 py-3 bg-gray-900 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-gray-300 text-sm font-medium mb-2">
                  Container Image
                </label>
                <select
                  value={selectedImage}
                  onChange={(e) => setSelectedImage(e.target.value)}
                  className="w-full px-4 py-3 bg-gray-900 border border-gray-600 rounded-lg text-white focus:border-blue-500 focus:outline-none"
                >
                  {containerImages.map((image) => (
                    <option key={image.value} value={image.value}>
                      {image.name}
                    </option>
                  ))}
                </select>
                <p className="text-gray-500 text-sm mt-2">
                  {containerImages.find(img => img.value === selectedImage)?.description}
                </p>
              </div>

              <button
                onClick={createSession}
                disabled={isLoading || !userId.trim()}
                className="w-full bg-gradient-to-r from-blue-600 to-purple-600 text-white px-6 py-3 rounded-lg font-semibold hover:from-blue-700 hover:to-purple-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isLoading ? 'Creating Session...' : 'Create Terminal Session'}
              </button>
            </div>

            {/* Container Images Preview */}
            <div className="bg-gray-900/50 p-6 rounded-lg border border-gray-600">
              <h4 className="text-lg font-semibold text-white mb-4">Available Environments</h4>
              <div className="space-y-2 max-h-64 overflow-y-auto">
                {containerImages.map((image) => (
                  <div
                    key={image.value}
                    className={`p-3 rounded border cursor-pointer transition-colors ${
                      selectedImage === image.value
                        ? 'border-blue-500 bg-blue-500/20'
                        : 'border-gray-700 hover:border-gray-500'
                    }`}
                    onClick={() => setSelectedImage(image.value)}
                  >
                    <div className="text-white font-medium">{image.name}</div>
                    <div className="text-gray-400 text-sm">{image.description}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Existing Sessions */}
        {userId && (
          <div className="bg-gray-800/30 rounded-lg p-8 border border-gray-700">
            <div className="flex justify-between items-center mb-6">
              <h3 className="text-2xl font-bold text-white">Your Sessions</h3>
              <button
                onClick={loadSessions}
                className="px-4 py-2 bg-gray-700 text-white rounded hover:bg-gray-600"
              >
                Refresh
              </button>
            </div>
            
            {sessions.length === 0 ? (
              <p className="text-gray-400 text-center py-8">
                No sessions found. Create your first session above.
              </p>
            ) : (
              <div className="grid gap-4">
                {sessions.map((session) => (
                  <div
                    key={session.id}
                    className="flex items-center justify-between p-4 bg-gray-900/50 rounded-lg border border-gray-700"
                  >
                    <div>
                      <div className="text-white font-medium">
                        {session.container_image}
                      </div>
                      <div className="text-gray-400 text-sm">
                        Session: {session.id.slice(0, 8)}... • Created: {new Date(session.created_at).toLocaleString()}
                      </div>
                    </div>
                    <button
                      onClick={() => openSession(session.id)}
                      className="px-6 py-2 bg-green-600 text-white rounded hover:bg-green-700"
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
        <div className="text-center mt-12 text-gray-500">
          <p>NOXTERM • Secure Containerized Terminal Platform</p>
          <p>✅ NOXTERM Ready • Full Terminal Functionality</p>
        </div>
      </div>
    </div>
  );
};