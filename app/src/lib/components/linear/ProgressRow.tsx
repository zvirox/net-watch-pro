import React from 'react';

export interface ProgressRowProps {
    label: string;
    value: number; // 0 - 100
    displayValue?: string;
    colorVar?: string; // CSS color or var(--accent-*)
    className?: string;
}

export const ProgressRow: React.FC<ProgressRowProps> = ({
    label,
    value,
    displayValue,
    colorVar = 'var(--accent-blue)',
    className = '',
}) => {
    const clamped = Math.min(100, Math.max(0, Number.isFinite(value) ? value : 0));

    return (
        <div className={`flex flex-col gap-1.5 w-full ${className}`}>
            <div className="flex items-center justify-between gap-2 text-xs">
                <span className="font-medium text-[var(--text)] truncate">{label}</span>
                <span className="text-[var(--text-muted)] tabular-nums shrink-0">
                    {displayValue ?? `${Math.round(clamped)}%`}
                </span>
            </div>
            <div className="h-1 w-full rounded bg-[var(--bg-bar-idle)] overflow-hidden">
                <div
                    className="h-full rounded transition-all duration-150 ease-out"
                    style={{ width: `${clamped}%`, backgroundColor: colorVar }}
                />
            </div>
        </div>
    );
};
