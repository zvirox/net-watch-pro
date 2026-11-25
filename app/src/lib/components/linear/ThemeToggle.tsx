import React, { useEffect, useState } from 'react';
import { Moon, Sun } from 'lucide-react';

export type ThemeMode = 'dark' | 'light';

const STORAGE_KEY = 'ui-theme';

export function readCurrentTheme(): ThemeMode {
    if (typeof document === 'undefined') return 'dark';
    const attr = document.documentElement.getAttribute('data-theme');
    if (attr === 'light' || attr === 'dark') return attr;
    return document.documentElement.classList.contains('light') ? 'light' : 'dark';
}

export function applyTheme(next: ThemeMode): void {
    if (typeof document === 'undefined') return;
    document.documentElement.setAttribute('data-theme', next);
    document.documentElement.classList.toggle('dark', next === 'dark');
    document.documentElement.classList.toggle('light', next === 'light');
    try {
        localStorage.setItem(STORAGE_KEY, next);
    } catch {
        // Ignore storage errors in restricted environments
    }
    window.dispatchEvent(new CustomEvent('theme-changed', { detail: { theme: next } }));
}

export interface ThemeToggleProps {
    className?: string;
    onToggle?: (theme: ThemeMode) => void;
}

export const ThemeToggle: React.FC<ThemeToggleProps> = ({ className = '', onToggle }) => {
    const [theme, setTheme] = useState<ThemeMode>('dark');

    useEffect(() => {
        setTheme(readCurrentTheme());
        const onThemeChanged = (e: Event) => {
            const detail = (e as CustomEvent<{ theme?: ThemeMode }>).detail;
            if (detail?.theme === 'light' || detail?.theme === 'dark') {
                setTheme(detail.theme);
            }
        };
        window.addEventListener('theme-changed', onThemeChanged);
        return () => window.removeEventListener('theme-changed', onThemeChanged);
    }, []);

    const handleToggle = () => {
        const next: ThemeMode = theme === 'dark' ? 'light' : 'dark';
        applyTheme(next);
        setTheme(next);
        onToggle?.(next);
    };

    return (
        <button
            type="button"
            onClick={handleToggle}
            aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`}
            title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
            className={`size-7 inline-flex items-center justify-center rounded-md text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-surface-2)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer ${className}`}
        >
            {theme === 'dark' ? (
                <Sun className="size-3.5" aria-hidden="true" />
            ) : (
                <Moon className="size-3.5" aria-hidden="true" />
            )}
        </button>
    );
};
