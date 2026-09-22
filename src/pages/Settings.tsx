/**
 * Settings page — PRD §18.6
 * Broker credentials (stored securely via DPAPI), app options, IPC config.
 */

import { useEffect, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
import { invoke } from '@tauri-apps/api/core';

export function Settings() {
  const { fetchSettings } = useAppStore();
  const [activeTab, setActiveTab] = useState<'broker' | 'app' | 'amibroker'>('broker');

  useEffect(() => {
    fetchSettings();
  }, []);

  return (
    <div className="flex flex-col h-full p-4 gap-4 max-w-4xl mx-auto w-full">
      <div>
        <h1 className="text-base font-semibold text-text-primary">Settings</h1>
        <p className="text-xs text-text-muted">Configuration for Broker, Application, and AmiBroker IPC</p>
      </div>

      <div className="flex gap-4 border-b border-border">
        {['broker', 'app', 'amibroker'].map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab as any)}
            className={`pb-2 px-1 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab
                ? 'border-accent text-accent'
                : 'border-transparent text-text-muted hover:text-text-primary'
            }`}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      <div className="panel p-6 flex-1">
        {activeTab === 'broker' && <BrokerSettings />}
        {activeTab === 'app' && <AppSettings />}
        {activeTab === 'amibroker' && <AmiBrokerSettings />}
      </div>
    </div>
  );
}

function BrokerSettings() {
  const { saveBrokerCredentials, status, fetchStatus, setSetting } = useAppStore();
  const [brokerId, setBrokerId] = useState('mock'); // would be 'broker_v1' in production
  const [apiKey, setApiKey] = useState('');
  const [apiSecret, setApiSecret] = useState('');
  const [loading, setLoading] = useState(false);
  const [sessionInfo, setSessionInfo] = useState<any>(null);

  useEffect(() => {
    if (status?.active_broker) {
      setBrokerId(status.active_broker);
    }
  }, [status]);

  useEffect(() => {
    invoke('get_session_status', { brokerId }).then(setSessionInfo).catch(console.error);
  }, [brokerId]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      await saveBrokerCredentials(brokerId, apiKey, apiSecret);
      await setSetting('active_broker_id', brokerId);
      setApiKey(''); setApiSecret('');
      alert('Credentials saved securely.');
      fetchStatus();
    } catch (e) {
      alert(e);
    } finally {
      setLoading(false);
    }
  };

  const handleAuth = async () => {
    setLoading(true);
    try {
      const res: any = await invoke('trigger_authentication', { brokerId });
      alert(`Connection Test Successful!\n\nStatus: ${res.status}\nExpires At: ${new Date(res.expires_at).toLocaleString()}`);
      invoke('get_session_status', { brokerId }).then(setSessionInfo);
      fetchStatus();
    } catch (e) {
      alert(`Connection Test Failed:\n\n${e}\n\nPlease check your App ID and Access Token.`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="max-w-md flex flex-col gap-6">
      <div>
        <h3 className="text-sm font-semibold mb-1">Broker Selection</h3>
        <select className="db-select w-full" value={brokerId} onChange={e => setBrokerId(e.target.value)}>
          <option value="mock">Mock Broker (Development)</option>
          <option value="broker_v1">Production Broker V1</option>
          <option value="fyers_v3">FYERS V3 API</option>
        </select>
      </div>

      <div className="border border-border rounded p-4 bg-bg-base">
        <h3 className="text-sm font-semibold mb-4">Credentials</h3>
        <p className="text-xs text-text-muted mb-4 leading-relaxed">
          Credentials are saved directly to the Windows Credential Manager using DPAPI. 
          They are never logged or stored in plaintext in the database.
        </p>
        <form onSubmit={handleSave} className="flex flex-col gap-3">
          <div>
            <label className="block text-xs text-text-muted mb-1">
              {brokerId === 'fyers_v3' ? 'App ID (client_id)' : 'API Key'}
            </label>
            <input type="password" required className="db-input font-mono text-xs" value={apiKey} onChange={e => setApiKey(e.target.value)} placeholder="••••••••••••" />
          </div>
          <div>
            <label className="block text-xs text-text-muted mb-1">
              {brokerId === 'fyers_v3' ? 'Access Token' : 'API Secret'}
            </label>
            <input type="password" required className="db-input font-mono text-xs" value={apiSecret} onChange={e => setApiSecret(e.target.value)} placeholder="••••••••••••" />
          </div>
          <button type="submit" className="btn btn-primary mt-2 self-start" disabled={loading}>
            Save Credentials
          </button>
        </form>
      </div>

      <div className="border border-border rounded p-4 bg-bg-base">
        <h3 className="text-sm font-semibold mb-4">Session Status</h3>
        <div className="flex items-center gap-4 text-sm mb-4">
          <div>Status: <strong className={sessionInfo?.status === 'active' ? 'text-status-green' : 'text-status-yellow'}>{sessionInfo?.status || 'Unknown'}</strong></div>
        </div>
        <button className="btn btn-ghost" onClick={handleAuth} disabled={loading}>
          Test Connection
        </button>
      </div>
    </div>
  );
}

function AppSettings() {
  const { settings, setSetting } = useAppStore();
  const [startWithWindows, setStartWithWindows] = useState(settings['start_with_windows'] === 'true');

  const handleToggle = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.checked;
    try {
      await invoke('set_start_with_windows', { enabled: val });
      setStartWithWindows(val);
      setSetting('start_with_windows', val.toString());
    } catch (err) {
      alert(err);
    }
  };

  return (
    <div className="max-w-md flex flex-col gap-6">
      <div>
        <h3 className="text-sm font-semibold mb-1">Startup Behavior</h3>
        <label className="flex items-center gap-3 mt-3 cursor-pointer">
          <input type="checkbox" className="w-4 h-4" checked={startWithWindows} onChange={handleToggle} />
          <span className="text-sm text-text-secondary">Start with Windows (System Tray)</span>
        </label>
        <p className="text-xs text-text-muted mt-2 ml-7">
          Registers the application in the Windows Registry (HKCU\Software\Microsoft\Windows\CurrentVersion\Run) to start automatically.
        </p>
      </div>
    </div>
  );
}

function AmiBrokerSettings() {
  const { settings, setSetting, status, fetchStatus } = useAppStore();
  const [port, setPort] = useState(settings['ipc_port'] || '7421');

  const handleSave = async () => {
    await setSetting('ipc_port', port);
    alert('IPC Port updated. Restart application for changes to take effect.');
    fetchStatus();
  };

  return (
    <div className="max-w-md flex flex-col gap-6">
      <div>
        <h3 className="text-sm font-semibold mb-1">Local IPC Server</h3>
        <p className="text-xs text-text-muted mb-4">
          The port AmiBroker plugin will connect to. Bound to 127.0.0.1 (loopback) only.
        </p>
        <div className="flex gap-2 items-center">
          <input type="number" className="db-input w-24" value={port} onChange={e => setPort(e.target.value)} />
          <button className="btn btn-primary" onClick={handleSave}>Save</button>
        </div>
        {status?.amibroker_connected ? (
          <div className="mt-4 text-xs text-status-green">● Client is currently connected</div>
        ) : (
          <div className="mt-4 text-xs text-text-muted">○ No client connected</div>
        )}
      </div>

      <div className="pt-4 border-t border-border">
        <h3 className="text-sm font-semibold mb-1">Plugin Installation</h3>
        <p className="text-xs text-text-muted mb-4">
          Install the DataBridge AmiBroker plugin (DataBridge.dll) into your AmiBroker Plugins folder.
        </p>
        <div className="flex flex-col gap-2">
          <input 
            type="text" 
            className="db-input" 
            placeholder="C:\Program Files\AmiBroker\Plugins" 
            id="plugin-path"
            defaultValue="C:\Program Files\AmiBroker\Plugins"
          />
          <button 
            className="btn btn-secondary w-full" 
            onClick={async () => {
              const path = (document.getElementById('plugin-path') as HTMLInputElement).value;
              try {
                await invoke('install_plugin', { path });
                alert('Plugin installed successfully!');
              } catch (e) {
                alert('Failed to install plugin: ' + e);
              }
            }}
          >
            Install Plugin
          </button>
        </div>
      </div>
    </div>
  );
}
