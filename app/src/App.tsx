import React, { useEffect } from 'react';
import { useDaemonStore } from '$lib/stores/useDaemonStore';
import { StatusChip } from '$lib/components/StatusChip';
import { Dashboard } from '$lib/screens/Dashboard';
import { Live } from '$lib/screens/Live';
import { Apps } from '$lib/screens/Apps';
import { History } from '$lib/screens/History';
import { Settings } from '$lib/screens/Settings';
import { LayoutDashboard, Radio, Package, History as HistoryIcon, Settings as SettingsIcon } from 'lucide-react';

export const App: React.FC = () => {
  const { activeScreen, setActiveScreen, start } = useDaemonStore();

  useEffect(() => {
    start();
  }, [start]);

  const navItems = [
    { key: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { key: 'live', label: 'Live Traffic', icon: Radio },
    { key: 'apps', label: 'Apps', icon: Package },
    { key: 'history', label: 'History', icon: HistoryIcon },
    { key: 'settings', label: 'Settings', icon: SettingsIcon },
  ];

  return (
    <div className="flex h-screen w-screen bg-[#08080a] text-[#f4f4f5] overflow-hidden">
      {/* Sidebar Navigation */}
      <aside className="w-56 border-r border-[#23232a] bg-[#121215] flex flex-col justify-between p-3 select-none">
        <div>
          {/* Brand Header */}
          <div className="flex items-center gap-2.5 px-3 py-3 border-b border-[#23232a] mb-4">
            <div className="size-6 rounded-md bg-cyan-500/10 border border-cyan-500/30 flex items-center justify-center text-cyan-400 font-bold text-xs">
              NW
            </div>
            <div>
              <div className="font-sans font-semibold text-xs tracking-tight text-zinc-100">
                Net Watch Pro
              </div>
              <div className="font-mono text-[10px] text-zinc-500">v0.9.2 (Linux)</div>
            </div>
          </div>

          {/* Navigation Links */}
          <nav className="space-y-1">
            {navItems.map((item) => {
              const Icon = item.icon;
              const isActive = activeScreen === item.key;
              return (
                <button
                  key={item.key}
                  onClick={() => setActiveScreen(item.key)}
                  className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-all ${
                    isActive
                      ? 'bg-[#17171c] text-zinc-100 border border-[#33333d] shadow-[inset_0_1px_0_0_rgba(255,255,255,0.07)]'
                      : 'text-zinc-400 hover:text-zinc-200 hover:bg-[#17171c]/50 border border-transparent'
                  }`}
                >
                  <Icon className={`size-4 ${isActive ? 'text-cyan-400' : 'text-zinc-500'}`} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        {/* Footer System Status */}
        <div className="pt-3 border-t border-[#23232a] px-1">
          <StatusChip />
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col overflow-hidden bg-[#08080a]">
        {/* Top Header */}
        <header className="h-12 border-b border-[#23232a] bg-[#121215] px-6 flex items-center justify-between select-none">
          <h2 className="text-xs font-mono uppercase tracking-wider text-zinc-400">
            {navItems.find((n) => n.key === activeScreen)?.label || 'Overview'}
          </h2>
          <div className="flex items-center gap-3 text-xs font-mono text-zinc-500">
            <span>UDS: /run/netwatch-pro/api.sock</span>
          </div>
        </header>

        {/* Screen Content View */}
        <div className="flex-1 overflow-y-auto p-6">
          {activeScreen === 'dashboard' && <Dashboard />}
          {activeScreen === 'live' && <Live />}
          {activeScreen === 'apps' && <Apps />}
          {activeScreen === 'history' && <History />}
          {activeScreen === 'settings' && <Settings />}
        </div>
      </main>
    </div>
  );
};
