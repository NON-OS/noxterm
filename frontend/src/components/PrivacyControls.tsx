import React, { useState, useEffect } from 'react';

interface PrivacyControlsProps {
  onPrivacyChange: (enabled: boolean) => void;
}

type CircuitStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

export const PrivacyControls: React.FC<PrivacyControlsProps> = ({ onPrivacyChange }) => {
  const [privacyEnabled, setPrivacyEnabled] = useState(false);
  const [circuitStatus, setCircuitStatus] = useState<CircuitStatus>('disconnected');
  const [isLoading, setIsLoading] = useState(false);
  
  const togglePrivacy = async () => {
    if (!privacyEnabled) {
      await enableAnonymity();
    } else {
      await disableAnonymity();
    }
  };
  
  const enableAnonymity = async () => {
    try {
      setIsLoading(true);
      setCircuitStatus('connecting');
      
      // Call backend to enable Anyone protocol
      const response = await fetch('/api/privacy/enable', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      });
      
      if (response.ok) {
        setPrivacyEnabled(true);
        setCircuitStatus('connected');
        onPrivacyChange(true);
      } else {
        throw new Error('Failed to enable privacy mode');
      }
      
    } catch (error) {
      console.error('Failed to enable anonymity:', error);
      setCircuitStatus('error');
      setPrivacyEnabled(false);
    } finally {
      setIsLoading(false);
    }
  };
  
  const disableAnonymity = async () => {
    try {
      setIsLoading(true);
      
      // Call backend to disable Anyone protocol
      const response = await fetch('/api/privacy/disable', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
      });
      
      if (response.ok) {
        setPrivacyEnabled(false);
        setCircuitStatus('disconnected');
        onPrivacyChange(false);
      }
      
    } catch (error) {
      console.error('Failed to disable anonymity:', error);
    } finally {
      setIsLoading(false);
    }
  };

  // Check privacy status on component mount
  useEffect(() => {
    const checkPrivacyStatus = async () => {
      try {
        const response = await fetch('/api/privacy/status');
        if (response.ok) {
          const data = await response.json();
          setPrivacyEnabled(data.enabled);
          setCircuitStatus(data.enabled ? 'connected' : 'disconnected');
        }
      } catch (error) {
        console.error('Failed to check privacy status:', error);
      }
    };
    
    checkPrivacyStatus();
  }, []);

  const getStatusColor = () => {
    switch (circuitStatus) {
      case 'connected': return 'text-[#66FFFF]';
      case 'connecting': return 'text-[#FFB366]';
      case 'error': return 'text-[#FF6666]';
      default: return 'text-gray-500';
    }
  };

  const getStatusDotColor = () => {
    switch (circuitStatus) {
      case 'connected': return 'bg-[#66FFFF]';
      case 'connecting': return 'bg-[#FFB366]';
      case 'error': return 'bg-[#FF6666]';
      default: return 'bg-gray-500';
    }
  };

  return (
    <div className="glass-card p-5 mb-6 max-w-xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-[rgba(102,255,255,0.1)] flex items-center justify-center">
            <svg className="w-4 h-4 text-[#66FFFF]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
          </div>
          <h3 className="text-base font-medium text-white">Network Privacy</h3>
        </div>

        <button
          onClick={togglePrivacy}
          disabled={isLoading}
          className={`px-4 py-2 rounded-lg font-medium text-sm transition-all ${
            privacyEnabled
              ? 'bg-[#2E5C5C] text-[#66FFFF] border border-[#66FFFF]/30'
              : 'bg-[#111] text-gray-400 border border-gray-700 hover:border-gray-600'
          } ${isLoading ? 'opacity-50 cursor-not-allowed' : ''}`}
        >
          {isLoading ? 'Processing...' : (privacyEnabled ? 'Enabled' : 'Disabled')}
        </button>
      </div>

      <div className="flex items-center justify-between">
        <div className="circuit-status flex items-center gap-2">
          <span className={`w-2 h-2 rounded-full ${getStatusDotColor()}`}></span>
          <span className={`text-sm font-mono ${getStatusColor()}`}>
            Circuit: {circuitStatus.toUpperCase()}
          </span>
        </div>

        {privacyEnabled && (
          <div className="privacy-badge text-xs px-3 py-1 rounded-full">
            IP Hidden
          </div>
        )}
      </div>

      {circuitStatus === 'error' && (
        <div className="mt-3 p-3 bg-[#FF6666]/10 border border-[#FF6666]/30 rounded-lg">
          <p className="text-[#FF6666] text-sm">
            Failed to establish anonymous connection. Check your connection and try again.
          </p>
        </div>
      )}

      {privacyEnabled && circuitStatus === 'connected' && (
        <div className="mt-3 p-3 bg-[rgba(102,255,255,0.05)] border border-[rgba(102,255,255,0.2)] rounded-lg">
          <p className="text-[#66FFFF] text-sm">
            Traffic routed through Anyone onion network.
          </p>
          <ul className="mt-2 text-gray-500 text-xs space-y-1">
            <li>Real IP address hidden</li>
            <li>Multi-hop encryption active</li>
            <li>Container networking anonymized</li>
          </ul>
        </div>
      )}
    </div>
  );
};