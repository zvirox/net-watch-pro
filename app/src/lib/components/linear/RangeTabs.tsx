import React from 'react';

export type RangeTabOption = string | { id: string; label: string };

export interface RangeTabsProps {
    options?: RangeTabOption[];
    active: string;
    onChange: (tabId: string) => void;
    ariaLabel?: string;
    className?: string;
}

const DEFAULT_OPTIONS: string[] = ['1W', '1M', '3W', 'YTD', 'Total'];

export const RangeTabs: React.FC<RangeTabsProps> = ({
    options = DEFAULT_OPTIONS,
    active,
    onChange,
    ariaLabel = 'Time range',
    className = '',
}) => {
    const normalized = options.map((opt) =>
        typeof opt === 'string' ? { id: opt, label: opt } : opt,
    );

    const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, idx: number) => {
        let next = idx;
        if (event.key === 'ArrowRight') next = (idx + 1) % normalized.length;
        else if (event.key === 'ArrowLeft')
            next = (idx - 1 + normalized.length) % normalized.length;
        else if (event.key === 'Home') next = 0;
        else if (event.key === 'End') next = normalized.length - 1;
        else return;
        event.preventDefault();
        const target = normalized[next];
        if (target) onChange(target.id);
    };

    return (
        <div
            role="group"
            aria-label={ariaLabel}
            className={`inline-flex items-center gap-0.5 p-0.5 rounded-md border border-[var(--border)] bg-[var(--bg-surface-2)] select-none ${className}`}
        >
            {normalized.map((tab, idx) => {
                const isActive = tab.id === active;
                return (
                    <button
                        key={tab.id}
                        type="button"
                        aria-pressed={isActive}
                        onClick={() => onChange(tab.id)}
                        onKeyDown={(e) => handleKeyDown(e, idx)}
                        className={`px-2.5 py-1 text-xs rounded font-medium transition-colors duration-150 ease-out focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] cursor-pointer ${
                            isActive
                                ? 'bg-[var(--bg-surface)] text-[var(--text)] border border-[var(--border)] shadow-[var(--shadow-card)] dark:shadow-none'
                                : 'text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-bar-idle)] border border-transparent'
                        }`}
                    >
                        {tab.label}
                    </button>
                );
            })}
        </div>
    );
};
