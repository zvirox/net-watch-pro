import React, { useEffect } from 'react';
import { useDaemonStore } from '$lib/stores/useDaemonStore';
import { StatusChip } from '$lib/components/StatusChip';
import { TitleBar } from '$lib/components/linear/TitleBar';
import { Dashboard } from '$lib/screens/Dashboard';
import { Live } from '$lib/screens/Live';
import { Apps } from '$lib/screens/Apps';
import { History } from '$lib/screens/History';
import { Settings } from '$lib/screens/Settings';
import {
    LayoutDashboard,
    Radio,
    Package,
    History as HistoryIcon,
    Settings as SettingsIcon,
} from 'lucide-react';

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
        <div className="flex flex-col h-screen w-screen bg-[var(--bg-app)] text-[var(--text)] overflow-hidden">
            {/* Universal 40px Linear TitleBar */}
            <TitleBar
                title="Net Watch Pro"
                version="v0.9.2-beta.5"
                icon={
                    <div className="size-4 rounded bg-[#3355FF] text-white flex items-center justify-center font-bold text-[9px]">
                        NW
                    </div>
                }
            >
                <span className="text-[11px] font-mono text-[var(--text-muted)] hidden sm:inline">
                    UDS: /run/netwatch-pro/api.sock
                </span>
            </TitleBar>

            <div className="flex flex-1 overflow-hidden">
                {/* Sidebar Navigation */}
                <aside className="w-56 border-r border-[var(--border)] bg-[var(--bg-surface)] flex flex-col justify-between p-3 select-none">
                    <nav className="space-y-1">
                        {navItems.map((item) => {
                            const Icon = item.icon;
                            const isActive = activeScreen === item.key;
                            return (
                                <button
                                    key={item.key}
                                    onClick={() => setActiveScreen(item.key)}
                                    className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                                        isActive
                                            ? 'bg-[var(--bg-surface-2)] text-[var(--text)] border border-[var(--border-strong)]'
                                            : 'text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] border border-transparent'
                                    }`}
                                >
                                    <Icon
                                        className={`size-4 ${isActive ? 'text-[#3355FF]' : 'text-[var(--text-muted)]'}`}
                                    />
                                    <span>{item.label}</span>
                                </button>
                            );
                        })}
                    </nav>

                    {/* Footer System Status */}
                    <div className="pt-3 border-t border-[var(--border)] px-1">
                        <StatusChip />
                    </div>
                </aside>

                {/* Main Content Area */}
                <main className="flex-1 flex flex-col overflow-hidden bg-[var(--bg-app)]">
                    {/* Top Section Header */}
                    <header className="h-10 border-b border-[var(--border)] bg-[var(--bg-surface)] px-6 flex items-center justify-between select-none">
                        <h2 className="text-xs font-mono uppercase tracking-wider text-[var(--text-muted)]">
                            {navItems.find((n) => n.key === activeScreen)?.label || 'Overview'}
                        </h2>
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
        </div>
    );
};
