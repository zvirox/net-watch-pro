import React from 'react';
import { ChevronRight } from 'lucide-react';

export interface BreadcrumbSegment {
    label: string;
    onClick?: () => void;
}

export interface BreadcrumbBarProps {
    segments: BreadcrumbSegment[];
    rightSlot?: React.ReactNode;
    className?: string;
}

export const BreadcrumbBar: React.FC<BreadcrumbBarProps> = ({
    segments,
    rightSlot,
    className = '',
}) => {
    return (
        <div
            className={`h-10 border-b border-[var(--border)] bg-[var(--bg-surface)] px-5 flex items-center justify-between gap-4 select-none shrink-0 ${className}`}
        >
            <nav aria-label="Breadcrumb" className="flex items-center gap-1.5 text-[13px] min-w-0">
                {segments.map((seg, idx) => {
                    const isLast = idx === segments.length - 1;
                    return (
                        <React.Fragment key={`${seg.label}-${String(idx)}`}>
                            {idx > 0 && (
                                <ChevronRight
                                    className="size-3.5 text-[var(--text-faint)] shrink-0"
                                    aria-hidden="true"
                                />
                            )}
                            {seg.onClick && !isLast ? (
                                <button
                                    type="button"
                                    onClick={seg.onClick}
                                    className="text-[var(--text-muted)] hover:text-[var(--text)] transition-colors duration-150 truncate cursor-pointer focus-visible:outline-2 focus-visible:outline-[var(--accent-blue)] rounded px-1 -mx-1"
                                >
                                    {seg.label}
                                </button>
                            ) : (
                                <span
                                    className={`truncate ${
                                        isLast
                                            ? 'font-semibold text-[var(--text)]'
                                            : 'text-[var(--text-muted)]'
                                    }`}
                                >
                                    {seg.label}
                                </span>
                            )}
                        </React.Fragment>
                    );
                })}
            </nav>

            {rightSlot && <div className="flex items-center gap-2 shrink-0">{rightSlot}</div>}
        </div>
    );
};
