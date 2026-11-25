import React, { useEffect, useState } from 'react';
import { Minus, Square, Copy, X } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeToggle, type ThemeMode } from './ThemeToggle';

export interface TitleBarProps {
    title?: string;
    version?: string;
    icon?: React.ReactNode;
    className?: string;
    onThemeToggle?: (theme: ThemeMode) => void;
}

const APP_VERSION = typeof __NWP_VERSION__ === 'string' ? `v${__NWP_VERSION__}` : 'v0.9.2-beta.5';

export const TitleBar: React.FC<TitleBarProps> = ({
    title = 'Net Watch Pro',
    version = APP_VERSION,
    icon,
    className = '',
    onThemeToggle,
}) => {
    const [isMaximized, setIsMaximized] = useState(false);

    useEffect(() => {
        try {
            if (
                typeof window !== 'undefined' &&
                ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
            ) {
                const appWindow = getCurrentWindow();
                appWindow
                    .isMaximized()
                    .then(setIsMaximized)
                    .catch(() => {});
                const unlisten = appWindow.onResized(() => {
                    appWindow
                        .isMaximized()
                        .then(setIsMaximized)
                        .catch(() => {});
                });
                return () => {
                    unlisten.then((fn) => fn()).catch(() => {});
                };
            }
        } catch {
            // Running outside Tauri runtime (Vite browser preview / tests)
        }
    }, []);

    const handleMinimize = async () => {
        try {
            await getCurrentWindow().minimize();
        } catch {
            // Browser fallback
        }
    };

    const handleToggleMaximize = async () => {
        try {
            const appWindow = getCurrentWindow();
            await appWindow.toggleMaximize();
            setIsMaximized(await appWindow.isMaximized());
        } catch {
            setIsMaximized((prev) => !prev);
        }
    };

    const handleClose = async () => {
        try {
            await getCurrentWindow().close();
        } catch {
            // Browser fallback
        }
    };

    return (
        <header
            role="region"
            aria-label="Window Title Bar"
            data-tauri-drag-region
            className={`h-10 w-full select-none bg-[var(--bg-app)] border-b border-[var(--border)] flex items-center justify-between px-3 text-[var(--text)] shrink-0 z-50 ${className}`}
        >
            {/* Left: 20px app logo, app name, 11px version tag (4px radius, 1px border) */}
            <div data-tauri-drag-region className="flex items-center gap-2 shrink-0">
                <span className="size-5 shrink-0 flex items-center justify-center rounded overflow-hidden">
                    {icon ?? <img src="/icon.png" alt="" className="size-5 object-contain" />}
                </span>
                <span className="text-[13px] font-semibold tracking-tight text-[var(--text)]">
                    {title}
                </span>
                <span className="px-1.5 py-0.5 text-[11px] leading-none rounded bg-[var(--bg-surface-2)] border border-[var(--border)] text-[var(--text-muted)]">
                    {version}
                </span>
            </div>

            {/* Center: empty drag region (double-click toggles maximize) */}
            <div
                data-tauri-drag-region
                onDoubleClick={handleToggleMaximize}
                className="flex-1 h-full min-w-8 cursor-default"
            />

            {/* Right: theme toggle, divider, minimize, maximize/restore, close */}
            <div className="flex items-center gap-1 shrink-0">
                <ThemeToggle onToggle={onThemeToggle} />

                <div className="w-px h-3.5 bg-[var(--border)] mx-1" aria-hidden="true" />

                <button
                    type="button"
                    onClick={handleMinimize}
                    aria-label="Minimize window"
                    title="Minimize"
                    className="size-7 inline-flex items-center justify-center rounded-md text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer"
                >
                    <Minus className="size-3.5" aria-hidden="true" />
                </button>

                <button
                    type="button"
                    onClick={handleToggleMaximize}
                    aria-label={isMaximized ? 'Restore window' : 'Maximize window'}
                    title={isMaximized ? 'Restore' : 'Maximize'}
                    className="size-7 inline-flex items-center justify-center rounded-md text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer"
                >
                    {isMaximized ? (
                        <Copy className="size-3" aria-hidden="true" />
                    ) : (
                        <Square className="size-3" aria-hidden="true" />
                    )}
                </button>

                <button
                    type="button"
                    onClick={handleClose}
                    aria-label="Close window"
                    title="Close"
                    className="size-7 inline-flex items-center justify-center rounded-md text-[var(--text-muted)] hover:bg-[var(--close-hover)] hover:text-[var(--on-accent)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer"
                >
                    <X className="size-3.5" aria-hidden="true" />
                </button>
            </div>
        </header>
    );
};
