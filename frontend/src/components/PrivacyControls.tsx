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
      case 'connected': return 'text-green-400';
      case 'connecting': return 'text-yellow-400';
      case 'error': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  const getStatusIcon = () => {
    switch (circuitStatus) {
      case 'connected': return '🟢';
      case 'connecting': return '🟡';
      case 'error': return '🔴';
      default: return '⚫';
    }
  };

  return (
    <div className="privacy-controls bg-gray-800/50 rounded-xl p-4 mb-6 border border-gray-600">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-white flex items-center gap-2">
          🛡️ Network Privacy Protection
        </h3>
        
        <button 
          onClick={togglePrivacy}
          disabled={isLoading}
          className={`px-4 py-2 rounded-lg font-medium transition-all ${
            privacyEnabled 
              ? 'bg-green-600 hover:bg-green-700 text-white' 
              : 'bg-gray-600 hover:bg-gray-500 text-white'
          } ${isLoading ? 'opacity-50 cursor-not-allowed' : ''}`}
        >
          {isLoading ? 'Processing...' : (privacyEnabled ? 'Privacy ON' : 'Privacy OFF')}
        </button>
      </div>
      
      <div className="flex items-center justify-between">
        <div className="circuit-status flex items-center gap-2">
          <span className="text-lg">{getStatusIcon()}</span>
          <span className={`text-sm font-medium ${getStatusColor()}`}>
            Onion Circuit: {circuitStatus.toUpperCase()}
          </span>
        </div>
        
        {privacyEnabled && (
          <div className="text-xs text-green-400 bg-green-900/30 px-3 py-1 rounded-full">
            🔐 IP Hidden via Anyone Network
          </div>
        )}
      </div>
      
      {circuitStatus === 'error' && (
        <div className="mt-3 p-3 bg-red-900/30 border border-red-500/30 rounded-lg">
          <p className="text-red-400 text-sm">
            ⚠️ Failed to establish anonymous connection. Please check your internet connection and try again.
          </p>
        </div>
      )}
      
      {privacyEnabled && circuitStatus === 'connected' && (
        <div className="mt-3 p-3 bg-green-900/30 border border-green-500/30 rounded-lg">
          <p className="text-green-400 text-sm">
            ✅ Your terminal traffic is now routed through the Anyone onion network for maximum privacy.
            <br />
            <span className="text-green-300">• Your real IP address is hidden</span><br />
            <span className="text-green-300">• All traffic is encrypted through multiple hops</span><br />
            <span className="text-green-300">• Container networking is anonymized</span>
          </p>
        </div>
      )}
    </div>
  );
};