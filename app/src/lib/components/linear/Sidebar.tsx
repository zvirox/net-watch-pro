import React from 'react';
import {
    Activity,
    ChevronLeft,
    ChevronRight,
    Clock,
    KeyRound,
    LayoutDashboard,
    Package,
    Radio,
    Settings as SettingsIcon,
    Shield,
} from 'lucide-react';
import { StatusChip } from '../StatusChip';

export type NavScreenId = 'dashboard' | 'live' | 'history' | 'apps' | 'diagnostics' | 'settings';

export interface SidebarProps {
    activeScreen: string;
    collapsed: boolean;
    isPro: boolean;
    demoMode: boolean;
    onSelectScreen: (screen: NavScreenId) => void;
    onToggleCollapse: () => void;
    onOpenUpgrade: () => void;
}

const NAV_ITEMS: Array<{
    id: NavScreenId;
    label: string;
    icon: React.ComponentType<{ className?: string }>;
}> = [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { id: 'live', label: 'Live', icon: Radio },
    { id: 'history', label: 'History', icon: Clock },
    { id: 'apps', label: 'Apps', icon: Package },
    { id: 'diagnostics', label: 'Diagnostics', icon: Activity },
    { id: 'settings', label: 'Settings', icon: SettingsIcon },
];

export const Sidebar: React.FC<SidebarProps> = ({
    activeScreen,
    collapsed,
    isPro,
    demoMode,
    onSelectScreen,
    onToggleCollapse,
    onOpenUpgrade,
}) => {
    const resolvedScreen = activeScreen === 'app-detail' ? 'apps' : activeScreen;

    return (
        <aside
            aria-label="Primary navigation"
            className={`border-r border-[var(--border)] bg-[var(--bg-app)] flex flex-col justify-between p-2 select-none shrink-0 transition-[width] duration-150 ease-out ${
                collapsed ? 'w-14' : 'w-60'
            }`}
        >
            <div className="space-y-3">
                {/* Workspace / host scope indicator */}
                <div
                    className={`flex items-center gap-2.5 h-9 px-2 rounded-md border border-[var(--border)] bg-[var(--bg-surface)] ${
                        collapsed ? 'justify-center px-0' : ''
                    }`}
                    title="Local host monitor"
                >
                    <span
                        className="size-5 rounded bg-[var(--tint-blue)] text-[var(--tint-blue-fg)] flex items-center justify-center shrink-0"
                        aria-hidden="true"
                    >
                        <Shield className="size-3.5" />
                    </span>
                    {!collapsed && (
                        <div className="flex items-center justify-between flex-1 min-w-0">
                            <span className="text-xs font-medium text-[var(--text)] truncate">
                                {demoMode ? 'Demo Sandbox' : 'Local Machine'}
                            </span>
                            <span className="text-[11px] px-1.5 py-0.5 rounded bg-[var(--bg-surface-2)] text-[var(--text-muted)]">
                                Linux
                            </span>
                        </div>
                    )}
                </div>

                {/* Navigation list — 36px height (h-9) per Linear standard */}
                <nav className="space-y-1">
                    {NAV_ITEMS.map((item) => {
                        const Icon = item.icon;
                        const isActive = resolvedScreen === item.id;
                        return (
                            <button
                                key={item.id}
                                type="button"
                                aria-current={isActive ? 'page' : undefined}
                                aria-label={collapsed ? item.label : undefined}
                                title={collapsed ? item.label : undefined}
                                onClick={() => onSelectScreen(item.id)}
                                className={`w-full h-9 flex items-center gap-2.5 rounded-md text-[13px] font-medium transition-colors duration-150 ease-out focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer ${
                                    collapsed ? 'justify-center px-0' : 'px-2.5'
                                } ${
                                    isActive
                                        ? 'bg-[var(--bg-surface-2)] text-[var(--text)] border border-[var(--border)]'
                                        : 'text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] border border-transparent'
                                }`}
                            >
                                <Icon
                                    className={`size-4 shrink-0 ${
                                        isActive
                                            ? 'text-[var(--accent-blue)]'
                                            : 'text-[var(--text-muted)]'
                                    }`}
                                />
                                {!collapsed && <span className="truncate">{item.label}</span>}
                            </button>
                        );
                    })}
                </nav>
            </div>

            {/* Bottom utilities: License tier + Status + Collapse toggle */}
            <div className="space-y-2 pt-2 border-t border-[var(--border)]">
                <button
                    type="button"
                    onClick={onOpenUpgrade}
                    aria-label={collapsed ? (isPro ? 'Pro license' : 'License') : undefined}
                    title={collapsed ? (isPro ? 'Pro active' : 'Activate license') : undefined}
                    className={`w-full h-9 flex items-center gap-2.5 rounded-md text-[13px] font-medium text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer ${
                        collapsed ? 'justify-center px-0' : 'px-2.5 justify-between'
                    }`}
                >
                    <span className="flex items-center gap-2.5 min-w-0">
                        <KeyRound className="size-4 shrink-0" aria-hidden="true" />
                        {!collapsed && <span className="truncate">License</span>}
                    </span>
                    {!collapsed && (
                        <span
                            className={`px-1.5 py-0.5 rounded text-[11px] font-medium ${
                                isPro
                                    ? 'bg-[var(--tint-blue)] text-[var(--tint-blue-fg)]'
                                    : 'bg-[var(--bg-surface-2)] text-[var(--text-muted)] border border-[var(--border)]'
                            }`}
                        >
                            {isPro ? 'Pro' : 'Free'}
                        </span>
                    )}
                </button>

                {!collapsed && (
                    <div className="px-0.5">
                        <StatusChip
                            className="w-full justify-between"
                            onClick={() => onSelectScreen('diagnostics')}
                        />
                    </div>
                )}

                <button
                    type="button"
                    onClick={onToggleCollapse}
                    aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
                    title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
                    className={`w-full h-8 flex items-center gap-2 rounded-md text-xs text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer ${
                        collapsed ? 'justify-center px-0' : 'px-2.5 justify-between'
                    }`}
                >
                    {!collapsed && <span>Collapse sidebar</span>}
                    {collapsed ? (
                        <ChevronRight className="size-4" aria-hidden="true" />
                    ) : (
                        <ChevronLeft className="size-4" aria-hidden="true" />
                    )}
                </button>
            </div>
        </aside>
    );
};
