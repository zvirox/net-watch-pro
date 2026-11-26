import React from 'react';

export interface StatusPillProps {
    label: string;
    status: 'online' | 'active' | 'warning' | 'alert' | 'idle';
    monoValue?: string;
    className?: string;
}

const statusDotColor = {
    online: 'bg-[var(--sky)]',
    active: 'bg-[var(--primary)]',
    warning: 'bg-[var(--amber)]',
    alert: 'bg-[var(--red)]',
    idle: 'bg-[var(--text-faint)]',
} as const;

export const StatusPill: React.FC<StatusPillProps> = ({
    label,
    status,
    monoValue,
    className = '',
}) => {
    return (
        <div
            className={`inline-flex items-center gap-2 rounded border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-1 text-xs select-none ${className}`}
        >
            <span
                className={`size-1.5 rounded-full shrink-0 ${statusDotColor[status]}`}
                aria-hidden="true"
            />
            <span className="font-sans font-medium text-[var(--text-muted)] truncate">{label}</span>
            {monoValue && (
                <span className="font-sans font-medium text-[var(--text)] pl-1.5 border-l border-[var(--border)] tabular-nums shrink-0">
                    {monoValue}
                </span>
            )}
        </div>
    );
};
