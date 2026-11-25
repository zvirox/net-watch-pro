import React, { useId } from 'react';
import { ArrowUpRight, ArrowDownRight } from 'lucide-react';
import { LinearCard } from './LinearCard';

export type TintTone = 'blue' | 'sky' | 'green' | 'red' | 'amber' | 'teal' | 'purple' | 'orange';

export interface StatCardProps {
    title: string;
    value: string | number;
    unit?: string;
    delta?: {
        value: string | number;
        positive?: boolean;
        comparison?: string;
    };
    sparkline?: number[];
    sparklineDirection?: 'up' | 'down';
    icon?: React.ReactNode;
    tone?: TintTone;
    className?: string;
}

const TONE_CLASSES: Record<TintTone, string> = {
    blue: 'bg-[var(--tint-blue)] text-[var(--tint-blue-fg)]',
    sky: 'bg-[var(--tint-sky)] text-[var(--tint-sky-fg)]',
    green: 'bg-[var(--tint-green)] text-[var(--tint-green-fg)]',
    red: 'bg-[var(--tint-red)] text-[var(--tint-red-fg)]',
    amber: 'bg-[var(--tint-amber)] text-[var(--tint-amber-fg)]',
    teal: 'bg-[var(--tint-teal)] text-[var(--tint-teal-fg)]',
    purple: 'bg-[var(--tint-purple)] text-[var(--tint-purple-fg)]',
    orange: 'bg-[var(--tint-orange)] text-[var(--tint-orange-fg)]',
};

function buildSparklinePaths(points: number[]): { line: string; area: string } | null {
    if (points.length < 2) return null;
    const w = 100;
    const h = 28;
    const pad = 2;
    const min = Math.min(...points);
    const max = Math.max(...points);
    const span = max - min || 1;
    const coords = points.map((p, i) => {
        const x = (i / (points.length - 1)) * w;
        const y = h - pad - ((p - min) / span) * (h - pad * 2);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    const line = `M ${coords.join(' L ')}`;
    const area = `${line} L ${w},${h} L 0,${h} Z`;
    return { line, area };
}

export const StatCard: React.FC<StatCardProps> = ({
    title,
    value,
    unit,
    delta,
    sparkline,
    sparklineDirection = 'up',
    icon,
    tone = 'blue',
    className = '',
}) => {
    const gradId = useId();
    const paths = sparkline ? buildSparklinePaths(sparkline) : null;
    const sparkStroke = sparklineDirection === 'down' ? 'var(--accent-red)' : 'var(--accent-green)';

    return (
        <LinearCard level={1} className={`p-5 flex flex-col justify-between ${className}`}>
            <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-medium uppercase tracking-wider text-[var(--text-muted)] select-none">
                    {title}
                </span>
                {icon && (
                    <div
                        className={`size-7 rounded flex items-center justify-center shrink-0 ${TONE_CLASSES[tone]}`}
                        aria-hidden="true"
                    >
                        {icon}
                    </div>
                )}
            </div>

            <div className="mt-3 flex items-end justify-between gap-3">
                <div className="flex items-baseline gap-1.5 min-w-0">
                    <span className="text-[28px] font-semibold tracking-tight text-[var(--text)] tabular-nums leading-none">
                        {value}
                    </span>
                    {unit && (
                        <span className="text-xs font-medium text-[var(--text-muted)] shrink-0">
                            {unit}
                        </span>
                    )}
                </div>

                {paths && (
                    <svg
                        viewBox="0 0 100 28"
                        className="w-20 h-7 shrink-0 overflow-visible"
                        aria-hidden="true"
                    >
                        <defs>
                            <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
                                <stop offset="0%" stopColor={sparkStroke} stopOpacity={0.28} />
                                <stop offset="100%" stopColor={sparkStroke} stopOpacity={0} />
                            </linearGradient>
                        </defs>
                        <path d={paths.area} fill={`url(#${gradId})`} />
                        <path
                            d={paths.line}
                            fill="none"
                            stroke={sparkStroke}
                            strokeWidth={1.75}
                            strokeLinecap="round"
                            strokeLinejoin="round"
                        />
                    </svg>
                )}
            </div>

            {delta && (
                <div className="mt-3.5 flex items-center gap-2 min-w-0">
                    <span
                        className={`inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[11px] font-medium tabular-nums shrink-0 ${
                            delta.positive
                                ? 'bg-[var(--tint-green)] text-[var(--tint-green-fg)]'
                                : 'bg-[var(--tint-red)] text-[var(--tint-red-fg)]'
                        }`}
                    >
                        {delta.positive ? (
                            <ArrowUpRight className="size-3 shrink-0" aria-hidden="true" />
                        ) : (
                            <ArrowDownRight className="size-3 shrink-0" aria-hidden="true" />
                        )}
                        {delta.value}
                    </span>
                    {delta.comparison && (
                        <span className="text-xs text-[var(--text-muted)] truncate">
                            {delta.comparison}
                        </span>
                    )}
                </div>
            )}
        </LinearCard>
    );
};
