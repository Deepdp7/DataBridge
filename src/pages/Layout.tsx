/**
 * Main application layout shell
 */

import { Outlet } from 'react-router-dom';
import { Sidebar } from '../components/Sidebar';
import { useAppStore } from '../store/useAppStore';

export function Layout() {
  const lastError = useAppStore((s) => s.lastError);
  const clearError = useAppStore((s) => s.clearError);

  return (
    <div className="flex h-screen w-screen bg-bg-surface overflow-hidden text-text-primary">
      <Sidebar />
      <main className="flex-1 flex flex-col relative overflow-hidden bg-bg-base">
        {/* Global error banner */}
        {lastError && (
          <div className="bg-status-red text-bg-base px-4 py-2 text-xs flex items-center justify-between z-50 shadow-md">
            <div className="flex items-center gap-2 font-medium">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
              </svg>
              <span>{lastError}</span>
            </div>
            <button onClick={clearError} className="hover:bg-black/20 p-1 rounded transition-colors">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                <path d="M18 6L6 18M6 6l12 12"/>
              </svg>
            </button>
          </div>
        )}

        <div className="flex-1 overflow-auto">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
